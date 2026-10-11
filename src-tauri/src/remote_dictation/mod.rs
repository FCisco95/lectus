//! Opt-in paired text delivery. This is separate from the loopback transcription API.
pub mod transport;

use anyhow::{anyhow, bail, Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, TcpListener},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use transport::Pairing;

const SERVICE: &str = "_lectus-text._tcp.local.";

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Local,
    Send,
    Receive,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    mode: Mode,
    receiver_code: String,
    peer_code: String,
}

#[derive(Clone, Serialize)]
pub struct View {
    pub mode: Mode,
    pub receiver_code: String,
    pub peer_code: String,
    pub status: String,
    pub error: bool,
}

struct Session {
    daemon: ServiceDaemon,
    stop: Arc<AtomicBool>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.daemon.shutdown();
    }
}

type Peers = HashMap<String, Vec<SocketAddr>>;
pub struct RemoteState {
    settings: Mutex<Settings>,
    session: Mutex<Option<Session>>,
    peers: Arc<Mutex<Peers>>,
    status: Mutex<(String, bool)>,
    /// Serializes saves; never held by the main-queue injection.
    change: Mutex<()>,
    delivery: Mutex<()>,
}

impl RemoteState {
    pub fn new() -> Self {
        let loaded = std::fs::read(settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default();
        Self {
            settings: Mutex::new(loaded),
            session: Mutex::new(None),
            peers: Arc::new(Mutex::new(HashMap::new())),
            status: Mutex::new(("This computer".into(), false)),
            change: Mutex::new(()),
            delivery: Mutex::new(()),
        }
    }

    pub fn mode(&self) -> Mode {
        self.settings.lock().unwrap().mode
    }

    pub fn view(&self) -> View {
        let s = self.settings.lock().unwrap().clone();
        let (status, error) = self.status.lock().unwrap().clone();
        View {
            mode: s.mode,
            receiver_code: s.receiver_code,
            peer_code: s.peer_code,
            status,
            error,
        }
    }

    pub fn destination(&self) -> Result<Option<String>> {
        let s = self.settings.lock().unwrap();
        match s.mode {
            Mode::Local => Ok(None),
            Mode::Send => {
                Pairing::parse(&s.peer_code)?;
                Ok(Some(s.peer_code.clone()))
            }
            Mode::Receive => bail!("Receive mode uses the microphone on your other computer"),
        }
    }

    fn message(&self, app: &tauri::AppHandle, text: impl Into<String>, error: bool) {
        *self.status.lock().unwrap() = (text.into(), error);
        // Status events deliberately never include pairing secrets.
        let (status, error) = self.status.lock().unwrap().clone();
        let _ = app.emit(
            "remote-delivery-status",
            serde_json::json!({"status": status, "error": error}),
        );
    }

    fn start(&self, app: &tauri::AppHandle) -> Result<()> {
        self.session.lock().unwrap().take();
        self.peers.lock().unwrap().clear();
        let settings = self.settings.lock().unwrap().clone();
        if settings.mode == Mode::Local {
            self.message(app, "Dictating on this computer", false);
            return Ok(());
        }
        let pairing = Pairing::parse(if settings.mode == Mode::Receive {
            &settings.receiver_code
        } else {
            &settings.peer_code
        })?;
        let daemon = ServiceDaemon::new().context("Local-network discovery could not start")?;
        let stop = Arc::new(AtomicBool::new(false));
        let session = Session {
            daemon: daemon.clone(),
            stop: stop.clone(),
        };
        // Construct the owner before fallible configuration, so errors stop the daemon.
        daemon.disable_interface(mdns_sd::IfKind::IPv6)?;
        daemon.set_ip_check_interval(5)?;
        if settings.mode == Mode::Receive {
            let listener = TcpListener::bind("0.0.0.0:0").context("Cannot start text receiver")?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            let info = ServiceInfo::new(
                SERVICE,
                &format!("Lectus-{}", pairing.id),
                &format!("lectus-{}.local.", pairing.id),
                "",
                port,
                &[("peer", pairing.id.as_str()), ("version", "1")][..],
            )?
            .enable_addr_auto();
            daemon.register(info)?;
            let handle = app.clone();
            let active = Arc::new(AtomicUsize::new(0));
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if active.load(Ordering::SeqCst) >= 4 {
                                continue;
                            }
                            active.fetch_add(1, Ordering::SeqCst);
                            let active = active.clone();
                            let handle = handle.clone();
                            let pairing = pairing.clone();
                            let stopped = stop.clone();
                            std::thread::spawn(move || {
                                let result = transport::receive(stream, &pairing, |text| {
                                    if stopped.load(Ordering::SeqCst) {
                                        bail!("Receiver is paused");
                                    }
                                    receive_text(&handle, &pairing.code(), text, &stopped)
                                });
                                if result.is_err() {
                                    // Invalid/unpaired traffic does not surface notifications or reveal secrets.
                                    log::debug!("remote dictation: connection closed without a confirmed delivery");
                                }
                                active.fetch_sub(1, Ordering::SeqCst);
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(40))
                        }
                        Err(_) => break,
                    }
                }
            });
            self.message(
                app,
                "Ready to receive. Dictate on your paired computer.",
                false,
            );
        } else {
            let events = daemon.browse(SERVICE)?;
            let peers = self.peers.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let Ok(event) = events.recv_timeout(Duration::from_millis(250)) else {
                        continue;
                    };
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    match event {
                        ServiceEvent::ServiceResolved(info)
                            if info.get_property_val_str("peer") == Some(pairing.id.as_str())
                                && info.get_property_val_str("version") == Some("1") =>
                        {
                            let mut addresses: Vec<_> = info
                                .get_addresses_v4()
                                .into_iter()
                                .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
                                .map(|ip| SocketAddr::new(IpAddr::V4(ip), info.get_port()))
                                .collect();
                            addresses.sort();
                            addresses.truncate(6);
                            let mut known = peers.lock().unwrap();
                            if known.len() < 8 || known.contains_key(info.get_fullname()) {
                                known.insert(info.get_fullname().into(), addresses);
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, name) => {
                            peers.lock().unwrap().remove(&name);
                        }
                        _ => {}
                    }
                }
            });
            self.message(
                app,
                "Finding your paired computer. Use Test connection before dictating.",
                false,
            );
        }
        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }

    pub fn send(&self, app: &tauri::AppHandle, code: &str, text: Option<&str>) -> Result<()> {
        // A destination is captured when recording starts. Never silently retarget later.
        {
            let current = self.settings.lock().unwrap();
            if current.mode != Mode::Send || current.peer_code != code {
                bail!("Destination changed during dictation. Recover the text from History.");
            }
        }
        let pairing = Pairing::parse(code)?;
        self.message(
            app,
            if text.is_some() {
                "Sending dictation…"
            } else {
                "Checking connection…"
            },
            false,
        );
        let deadline = Instant::now() + Duration::from_secs(4);
        let addresses = loop {
            let list: Vec<_> = self
                .peers
                .lock()
                .unwrap()
                .values()
                .flatten()
                .copied()
                .collect();
            if !list.is_empty() {
                break list;
            }
            if Instant::now() >= deadline {
                bail!("Paired computer not found. Keep both on the same network, open Lectus in Receive mode there, and allow local-network access. Your text stays in History.");
            }
            std::thread::sleep(Duration::from_millis(80));
        };
        transport::send(&addresses, &pairing, text)?;
        self.message(
            app,
            if text.is_some() {
                "Dictation delivered"
            } else {
                "Connected. Ready to dictate."
            },
            false,
        );
        Ok(())
    }
}

fn settings_path() -> PathBuf {
    crate::config::Config::app_support_dir().join("remote-dictation.json")
}

fn persist(settings: &Settings) -> Result<()> {
    let path = settings_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("json.tmp");
    {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.sync_all()?;
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn receive_text(
    app: &tauri::AppHandle,
    code: &str,
    text: Option<&str>,
    stopped: &Arc<AtomicBool>,
) -> Result<()> {
    let state = app.state::<RemoteState>();
    let _delivery = state
        .delivery
        .try_lock()
        .map_err(|_| anyhow!("Another dictation is being inserted"))?;
    // Check roles and permissions on the UI thread immediately before the actual insertion.
    // A stalled main queue must not insert an old phrase long after the sender timed out.
    let deadline = Instant::now() + Duration::from_secs(3);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    let code = code.to_owned();
    let text = text.map(str::to_owned);
    let stopped = stopped.clone();
    app.run_on_main_thread(move || {
        let result = (|| -> Result<()> {
            if Instant::now() >= deadline || stopped.load(Ordering::SeqCst) {
                bail!("Delivery expired; nothing was inserted");
            }
            let state = handle.state::<RemoteState>();
            {
                let settings = state.settings.lock().unwrap();
                if settings.mode != Mode::Receive || settings.receiver_code != code {
                    bail!("Receiver is paused");
                }
            }
            if !handle
                .state::<crate::license::LicenseState>()
                .status(crate::now_millis())
                .allows_dictation()
            {
                bail!("Unlock Lectus on the receiving computer first");
            }
            if !crate::accessibility_status() {
                bail!("Allow Lectus in macOS Privacy & Security → Accessibility");
            }
            if let Some(text) = text {
                if *handle.state::<crate::AppState>().recording.lock().unwrap()
                    != crate::RecordingState::Idle
                {
                    bail!("Receiving computer is busy dictating");
                }
                // The receiver pastes locally; no cross-computer clipboard timing dependency.
                crate::injection::inject_remote_text(&text)?;
                state.message(&handle, "Dictation received", false);
            }
            Ok(())
        })();
        let _ = tx.send(result);
    })?;
    rx.recv_timeout(Duration::from_secs(4))
        .context("Receiving app did not respond in time")?
}

pub fn start_at_login(app: &tauri::AppHandle) {
    let state = app.state::<RemoteState>();
    if let Err(e) = state.start(app) {
        state.message(app, e.to_string(), true);
    }
}

pub fn report_error(app: &tauri::AppHandle, error: &str) {
    app.state::<RemoteState>().message(app, error, true);
    crate::show_app_window(app, false);
}

#[tauri::command]
pub fn remote_get_settings(state: tauri::State<RemoteState>) -> View {
    state.view()
}

#[tauri::command]
pub async fn remote_save_settings(
    app: tauri::AppHandle,
    mode: Mode,
    peer_code: String,
) -> Result<View, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<View> {
        let state = app.state::<RemoteState>();
        let _change = state.change.lock().unwrap();
        if *app.state::<crate::AppState>().recording.lock().unwrap() != crate::RecordingState::Idle
        {
            bail!("Finish the current dictation before changing destination");
        }
        let mut settings = state.settings.lock().unwrap().clone();
        settings.mode = mode;
        if mode == Mode::Receive && settings.receiver_code.is_empty() {
            settings.receiver_code = Pairing::generate()?.code();
        }
        if mode == Mode::Send {
            Pairing::parse(&peer_code)?;
            settings.peer_code = peer_code.trim().into();
        }
        // Persist before activating. A disk failure leaves the running mode unchanged.
        persist(&settings)?;
        *state.settings.lock().unwrap() = settings;
        let start = state.start(&app);
        // Receive mode must never record from the Mac mic when the shared key is pressed.
        let device = app
            .state::<crate::AppState>()
            .config
            .lock()
            .unwrap()
            .input_device
            .clone();
        crate::restart_preroll(&app, &device);
        if let Err(e) = start {
            state.message(&app, e.to_string(), true);
            return Err(e);
        }
        Ok(state.view())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_test_connection(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let state = app.state::<RemoteState>();
        let code = state
            .destination()?
            .ok_or_else(|| anyhow!("Choose Send to another computer first"))?;
        let result = state.send(&app, &code, None);
        if let Err(e) = &result {
            state.message(&app, e.to_string(), true);
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Explicit integration gate: advertises on the real LAN and delivers over a discovered address.
    #[test]
    #[ignore = "requires local-network multicast permission"]
    fn lan_discovery_delivers_to_the_paired_receiver() {
        let pair = Pairing::generate().unwrap();
        let receiver = Session {
            daemon: ServiceDaemon::new().unwrap(),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let sender = Session {
            daemon: ServiceDaemon::new().unwrap(),
            stop: Arc::new(AtomicBool::new(false)),
        };
        receiver
            .daemon
            .disable_interface(mdns_sd::IfKind::IPv6)
            .unwrap();
        sender
            .daemon
            .disable_interface(mdns_sd::IfKind::IPv6)
            .unwrap();
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let info = ServiceInfo::new(
            SERVICE,
            &format!("Lectus-{}", pair.id),
            &format!("lectus-{}.local.", pair.id),
            "",
            listener.local_addr().unwrap().port(),
            &[("peer", pair.id.as_str()), ("version", "1")][..],
        )
        .unwrap()
        .enable_addr_auto();
        let events = sender.daemon.browse(SERVICE).unwrap();
        receiver.daemon.register(info).unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        let addresses = loop {
            assert!(
                Instant::now() < deadline,
                "LAN discovery did not resolve the paired receiver"
            );
            if let Ok(ServiceEvent::ServiceResolved(info)) =
                events.recv_timeout(Duration::from_millis(200))
            {
                if info.get_property_val_str("peer") == Some(pair.id.as_str()) {
                    let addresses: Vec<_> = info
                        .get_addresses_v4()
                        .into_iter()
                        .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
                        .map(|ip| SocketAddr::new(IpAddr::V4(ip), info.get_port()))
                        .collect();
                    if !addresses.is_empty() {
                        break addresses;
                    }
                }
            }
        };
        let receiver_pair = pair.clone();
        let task = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(e) => return Err(anyhow!(e)),
                }
            };
            transport::receive(stream, &receiver_pair, |text| {
                assert_eq!(text, Some("LAN test: Olá João"));
                Ok(())
            })
        });
        let sent = transport::send(&addresses, &pair, Some("LAN test: Olá João"));
        let received = task.join().unwrap();
        assert!(
            sent.is_ok() && received.is_ok(),
            "sender: {sent:?}; receiver: {received:?}"
        );
    }
}
