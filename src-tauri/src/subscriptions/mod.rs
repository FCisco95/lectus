//! Desktop-only subscription sessions. No imported/global credentials or API-key fallback.
mod chatgpt;
mod claude;
mod claude_policy;
mod protocol;
mod store;

use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub struct ConnectionStatus {
    pub provider: String,
    pub state: String,
    pub account_label: Option<String>,
    pub message: String,
}
#[derive(Clone, Serialize)]
pub struct SubscriptionModel {
    pub id: String,
    pub name: String,
}
#[derive(Serialize)]
pub struct TestResult {
    pub text: String,
    pub elapsed_ms: u64,
}

struct Runtime {
    statuses: [Option<ConnectionStatus>; 2],
}
static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
static EPOCHS: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
static INFERENCE_EPOCH: AtomicU64 = AtomicU64::new(0);
struct AccountAuthority(AtomicBool);
impl AccountAuthority {
    const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    fn deny(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn check(&self) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            bail!("Subscription disconnected. Complete a new sign-in before making requests.");
        }
        Ok(())
    }
    fn reconnect(&self, activate: impl FnOnce() -> Result<()>) -> Result<()> {
        activate()?;
        self.0.store(false, Ordering::SeqCst);
        Ok(())
    }
}
static AUTHORITY: [AccountAuthority; 2] = [AccountAuthority::new(), AccountAuthority::new()];

fn denied_marker(slot: usize) -> Result<PathBuf> {
    Ok(root()?.join(if slot == 0 {
        "chatgpt-disconnected"
    } else {
        "claude-disconnected"
    }))
}
fn recover_authority(slot: usize) -> Result<()> {
    recover_under_lock(runtime(), &AUTHORITY[slot], || {
        denied_marker(slot)?
            .try_exists()
            .map_err(|_| anyhow!("Cannot verify subscription disconnect state. Sign in again."))
    })
}
fn recover_under_lock<T>(
    lock: &Mutex<T>,
    authority: &AccountAuthority,
    observe_marker: impl FnOnce() -> Result<bool>,
) -> Result<()> {
    // Serialize marker observation/denial with marker removal and activation.
    let _guard = lock
        .lock()
        .map_err(|_| anyhow!("Subscription state is unavailable."))?;
    if observe_marker()? {
        authority.deny();
    }
    authority.check()
}
fn inspect_connection(provider: &str, lease: Lease) -> Result<ConnectionStatus> {
    if let Err(error) = recover_authority(lease.slot) {
        return Ok(status(provider, "disconnected", None, &error.to_string()));
    }
    match provider {
        "chatgpt" => chatgpt::connection(lease),
        _ => claude::connection(lease),
    }
}
fn persist_disconnect(lease: Lease) -> Result<()> {
    lease.commit(|| std::fs::write(denied_marker(lease.slot)?, b"disconnected").map_err(|_| anyhow!("Disconnected for this session, but cannot save disconnect state. Retry disconnect before restarting Lectus.")))
}
fn activate_connection(lease: Lease) -> Result<()> {
    lease.commit(|| {
        AUTHORITY[lease.slot].reconnect(|| match std::fs::remove_file(denied_marker(lease.slot)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => bail!("Cannot activate subscription after disconnect. Retry sign-in."),
        })
    })
}

/// Call before saving changed cloud consent/routing/profile authority. This
/// invalidates preflight work and output without cancelling a browser sign-in.
pub fn invalidate_inference() {
    INFERENCE_EPOCH.fetch_add(1, Ordering::SeqCst);
}
pub fn inference_epoch() -> u64 {
    INFERENCE_EPOCH.load(Ordering::SeqCst)
}
fn runtime() -> &'static Mutex<Runtime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(Runtime {
            statuses: [None, None],
        })
    })
}
fn index(provider: &str) -> Result<usize> {
    match provider {
        "chatgpt" => Ok(0),
        "claude" => Ok(1),
        _ => bail!("Choose ChatGPT or Claude."),
    }
}
pub(super) fn root() -> Result<PathBuf> {
    let config = crate::config::Config::config_file();
    let path = config
        .parent()
        .ok_or_else(|| anyhow!("Lectus configuration directory is unavailable."))?
        .join("subscriptions");
    std::fs::create_dir_all(&path)
        .map_err(|_| anyhow!("Cannot create Lectus subscription storage."))?;
    Ok(path)
}
#[derive(Clone, Copy)]
pub(super) struct Lease {
    slot: usize,
    epoch: u64,
    inference_epoch: Option<u64>,
    authority: Option<&'static AccountAuthority>,
}
impl Lease {
    fn with_authority(mut self, authority: &'static AccountAuthority) -> Result<Self> {
        self.authority = Some(authority);
        self.check()?;
        Ok(self)
    }
    pub(super) fn session_only(mut self) -> Self {
        self.inference_epoch = None;
        self.authority = None;
        self
    }
    fn current(provider: &str) -> Result<Self> {
        let slot = index(provider)?;
        Ok(Self {
            slot,
            epoch: EPOCHS[slot].load(Ordering::SeqCst),
            inference_epoch: None,
            authority: None,
        })
    }
    pub(super) fn check(self) -> Result<()> {
        if EPOCHS[self.slot].load(Ordering::SeqCst) != self.epoch {
            bail!("Subscription operation was cancelled or disconnected.");
        }
        if self
            .inference_epoch
            .is_some_and(|epoch| INFERENCE_EPOCH.load(Ordering::SeqCst) != epoch)
        {
            bail!(
                "Text processing permission or selection changed. Subscription request cancelled."
            );
        }
        if let Some(authority) = self.authority {
            authority.check()?;
        }
        Ok(())
    }
    /// Final authority boundary after potentially slow auth/catalog preflight.
    pub(super) fn dispatch_text<T>(self, dispatch: impl FnOnce() -> Result<T>) -> Result<T> {
        self.check()?;
        dispatch()
    }
    fn release_text(self, text: String) -> Result<String> {
        self.check()?;
        Ok(text)
    }
    // Credential commit and epoch invalidation share this mutex: a stale refresh/login
    // can never commit between the final check and a disconnect.
    pub(super) fn commit<T>(self, action: impl FnOnce() -> Result<T>) -> Result<T> {
        let _guard = runtime()
            .lock()
            .map_err(|_| anyhow!("Subscription state is unavailable."))?;
        self.check()?;
        action()
    }
}
pub(super) fn status(
    provider: &str,
    state: &str,
    label: Option<String>,
    message: &str,
) -> ConnectionStatus {
    ConnectionStatus {
        provider: provider.into(),
        state: state.into(),
        account_label: label,
        message: message.into(),
    }
}
fn change(provider: &str, state: &str, message: &str) -> Result<Lease> {
    transition(provider, state, message, false)
}
fn transition(provider: &str, state: &str, message: &str, deny: bool) -> Result<Lease> {
    let slot = index(provider)?;
    let mut rt = runtime()
        .lock()
        .map_err(|_| anyhow!("Subscription state is unavailable."))?;
    if deny {
        AUTHORITY[slot].deny();
    }
    let epoch = EPOCHS[slot].fetch_add(1, Ordering::SeqCst) + 1;
    rt.statuses[slot] = Some(status(provider, state, None, message));
    Ok(Lease {
        slot,
        epoch,
        inference_epoch: None,
        authority: None,
    })
}
fn finish(lease: Lease, next: ConnectionStatus) {
    if let Ok(mut rt) = runtime().lock() {
        if lease.check().is_ok() {
            rt.statuses[lease.slot] = Some(next);
        }
    }
}
fn error_status(provider: &str, error: anyhow::Error) -> ConnectionStatus {
    status(provider, "error", None, &error.to_string())
}
pub(super) fn client(timeout: Duration) -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| anyhow!("Cannot initialize subscription transport."))
}
pub(super) fn read_bounded(mut reader: impl std::io::Read, limit: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    (&mut reader)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("Subscription response interrupted or timed out."))?;
    if bytes.len() > limit {
        bail!("Subscription response exceeded the size limit.");
    }
    Ok(bytes)
}
pub(super) fn json_response(response: reqwest::blocking::Response) -> Result<serde_json::Value> {
    let code = response.status();
    if !code.is_success() {
        // Do not echo response bodies, request URLs, CLI output, or transport errors.
        bail!(match code.as_u16() {
            401 | 403 =>
                "Subscription authorization was rejected. Sign in again and check plan access.",
            429 => "Subscription usage limit reached. Check usage with the provider and try later.",
            _ => "Subscription service is unavailable. Try again later.",
        });
    }
    serde_json::from_slice(&read_bounded(response, 1024 * 1024)?)
        .map_err(|_| anyhow!("Subscription service returned invalid data."))
}

/// Synchronous worker API; callers must dispatch off the UI thread.
pub fn generate(provider: &str, text: &str, instructions: &str, model: &str) -> Result<String> {
    generate_guarded(provider, text, instructions, model, inference_epoch())
}

/// The caller captures expected_epoch while holding its configuration lock and
/// passes that same epoch with the authorized transcript snapshot.
pub fn generate_guarded(
    provider: &str,
    text: &str,
    instructions: &str,
    model: &str,
    expected_epoch: u64,
) -> Result<String> {
    let mut lease = Lease::current(provider)?;
    lease.inference_epoch = Some(expected_epoch);
    lease.check()?;
    recover_authority(lease.slot)?;
    lease = lease.with_authority(&AUTHORITY[lease.slot])?;
    if text.len() > 64 * 1024 || instructions.len() > 16 * 1024 || model.len() > 200 {
        bail!("Subscription request exceeded the size limit.");
    }
    {
        let rt = runtime()
            .lock()
            .map_err(|_| anyhow!("Subscription state is unavailable."))?;
        if rt.statuses[lease.slot]
            .as_ref()
            .is_some_and(|s| s.state == "connecting")
        {
            bail!("Wait for subscription sign-in to finish.");
        }
    }
    let result = match provider {
        "chatgpt" => chatgpt::generate(lease, text, instructions, model),
        "claude" => claude::generate(lease, text, instructions, model),
        _ => unreachable!(),
    }?;
    lease.release_text(result)
}

#[tauri::command]
pub async fn get_ai_connections(refresh: Option<bool>) -> Vec<ConnectionStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        ["chatgpt", "claude"]
            .into_iter()
            .map(|provider| {
                let lease = Lease::current(provider).expect("fixed provider");
                if let Ok(rt) = runtime().lock() {
                    if let Some(current) = &rt.statuses[lease.slot] {
                        // Connecting/errors reflect the current attempt. Connected state is
                        // rechecked by every model-list/inference operation, never inferred.
                        if current.state == "connecting" || !refresh.unwrap_or(false) {
                            return current.clone();
                        }
                    }
                }
                let next = inspect_connection(provider, lease)
                    .unwrap_or_else(|e| error_status(provider, e));
                finish(lease, next.clone());
                next
            })
            .collect()
    })
    .await
    .unwrap_or_else(|_| {
        ["chatgpt", "claude"]
            .into_iter()
            .map(|p| status(p, "error", None, "Cannot check subscription state."))
            .collect()
    })
}

#[tauri::command]
pub async fn start_ai_login(app: tauri::AppHandle, provider: String) -> Result<(), String> {
    let lease = change(
        &provider,
        "connecting",
        "Complete sign-in in your system browser.",
    )
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let result = match provider.as_str() {
            "chatgpt" => chatgpt::login(lease, &app),
            "claude" => claude::login(lease),
            _ => unreachable!(),
        };
        let result = result.and_then(|next| {
            activate_connection(lease)?;
            Ok(next)
        });
        let next = result.unwrap_or_else(|e| error_status(&provider, e));
        finish(lease, next);
    });
    Ok(())
}
#[tauri::command]
pub async fn cancel_ai_login(provider: String) -> Result<(), String> {
    let lease =
        change(&provider, "disconnected", "Sign-in cancelled.").map_err(|e| e.to_string())?;
    // Attempts use isolated CLI dirs and cannot become active after cancellation.
    tauri::async_runtime::spawn_blocking(move || {
        let next =
            inspect_connection(&provider, lease).unwrap_or_else(|e| error_status(&provider, e));
        finish(lease, next);
    });
    Ok(())
}
#[tauri::command]
pub async fn disconnect_ai(provider: String) -> Result<(), String> {
    let lease = transition(&provider, "disconnected", "Disconnected from Lectus.", true)
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        // Denial is already authoritative before this worker can be delayed or
        // fail. Keep a restart marker before attempting credential deletion.
        let persisted = persist_disconnect(lease);
        let result = match provider.as_str() {
            "chatgpt" => chatgpt::disconnect(lease),
            _ => claude::disconnect(lease),
        };
        let result = result.and_then(|message| {
            persisted?;
            Ok(message)
        });
        match result {
            Ok(message) => {
                finish(lease, status(&provider, "disconnected", None, &message));
                Ok(())
            }
            Err(e) => {
                let message = e.to_string();
                finish(lease, error_status(&provider, e));
                Err(message)
            }
        }
    })
    .await
    .map_err(|_| "Cannot disconnect subscription.".to_string())?
}
#[tauri::command]
pub async fn list_subscription_models(provider: String) -> Result<Vec<SubscriptionModel>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut lease = Lease::current(&provider)?;
        recover_authority(lease.slot)?;
        lease = lease.with_authority(&AUTHORITY[lease.slot])?;
        let models = match provider.as_str() {
            "chatgpt" => chatgpt::models(lease),
            _ => claude::models(lease),
        }?;
        lease.check()?;
        Ok(models)
    })
    .await
    .map_err(|_| "Cannot list subscription models.".to_string())?
    .map_err(|e: anyhow::Error| e.to_string())
}
#[tauri::command]
pub async fn test_subscription(provider: String, model: String) -> Result<TestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let start = Instant::now();
        let text = generate(
            &provider,
            "Hello from Lectus.",
            "Return exactly the user's text. This is a synthetic connection test.",
            &model,
        )?;
        Ok(TestResult {
            text,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    })
    .await
    .map_err(|_| "Subscription test failed.".to_string())?
    .map_err(|e: anyhow::Error| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_lease_cannot_commit_or_release_output() {
        let old = change("chatgpt", "connecting", "fixture").unwrap();
        change("chatgpt", "disconnected", "fixture").unwrap();
        let mut committed = false;
        assert!(old
            .commit(|| {
                committed = true;
                Ok(())
            })
            .is_err());
        assert!(!committed);
        assert!(old.check().is_err());
    }
    #[test]
    fn bounded_response_rejects_overflow_without_echoing_body() {
        assert_eq!(read_bounded(&b"ok"[..], 2).unwrap(), b"ok");
        let error = read_bounded(&b"secret"[..], 2).unwrap_err().to_string();
        assert!(!error.contains("secret"));
    }
    #[test]
    fn consent_change_cancels_old_inference_without_cancelling_login() {
        let login = Lease::current("claude").unwrap();
        let old_epoch = inference_epoch();
        let mut inference = login;
        inference.inference_epoch = Some(old_epoch);
        invalidate_inference();
        assert!(login.check().is_ok());
        assert!(inference.check().is_err());
        // This must fail before touching any real account/CLI/network.
        let error = generate_guarded("claude", "private fixture", "clean", "", old_epoch)
            .unwrap_err()
            .to_string();
        assert!(error.contains("permission or selection changed"));
        assert!(!error.contains("private fixture"));
    }
    #[test]
    fn delayed_preflight_revocation_prevents_text_dispatch_and_old_output() {
        use std::sync::{mpsc::sync_channel, Arc};
        let login = Lease::current("claude").unwrap();
        let mut inference = login;
        inference.inference_epoch = Some(inference_epoch());
        let dispatches = Arc::new(AtomicU64::new(0));
        let (entered_preflight, preflight_entered) = sync_channel(0);
        let (release_preflight, preflight_released) = sync_channel(0);
        let observed_dispatches = dispatches.clone();
        let worker = std::thread::spawn(move || {
            // Stand-in for an OAuth refresh/catalog fetch: the production
            // transport's authority boundary runs only after this returns.
            entered_preflight.send(()).unwrap();
            preflight_released
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            inference.dispatch_text(|| {
                observed_dispatches.fetch_add(1, Ordering::SeqCst);
                Ok("private fixture".to_string())
            })
        });
        preflight_entered
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        invalidate_inference();
        release_preflight.send(()).unwrap();
        let error = worker.join().unwrap().unwrap_err().to_string();
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);
        assert!(error.contains("permission or selection changed"));
        assert!(
            login.check().is_ok(),
            "privacy changes must not cancel sign-in"
        );
        assert!(inference.release_text("old private output".into()).is_err());
    }
    #[test]
    fn disconnect_denies_fresh_leases_before_deferred_deletion_and_until_successful_reconnect() {
        use std::sync::{mpsc::sync_channel, Arc};
        // The same authority primitive/lease attachment used by production.
        // Fixture authority avoids touching real account state or credential files.
        let authority = Box::leak(Box::new(AccountAuthority::new()));
        let authority: &'static AccountAuthority = authority;
        let credentials_readable = Arc::new(AtomicBool::new(true));
        let (cleanup_entered, entered) = sync_channel(0);
        let (release_cleanup, released) = sync_channel(0);
        authority.deny(); // synchronous disconnect transition, before worker scheduling
        let readable = credentials_readable.clone();
        let cleanup = std::thread::spawn(move || {
            cleanup_entered.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(readable.load(Ordering::SeqCst));
            Err::<(), _>(anyhow!("fixture credential deletion failed"))
        });
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let dispatches = AtomicU64::new(0);
        let fresh = || Lease::current("claude").unwrap().with_authority(authority);
        assert!(credentials_readable.load(Ordering::SeqCst));
        assert!(fresh()
            .and_then(|lease| lease.dispatch_text(|| {
                dispatches.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .is_err());
        release_cleanup.send(()).unwrap();
        assert!(cleanup.join().unwrap().is_err());
        assert!(
            fresh().is_err(),
            "deletion failure must not restore authority"
        );
        // A connection/status check is not a reconnect and cannot allow requests.
        let login_or_status = Lease::current("claude").unwrap();
        assert!(login_or_status.check().is_ok());
        assert!(fresh().is_err());
        assert!(authority
            .reconnect(|| bail!("fixture activation failed"))
            .is_err());
        assert!(fresh().is_err());
        authority.reconnect(|| Ok(())).unwrap();
        fresh()
            .unwrap()
            .dispatch_text(|| {
                dispatches.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .unwrap();
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn marker_recovery_cannot_override_a_successful_reconnect() {
        use std::sync::{mpsc::sync_channel, Arc};
        let lock = Arc::new(Mutex::new(()));
        let authority = Arc::new(AccountAuthority::new());
        let marker_present = Arc::new(AtomicBool::new(true));
        let (observed, observation) = sync_channel(0);
        let (resume, resumed) = sync_channel(0);
        let recovery_lock = lock.clone();
        let recovery_authority = authority.clone();
        let recovery_marker = marker_present.clone();
        let recovery = std::thread::spawn(move || {
            recover_under_lock(&recovery_lock, &recovery_authority, || {
                let old_marker = recovery_marker.load(Ordering::SeqCst);
                observed.send(()).unwrap();
                resumed.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(old_marker)
            })
        });
        observation.recv_timeout(Duration::from_secs(5)).unwrap();
        // Reconnect uses this same mutex in production Lease::commit. An old
        // recovery must hold it across observation and denial, not just denial.
        assert!(lock.try_lock().is_err());
        let reconnect_lock = lock.clone();
        let reconnect_authority = authority.clone();
        let reconnect_marker = marker_present.clone();
        let reconnect = std::thread::spawn(move || {
            let _guard = reconnect_lock.lock().unwrap();
            reconnect_authority.reconnect(|| {
                reconnect_marker.store(false, Ordering::SeqCst);
                Ok(())
            })
        });
        resume.send(()).unwrap();
        assert!(recovery.join().unwrap().is_err());
        reconnect.join().unwrap().unwrap();
        assert!(authority.check().is_ok());
        assert!(recover_under_lock(&lock, &authority, || Ok(
            marker_present.load(Ordering::SeqCst)
        ))
        .is_ok());
    }
}
