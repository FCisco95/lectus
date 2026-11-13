//! Opt-in, authenticated loopback transcription. This service never captures
//! audio, inserts text, writes history, or makes outbound requests.
mod audio;
mod server;
mod token;

use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub port: u16,
    pub allowed_origins: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self { Self { enabled: false, port: 8756, allowed_origins: vec![] } }
}

#[derive(Clone, Debug, Serialize)]
pub struct CallRecord {
    pub timestamp_ms: i64,
    pub origin: Option<String>,
    pub duration_ms: Option<u64>,
    pub result_chars: Option<usize>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub running: bool,
    /// API inference only; uploads and the user's own dictation do not pulse
    /// the API indicator. The HTTP health response includes all engine work.
    pub busy: bool,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub error: Option<String>,
    pub calls: Vec<CallRecord>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Transcript {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
    pub transcribe_ms: u64,
    pub engine: String,
}

#[derive(Clone, Debug)]
pub struct ApiError {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(status: u16, code: &str, message: &str) -> Self {
        Self { status, code: code.into(), message: message.into() }
    }
    pub fn internal(message: &str) -> Self { Self::new(500, "internal_error", message) }
    pub fn busy() -> Self { Self::new(503, "busy", "Lectus is busy. Try again after the current dictation.") }
    pub fn locked() -> Self { Self::new(403, "locked", "Unlock Lectus before using the local API.") }
    pub fn unavailable() -> Self { Self::new(503, "engine_unavailable", "The local transcription engine is unavailable.") }
    pub fn too_large() -> Self { Self::new(413, "payload_too_large", "Audio must be at most 8 MiB and 120 seconds.") }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for ApiError {}

/// The app adapter must use local inference/cleanup only and re-check dictation
/// priority when claiming its worker. No queued API jobs are created here.
pub trait Backend: Send + Sync {
    fn allowed(&self) -> bool;
    fn busy(&self) -> bool;
    fn ready(&self) -> bool { true }
    fn transcribe(&self, samples: Vec<f32>, language: Option<String>, cleanup: bool)
        -> Result<Transcript, ApiError>;
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    settings: Settings,
    token: String,
}

struct State {
    stored: Stored,
    generation: u64,
    error: Option<String>,
    calls: VecDeque<CallRecord>,
}

struct Session {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl Session {
    fn stop(self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.thread.join();
    }
}

struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    change: Mutex<()>,
    session: Mutex<Option<Session>>,
    running: AtomicBool,
    busy: AtomicBool,
    processing: AtomicBool,
    clients: AtomicUsize,
    rate: Mutex<VecDeque<Instant>>,
    backend: Arc<dyn Backend>,
    notify: Arc<dyn Fn(Status) + Send + Sync>,
}

pub struct Service { inner: Arc<Inner> }

impl Service {
    /// Loading a fresh, disabled install generates no token and writes no file.
    /// api.json owns settings and token atomically; generic UI config cannot
    /// accidentally overwrite these credentials or lifecycle settings.
    pub fn new(path: PathBuf, backend: Arc<dyn Backend>, notify: Arc<dyn Fn(Status) + Send + Sync>)
        -> Result<Self, ApiError>
    {
        let stored: Stored = if path.exists() {
            let bytes = std::fs::read(&path).map_err(|_| ApiError::internal("Could not read API settings."))?;
            if bytes.len() > 32_768 { return Err(ApiError::internal("API settings are invalid.")); }
            serde_json::from_slice(&bytes).map_err(|_| ApiError::internal("API settings are invalid."))?
        } else { Stored::default() };
        validate_settings(&stored.settings)?;
        if (!stored.token.is_empty() && !token::valid(&stored.token))
            || (stored.settings.enabled && stored.token.is_empty())
        { return Err(ApiError::internal("API token is invalid. Restore API settings or generate a new token.")); }
        Ok(Self { inner: Arc::new(Inner {
            path, state: Mutex::new(State { stored, generation: 0, error: None, calls: VecDeque::new() }),
            change: Mutex::new(()), session: Mutex::new(None), running: AtomicBool::new(false),
            busy: AtomicBool::new(false), processing: AtomicBool::new(false), clients: AtomicUsize::new(0), rate: Mutex::new(VecDeque::new()),
            backend, notify,
        }) })
    }

    pub fn start(&self) -> Result<(), ApiError> {
        let settings = self.inner.state.lock().unwrap().stored.settings.clone();
        if !settings.enabled { return Ok(()); }
        self.configure(settings).map(|_| ())
    }

    pub fn status(&self) -> Status { self.inner.status() }

    pub fn token(&self) -> String { self.inner.state.lock().unwrap().stored.token.clone() }

    pub fn configure(&self, settings: Settings) -> Result<Status, ApiError> {
        let result = self.configure_inner(settings);
        if let Err(error) = &result { self.inner.state.lock().unwrap().error = Some(error.message.clone()); }
        self.inner.notify();
        result.map(|_| self.status())
    }

    fn configure_inner(&self, settings: Settings) -> Result<(), ApiError> {
        validate_settings(&settings)?;
        let _change = self.inner.change.lock().unwrap();
        let previous = self.inner.state.lock().unwrap().stored.clone();
        let reusable = self.inner.running.load(Ordering::SeqCst) && previous.settings.port == settings.port;
        // Bind before persisting. An occupied port leaves the working service
        // and its saved settings intact; the error is separately visible.
        let listener = if settings.enabled && !reusable {
            let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, settings.port))
                .map_err(|_| ApiError::new(409, "port_unavailable", "Could not bind the API port. Choose an unused port."))?;
            listener.set_nonblocking(true).map_err(|_| ApiError::internal("Could not configure the API listener."))?;
            Some(listener)
        } else { None };
        let token = if settings.enabled && previous.token.is_empty() { token::generate()? } else { previous.token };
        let stored = Stored { settings: settings.clone(), token };
        persist(&self.inner.path, &stored)?;
        {
            let mut state = self.inner.state.lock().unwrap();
            state.stored = stored;
            state.generation = state.generation.wrapping_add(1);
            state.error = None;
        }
        if !settings.enabled || listener.is_some() {
            if let Some(old) = self.inner.session.lock().unwrap().take() { old.stop(); }
            self.inner.running.store(false, Ordering::SeqCst);
        }
        if let Some(listener) = listener {
            self.inner.running.store(true, Ordering::SeqCst);
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = stop.clone();
            let inner = Arc::downgrade(&self.inner);
            let thread = std::thread::spawn(move || server::serve(listener, inner, thread_stop));
            *self.inner.session.lock().unwrap() = Some(Session { stop, thread });
        }
        Ok(())
    }

    pub fn regenerate_token(&self) -> Result<String, ApiError> {
        let result: Result<String, ApiError> = (|| {
            let _change = self.inner.change.lock().unwrap();
            let mut state = self.inner.state.lock().unwrap();
            let mut next = state.stored.clone();
            next.token = token::generate()?;
            persist(&self.inner.path, &next)?;
            let token = next.token.clone();
            state.stored = next;
            state.generation = state.generation.wrapping_add(1);
            state.error = None;
            Ok(token)
        })();
        if let Err(error) = &result { self.inner.state.lock().unwrap().error = Some(error.message.clone()); }
        self.inner.notify();
        result
    }

    /// Runtime shutdown preserves the saved opt-in setting for the next launch.
    pub fn stop(&self) {
        {
            let _change = self.inner.change.lock().unwrap();
            self.inner.state.lock().unwrap().generation += 1;
            self.inner.running.store(false, Ordering::SeqCst);
            if let Some(old) = self.inner.session.lock().unwrap().take() { old.stop(); }
        }
        self.inner.notify();
    }
}

impl Drop for Service { fn drop(&mut self) { self.stop(); } }

impl Inner {
    fn status(&self) -> Status {
        let state = self.state.lock().unwrap();
        Status {
            enabled: state.stored.settings.enabled, running: self.running.load(Ordering::SeqCst),
            busy: self.processing.load(Ordering::SeqCst), port: state.stored.settings.port,
            allowed_origins: state.stored.settings.allowed_origins.clone(), error: state.error.clone(),
            calls: state.calls.iter().cloned().collect(),
        }
    }
    fn notify(&self) { (self.notify)(self.status()); }
    fn current(&self, generation: u64) -> bool {
        let state = self.state.lock().unwrap();
        state.generation == generation && state.stored.settings.enabled && self.running.load(Ordering::SeqCst)
    }
    /// Admission is the lifecycle boundary: changes revoke uploads/decodes,
    /// while an already admitted inference is allowed to finish. Never hold
    /// this mutex across notification callbacks or the inference itself.
    fn admit(&self, generation: u64) -> bool {
        let state = self.state.lock().unwrap();
        if state.generation != generation || !state.stored.settings.enabled || !self.running.load(Ordering::SeqCst) {
            return false;
        }
        self.processing.store(true, Ordering::SeqCst);
        true
    }
    fn rate_allowed(&self) -> bool {
        let now = Instant::now();
        let mut calls = self.rate.lock().unwrap();
        while calls.front().is_some_and(|time| now.duration_since(*time) >= Duration::from_secs(60)) { calls.pop_front(); }
        if calls.len() >= 30 { return false; }
        calls.push_back(now);
        true
    }
    fn record(&self, origin: Option<String>, duration_ms: Option<u64>, result: &Result<Transcript, ApiError>) {
        {
            let mut state = self.state.lock().unwrap();
            state.calls.push_back(CallRecord {
                timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64,
                origin, duration_ms, result_chars: result.as_ref().ok().map(|t| t.text.chars().count()),
                error: result.as_ref().err().map(|e| e.code.clone()),
            });
            while state.calls.len() > 5 { state.calls.pop_front(); }
        }
        self.notify();
    }
}

fn validate_settings(settings: &Settings) -> Result<(), ApiError> {
    if settings.port == 0 { return Err(ApiError::new(400, "invalid_settings", "Choose a nonzero API port.")); }
    if settings.allowed_origins.len() > 32 { return Err(ApiError::new(400, "invalid_settings", "Allow at most 32 origins.")); }
    for origin in &settings.allowed_origins {
        let valid = origin.len() <= 256 && !origin.contains('*') && !origin.bytes().any(|b| b <= 32 || b == 127)
            && reqwest::Url::parse(origin).ok().is_some_and(|url|
            matches!(url.scheme(), "https" | "http") && url.username().is_empty() && url.password().is_none()
                && url.query().is_none() && url.fragment().is_none() && url.origin().ascii_serialization() == *origin);
        if !valid { return Err(ApiError::new(400, "invalid_settings", "Use exact HTTP(S) origins without paths, wildcards or trailing slashes.")); }
    }
    Ok(())
}

fn persist(path: &Path, stored: &Stored) -> Result<(), ApiError> {
    let write = || -> anyhow::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let tmp = path.with_extension("json.tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(stored)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(tmp, path)?;
        Ok(())
    };
    write().map_err(|_| ApiError::internal("Could not save API settings. The previous configuration is unchanged."))
}

#[cfg(test)]
mod tests;
