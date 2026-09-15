mod ai;
mod audio;
mod autostart;
mod config;
mod context;
mod history;
mod hook;
mod hotkey;
mod injection;
mod state;
mod transcription;
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

/// Whether the Claude Code CLI is available for AI cleanup (shown in Settings).
#[tauri::command]
fn check_claude_cli() -> bool {
    ai::claude_available()
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
    let snapshot = {
        let mut guard = app_state.config.lock().unwrap();
        guard.pill_x = x;
        guard.pill_y = y;
        guard.clone()
    };
    if let Ok(cfg_dir) = app_handle.path().app_config_dir() {
        let cfg_path = cfg_dir.join("config.json");
        snapshot.save_to(&cfg_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Persist edited config to disk, rebuild the cloud engine, and update in-memory state.
#[tauri::command]
fn save_config(
    new_config: config::Config,
    app_state: tauri::State<AppState>,
    whisper_state: tauri::State<WhisperState>,
    activation: tauri::State<Activation>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if new_config.use_cloud && new_config.cloud_api_key.trim().is_empty() {
        return Err("Cannot enable cloud transcription without an API key".to_string());
    }

    let (device_changed, local_cleanup_enabled, theme_changed, snapshot) = {
        let mut guard = app_state.config.lock().unwrap();
        let changed = guard.input_device != new_config.input_device;
        let was_local = guard.ai_cleanup_enabled && guard.ai_cleanup_engine == "local";
        let now_local = new_config.ai_cleanup_enabled && new_config.ai_cleanup_engine == "local";
        let theme_changed = guard.theme != new_config.theme;
        guard.apply_ui_update(new_config);
        (
            changed,
            !was_local && now_local,
            theme_changed,
            guard.clone(),
        )
    };

    if let Ok(cfg_dir) = app_handle.path().app_config_dir() {
        snapshot
            .save_to(&cfg_dir.join("config.json"))
            .map_err(|e| e.to_string())?;
    }

    *whisper_state.cloud.lock().unwrap() = Some(transcription::cloud::CloudWhisper::new(
        &snapshot.cloud_base_url,
        &snapshot.cloud_api_key,
    ));

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
    Ok(())
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
}

#[tauri::command]
fn get_cleanup_model_status(app_handle: tauri::AppHandle) -> CleanupModelStatus {
    CleanupModelStatus {
        downloaded: transcription::model::is_downloaded(&app_handle, ai::local_llm::CLEANUP_MODEL_NAME),
        size_mb: ai::local_llm::CLEANUP_MODEL_SIZE_MB,
    }
}

/// Download the local cleanup LLM (progress via `model-download-progress`),
/// then load it resident so the first dictation doesn't pay the load cost.
#[tauri::command]
async fn download_cleanup_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
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

/// Run a short silent inference on the whisper worker so the GPU backend
/// compiles its pipelines up front instead of on the first real dictation
/// (~8 s of Vulkan shader compilation on first `whisper_full`).
fn warmup_engine(app: &tauri::AppHandle) {
    let local = app.state::<WhisperState>().local.clone();
    let worker = app.state::<worker::TranscribeWorker>().inner().clone();
    std::thread::spawn(move || {
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
    });
}

/// Return the download/active status of every known model.
#[tauri::command]
fn get_models_status(
    app_state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Vec<ModelStatus> {
    let current = app_state.config.lock().unwrap().model_name.clone();
    transcription::model::AVAILABLE_MODELS
        .iter()
        .map(|m| ModelStatus {
            name: m.name.to_string(),
            display_name: m.display_name.to_string(),
            size_mb: m.size_mb,
            multilingual: m.multilingual,
            description: m.description.to_string(),
            downloaded: transcription::model::is_downloaded(&app_handle, m.name),
            active: current == m.name,
        })
        .collect()
}

/// Kick off a background download for a model. Emits `model-download-progress`
/// (f64 0..1) and `model-ready` (String model_name) when done.
#[tauri::command]
fn download_model(model_name: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    // Validate the name is one we know about.
    if !transcription::model::AVAILABLE_MODELS.iter().any(|m| m.name == model_name) {
        return Err(format!("Unknown model: {model_name}"));
    }
    let app = app_handle.clone();
    std::thread::spawn(move || {
        if let Err(e) = transcription::model::ensure_model(&app, &model_name) {
            eprintln!("model download failed: {e}");
        }
    });
    Ok(())
}

/// Load a downloaded model and make it the active engine. Non-blocking — emits
/// `model-loading` (String model_name) immediately, then `model-active` (String)
/// once swapped in, or `model-load-failed` (String) on error.
#[tauri::command]
fn select_model(
    model_name: String,
    app_state: tauri::State<AppState>,
    whisper_state: tauri::State<WhisperState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path = transcription::model::model_path(&app_handle, &model_name)
        .map_err(|e| e.to_string())?;
    if !path.exists() {
        return Err(format!("Model '{model_name}' is not downloaded yet"));
    }

    let _ = app_handle.emit("model-loading", model_name.clone());

    let local = Arc::clone(&whisper_state.local);
    let app = app_handle.clone();
    let name_clone = model_name.clone();
    std::thread::spawn(move || {
        match transcription::local::LocalWhisper::new(&path) {
            Ok(engine) => {
                *local.lock().unwrap() = Some(engine);
                // Update in-memory config (model_name + model_path).
                let app_state = app.state::<AppState>();
                {
                    let mut cfg = app_state.config.lock().unwrap();
                    cfg.model_name = name_clone.clone();
                    cfg.model_path = path;
                }
                // Persist.
                if let Ok(cfg_dir) = app.path().app_config_dir() {
                    let cfg = app_state.config.lock().unwrap().clone();
                    let _ = cfg.save_to(&cfg_dir.join("config.json"));
                }
                warmup_engine(&app);
                let _ = app.emit("model-active", name_clone);
            }
            Err(e) => {
                let _ = app.emit("model-load-failed", e.to_string());
            }
        }
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
) -> Result<String, String> {
    let use_cloud = cfg.use_cloud;
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
    let (mut transcript, detected_language) = tokio::task::spawn_blocking(move || {
        worker.run(move || {
            if use_cloud {
                let guard = cloud.lock().unwrap();
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

    // 4. Post-process: replacement rules + optional AI cleanup.
    transcript = transcription::dictionary::apply_rules(&transcript, &cfg.replacement_rules);
    if cfg.ai_cleanup_enabled && !transcript.is_empty() {
        let t_cleanup = std::time::Instant::now();
        let to_clean = transcript.clone();
        let cfg_for_cleanup = cfg.clone();
        let llm_path = transcription::model::model_path(&app_handle, ai::local_llm::CLEANUP_MODEL_NAME).ok();
        let lang_for_cleanup = detected_language.clone();
        let cleaned = tokio::task::spawn_blocking(move || {
            ai::cleanup(&to_clean, &cfg_for_cleanup, llm_path.as_deref(), lang_for_cleanup.as_deref())
        })
        .await
        .map_err(|e| e.to_string())?;
        match cleaned {
            Ok(text) if !text.trim().is_empty() => transcript = text,
            Ok(_) => {}
            Err(e) => eprintln!("warning: AI cleanup failed, using raw transcript: {e}"),
        }
        log::info!("pipeline: AI cleanup took {} ms", t_cleanup.elapsed().as_millis());
    }

    // 5. Persist history as soon as we have text, then inject. Injection can
    // fail (UIPI, no focused field) without that meaning the dictation is lost.
    if !transcript.is_empty() {
        if let Ok(dir) = app_handle.path().app_data_dir() {
            // Prefer whisper's detected language; fall back to a forced config value.
            let language = detected_language.clone().or_else(|| match cfg.language.trim() {
                "" | "auto" => None,
                l => Some(l.to_string()),
            });
            let entry = history::HistoryEntry {
                text: transcript.clone(),
                timestamp: now_millis(),
                language,
            };
            if history::append(&dir, entry.clone()).is_ok() {
                let _ = app_handle.emit("history-added", entry);
            }
        }

        let t_inject = std::time::Instant::now();
        injection::inject_text(&transcript, &cfg.injection_mode).map_err(|e| e.to_string())?;
        log::info!("pipeline: injection took {} ms", t_inject.elapsed().as_millis());
    }

    do_set_state("idle", app_state, app_handle)?;
    Ok(transcript)
}

/// One dictation cycle: record → transcribe → inject.
#[tauri::command]
async fn run_pipeline(
    app_state: tauri::State<'_, AppState>,
    whisper_state: tauri::State<'_, WhisperState>,
    activation: tauri::State<'_, Activation>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let local = whisper_state.local.clone();
    let cloud = whisper_state.cloud.clone();
    let recording_active = activation.recording_active.clone();
    // Snapshot config with the focused app's profile applied — the focused
    // app at trigger time is where the text will land.
    let cfg = app_state
        .config
        .lock()
        .unwrap()
        .with_app_profile(context::foreground_app().as_deref());
    recording_active.store(true, Ordering::SeqCst);
    let outcome = do_pipeline(
        &app_state,
        &app_handle,
        recording_active.clone(),
        local,
        cloud,
        cfg,
    )
    .await;
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
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
    let cfg = app_state
        .config
        .lock()
        .unwrap()
        .with_app_profile(context::foreground_app().as_deref());

    let outcome = do_pipeline(
        app_state.inner(),
        &app,
        recording_active.clone(),
        local,
        cloud,
        cfg,
    )
    .await;

    recording_active.store(false, Ordering::SeqCst);
    if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
        hook.set_toggle_state(false);
    }
    if outcome.is_err() {
        do_set_state("idle", app_state.inner(), &app).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Default to info-level logs (stage timings etc.); RUST_LOG overrides.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(updates::PendingUpdate::new())
        .manage(updates::LastUpdateError(Mutex::new(None)))
        .manage(AppState::new())
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
        .invoke_handler(tauri::generate_handler![
            list_input_devices,
            set_recording_state,
            toggle_recording,
            run_pipeline,
            get_config,
            save_config,
            save_pill_position,
            check_claude_cli,
            get_history,
            clear_history,
            copy_to_clipboard,
            get_models_status,
            download_model,
            select_model,
            get_cleanup_model_status,
            download_cleanup_model,
            get_foreground_app,
            accessibility_status,
            microphone_status,
            request_microphone_access,
            open_accessibility_settings,
            relaunch_app,
            updates::check_for_updates,
            updates::last_update_error,
            updates::restart_and_apply,
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
                if cfg!(target_os = "macos") { "Settings…" } else { "Settings" },
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
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
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
                    if let Some(w) = tray.app_handle().get_webview_window("settings") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            });
            tray.build(app)?;

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

            // First run: surface the (normally hidden) settings window so the
            // onboarding flow can walk through permissions and the hotkey.
            let needs_onboarding =
                !app.state::<AppState>().config.lock().unwrap().onboarding_completed;
            if needs_onboarding {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }

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
            {
                let app_state = app.state::<AppState>();
                let model_name = app_state.config.lock().unwrap().model_name.clone();
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
                if let Some(path) = resolved {
                    app_state.config.lock().unwrap().model_path = path;
                }
            }

            // Pre-load the configured model into the engine cache.
            let model_path = app.state::<AppState>().config.lock().unwrap().model_path.clone();
            match transcription::local::LocalWhisper::new(&model_path) {
                Ok(engine) => {
                    *app.state::<WhisperState>().local.lock().unwrap() = Some(engine);
                    warmup_engine(&app.handle().clone());
                }
                Err(e) => {
                    eprintln!("warning: could not load whisper model at {model_path:?}: {e}");
                }
            }

            // On first run the bundled model is English-only. Download the
            // configured multilingual model in background, then hot-swap it.
            {
                let bg = app.handle().clone();
                let model_name = app.state::<AppState>().config.lock().unwrap().model_name.clone();
                let already = transcription::model::is_downloaded(&bg, &model_name);
                if !already {
                    std::thread::spawn(move || {
                        match transcription::model::ensure_model(&bg, &model_name) {
                            Ok(path) => match transcription::local::LocalWhisper::new(&path) {
                                Ok(engine) => {
                                    *bg.state::<WhisperState>().local.lock().unwrap() = Some(engine);
                                    bg.state::<AppState>().config.lock().unwrap().model_path = path;
                                    warmup_engine(&bg);
                                    eprintln!("multilingual model loaded; language auto-detect active");
                                }
                                Err(e) => eprintln!("warning: failed to load downloaded model: {e}"),
                            },
                            Err(e) => eprintln!("warning: multilingual model not available: {e}"),
                        }
                    });
                }
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
                    Some(transcription::cloud::CloudWhisper::new(
                        &cfg.cloud_base_url,
                        &cfg.cloud_api_key,
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

            let pipeline_handle = app.handle().clone();
            app.listen("hold-start", move |_event| {
                let h = pipeline_handle.clone();
                tauri::async_runtime::spawn(run_hold_pipeline(h));
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // No Dock icon (Accessory policy) means no automatic reopen
            // behavior: without this, clicking the app again while it's
            // already running does nothing visible, which reads as "it
            // won't open" even though it's running.
            // (RunEvent::Reopen only exists on macOS.)
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(w) = app_handle.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{safety_cap_secs, should_continue};

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
