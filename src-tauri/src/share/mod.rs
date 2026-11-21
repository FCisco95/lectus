//! Lectus Share, milestone 1: the PC lends its mouse to the paired Mac.
//!
//! Design: `docs/plans/2026-11-20-lectus-share.md`. Off by default. It reuses the remote
//! dictation pairing: the PC in Send mode controls, the Mac in Receive mode is controlled. The
//! PC finds the Mac by mDNS and connects by itself whenever the Mac is reachable, so there is no
//! address to type. The Mac is to the left of the PC (fixed in M1). Keyboard forwarding is M2.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))] // accept() is the Mac side
pub mod link;
// Used by the Mac side; tested everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod pointer;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod keys;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;


use crate::remote_dictation::{transport::Pairing, Mode, RemoteState};
use anyhow::{Context, Result};
use mdns_sd::ServiceDaemon;
#[cfg(target_os = "windows")]
use mdns_sd::ServiceEvent;
#[cfg(target_os = "macos")]
use mdns_sd::ServiceInfo;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::net::TcpListener;
#[cfg(target_os = "windows")]
use std::net::{IpAddr, SocketAddr};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};

const SERVICE: &str = "_lectus-share._tcp.local.";

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    enabled: bool,
}

struct Running {
    daemon: ServiceDaemon,
    stop: Arc<AtomicBool>,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.daemon.shutdown();
    }
}

pub struct ShareState {
    settings: Mutex<Settings>,
    running: Mutex<Option<Running>>,
    status: Mutex<(String, bool)>,
}

#[derive(Clone, Serialize)]
pub struct View {
    pub enabled: bool,
    pub status: String,
    pub error: bool,
}

impl ShareState {
    pub fn new() -> Self {
        let loaded = std::fs::read(settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default();
        Self {
            settings: Mutex::new(loaded),
            running: Mutex::new(None),
            status: Mutex::new(("Off".into(), false)),
        }
    }

    fn view(&self) -> View {
        let (status, error) = self.status.lock().unwrap().clone();
        View {
            enabled: self.settings.lock().unwrap().enabled,
            status,
            error,
        }
    }
}

fn settings_path() -> PathBuf {
    crate::config::Config::app_support_dir().join("share.json")
}

pub(crate) fn status(app: &tauri::AppHandle, text: impl Into<String>, error: bool) {
    let text = text.into();
    log::info!("share: {text}");
    let state = app.state::<ShareState>();
    *state.status.lock().unwrap() = (text.clone(), error);
    let _ = app.emit("share-status", serde_json::json!({"status": text, "error": error}));
}

/// Where dictation goes while Share runs on the PC: `Some(true)` the Mac (the mouse is there),
/// `Some(false)` this PC (the mouse is here, or the Mac is out of reach). `None` when Share is
/// off, so the remote dictation setting decides alone. Read once, when a dictation starts.
pub fn follow() -> Option<bool> {
    #[cfg(target_os = "windows")]
    return windows::follow();
    #[cfg(not(target_os = "windows"))]
    None
}

pub fn stop(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<ShareState>() else {
        return;
    };
    let old = state.running.lock().unwrap().take();
    if old.is_some() {
        drop(old);
        #[cfg(target_os = "windows")]
        windows::uninstall();
        #[cfg(target_os = "macos")]
        macos::release_all();
    }
}

/// Stops Share and starts it again from the saved toggle and the current remote route.
pub fn restart(app: &tauri::AppHandle) {
    stop(app);
    let Some(state) = app.try_state::<ShareState>() else {
        return;
    };
    if !state.settings.lock().unwrap().enabled {
        status(app, "Off", false);
        return;
    }
    let route = app.state::<RemoteState>().share_route();
    let result = match route {
        None => {
            status(
                app,
                "Pair the two computers first: Send on this PC, Receive on the Mac.",
                true,
            );
            return;
        }
        Some((Mode::Send, pairing)) => start_controller(app, pairing),
        Some((Mode::Receive, pairing)) => start_receiver(app, pairing),
        Some((Mode::Local, _)) => return,
    };
    match result {
        Ok(running) => *state.running.lock().unwrap() = Some(running),
        Err(e) => status(app, format!("{e:#}"), true),
    }
}

fn new_daemon() -> Result<ServiceDaemon> {
    let daemon = ServiceDaemon::new().context("Local-network discovery could not start")?;
    daemon.disable_interface(mdns_sd::IfKind::IPv6)?;
    daemon.set_ip_check_interval(5)?;
    Ok(daemon)
}

/// PC side: find the paired Mac, connect, reconnect every second while it is out of reach.
#[cfg(target_os = "windows")]
fn start_controller(app: &tauri::AppHandle, pairing: Pairing) -> Result<Running> {
    let daemon = new_daemon()?;
    let stop = Arc::new(AtomicBool::new(false));
    let running = Running {
        daemon: daemon.clone(),
        stop: stop.clone(),
    };
    let events = daemon.browse(SERVICE)?;
    let addresses: Arc<Mutex<Vec<SocketAddr>>> = Arc::default();
    {
        let addresses = addresses.clone();
        let stop = stop.clone();
        let id = pairing.id.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let Ok(event) = events.recv_timeout(Duration::from_millis(250)) else {
                    continue;
                };
                match event {
                    ServiceEvent::ServiceResolved(info)
                        if info.get_property_val_str("peer") == Some(id.as_str())
                            && info.get_property_val_str("version") == Some("1") =>
                    {
                        let mut found: Vec<_> = info
                            .get_addresses_v4()
                            .into_iter()
                            .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
                            .map(|ip| SocketAddr::new(IpAddr::V4(ip), info.get_port()))
                            .collect();
                        found.sort();
                        found.truncate(6);
                        *addresses.lock().unwrap() = found;
                    }
                    ServiceEvent::ServiceRemoved(..) => addresses.lock().unwrap().clear(),
                    _ => {}
                }
            }
        });
    }
    windows::install()?;
    let handle = app.clone();
    std::thread::spawn(move || {
        status(&handle, "Looking for the paired Mac…", false);
        while !stop.load(Ordering::SeqCst) {
            let list = addresses.lock().unwrap().clone();
            let mut connected = None;
            for address in &list {
                match link::connect(*address, &pairing) {
                    Ok(session) => {
                        connected = Some(session);
                        break;
                    }
                    Err(e) => log::debug!("share: connect {address} failed: {e:#}"),
                }
            }
            if let Some(session) = connected {
                status(&handle, "Connected. Push the cursor past the left edge to use the Mac.", false);
                let reason = windows::run_session(session, &stop);
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                status(&handle, format!("Mac out of reach ({reason}). Reconnecting…"), false);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    Ok(running)
}

#[cfg(not(target_os = "windows"))]
fn start_controller(_: &tauri::AppHandle, _: Pairing) -> Result<Running> {
    anyhow::bail!("In this test version only a Windows PC can lend its mouse")
}

/// Mac side: listen, advertise, and accept the paired PC. The newest connection wins.
#[cfg(target_os = "macos")]
fn start_receiver(app: &tauri::AppHandle, pairing: Pairing) -> Result<Running> {
    if !crate::hook::accessibility_trusted() {
        anyhow::bail!("Allow Lectus in System Settings > Privacy & Security > Accessibility, then turn Share on again");
    }
    let daemon = new_daemon()?;
    let stop = Arc::new(AtomicBool::new(false));
    let running = Running {
        daemon: daemon.clone(),
        stop: stop.clone(),
    };
    let listener = TcpListener::bind("0.0.0.0:0").context("Cannot start Share")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let info = ServiceInfo::new(
        SERVICE,
        &format!("LectusShare-{}", pairing.id),
        &format!("lectus-share-{}.local.", pairing.id),
        "",
        port,
        &[("peer", pairing.id.as_str()), ("version", "1")][..],
    )?
    .enable_addr_auto();
    daemon.register(info)?;
    let handle = app.clone();
    std::thread::spawn(move || {
        status(&handle, "Waiting for the paired PC…", false);
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, from)) => {
                    let pairing = pairing.clone();
                    let handle = handle.clone();
                    let stop = stop.clone();
                    std::thread::spawn(move || {
                        let Ok(session) = link::accept(stream, &pairing) else {
                            log::debug!("share: rejected a connection from {from}");
                            return;
                        };
                        status(&handle, "PC connected. Its mouse can move here.", false);
                        let reason = macos::run_session(session, &stop);
                        if !stop.load(Ordering::SeqCst) {
                            status(&handle, format!("PC disconnected ({reason}). Waiting…"), false);
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(40))
                }
                Err(_) => break,
            }
        }
    });
    Ok(running)
}

#[cfg(not(target_os = "macos"))]
fn start_receiver(_: &tauri::AppHandle, _: Pairing) -> Result<Running> {
    anyhow::bail!("In this test version only a Mac can receive the mouse")
}

#[tauri::command]
pub fn share_get(state: tauri::State<ShareState>) -> View {
    state.view()
}

#[tauri::command]
pub async fn share_set(app: tauri::AppHandle, enabled: bool) -> Result<View, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<View> {
        let state = app.state::<ShareState>();
        let settings = Settings { enabled };
        let path = settings_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&settings)?)?;
        *state.settings.lock().unwrap() = settings;
        restart(&app);
        Ok(state.view())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
