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

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::state::{AppState, RecordingState};

/// Payload emitted on `update-downloaded` and returned by `check_for_updates`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// `<app_data>/whats_new.json`: the release notes of the build about to be
/// installed, written just before the installer runs and read once by the
/// build that comes up afterwards. The file is the whole "shown once"
/// mechanism — taking it deletes it.
fn whats_new_path(dir: &Path) -> PathBuf {
    dir.join("whats_new.json")
}

/// Park the notes for the next launch. Best effort: a failure here only
/// costs the popup.
pub fn stash_whats_new(dir: &Path, info: &UpdateInfo) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(info).map_err(std::io::Error::other)?;
    std::fs::write(whats_new_path(dir), text)
}

/// The parked notes, if any, removed on read. `current_version` is what is
/// running now: notes stashed for some other version (an install that never
/// happened, a downgrade) are discarded rather than shown.
pub fn take_whats_new(dir: &Path, current_version: &str) -> Option<UpdateInfo> {
    let path = whats_new_path(dir);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let info: UpdateInfo = serde_json::from_str(&text).ok()?;
    (info.version == current_version).then_some(info)
}

/// Tauri command: release notes to show once after an update, or null.
#[tauri::command]
pub fn take_whats_new_notes(app: AppHandle) -> Option<UpdateInfo> {
    let dir = app.path().app_data_dir().ok()?;
    take_whats_new(&dir, &app.package_info().version.to_string())
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
    // Park the notes for the build that comes up after the installer; if the
    // install fails below the stash is harmless (version check on read).
    if let Ok(dir) = app.path().app_data_dir() {
        let info = UpdateInfo { version: update.version.clone(), notes: update.body.clone() };
        if let Err(e) = stash_whats_new(&dir, &info) {
            log::warn!("could not stash release notes: {e}");
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn info(version: &str) -> UpdateInfo {
        UpdateInfo { version: version.into(), notes: Some("- faster\n- prettier".into()) }
    }

    #[test]
    fn take_returns_the_stash_once() {
        let dir = tempdir().unwrap();
        stash_whats_new(dir.path(), &info("0.7.0")).unwrap();
        assert_eq!(take_whats_new(dir.path(), "0.7.0"), Some(info("0.7.0")));
        // Second launch: nothing, the file is gone.
        assert_eq!(take_whats_new(dir.path(), "0.7.0"), None);
        assert!(!whats_new_path(dir.path()).exists());
    }

    #[test]
    fn take_on_missing_file_is_none() {
        let dir = tempdir().unwrap();
        assert_eq!(take_whats_new(dir.path(), "0.7.0"), None);
    }

    #[test]
    fn take_discards_notes_for_another_version() {
        // Stashed for 0.7.0 but 0.6.0 came up (install failed or rolled
        // back): drop the stash instead of announcing a build not running.
        let dir = tempdir().unwrap();
        stash_whats_new(dir.path(), &info("0.7.0")).unwrap();
        assert_eq!(take_whats_new(dir.path(), "0.6.0"), None);
        assert!(!whats_new_path(dir.path()).exists());
    }

    #[test]
    fn take_survives_garbage() {
        let dir = tempdir().unwrap();
        std::fs::write(whats_new_path(dir.path()), "{not json").unwrap();
        assert_eq!(take_whats_new(dir.path(), "0.7.0"), None);
        assert!(!whats_new_path(dir.path()).exists());
    }
}
