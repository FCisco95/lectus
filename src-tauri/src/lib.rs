mod ai;
mod api;
mod api_adapter;
#[cfg(test)]
mod api_inference_test;
#[cfg(test)]
mod model_quality_test;
mod audio;
mod autostart;
mod config;
mod context;
mod history;
mod hook;
mod hotkey;
mod identity;
mod injection;
mod license;
mod legal;
mod model_license;
mod playback;
mod remote_dictation;
mod resources;
mod state;
mod subscriptions;
mod transcription;
mod translation;
mod updates;
mod worker;

use state::{AppState, RecordingState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Listener, Manager};
use hook::{HookContext, KeyboardHook};

/// Whether the capture loop should keep recording: active (key still held / latch
/// on) AND under the safety cap. The cap is mode-dependent — short for hold
/// (push-to-talk bursts), long for toggle (hands-free dictation).
fn should_continue(active: bool, elapsed_secs: u64, max_secs: u64) -> bool {
    active && elapsed_secs < max_secs
}

/// Current Unix time in milliseconds (0 if the clock is before the epoch).
fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Safety cap for the capture loop, by trigger mode.
fn safety_cap_secs(trigger_mode: &str) -> u64 {
    // Generous runaway guard only — long dictations are normal (WhisperFlow
    // parity). Hold mode ends on key release anyway.
    let _ = trigger_mode;
    300
}

/// True when this process was started by the login Run key / autostart plugin.
fn args_mean_autostart<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|a| a.as_ref() == "--autostart")
}

fn launched_by_autostart() -> bool {
    args_mean_autostart(std::env::args())
}

/// Show the app window. `force_home_tab` emits `open-home` so a hidden window
/// lands on Home; a window that is already visible is only focused.
fn show_app_window(app: &tauri::AppHandle, force_home_tab: bool) {
    if let Some(w) = app.get_webview_window("settings") {
        let was_visible = w.is_visible().unwrap_or(false);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if force_home_tab || !was_visible {
            let _ = app.emit("open-home", ());
        }
    }
}

fn user_opened_app(app: &tauri::AppHandle) {
    let hidden = app
        .get_webview_window("settings")
        .map(|w| !w.is_visible().unwrap_or(false))
        .unwrap_or(true);
    show_app_window(app, hidden);
}

/// The key (or two-key chord) the hook should watch given the active trigger
/// mode: the toggle key in toggle mode, otherwise the hold key. Falls back to
/// Right Ctrl if the configured value isn't supported.
fn active_trigger_key(cfg: &config::Config) -> String {
    let key = if cfg.trigger_mode.eq_ignore_ascii_case("toggle") {
        &cfg.toggle_hotkey
    } else {
        &cfg.hold_hotkey
    };
    if hook::is_supported_hold_key(key) {
        key.clone()
    } else {
        "RControl".to_string()
    }
}

/// Managed activation state: the shared stop flag + the installed hook.
struct Activation {
    recording_active: Arc<AtomicBool>,
    /// Kept alive for the app's lifetime; `rearm` swaps the key live.
    #[allow(dead_code)]
    hook: Mutex<Option<hook::PlatformHook>>,
}

/// Always-on capture stream feeding the 500 ms pre-roll ring. `None` when the
/// mic could not be opened at startup — the pipeline then falls back to a
/// per-dictation stream (no pre-roll, first word may clip).
struct PreRoll(Mutex<Option<audio::PersistentCapture>>);

/// (Re)open the persistent pre-roll stream on the configured device, replacing
/// any existing one. Failure leaves the fallback path in place.
fn restart_preroll(app: &tauri::AppHandle, device: &str) {
    let preroll = app.state::<PreRoll>();
    let mut guard = preroll.0.lock().unwrap();
    *guard = None; // drop the old stream before opening the device again
    if !app.state::<AppState>().config.lock().unwrap().onboarding_completed || legal::requires_acceptance() {
        return;
    }
    if app.state::<remote_dictation::RemoteState>().mode() == remote_dictation::Mode::Receive {
        return;
    }
    match audio::PersistentCapture::start(Some(device)) {
        Ok(cap) => *guard = Some(cap),
        Err(e) => log::warn!("pre-roll capture unavailable, falling back to per-dictation stream: {e}"),
    }
}

/// Cached transcription engines. Loaded once at startup to avoid per-call setup cost.
struct WhisperState {
    local: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
    cloud: Arc<Mutex<Option<transcription::cloud::CloudWhisper>>>,
}

static MODEL_LOAD: transcription::model::LoadGate = transcription::model::LoadGate::new();
static ACTIVE_MODEL: Mutex<Option<String>> = Mutex::new(None);

#[tauri::command]
fn hardware_profile() -> resources::HardwareProfile { resources::hardware_profile() }

/// A downloadable model plus its current status on disk.
#[derive(serde::Serialize, Clone)]
struct ModelStatus {
    name: String,
    display_name: String,
    size_mb: u32,
    multilingual: bool,
    description: String,
    downloaded: bool,
    active: bool,
    loading: bool,
    download: transcription::model::DownloadStatus,
}

fn playback_on(app: &tauri::AppHandle, event: playback::PlaybackEvent) {
    let enabled = app
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .mute_while_dictating;
    let ducker = app.state::<playback::AppPlayback>();
    playback::on_event(&*ducker, enabled, event);
}

fn do_set_state(
    state_str: &str,
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let next = match state_str {
        "recording" => RecordingState::Recording,
        "transcribing" => RecordingState::Transcribing,
        _ => RecordingState::Idle,
    };
    if next == RecordingState::Idle {
        playback_on(app_handle, playback::PlaybackEvent::Idle);
    }
    app_state.set_state(next.clone());
    app_handle.emit("state-changed", state_str).map_err(|e| e.to_string())?;

    if let Some(tray) = app_handle.tray_by_id("main") {
        if let Ok(res_dir) = app_handle.path().resource_dir() {
            if let Ok(icon) = tauri::image::Image::from_path(res_dir.join(tray_icon_path(&next))) {
                tray.set_icon(Some(icon)).ok();
                // set_icon clears the template flag on macOS; re-assert it so the
                // menu bar keeps adapting the glyph to light/dark appearance.
                #[cfg(target_os = "macos")]
                tray.set_icon_as_template(true).ok();
            }
        }
    }

    if let Some(pill) = app_handle.get_webview_window("pill") {
        let size = match &next {
            RecordingState::Recording | RecordingState::Transcribing => {
                tauri::LogicalSize::new(240.0, 64.0)
            }
            _ => tauri::LogicalSize::new(64.0, 64.0),
        };
        let _ = pill.set_size(size);
        let _ = pill.show();
    }
    Ok(())
}

/// Tray icon for a state. Windows/Linux use the colored PNGs; macOS uses
/// monochrome template PNGs (36 px = 18 pt @2x) so the menu bar can adapt
/// them to light/dark appearance and highlight states.
fn tray_icon_path(state: &RecordingState) -> &'static str {
    #[cfg(target_os = "macos")]
    return match state {
        RecordingState::Recording => "icons/tray-template-recording.png",
        RecordingState::Transcribing => "icons/tray-template-transcribing.png",
        _ => "icons/tray-template-idle.png",
    };
    #[cfg(not(target_os = "macos"))]
    match state {
        RecordingState::Recording => "icons/tray-recording.png",
        RecordingState::Transcribing => "icons/tray-transcribing.png",
        _ => "icons/tray-idle.png",
    }
}

#[tauri::command]
fn set_recording_state(
    state_str: String,
    app_state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    do_set_state(&state_str, &app_state, &app_handle)
}

/// Toggle recording from the UI (e.g. clicking the pill). Idle → start, else
/// stop. Mirrors the physical toggle key and keeps the hook's latch in sync.
#[tauri::command]
fn toggle_recording(
    app_state: tauri::State<AppState>,
    activation: tauri::State<Activation>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if receiver_blocks_capture(&app_handle) { return Ok(()); }
    let is_idle = *app_state.recording.lock().unwrap() == RecordingState::Idle;
    let start = is_idle;
    activation.recording_active.store(start, Ordering::SeqCst);
    if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
        hook.set_toggle_state(start);
    }
    app_handle
        .emit(if start { "hold-start" } else { "hold-stop" }, ())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Return the current in-memory config for the Settings UI to display.
#[tauri::command]
fn get_config(app_state: tauri::State<AppState>) -> config::Config {
    app_state.config.lock().unwrap().clone()
}

fn commit_config_snapshot(
    current: &mut config::Config,
    next: config::Config,
    path: &std::path::Path,
) -> Result<(), String> {
    next.save_to(path).map_err(|e| e.to_string())?;
    if current.text_processing_changed(&next) {
        // The caller holds the config lock. Inference captures its epoch under
        // that same lock, so old preflight work cannot acquire new authority.
        subscriptions::invalidate_inference();
    }
    *current = next;
    Ok(())
}

/// Mark first-run setup done without writing the rest of a stale Settings
/// snapshot (which used to wipe dictionary + model back to defaults).
#[tauri::command]
fn complete_onboarding(
    app_state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    legal::ensure_accepted()?;
    let mut guard = app_state.config.lock().unwrap();
    let mut next = guard.clone();
    next.onboarding_completed = true;
    let path = app_handle.path().app_config_dir().map_err(|e| e.to_string())?.join("config.json");
    commit_config_snapshot(&mut guard, next, &path)?;
    let device = guard.input_device.clone();
    drop(guard);
    std::thread::spawn(move || restart_preroll(&app_handle, &device));
    Ok(())
}

#[tauri::command]
fn translation_targets() -> &'static [translation::Target] { translation::targets() }

#[tauri::command]
async fn get_translation_model_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let name = translation::LOCAL_MODEL_NAME;
        let downloading = transcription::model::download_status_for(&app, name).downloading;
        let downloaded = !downloading && transcription::model::is_downloaded(&app, name)
            && transcription::model::verify_model_file(&app, name, translation::LOCAL_MODEL_SHA256).is_ok();
        serde_json::json!({"name": name, "display_name": "Qwen3 4B · offline translation", "size_mb": translation::LOCAL_MODEL_SIZE_MB,
            "downloaded": downloaded, "download": transcription::model::download_status_for(&app, name)})
    }).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_translation_model(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        transcription::model::ensure_verified_file_from_url(&app, &translation::local_model_url(), translation::LOCAL_MODEL_NAME, translation::LOCAL_MODEL_SHA256).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

/// Whether the local Whisper engine has finished loading. The pill uses this
/// so daily launch can show "Warming up…" instead of a dead orb.
#[tauri::command]
fn engine_ready(whisper_state: tauri::State<WhisperState>) -> bool {
    whisper_state.local.lock().unwrap().is_some()
}

/// Whether the Claude Code CLI is available for AI cleanup (shown in Settings).
#[tauri::command]
fn check_claude_cli() -> bool {
    ai::claude_available()
}

/// Read provider model availability without uploading audio or creating completions.
#[tauri::command]
async fn get_cloud_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        transcription::cloud::available_models(&base_url, &api_key).map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Recent dictations, newest last.
#[tauri::command]
fn get_history(app_handle: tauri::AppHandle) -> Vec<history::HistoryEntry> {
    app_handle
        .path()
        .app_data_dir()
        .map(|d| history::load(&d))
        .unwrap_or_default()
}

/// Rolling-week + lifetime counts and the day streak for the Home cards.
/// `tz_offset_min` is JavaScript's `getTimezoneOffset()` — minutes *behind*
/// UTC — so the streak's day boundaries fall on the user's local midnight.
#[tauri::command]
fn get_history_stats(app_handle: tauri::AppHandle, tz_offset_min: Option<i32>) -> history::HistoryStats {
    let entries = get_history(app_handle);
    let utc_offset_ms = -(tz_offset_min.unwrap_or(0) as i64) * 60_000;
    history::stats(&entries, now_millis(), utc_offset_ms)
}

/// Pill / tray / frontend: show the app window on Home.
#[tauri::command]
fn show_home(app_handle: tauri::AppHandle) {
    show_app_window(&app_handle, true);
}

/// The greeting name when `display_name` is empty: the OS display name, or
/// a prettified account name when there is none.
#[tauri::command]
fn os_display_name() -> String {
    identity::os_display_name()
}

/// Sidebar → Help: the README in the system browser. The URL is fixed here
/// on purpose — the webview cannot ask for an arbitrary one.
#[tauri::command]
fn open_help() -> Result<(), String> {
    license::connect::open_in_browser("https://github.com/FCisco95/lectus-releases#readme")
        .map_err(|e| e.to_string())
}

/// Erase all stored history.
#[tauri::command]
fn clear_history(app_handle: tauri::AppHandle) -> Result<(), String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    history::clear(&dir).map_err(|e| e.to_string())
}

/// Copy a past transcript back to the clipboard.
#[tauri::command]
fn copy_to_clipboard(text: String) -> Result<(), String> {
    injection::clipboard::set_clipboard(&text).map_err(|e| e.to_string())
}

/// Persist the pill's on-screen position after the user drags it.
#[tauri::command]
fn save_pill_position(
    x: i32,
    y: i32,
    app_state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = app_state.config.lock().unwrap();
    let mut next = guard.clone();
    next.pill_x = x;
    next.pill_y = y;
    let path = app_handle.path().app_config_dir().map_err(|e| e.to_string())?.join("config.json");
    commit_config_snapshot(&mut guard, next, &path)
}

/// Persist edited config to disk, rebuild the cloud engine, and update in-memory state.
#[tauri::command]
fn save_config(
    mut new_config: config::Config,
    app_state: tauri::State<AppState>,
    whisper_state: tauri::State<WhisperState>,
    activation: tauri::State<Activation>,
    app_handle: tauri::AppHandle,
) -> Result<config::Config, String> {
    if new_config.use_cloud && new_config.cloud_api_key.trim().is_empty() {
        return Err("Cannot enable cloud transcription without an API key".to_string());
    }

    let (device_changed, local_cleanup_enabled, theme_changed) = {
        let mut guard = app_state.config.lock().unwrap();
        if guard.cloud_base_url != new_config.cloud_base_url { new_config.cloud_processing_acknowledged = false; }
        translation::validate_target(&new_config.output_language).map_err(|e| e.to_string())?;
        let changed = guard.input_device != new_config.input_device;
        let was_local = guard.ai_cleanup_enabled && guard.ai_cleanup_engine == "local";
        let now_local = new_config.ai_cleanup_enabled && new_config.ai_cleanup_engine == "local";
        let theme_changed = guard.theme != new_config.theme;
        let cloud_changed = guard.cloud_base_url != new_config.cloud_base_url
            || guard.cloud_api_key != new_config.cloud_api_key
            || guard.cloud_processing_acknowledged != new_config.cloud_processing_acknowledged
            || guard.cloud_transcription_model != new_config.cloud_transcription_model;
        let mut snapshot = guard.clone();
        snapshot.apply_ui_update(new_config);
        let cfg_dir = app_handle.path().app_config_dir().map_err(|e| e.to_string())?;
        commit_config_snapshot(&mut guard, snapshot.clone(), &cfg_dir.join("config.json"))?;
        // Commit only after persistence succeeds. Holding the config lock also
        // preserves model/pill updates that would otherwise race this snapshot.
        if cloud_changed {
            *whisper_state.cloud.lock().unwrap() = Some(transcription::cloud::CloudWhisper::with_model(
                &snapshot.cloud_base_url,
                if snapshot.cloud_processing_acknowledged { &snapshot.cloud_api_key } else { "" },
                &snapshot.cloud_transcription_model,
            ));
        }
        (
            changed,
            !was_local && now_local,
            theme_changed,
        )
    };

    // Every webview (settings + pill) resolves `data-theme` itself on load;
    // broadcast so an already-open window updates without a reload.
    if theme_changed {
        let theme = app_state.config.lock().unwrap().theme.clone();
        let _ = app_handle.emit("theme-changed", theme);
    }

    // Move the always-on pre-roll stream to the new mic. Done off-thread: the
    // pipeline holds the PreRoll lock for the whole dictation, and save fires
    // from the UI (auto-save) — blocking here would freeze settings.
    if device_changed {
        let handle = app_handle.clone();
        let device = app_state.config.lock().unwrap().input_device.clone();
        std::thread::spawn(move || restart_preroll(&handle, &device));
    }

    // Local cleanup was just switched on: warm the LLM now so the next
    // dictation doesn't pay the Vulkan pipeline compile (~20 s cold).
    if local_cleanup_enabled {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            if let Ok(path) =
                transcription::model::model_path(&handle, ai::local_llm::CLEANUP_MODEL_NAME)
            {
                if path.exists() {
                    if let Err(e) = ai::local_llm::warmup(&path) {
                        log::warn!("cleanup LLM warmup failed: {e}");
                    }
                }
            }
        });
    }

    {
        let (active_key, mode) = {
            let cfg = app_state.config.lock().unwrap();
            (active_trigger_key(&cfg), cfg.trigger_mode.clone())
        };
        if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
            hook.set_mode(&mode);
            hook.rearm(&active_key);
        }
    }
    Ok(app_state.config.lock().unwrap().clone())
}

/// Names of the available recording devices for the settings mic picker.
#[tauri::command]
fn list_input_devices() -> Vec<String> {
    audio::list_input_devices()
}

/// Exe name of the currently focused app (for the Apps settings panel's
/// "use current app" helper).
#[tauri::command]
fn get_foreground_app() -> Option<String> {
    context::foreground_app()
}

/// Whether the OS trusts this process for Accessibility (macOS). Non-prompting,
/// safe to poll from onboarding / the settings banner. Always true elsewhere.
#[tauri::command]
fn accessibility_status() -> bool {
    #[cfg(target_os = "macos")]
    return hook::accessibility_trusted();
    #[cfg(not(target_os = "macos"))]
    true
}

/// Coarse microphone permission state: "granted" | "denied" | "undetermined" |
/// "unknown". macOS asks TCC via AVFoundation; elsewhere we probe whether the
/// default input device is usable.
#[tauri::command]
fn microphone_status() -> String {
    #[cfg(target_os = "macos")]
    {
        use objc::{class, msg_send, sel, sel_impl};
        #[link(name = "AVFoundation", kind = "framework")]
        extern "C" {
            static AVMediaTypeAudio: *mut objc::runtime::Object;
        }
        // AVAuthorizationStatus: 0 notDetermined, 1 restricted, 2 denied, 3 authorized
        let status: i64 = unsafe {
            msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeAudio]
        };
        match status {
            3 => "granted",
            1 | 2 => "denied",
            0 => "undetermined",
            _ => "unknown",
        }
        .to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        match cpal::default_host()
            .default_input_device()
            .and_then(|d| d.default_input_config().ok())
        {
            Some(_) => "granted".to_string(),
            None => "unknown".to_string(),
        }
    }
}

/// Trigger the OS microphone permission prompt by briefly opening an input
/// stream (on macOS the first open raises the TCC dialog; the app's pre-roll
/// stream may already have done so at launch). Fire-and-forget.
#[tauri::command]
fn request_microphone_access() {
    std::thread::spawn(|| {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let Some(device) = cpal::default_host().default_input_device() else { return };
        let Ok(config) = device.default_input_config() else { return };
        if let Ok(stream) = device.build_input_stream(
            &config.into(),
            |_: &[f32], _| {},
            |_| {},
            None,
        ) {
            let _ = stream.play();
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    });
}

/// Terminate the current process without running libc `atexit`/C++ static
/// destructors. `std::process::exit` (and therefore `app_handle.exit`/
/// `app_handle.restart`) runs those on the way out, which crashes here: once
/// whisper's Metal backend has been initialized (it warms up at every
/// startup — see `warmup_engine`), ggml-metal's own static device-registry
/// destructor hits `GGML_ASSERT([rsets->data count] == 0)` during teardown —
/// a known upstream bug (ggml-org/llama.cpp#17869), not anything in our
/// state. Skipping straight to the `_exit` syscall sidesteps it entirely;
/// there's nothing on our side left to flush (config saves are synchronous).
fn force_exit(code: i32) -> ! {
    unsafe { libc::_exit(code) }
}

/// Spawn a fresh instance of this binary, then hand off via `force_exit`
/// instead of the normal exit path (see its doc comment for why).
#[tauri::command]
fn relaunch_app() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    force_exit(0);
}

/// Deep-link into System Settings' Accessibility pane (macOS only — the
/// onboarding step and the General-tab banner both need this, so it lives
/// here rather than duplicated in the frontend).
#[tauri::command]
fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

/// Status of the local cleanup LLM, for the AI settings panel.
#[derive(serde::Serialize)]
struct CleanupModelStatus {
    downloaded: bool,
    size_mb: u32,
    name: &'static str,
    download: transcription::model::DownloadStatus,
}

#[tauri::command]
async fn get_cleanup_model_status(app_handle: tauri::AppHandle) -> Result<CleanupModelStatus, String> {
    tauri::async_runtime::spawn_blocking(move || CleanupModelStatus {
        downloaded: transcription::model::is_downloaded(&app_handle, ai::local_llm::CLEANUP_MODEL_NAME),
        size_mb: ai::local_llm::CLEANUP_MODEL_SIZE_MB,
        name: ai::local_llm::CLEANUP_MODEL_NAME,
        download: transcription::model::download_status_for(&app_handle, ai::local_llm::CLEANUP_MODEL_NAME),
    }).await.map_err(|error| error.to_string())
}

/// Download the local cleanup LLM (progress via `model-download-progress`),
/// then load it resident so the first dictation doesn't pay the load cost.
#[tauri::command]
async fn download_cleanup_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        model_license::ensure_accepted().map_err(|e| e.to_string())?;
        let path = transcription::model::ensure_file_from_url(
            &app_handle,
            &ai::local_llm::cleanup_model_url(),
            ai::local_llm::CLEANUP_MODEL_NAME,
        )
        .map_err(|e| e.to_string())?;
        ai::local_llm::warmup(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Block until the local engine is in the cache, or 90s elapses. Daily launch
/// loads large-v3-turbo off the setup thread; a hold that lands during that
/// window should wait instead of injecting "model not loaded".
fn wait_for_local_engine(
    local: &Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if local.lock().unwrap().is_some() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err("Whisper model is still loading — try again in a moment".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Run a short silent inference on the whisper worker so the GPU backend
/// compiles its pipelines up front instead of on the first real dictation
/// (~8 s of Vulkan shader compilation on first `whisper_full`).
/// Called from the model loader thread, which retains its load reservation
/// until this returns and the completion event has been emitted.
fn warmup_engine(app: &tauri::AppHandle) {
    let local = app.state::<WhisperState>().local.clone();
    let worker = app.state::<worker::TranscribeWorker>().inner().clone();
    worker.run(move || {
        let mut guard = local.lock().unwrap();
        if let Some(engine) = guard.as_mut() {
            let silence = vec![0.0_f32; 16_000];
            let opts = transcription::TranscribeOptions::from_config("en", None);
            let t = std::time::Instant::now();
            let _ = engine.transcribe(&silence, &opts);
            log::info!("whisper warmup finished in {} ms", t.elapsed().as_millis());
        }
    });
}

/// Return the download/active status of every known model.
#[tauri::command]
async fn get_models_status(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ModelStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
    let current = ACTIVE_MODEL.lock().unwrap().clone();
    let loading = MODEL_LOAD.current();
    transcription::model::AVAILABLE_MODELS
        .iter()
        .map(|m| ModelStatus {
            name: m.name.to_string(),
            display_name: m.display_name.to_string(),
            size_mb: m.size_mb,
            multilingual: m.multilingual,
            description: m.description.to_string(),
            downloaded: transcription::model::is_downloaded(&app_handle, m.name),
            active: current.as_deref() == Some(m.name),
            loading: loading.as_deref() == Some(m.name),
            download: transcription::model::download_status_for(&app_handle, m.name),
        })
        .collect()
    }).await.map_err(|error| error.to_string())
}

/// Kick off a background download for a model. Emits `model-download-progress`
/// (f64 0..1) and `model-ready` (String model_name) when done.
#[tauri::command]
async fn download_model(model_name: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // Validate the name is one we know about.
    if !transcription::model::AVAILABLE_MODELS.iter().any(|m| m.name == model_name) {
        return Err(format!("Unknown model: {model_name}"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        transcription::model::ensure_model(&app_handle, &model_name)
            .map(|_| ()).map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}

/// Pause only known model downloads; retry uses the validated partial file.
#[tauri::command]
fn cancel_model_download(model_name: String) -> Result<(), String> {
    if !transcription::model::AVAILABLE_MODELS.iter().any(|model| model.name == model_name)
        && model_name != translation::LOCAL_MODEL_NAME
        && model_name != ai::local_llm::CLEANUP_MODEL_NAME
    {
        return Err("Unknown model download.".into());
    }
    transcription::model::cancel_download(&model_name).map_err(|error| error.to_string())
}

/// Load a downloaded model and make it the active engine. Non-blocking — emits
/// `model-loading` (String model_name) immediately, then `model-active` (String)
/// once swapped in, or `model-load-failed` (String) on error.
#[tauri::command]
fn select_model(
    model_name: String,
    whisper_state: tauri::State<WhisperState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if !transcription::model::AVAILABLE_MODELS.iter().any(|m| m.name == model_name) {
        return Err(format!("Unknown model: {model_name}"));
    }
    let path = transcription::model::model_path(&app_handle, &model_name)
        .map_err(|e| e.to_string())?;
    if !path.exists() {
        return Err(format!("Model '{model_name}' is not downloaded yet"));
    }

    let permit = MODEL_LOAD.begin(&model_name).map_err(|e| e.to_string())?;
    let _ = app_handle.emit("model-loading", model_name.clone());

    let local = Arc::clone(&whisper_state.local);
    let app = app_handle.clone();
    let name_clone = model_name.clone();
    std::thread::spawn(move || {
        let result = (|| -> anyhow::Result<()> {
            let checksum = transcription::model::model_checksum(&name_clone)
                .ok_or_else(|| anyhow::anyhow!("The selected model has no approved checksum."))?;
            // IPC callers cannot bypass readiness by selecting an existing but
            // incomplete or corrupted final file. Hash off the UI thread.
            transcription::model::verify_model_file(&app, &name_clone, checksum)?;
            let engine = transcription::local::LocalWhisper::new(&path)?;
            let app_state = app.state::<AppState>();
            let mut cfg = app_state.config.lock().unwrap();
            let mut next = cfg.clone();
            next.model_name = name_clone.clone();
            next.model_path = path;
            next.save_to(&app.path().app_config_dir()?.join("config.json"))?;
            *local.lock().unwrap() = Some(engine);
            *ACTIVE_MODEL.lock().unwrap() = Some(name_clone.clone());
            *cfg = next;
            Ok(())
        })();
        match result {
            Ok(()) => {
                warmup_engine(&app);
                let _ = app.emit("model-active", name_clone);
            }
            Err(e) => { let _ = app.emit("model-load-failed", e.to_string()); }
        }
        drop(permit);
    });

    Ok(())
}

/// Inner pipeline logic. Separated so `run_pipeline` can reset state on any error path.
async fn do_pipeline(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    recording_active: Arc<AtomicBool>,
    local: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
    cloud: Arc<Mutex<Option<transcription::cloud::CloudWhisper>>>,
    cfg: config::Config,
    original_app: Option<String>,
) -> Result<String, String> {
    let remote_destination = app_handle.state::<remote_dictation::RemoteState>()
        .destination().map_err(|e| e.to_string())?;
    let use_cloud = cfg.use_cloud;
    if use_cloud && !cfg.cloud_processing_acknowledged {
        let message = "Cloud speech is paused. Review the provider processing choice in Settings → General.";
        let _ = app_handle.emit("processing-error", message);
        return Err(message.into());
    }
    // 1. Atomic claim: reject if another pipeline is already running.
    {
        let mut rec = app_state.recording.lock().unwrap();
        if *rec != RecordingState::Idle {
            log::warn!("pipeline: skipped overlapping dictation (already {rec:?})");
            return Ok(String::new());
        }
        *rec = RecordingState::Recording;
    }
    do_set_state("recording", app_state, app_handle)?;

    // 2. Capture audio. Preferred path: the always-on stream, which contributes
    // up to 500 ms of pre-roll from before the hotkey landed. Fallback: open a
    // per-dictation stream (no pre-roll) if the persistent one is unavailable.
    let active = recording_active.clone();
    let level_app = app_handle.clone();
    let max_secs = safety_cap_secs(&cfg.trigger_mode);
    let input_device = cfg.input_device.clone();
    let accumulated = tokio::task::spawn_blocking(move || {
        use audio::AudioCapture;
        let preroll_state = level_app.state::<PreRoll>();
        let preroll = preroll_state.0.lock().unwrap();

        let mut fallback: Option<AudioCapture> = None;
        let mut accumulated: Vec<f32> = match preroll.as_ref() {
            Some(cap) => {
                let snapshot = cap.begin();
                log::info!("pipeline: pre-roll contributed {:.0} ms", snapshot.len() as f32 / 16.0);
                snapshot
            }
            None => {
                fallback = Some(AudioCapture::start(Some(&input_device)).map_err(|e| e.to_string())?);
                Vec::new()
            }
        };

        let start = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(32));
            let chunk = match (preroll.as_ref(), fallback.as_ref()) {
                (Some(cap), _) => cap.drain_live(),
                (None, Some(cap)) => cap.drain(),
                (None, None) => unreachable!(),
            };
            if !chunk.is_empty() {
                let _ = level_app.emit("audio-level", audio::vad::rms(&chunk));
                accumulated.extend_from_slice(&chunk);
            }
            if !should_continue(active.load(Ordering::SeqCst), start.elapsed().as_secs(), max_secs) {
                break;
            }
        }
        if let Some(cap) = preroll.as_ref() {
            accumulated.extend_from_slice(&cap.end());
        }
        drop(fallback);
        let _ = level_app.emit("audio-level", 0.0_f32);
        Ok::<Vec<f32>, String>(accumulated)
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Transcribe.
    log::info!("pipeline: captured {:.2}s of audio", accumulated.len() as f32 / 16000.0);
    let t_transcribe = std::time::Instant::now();
    do_set_state("transcribing", app_state, app_handle)?;
    let bias = transcription::dictionary::bias_prompt(&cfg.dictionary_words);
    let mut opts = transcription::TranscribeOptions::from_config(&cfg.language, bias);
    if cfg.vad_enabled && !use_cloud {
        if let Ok(p) = transcription::model::model_path(&app_handle, transcription::model::VAD_MODEL_NAME) {
            if p.exists() {
                opts.vad_model_path = p.to_str().map(String::from);
            }
        }
    }
    let worker = app_handle.state::<worker::TranscribeWorker>().inner().clone();
    let cloud_authority = app_handle.clone();
    let cloud_snapshot = cfg.clone();
    let (mut transcript, detected_language) = tokio::task::spawn_blocking(move || {
        if !use_cloud {
            wait_for_local_engine(&local)?;
        }
        worker.run(move || {
            if use_cloud {
                let current_state = cloud_authority.state::<AppState>();
                let current = current_state.config.lock().unwrap();
                if !current.use_cloud || !current.allows_cloud_snapshot(&cloud_snapshot) {
                    return Err("Cloud speech permission or provider changed. Start a new dictation with the current settings.".into());
                }
                let guard = cloud.lock().unwrap();
                drop(current); // chosen client is now fixed for this admitted request
                match guard.as_ref() {
                    Some(engine) => engine
                        .transcribe(&accumulated, &opts)
                        .map(|text| (text, None))
                        .map_err(|e| e.to_string()),
                    None => Err("Cloud backend not configured — set an API key in Settings".to_string()),
                }
            } else {
                let mut guard = local.lock().unwrap();
                match guard.as_mut() {
                    Some(engine) => engine.transcribe(&accumulated, &opts).map_err(|e| e.to_string()),
                    None => Err(
                        "Whisper model not loaded — download one in Settings → Models".to_string(),
                    ),
                }
            }
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    log::info!(
        "pipeline: transcription took {} ms ({} chars)",
        t_transcribe.elapsed().as_millis(),
        transcript.len()
    );

    // 4. Post-process: dictionary spellings, replacement rules, optional AI cleanup.
    transcript = transcription::dictionary::apply_dictionary_spellings(
        &transcript,
        &cfg.dictionary_words,
    );
    transcript = transcription::dictionary::apply_rules(&transcript, &cfg.replacement_rules);
    if cfg.ai_cleanup_enabled && !transcript.is_empty() {
        let t_cleanup = std::time::Instant::now();
        let to_clean = transcript.clone();
        let cfg_for_cleanup = cfg.clone();
        let cleanup_target = original_app.clone();
        let cleanup_authority = app_handle.clone();
        let llm_path = transcription::model::model_path(&app_handle, ai::local_llm::CLEANUP_MODEL_NAME).ok();
        let lang_for_cleanup = detected_language.clone();
        let cleaned = tokio::task::spawn_blocking(move || {
            let inference_epoch = {
                let state = cleanup_authority.state::<AppState>();
                let current = state.config.lock().unwrap();
                if !current.allows_cleanup_snapshot(&cfg_for_cleanup, cleanup_target.as_deref()) {
                    return Err(anyhow::anyhow!("AI cleanup settings changed. Start a new dictation."));
                }
                subscriptions::inference_epoch()
            };
            let result = ai::cleanup_guarded(&to_clean, &cfg_for_cleanup, llm_path.as_deref(), lang_for_cleanup.as_deref(), inference_epoch);
            let current = cleanup_authority.state::<AppState>().config.lock().unwrap().clone();
            if !current.allows_cleanup_snapshot(&cfg_for_cleanup, cleanup_target.as_deref()) {
                return Err(anyhow::anyhow!("AI cleanup settings changed during processing."));
            }
            result
        })
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("The cleanup worker stopped")));
        if !ai::apply_cleanup_result(&mut transcript, cleaned) {
            log::warn!("AI cleanup failed; preserving original transcript");
            let _ = app_handle.emit("processing-notice", "AI cleanup was unavailable. Your original text was kept.");
        }
        log::info!("pipeline: AI cleanup took {} ms", t_cleanup.elapsed().as_millis());
    }

    // Translation is explicit and independent of recognition/cleanup. A failed
    // translation preserves the original and must never insert the wrong language.
    let original = transcript.clone();
    let mut translation_error = None;
    let translating = cfg.output_language != "same" && !transcript.is_empty();
    if translating {
        let cfg_for_translation = cfg.clone();
        let translation_target = original_app.clone();
        let translation_authority = app_handle.clone();
        let source = transcript.clone();
        let result = tokio::task::spawn_blocking(move || {
            let inference_epoch = {
                let state = translation_authority.state::<AppState>();
                let current = state.config.lock().unwrap();
                if !current.allows_translation_snapshot(&cfg_for_translation, translation_target.as_deref()) {
                    return Err(anyhow::anyhow!("Translation settings changed. Start a new dictation."));
                }
                subscriptions::inference_epoch()
            };
            let model_path = if cfg_for_translation.translation_engine == "local" {
                Some(transcription::model::verify_model_file(&translation_authority, translation::LOCAL_MODEL_NAME, translation::LOCAL_MODEL_SHA256)?)
            } else { None };
            let result = translation::translate_guarded(&source, &cfg_for_translation, model_path.as_deref(), inference_epoch);
            let current = translation_authority.state::<AppState>().config.lock().unwrap().clone();
            if !current.allows_translation_snapshot(&cfg_for_translation, translation_target.as_deref()) {
                return Err(anyhow::anyhow!("Translation settings changed during processing."));
            }
            result
        })
            .await.unwrap_or_else(|_| Err(anyhow::anyhow!("The translation worker stopped")));
        match result {
            Ok(text) => transcript = text,
            Err(error) => translation_error = Some(format!("Translation failed: {error}")),
        }
    }

    // 5. Persist history as soon as we have text, then inject. Injection can
    // fail (UIPI, no focused field) without that meaning the dictation is lost.
    if !transcript.is_empty() {
        let mut history_saved = false;
        if let Ok(dir) = app_handle.path().app_data_dir() {
            // Prefer whisper's detected language; fall back to a forced config value.
            let language = detected_language.clone().or_else(|| match cfg.language.trim() {
                "" | "auto" => None,
                l => Some(l.to_string()),
            });
            let entry = history::HistoryEntry {
                text: transcript.clone(),
                timestamp: now_millis(),
                language: if translating && translation_error.is_none() { Some(cfg.output_language.clone()) } else { language.clone() },
                source_text: if translating { Some(original.clone()) } else { None },
                source_language: if translating { language } else { None },
                translation_error: translation_error.clone(),
            };
            if history::append(&dir, entry.clone()).is_ok() {
                history_saved = true;
                let _ = app_handle.emit("history-added", entry);
            }
        }

        if let Some(error) = translation_error {
            let error = if history_saved { format!("{error}. Your original text is saved in Dictations; nothing was inserted.") }
                else { format!("{error}. The original could not be saved to history; nothing was inserted. Check available disk space and try again.") };
            let _ = app_handle.emit("processing-error", &error);
            show_app_window(app_handle, false);
            return Err(error);
        }

        let t_inject = std::time::Instant::now();
        if let Some(code) = remote_destination {
            let handle = app_handle.clone();
            let text = transcript.clone();
            let delivered = tokio::task::spawn_blocking(move || {
                handle.state::<remote_dictation::RemoteState>().send(&handle, &code, Some(&text))
                    .map_err(|e| e.to_string())
            }).await.map_err(|e| e.to_string())?;
            if let Err(error) = delivered {
                remote_dictation::report_error(app_handle, &error);
                return Err(error);
            }
        } else {
            injection::inject_text(&transcript, &cfg.injection_mode).map_err(|e| e.to_string())?;
        }
        log::info!("pipeline: injection took {} ms", t_inject.elapsed().as_millis());
    }

    do_set_state("idle", app_state, app_handle)?;
    Ok(transcript)
}

/// The Organic token gate, checked at the mouth of every dictation.
///
/// Returns the blocking status when dictation must be refused, `None` when it
/// may proceed. Only fully `Locked` and `Unlinked` wallets are refused: a
/// holder inside the grace window keeps dictating and sees a banner instead.
/// Nothing here touches an in-flight dictation — the gate can only decline to
/// start one.
fn gate_blocks_dictation(app: &tauri::AppHandle) -> Option<license::Status> {
    let status = app.state::<license::LicenseState>().status(now_millis());
    if status.allows_dictation() {
        None
    } else {
        log::info!("dictation refused by the Organic gate: {status:?}");
        // Bring Home forward so the refusal is explained rather than silent.
        let _ = app.emit("license-blocked", status.clone());
        show_app_window(app, true);
        Some(status)
    }
}

fn setup_blocks_dictation(app: &tauri::AppHandle) -> bool {
    let incomplete = !app.state::<AppState>().config.lock().unwrap().onboarding_completed || legal::requires_acceptance();
    if incomplete {
        let activation = app.state::<Activation>();
        activation.recording_active.store(false, Ordering::SeqCst);
        if let Some(hook) = activation.hook.lock().unwrap().as_ref() { hook.set_toggle_state(false); }
        let _ = app.emit("processing-error", "Complete setup and review the current Terms before dictating.");
        show_app_window(app, false);
    }
    incomplete
}

/// A receive-only client must ignore the physical hotkey forwarded by Input Leap.
fn receiver_blocks_capture(app: &tauri::AppHandle) -> bool {
    if app.state::<remote_dictation::RemoteState>().mode() != remote_dictation::Mode::Receive {
        return false;
    }
    let activation = app.state::<Activation>();
    activation.recording_active.store(false, Ordering::SeqCst);
    if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
        hook.set_toggle_state(false);
    }
    true
}

/// One dictation cycle: record → transcribe → inject.
#[tauri::command]
async fn run_pipeline(
    app_state: tauri::State<'_, AppState>,
    whisper_state: tauri::State<'_, WhisperState>,
    activation: tauri::State<'_, Activation>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    if setup_blocks_dictation(&app_handle) || receiver_blocks_capture(&app_handle) { return Ok(String::new()); }
    if gate_blocks_dictation(&app_handle).is_some() {
        return Err("Lectus is locked: link a wallet holding ORGANIC or Mycel.".into());
    }
    let local = whisper_state.local.clone();
    let cloud = whisper_state.cloud.clone();
    let recording_active = activation.recording_active.clone();
    // Snapshot config with the focused app's profile applied — the focused
    // app at trigger time is where the text will land.
    let original_app = context::foreground_app();
    let cfg = app_state
        .config
        .lock()
        .unwrap()
        .with_app_profile(original_app.as_deref());
    recording_active.store(true, Ordering::SeqCst);
    let outcome = do_pipeline(
        &app_state,
        &app_handle,
        recording_active.clone(),
        local,
        cloud,
        cfg,
        original_app,
    )
    .await;
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
        playback_on(&app_handle, playback::PlaybackEvent::PipelineError);
        do_set_state("idle", &app_state, &app_handle).ok();
    }
    outcome
}

/// Run one dictation cycle triggered by the hold-to-talk hook.
async fn run_hold_pipeline(app: tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let whisper = app.state::<WhisperState>();
    let activation = app.state::<Activation>();
    let local = whisper.local.clone();
    let cloud = whisper.cloud.clone();
    let recording_active = activation.recording_active.clone();
    let original_app = context::foreground_app();
    let cfg = app_state
        .config
        .lock()
        .unwrap()
        .with_app_profile(original_app.as_deref());

    let outcome = do_pipeline(
        app_state.inner(),
        &app,
        recording_active.clone(),
        local,
        cloud,
        cfg,
        original_app,
    )
    .await;

    recording_active.store(false, Ordering::SeqCst);
    if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
        hook.set_toggle_state(false);
    }
    if outcome.is_err() {
        playback_on(&app, playback::PlaybackEvent::PipelineError);
        do_set_state("idle", app_state.inner(), &app).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Default to info-level logs (stage timings etc.); RUST_LOG overrides.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // A newer chirp.exe (local cargo target vs an older NSIS install)
            // must actually replace this process. Otherwise the second launch
            // just focuses the stale window and the user keeps seeing 0.6.0.
            #[cfg(windows)]
            if let Ok(running) = std::env::current_exe() {
                if let Some(newer) = autostart::takeover_exe(&running, &args) {
                    autostart::spawn_replacing(&newer);
                    force_exit(0);
                }
            }
            user_opened_app(app);
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .manage(updates::PendingUpdate::new())
        .manage(updates::LastUpdateError(Mutex::new(None)))
        .manage(AppState::with_config(config::Config::load_or_default()))
        .manage(license::LicenseState::new(license::commands::load_at_startup(
            &config::Config::app_support_dir(),
        )))
        .manage(WhisperState {
            local: Arc::new(Mutex::new(None)),
            cloud: Arc::new(Mutex::new(None)),
        })
        .manage(worker::TranscribeWorker::new())
        .manage(Activation {
            recording_active: Arc::new(AtomicBool::new(false)),
            hook: Mutex::new(None),
        })
        .manage(PreRoll(Mutex::new(None)))
        .manage(remote_dictation::RemoteState::new())
        .manage(api_adapter::ApiServiceState::default())
        .manage(playback::platform_ducker())
        .invoke_handler(tauri::generate_handler![
            list_input_devices,
            api_adapter::get_api_status,
            api_adapter::save_api_settings,
            api_adapter::get_api_token,
            api_adapter::regenerate_api_token,
            set_recording_state,
            toggle_recording,
            run_pipeline,
            get_config,
            legal::get_legal_status,
            model_license::get_cleanup_model_license,
            model_license::accept_cleanup_model_license,
            model_license::revoke_cleanup_model_license,
            legal::open_legal_notices,
            legal::accept_legal_terms,
            translation_targets,
            get_translation_model_status,
            download_translation_model,
            save_config,
            complete_onboarding,
            engine_ready,
            show_home,
            save_pill_position,
            check_claude_cli,
            subscriptions::get_ai_connections,
            subscriptions::start_ai_login,
            subscriptions::cancel_ai_login,
            subscriptions::disconnect_ai,
            subscriptions::list_subscription_models,
            subscriptions::test_subscription,
            get_cloud_models,
            get_history,
            get_history_stats,
            clear_history,
            open_help,
            os_display_name,
            copy_to_clipboard,
            get_models_status,
            download_model,
            cancel_model_download,
            hardware_profile,
            select_model,
            get_cleanup_model_status,
            download_cleanup_model,
            get_foreground_app,
            accessibility_status,
            microphone_status,
            request_microphone_access,
            open_accessibility_settings,
            relaunch_app,
            license::commands::license_status,
            license::commands::license_floor,
            license::commands::link_wallet,
            license::commands::unlink_wallet,
            license::commands::refresh_license,
            updates::check_for_updates,
            updates::last_update_error,
            remote_dictation::remote_get_settings,
            remote_dictation::remote_save_settings,
            remote_dictation::remote_test_connection,
            updates::restart_and_apply,
            updates::take_whats_new_notes,
        ])
        .setup(|app| {
            // Menu-bar app: no Dock icon, no app switcher entry. The settings
            // window is reached via the tray (its handlers show()+set_focus()).
            // If keyboard focus ever proves unreliable under Accessory, the
            // fallback is flipping Regular↔Accessory on settings show/hide.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Finder-style sidebar vibrancy behind the settings webview. The
            // window + sidebar CSS are transparent on macOS so the material
            // shows through; the content pane stays opaque.
            #[cfg(target_os = "macos")]
            if let Some(w) = app.get_webview_window("settings") {
                let _ = window_vibrancy::apply_vibrancy(
                    &w,
                    window_vibrancy::NSVisualEffectMaterial::Sidebar,
                    None,
                    None,
                );
            }

            let settings_item = MenuItem::with_id(
                app,
                "settings",
                "Home",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Lectus", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;
            // Start on the real idle tray asset (the app icon looked wrong in the
            // tray until the first state change) and keep the platform's tray
            // conventions: macOS = template glyph + menu on any click; Windows =
            // colored icon, left-click opens Settings, right-click shows the menu.
            let idle_icon = app
                .path()
                .resource_dir()
                .ok()
                .and_then(|d| {
                    tauri::image::Image::from_path(
                        d.join(tray_icon_path(&RecordingState::Idle)),
                    )
                    .ok()
                })
                .unwrap_or_else(|| app.default_window_icon().unwrap().clone());
            let tray = TrayIconBuilder::with_id("main")
                .icon(idle_icon)
                .tooltip("Lectus")
                .menu(&menu)
                // Left click opens Settings directly on both platforms (one
                // click, not click-then-click-Settings-in-the-menu); the
                // dropdown menu (Settings…/Quit) is right-click only, which
                // is macOS's native convention for menu-bar extras anyway.
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => show_app_window(app, true),
                    // Not app.exit(0): that runs the normal libc exit path,
                    // which crashes on ggml-metal teardown — see force_exit.
                    "quit" => force_exit(0),
                    _ => {}
                });
            #[cfg(target_os = "macos")]
            let tray = tray.icon_as_template(true);
            let tray = tray.on_tray_icon_event(|tray, event| {
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    user_opened_app(tray.app_handle());
                }
            });
            tray.build(app)?;
            api_adapter::initialize(app.handle());

            if let Some(w) = app.get_webview_window("settings") {
                let w_for_event = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_for_event.hide();
                    }
                });
            }

            if let Ok(cfg_dir) = app.path().app_config_dir() {
                let cfg_path = cfg_dir.join("config.json");
                if let Ok(loaded) = config::Config::load_from(&cfg_path) {
                    *app.state::<AppState>().config.lock().unwrap() = loaded;
                }
            }

            // Launch-at-login must point at the NSIS install, not a leftover
            // cargo-target copy. Repair on every boot so a later local run
            // that re-enabled autostart cannot stick.
            #[cfg(windows)]
            {
                use tauri_plugin_autostart::ManagerExt;
                let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                autostart::repair_windows_run_keys(enabled);
            }

            // First run: surface onboarding. User double-click (no --autostart)
            // opens Home. Login-start stays in the tray.
            let needs_onboarding =
                !app.state::<AppState>().config.lock().unwrap().onboarding_completed;
            if needs_onboarding || !launched_by_autostart() {
                show_app_window(app.handle(), !needs_onboarding);
            }

            remote_dictation::start_at_login(app.handle());

            // Start the always-on pre-roll capture stream (500 ms ring).
            {
                let device = app.state::<AppState>().config.lock().unwrap().input_device.clone();
                restart_preroll(&app.handle().clone(), &device);
            }

            if let Some(pill) = app.get_webview_window("pill") {
                let (x, y) = {
                    let st = app.state::<AppState>();
                    let cfg = st.config.lock().unwrap();
                    (cfg.pill_x, cfg.pill_y)
                };
                let _ = pill.set_position(tauri::PhysicalPosition::new(x, y));
                let _ = pill.show();
            }

            // Resolve the active model: prefer the downloaded multilingual model;
            // otherwise fall back to the bundled tiny.en so the app works on first run.
            // Load off this thread so tray + hotkey appear immediately — blocking
            // here on large-v3-turbo is what made every daily launch feel dead.
            let model_name = app.state::<AppState>().config.lock().unwrap().model_name.clone();
            let resolved = transcription::model::model_path(app.handle(), &model_name)
                .ok()
                .filter(|p| p.exists())
                .or_else(|| {
                    app.path()
                        .resource_dir()
                        .ok()
                        .map(|d| d.join("models/ggml-tiny.en.bin"))
                        .filter(|p| p.exists())
                });
            if let Some(path) = resolved.clone() {
                app.state::<AppState>().config.lock().unwrap().model_path = path;
            }
            let already_downloaded =
                transcription::model::is_downloaded(app.handle(), &model_name);
            {
                let load_permit = MODEL_LOAD.begin(&model_name)?;
                let bg = app.handle().clone();
                let path_for_load = resolved;
                let name_for_events = model_name.clone();
                let name_for_download = model_name;
                let _ = bg.emit("model-loading", name_for_events.clone());
                std::thread::spawn(move || {
                    let _load_permit = load_permit;
                    if let Some(path) = path_for_load {
                        match transcription::local::LocalWhisper::new(&path) {
                            Ok(engine) => {
                                *bg.state::<WhisperState>().local.lock().unwrap() = Some(engine);
                                *ACTIVE_MODEL.lock().unwrap() = path.file_name().map(|n| n.to_string_lossy().into_owned());
                                bg.state::<AppState>().config.lock().unwrap().model_path = path;
                                warmup_engine(&bg);
                                let _ = bg.emit("model-active", name_for_events.clone());
                            }
                            Err(e) => {
                                eprintln!("warning: could not load whisper model: {e}");
                                let _ = bg.emit("model-load-failed", e.to_string());
                            }
                        }
                    }
                    if !already_downloaded {
                        match transcription::model::ensure_model(&bg, &name_for_download) {
                            Ok(path) => match transcription::local::LocalWhisper::new(&path) {
                                Ok(engine) => {
                                    *bg.state::<WhisperState>().local.lock().unwrap() = Some(engine);
                                    *ACTIVE_MODEL.lock().unwrap() = Some(name_for_download.clone());
                                    bg.state::<AppState>().config.lock().unwrap().model_path = path;
                                    warmup_engine(&bg);
                                    let _ = bg.emit("model-active", name_for_download);
                                    eprintln!(
                                        "multilingual model loaded; language auto-detect active"
                                    );
                                }
                                Err(e) => {
                                    eprintln!("warning: failed to load downloaded model: {e}");
                                    let _ = bg.emit("model-load-failed", e.to_string());
                                }
                            },
                            Err(e) => {
                                eprintln!("warning: multilingual model not available: {e}")
                            }
                        }
                    }
                });
            }

            // Fetch the tiny Silero VAD model in background (one-time, ~0.9 MB).
            {
                let bg = app.handle().clone();
                if !transcription::model::is_downloaded(app.handle(), transcription::model::VAD_MODEL_NAME) {
                    std::thread::spawn(move || {
                        if let Err(e) = transcription::model::ensure_file_from_url(
                            &bg,
                            &transcription::model::vad_model_url(),
                            transcription::model::VAD_MODEL_NAME,
                        ) {
                            log::warn!("VAD model download failed (VAD stays off): {e}");
                        }
                    });
                }
            }

            // Warm the local cleanup LLM so the first dictation doesn't pay
            // the model-load cost. Only when it's the selected engine.
            {
                let cfg = app.state::<AppState>().config.lock().unwrap().clone();
                if cfg.ai_cleanup_enabled && cfg.ai_cleanup_engine == "local" {
                    let bg = app.handle().clone();
                    std::thread::spawn(move || {
                        if let Ok(path) =
                            transcription::model::model_path(&bg, ai::local_llm::CLEANUP_MODEL_NAME)
                        {
                            if path.exists() {
                                if let Err(e) = ai::local_llm::warmup(&path) {
                                    log::warn!("cleanup LLM warmup failed: {e}");
                                }
                            }
                        }
                    });
                }
            }

            {
                let cfg = app.state::<AppState>().config.lock().unwrap().clone();
                *app.state::<WhisperState>().cloud.lock().unwrap() =
                    Some(transcription::cloud::CloudWhisper::with_model(
                        &cfg.cloud_base_url,
                        if cfg.cloud_processing_acknowledged { &cfg.cloud_api_key } else { "" },
                        &cfg.cloud_transcription_model,
                    ));
            }

            let (active_key, trigger_mode) = {
                let app_state = app.state::<AppState>();
                let cfg = app_state.config.lock().unwrap();
                (active_trigger_key(&cfg), cfg.trigger_mode.clone())
            };
            let recording_active = app.state::<Activation>().recording_active.clone();
            match hook::PlatformHook::install(HookContext {
                app: app.handle().clone(),
                recording_active,
                target_key: active_key,
                trigger_mode,
            }) {
                Ok(installed) => {
                    *app.state::<Activation>().hook.lock().unwrap() = Some(installed);

                    // Watchdog: Windows silently drops WH_KEYBOARD_LL hooks whose
                    // callback times out (sleep, RDP, load spikes) — the hotkey
                    // then dies until restart. Renew the registration every 60 s,
                    // skipping while a dictation is active.
                    let watchdog = app.handle().clone();
                    std::thread::spawn(move || loop {
                        std::thread::sleep(std::time::Duration::from_secs(60));
                        let activation = watchdog.state::<Activation>();
                        if activation.recording_active.load(Ordering::SeqCst) {
                            continue;
                        }
                        let mut guard = activation.hook.lock().unwrap();
                        if let Some(hook) = guard.as_mut() {
                            hook.reinstall();
                        }
                        drop(guard);
                    });
                }
                Err(e) => eprintln!("warning: keyboard hook not installed: {e}"),
            }

            // Background update check: query the GitHub Releases feed, download any newer build (signature-verified),
            // and notify Settings. All failures are silent/log-only — never blocks dictation. Install only on user click.
            updates::spawn_background_check(app.handle().clone());

            // Organic token gate: re-read the linked wallet's balance if the
            // cadence is due. Silent and failure-tolerant — see license::commands.
            license::commands::spawn_background_check(app.handle().clone());

            let pipeline_handle = app.handle().clone();
            app.listen("hold-start", move |_event| {
                let h = pipeline_handle.clone();
                // Ahead of the chime and the ducker: a refusal must not mute
                // the user's audio or click at them.
                if setup_blocks_dictation(&h) || receiver_blocks_capture(&h) || gate_blocks_dictation(&h).is_some() {
                    return;
                }
                playback_on(&h, playback::PlaybackEvent::HoldStart);
                tauri::async_runtime::spawn(run_hold_pipeline(h));
            });
            let stop_handle = app.handle().clone();
            app.listen("hold-stop", move |_event| {
                playback_on(&stop_handle, playback::PlaybackEvent::HoldStop);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                playback_on(app_handle, playback::PlaybackEvent::ProcessExit);
                api_adapter::stop(app_handle);
            }
            // No Dock icon (Accessory policy) means no automatic reopen
            // behavior: without this, clicking the app again while it's
            // already running does nothing visible, which reads as "it
            // won't open" even though it's running.
            // (RunEvent::Reopen only exists on macOS.)
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                user_opened_app(app_handle);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{args_mean_autostart, safety_cap_secs, should_continue};

    #[test]
    fn failed_config_persistence_preserves_live_settings() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-directory");
        std::fs::write(&blocker, "fixture").unwrap();
        let mut live = super::config::Config::default();
        let before = live.clone();
        let mut next = live.clone();
        next.display_name = "Changed".into();
        let result = super::commit_config_snapshot(&mut live, next, &blocker.join("config.json"));
        assert!(result.is_err());
        assert_eq!(live, before);
    }

    #[test]
    fn successful_config_persistence_commits_disk_and_memory_together() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut live = super::config::Config::default();
        let mut next = live.clone();
        next.display_name = "Changed".into();
        super::commit_config_snapshot(&mut live, next.clone(), &path).unwrap();
        assert_eq!(live, next);
        assert_eq!(super::config::Config::load_from(&path).unwrap(), next);
    }

    #[test]
    fn autostart_flag_is_detected() {
        assert!(args_mean_autostart(["chirp.exe", "--autostart"]));
        assert!(!args_mean_autostart(["chirp.exe"]));
        assert!(!args_mean_autostart(["chirp.exe", "--something-else"]));
    }

    #[test]
    fn continues_while_active_and_under_cap() {
        assert!(should_continue(true, 0, 30));
        assert!(should_continue(true, 29, 30));
    }

    #[test]
    fn stops_on_release() {
        assert!(!should_continue(false, 0, 30));
    }

    #[test]
    fn stops_at_safety_cap() {
        assert!(!should_continue(true, 30, 30));
        assert!(!should_continue(true, 45, 30));
        assert!(should_continue(true, 45, 300));
    }

    #[test]
    fn safety_cap_is_uniform_runaway_guard() {
        // Unified to 300 s for every mode — long dictations are normal.
        assert_eq!(safety_cap_secs("hold"), 300);
        assert_eq!(safety_cap_secs("toggle"), 300);
        assert_eq!(safety_cap_secs("TOGGLE"), 300);
        assert_eq!(safety_cap_secs("whatever"), 300);
    }
}
