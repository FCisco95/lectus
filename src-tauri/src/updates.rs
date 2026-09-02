//! Background + manual update checking via tauri-plugin-updater.
//!
//! Flow: on launch (after windows are ready) a background task checks the GitHub Releases `latest.json` feed.
//! A newer build is downloaded in the background (signature-verified against the pubkey embedded in
//! tauri.conf.json), the Settings window gets an `update-downloaded` event → the banner there offers
//! "Restart now / Later". Install only happens when the user picks "Restart now" — never mid-recording.
//!
//! Failure policy: background-check failures (offline, GitHub down, signature mismatch) are logged +
//! emitted as `update-error` and remembered in `last_update_error` for Settings → About; they never
//! interrupt dictation.
//! Failures are logged and reported via `update-error`; silent in UI except the About line.

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::state::{AppState, RecordingState};

/// Payload emitted on `update-downloaded` and returned by `check_for_updates`.
#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// Managed state holding the downloaded update until the user confirms the restart. The download +
/// signature verification happens once in the background; `install_update` just runs the platform
/// installer.
pub struct PendingUpdate(pub Mutex<Option<(tauri_plugin_updater::Update, Vec<u8>)>>);

impl PendingUpdate {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

/// Persisted detail of the last update-check failure, surfaced in Settings → About so a silently-failing
/// background check stays diagnosable without reading logs.
pub struct LastUpdateError(pub Mutex<Option<String>>);

#[tauri::command]
pub fn last_update_error(state: tauri::State<'_, LastUpdateError>) -> Option<String> {
    state.0.lock().unwrap().clone()
}

fn set_error(app: &AppHandle, message: String) {
    if let Some(s) = app.try_state::<LastUpdateError>() {
        *s.0.lock().unwrap() = Some(message.clone());
        let _ = app.emit("update-error", message.clone());
    }
    log::warn!("update check failed: {message}");
}

fn clear_error(app: &AppHandle) {
    if let Some(s) = app.try_state::<LastUpdateError>() {
        *s.0.lock().unwrap() = None;
    }
}

/// Check the feed, download the update if present, and park it in `PendingUpdate`. Shared by the
/// background check and the manual "Check for updates" button. Returns the info for a manual check;
/// background callers discard the result (they already emitted the events).
pub async fn check_and_download(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = match updater.check().await {
        Ok(Some(u)) => u,
        Ok(None) => {
            clear_error(app);
            return Ok(None);
        }
        Err(e) => {
            let msg = e.to_string();
            set_error(app, msg.clone());
            return Err(msg);
        }
    };
    let info = UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
    };
    // Download + signature-verify now so "Restart now" later is instant; install happens on
    // user click, not here — on Windows `install` launches the NSIS installer and exits the app.
    let bytes = match update.download(|_, _| {}, || {}).await {
        Ok(b) => b,
        Err(e) => {
            let msg = e.to_string();
            set_error(app, msg.clone());
            return Err(msg);
        }
    };
    clear_error(app);
    *app.state::<PendingUpdate>().0.lock().unwrap() = Some((update, bytes));
    Ok(Some(info))
}

/// Manual "Check for updates" button (Settings → About) and background launch check both funnel here. Emits
/// `update-downloaded` (with UpdateInfo) when a new build is parked in PendingUpdate.
async fn run_check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let result = check_and_download(&app).await;
    if let Ok(Some(info)) = &result {
        let _ = app.emit("update-downloaded", info.clone());
    }
    result
}

/// Background check triggered from the app setup hook, after windows are ready. Silent by design.
pub fn spawn_background_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = run_check(app).await;
    });
}

/// Tauri command: manual check from Settings → About. Errors returned to the frontend for display.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    run_check(app).await
}

/// Tauri command: user clicked "Restart now". Gated on idle recording state so the restart never interrupts dictation.
/// On Windows `install` launches the NSIS installer and exits this process; on macOS it swaps the bundle and relaunches.
#[tauri::command]
pub fn restart_and_apply(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if *state.recording.lock().unwrap() != RecordingState::Idle {
        return Err("Recording in progress — wait until dictation is idle to install the update.".to_string());
    }
    let taken = app.state::<PendingUpdate>().0.lock().unwrap().take();
    let (update, bytes) = match taken {
        Some(x) => x,
        None => return Err("No downloaded update available.".to_string()),
    };
    if let Err(e) = update.install(&bytes) {
        // Put it back so the user can retry instead of silently losing the downloaded payload.
        *app.state::<PendingUpdate>().0.lock().unwrap() = Some((update, bytes));
        return Err(e.to_string());
    }
    // Windows: installer launched, app exits itself. macOS: install returns Ok and app must restart.
    #[cfg(target_os = "macos")]
    {
        // Same handoff as `relaunch_app` in lib.rs: spawn + force_exit, so the ggml-metal teardown crash can't hit mid-restart.
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
        unsafe { libc::_exit(0) }
    }
    Ok(())
}
