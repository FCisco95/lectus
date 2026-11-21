//! Lectus Link: one keyboard, mouse and microphone across two computers.
//!
//! Design: `docs/plans/2026-11-20-lectus-share.md`. Off by default, its own Settings section.
//! The main computer (Windows for now) owns the keyboard, mouse and microphone; the receiver
//! (macOS for now) is controlled. Pairing happens once with a 6-digit code shown on the
//! receiver; afterwards the main computer finds the receiver by mDNS and connects by itself
//! whenever it is in reach. Dictation goes to the computer the mouse is on at key-down.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))] // accept() is the receiver side
pub mod link;
#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
mod pair;
// Used by the receiver; tested everywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod pointer;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod keys;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;

use crate::remote_dictation::transport::Pairing;
use anyhow::{anyhow, bail, Context, Result};
use link::Edge;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};

const SERVICE: &str = "_lectus-link._tcp.local.";
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const MAX_PAIRING_FAILURES: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Main,
    Receiver,
}

impl Role {
    fn supported(self) -> bool {
        match self {
            Role::Main => cfg!(target_os = "windows"),
            Role::Receiver => cfg!(target_os = "macos"),
        }
    }

    fn default_here() -> Role {
        if cfg!(target_os = "macos") {
            Role::Receiver
        } else {
            Role::Main
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Peer {
    id: String,
    name: String,
    /// The shared key, as a `LECTUS1.<receiver id>.<key>` code. Never shown or logged.
    code: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    enabled: bool,
    role: Option<Role>,
    /// Where the other computer stands, seen from the main computer.
    side: Edge,
    own_id: String,
    peer: Option<Peer>,
}

impl Settings {
    fn role(&self) -> Role {
        self.role.unwrap_or_else(Role::default_here)
    }
}

#[derive(Clone)]
struct Found {
    id: String,
    name: String,
    pairing: bool,
    addresses: Vec<SocketAddr>,
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

pub struct LinkState {
    settings: Mutex<Settings>,
    running: Mutex<Option<Running>>,
    status: Mutex<(String, bool)>,
    connected: AtomicBool,
    nearby: Arc<Mutex<HashMap<String, Found>>>,
    /// Receiver only: the code shown for pairing, and failed attempts against it.
    code: Mutex<Option<(String, u8)>>,
    /// Serializes setting changes.
    change: Mutex<()>,
}

#[derive(Clone, Serialize)]
pub struct RoleOption {
    role: Role,
    supported: bool,
}

#[derive(Clone, Serialize)]
pub struct NearbyView {
    id: String,
    name: String,
    pairing: bool,
}

#[derive(Clone, Serialize)]
pub struct View {
    enabled: bool,
    role: Role,
    side: Edge,
    roles: Vec<RoleOption>,
    this_name: String,
    peer_name: Option<String>,
    connected: bool,
    status: String,
    error: bool,
    pairing_code: Option<String>,
    nearby: Vec<NearbyView>,
}

fn settings_path() -> PathBuf {
    crate::config::Config::app_support_dir().join("link.json")
}

fn computer_name() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("scutil").args(["--get", "ComputerName"]).output() {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "This computer".into())
}

impl LinkState {
    pub fn new() -> Self {
        let mut loaded = std::fs::read(settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default();
        if loaded.own_id.is_empty() {
            if let Ok(p) = Pairing::generate() {
                loaded.own_id = p.id;
                let _ = persist(&loaded);
            }
        }
        Self {
            settings: Mutex::new(loaded),
            running: Mutex::new(None),
            status: Mutex::new(("Off".into(), false)),
            connected: AtomicBool::new(false),
            nearby: Arc::default(),
            code: Mutex::new(None),
            change: Mutex::new(()),
        }
    }

    fn view(&self) -> View {
        let s = self.settings.lock().unwrap().clone();
        let (status, error) = self.status.lock().unwrap().clone();
        let mut nearby: Vec<NearbyView> = self
            .nearby
            .lock()
            .unwrap()
            .values()
            .map(|f| NearbyView { id: f.id.clone(), name: f.name.clone(), pairing: f.pairing })
            .collect();
        nearby.sort_by(|a, b| a.name.cmp(&b.name));
        nearby.dedup_by(|a, b| a.id == b.id);
        View {
            enabled: s.enabled,
            role: s.role(),
            side: s.side,
            roles: [Role::Main, Role::Receiver]
                .into_iter()
                .map(|role| RoleOption { role, supported: role.supported() })
                .collect(),
            this_name: computer_name(),
            peer_name: s.peer.map(|p| p.name),
            connected: self.connected.load(Ordering::SeqCst),
            status,
            error,
            pairing_code: self.code.lock().unwrap().as_ref().map(|(c, _)| c.clone()),
            nearby,
        }
    }
}

fn persist(settings: &Settings) -> Result<()> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("link-changed", ());
}

pub(crate) fn status(app: &tauri::AppHandle, text: impl Into<String>, error: bool) {
    let text = text.into();
    log::info!("share: {text}");
    if let Some(state) = app.try_state::<LinkState>() {
        *state.status.lock().unwrap() = (text, error);
    }
    changed(app);
}

fn set_connected(app: &tauri::AppHandle, connected: bool) {
    if let Some(state) = app.try_state::<LinkState>() {
        state.connected.store(connected, Ordering::SeqCst);
    }
}

/// Where dictation goes while Link runs as the main computer: `Some(true)` the other computer
/// (the mouse is there), `Some(false)` this computer (the mouse is here, or the other one is out
/// of reach). `None` when Link is not running here as main. Read once, when a dictation starts.
pub fn follow() -> Option<bool> {
    #[cfg(target_os = "windows")]
    return windows::follow();
    #[cfg(not(target_os = "windows"))]
    None
}

/// Delivers a finished dictation to the other computer. Only after `follow()` said so.
pub fn send_text(text: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    return windows::send_text(text);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        bail!("Link cannot send from this computer")
    }
}

pub fn stop(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<LinkState>() else {
        return;
    };
    let old = state.running.lock().unwrap().take();
    state.nearby.lock().unwrap().clear();
    set_connected(app, false);
    if old.is_some() {
        drop(old);
        #[cfg(target_os = "windows")]
        windows::uninstall();
        #[cfg(target_os = "macos")]
        macos::release_all();
    }
}

/// Stops Link and starts it again from the saved settings.
pub fn restart(app: &tauri::AppHandle) {
    stop(app);
    let state = app.state::<LinkState>();
    let settings = state.settings.lock().unwrap().clone();
    if !settings.enabled {
        *state.code.lock().unwrap() = None;
        status(app, "Off", false);
        return;
    }
    let role = settings.role();
    if !role.supported() {
        status(app, "This role is not available on this computer yet", true);
        return;
    }
    let result = match role {
        Role::Main => start_main(app, &settings),
        Role::Receiver => start_receiver(app, &settings),
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

fn found_from(info: &mdns_sd::ResolvedService) -> Option<Found> {
    if info.get_property_val_str("v") != Some("2") || info.get_property_val_str("role") != Some("receiver") {
        return None;
    }
    let mut addresses: Vec<_> = info
        .get_addresses_v4()
        .into_iter()
        .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
        .map(|ip| SocketAddr::new(IpAddr::V4(ip), info.get_port()))
        .collect();
    addresses.sort();
    addresses.truncate(6);
    Some(Found {
        id: info.get_property_val_str("id")?.to_string(),
        name: info.get_property_val_str("name").unwrap_or("Computer").chars().take(64).collect(),
        pairing: info.get_property_val_str("pairing") == Some("1"),
        addresses,
    })
}

/// Main computer: list receivers nearby; when paired, connect to ours and keep reconnecting.
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
fn start_main(app: &tauri::AppHandle, settings: &Settings) -> Result<Running> {
    #[cfg(not(target_os = "windows"))]
    bail!("Only a Windows PC can be the main computer in this version");
    #[cfg(target_os = "windows")]
    {
        let daemon = new_daemon()?;
        let stop = Arc::new(AtomicBool::new(false));
        let running = Running { daemon: daemon.clone(), stop: stop.clone() };
        let events = daemon.browse(SERVICE)?;
        let nearby = app.state::<LinkState>().nearby.clone();
        {
            let stop = stop.clone();
            let handle = app.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let Ok(event) = events.recv_timeout(Duration::from_millis(250)) else {
                        continue;
                    };
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            if let Some(found) = found_from(&info) {
                                let mut map = nearby.lock().unwrap();
                                if map.len() < 16 || map.contains_key(info.get_fullname()) {
                                    map.insert(info.get_fullname().into(), found);
                                }
                                drop(map);
                                changed(&handle);
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, name) => {
                            nearby.lock().unwrap().remove(&name);
                            changed(&handle);
                        }
                        _ => {}
                    }
                }
            });
        }
        windows::set_side(settings.side);
        windows::install()?;
        let Some(peer) = settings.peer.clone() else {
            status(app, "Choose your other computer below and type the code it shows.", false);
            return Ok(running);
        };
        let pairing = Pairing::parse(&peer.code)?;
        let handle = app.clone();
        let nearby = app.state::<LinkState>().nearby.clone();
        std::thread::spawn(move || {
            status(&handle, format!("Looking for {}…", peer.name), false);
            while !stop.load(Ordering::SeqCst) {
                let list: Vec<SocketAddr> = nearby
                    .lock()
                    .unwrap()
                    .values()
                    .filter(|f| f.id == peer.id)
                    .flat_map(|f| f.addresses.clone())
                    .collect();
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
                    set_connected(&handle, true);
                    status(&handle, format!("Connected to {}. Move the mouse past the edge to use it.", peer.name), false);
                    let reason = windows::run_session(session, &stop);
                    set_connected(&handle, false);
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    status(&handle, format!("{} is out of reach ({reason}). Reconnecting…", peer.name), false);
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        Ok(running)
    }
}

/// Receiving computer: advertise, show a pairing code when unpaired, accept the main computer.
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn start_receiver(app: &tauri::AppHandle, settings: &Settings) -> Result<Running> {
    #[cfg(not(target_os = "macos"))]
    bail!("Only a Mac can be the receiving computer in this version");
    #[cfg(target_os = "macos")]
    {
        if !crate::hook::accessibility_trusted() {
            bail!("Allow Lectus in System Settings > Privacy & Security > Accessibility, then turn Link on again");
        }
        let state = app.state::<LinkState>();
        if settings.peer.is_none() && state.code.lock().unwrap().is_none() {
            *state.code.lock().unwrap() = Some((pair::new_code()?, 0));
        }
        let daemon = new_daemon()?;
        let stop = Arc::new(AtomicBool::new(false));
        let running = Running { daemon: daemon.clone(), stop: stop.clone() };
        let listener = std::net::TcpListener::bind("0.0.0.0:0").context("Cannot start Link")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        register(&daemon, settings, port, state.code.lock().unwrap().is_some())?;
        let handle = app.clone();
        status(
            app,
            match &settings.peer {
                Some(p) => format!("Waiting for {}…", p.name),
                None => "Type this code on your main computer.".into(),
            },
            false,
        );
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, from)) => {
                        let handle = handle.clone();
                        let stop = stop.clone();
                        let daemon = daemon.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = serve_receiver(&handle, stream, &stop, &daemon, port) {
                                log::debug!("share: connection from {from} ended: {e:#}");
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
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn register(daemon: &ServiceDaemon, settings: &Settings, port: u16, pairing: bool) -> Result<()> {
    let name = computer_name();
    let info = ServiceInfo::new(
        SERVICE,
        &format!("LectusLink-{}", settings.own_id),
        &format!("lectus-link-{}.local.", settings.own_id),
        "",
        port,
        &[
            ("v", "2"),
            ("role", "receiver"),
            ("id", settings.own_id.as_str()),
            ("name", name.as_str()),
            ("pairing", if pairing { "1" } else { "0" }),
        ][..],
    )?
    .enable_addr_auto();
    daemon.register(info)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn serve_receiver(
    app: &tauri::AppHandle,
    mut stream: std::net::TcpStream,
    stop: &Arc<AtomicBool>,
    daemon: &ServiceDaemon,
    port: u16,
) -> Result<()> {
    use std::io::Read;
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut magic = [0; 8];
    stream.peek(&mut magic)?;
    let state = app.state::<LinkState>();
    if &magic == pair::MAGIC {
        stream.read_exact(&mut magic)?;
        let Some((code, _)) = state.code.lock().unwrap().clone() else {
            bail!("not accepting pairing now");
        };
        let me = pair::Hello { id: state.settings.lock().unwrap().own_id.clone(), name: computer_name() };
        match pair::serve(&mut stream, &code, &me)? {
            Some((them, key)) => {
                let _change = state.change.lock().unwrap();
                let mut settings = state.settings.lock().unwrap().clone();
                settings.peer = Some(Peer {
                    id: them.id,
                    name: them.name.chars().take(64).collect(),
                    code: Pairing::from_key(settings.own_id.clone(), key).code(),
                });
                persist(&settings)?;
                *state.settings.lock().unwrap() = settings.clone();
                *state.code.lock().unwrap() = None;
                register(daemon, &settings, port, false)?;
                status(app, format!("Paired with {}. Waiting for it…", settings.peer.as_ref().map(|p| p.name.as_str()).unwrap_or("it")), false);
            }
            None => {
                let mut code = state.code.lock().unwrap();
                if let Some((_, failures)) = code.as_mut() {
                    *failures += 1;
                    if *failures >= MAX_PAIRING_FAILURES {
                        *code = Some((pair::new_code()?, 0));
                        drop(code);
                        status(app, "Too many wrong codes. Use the new code.", true);
                    }
                }
                changed(app);
            }
        }
        return Ok(());
    }
    let Some(peer) = state.settings.lock().unwrap().peer.clone() else {
        bail!("not paired");
    };
    let session = link::accept(stream, &Pairing::parse(&peer.code)?)?;
    set_connected(app, true);
    status(app, format!("{} is connected. Its mouse and keyboard can move here.", peer.name), false);
    let reason = macos::run_session(app, session, stop);
    set_connected(app, false);
    if !stop.load(Ordering::SeqCst) {
        status(app, format!("{} disconnected ({reason}). Waiting…", peer.name), false);
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn serve_receiver(_: &tauri::AppHandle, _: std::net::TcpStream, _: &Arc<AtomicBool>, _: &ServiceDaemon, _: u16) -> Result<()> {
    bail!("not a receiver")
}

fn update(app: &tauri::AppHandle, edit: impl FnOnce(&mut Settings) -> Result<()>) -> Result<View> {
    let state = app.state::<LinkState>();
    let _change = state.change.lock().unwrap();
    let mut settings = state.settings.lock().unwrap().clone();
    edit(&mut settings)?;
    persist(&settings)?;
    *state.settings.lock().unwrap() = settings;
    restart(app);
    Ok(state.view())
}

async fn blocking(app: tauri::AppHandle, f: impl FnOnce(&tauri::AppHandle) -> Result<View> + Send + 'static) -> Result<View, String> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn link_get(state: tauri::State<LinkState>) -> View {
    state.view()
}

#[tauri::command]
pub async fn link_set(app: tauri::AppHandle, enabled: bool, role: Role, side: Edge) -> Result<View, String> {
    blocking(app, move |app| {
        update(app, |s| {
            if role != s.role() {
                s.peer = None; // a new role needs a new pairing
            }
            s.enabled = enabled;
            s.role = Some(role);
            s.side = side;
            Ok(())
        })
    })
    .await
}

#[tauri::command]
pub async fn link_pair(app: tauri::AppHandle, id: String, code: String) -> Result<View, String> {
    blocking(app, move |app| {
        let state = app.state::<LinkState>();
        let found = state
            .nearby
            .lock()
            .unwrap()
            .values()
            .find(|f| f.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("That computer is no longer in reach"))?;
        let me = pair::Hello { id: state.settings.lock().unwrap().own_id.clone(), name: computer_name() };
        let mut last = anyhow!("That computer is no longer in reach");
        for address in &found.addresses {
            match pair::pair_with(*address, &code, &me) {
                Ok((them, pairing)) => {
                    return update(app, |s| {
                        s.peer = Some(Peer { id: them.id, name: them.name.chars().take(64).collect(), code: pairing.code() });
                        Ok(())
                    });
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    })
    .await
}

#[tauri::command]
pub async fn link_forget(app: tauri::AppHandle) -> Result<View, String> {
    blocking(app, |app| {
        *app.state::<LinkState>().code.lock().unwrap() = None;
        update(app, |s| {
            s.peer = None;
            Ok(())
        })
    })
    .await
}

#[tauri::command]
pub async fn link_new_code(app: tauri::AppHandle) -> Result<View, String> {
    blocking(app, |app| {
        *app.state::<LinkState>().code.lock().unwrap() = Some((pair::new_code()?, 0));
        restart(app);
        Ok(app.state::<LinkState>().view())
    })
    .await
}
