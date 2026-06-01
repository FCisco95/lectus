mod audio;
mod config;
mod hotkey;
mod injection;
mod state;
mod transcription;

use state::{AppState, RecordingState};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

/// Cached transcription engines. Loaded once at startup to avoid per-call setup cost.
struct WhisperState {
    local: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
    cloud: Arc<Mutex<Option<transcription::cloud::CloudWhisper>>>,
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

    let icon_path = match next {
        RecordingState::Recording => "icons/tray-recording.png",
        RecordingState::Transcribing => "icons/tray-transcribing.png",
        _ => "icons/tray-idle.png",
    };
    if let Some(tray) = app_handle.tray_by_id("main") {
        if let Ok(res_dir) = app_handle.path().resource_dir() {
            if let Ok(icon) = tauri::image::Image::from_path(res_dir.join(icon_path)) {
                tray.set_icon(Some(icon)).ok();
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn set_recording_state(
    state_str: String,
    app_state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    do_set_state(&state_str, &app_state, &app_handle)
}

/// Return the current in-memory config for the Settings UI to display.
#[tauri::command]
fn get_config(app_state: tauri::State<AppState>) -> config::Config {
    app_state.config.lock().unwrap().clone()
}

/// Persist edited config to disk, rebuild the cloud engine, and update in-memory state.
/// `model_path` from the UI is ignored — the bundled/resolved path is preserved.
#[tauri::command]
fn save_config(
    new_config: config::Config,
    app_state: tauri::State<AppState>,
    whisper_state: tauri::State<WhisperState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Persist to disk.
    if let Ok(cfg_dir) = app_handle.path().app_config_dir() {
        let cfg_path = cfg_dir.join("config.json");
        new_config.save_to(&cfg_path).map_err(|e| e.to_string())?;
    }

    // Rebuild the cloud engine with the new credentials/URL.
    *whisper_state.cloud.lock().unwrap() = Some(transcription::cloud::CloudWhisper::new(
        &new_config.cloud_base_url,
        &new_config.cloud_api_key,
    ));

    // Update in-memory config, preserving the resolved model_path.
    {
        let mut guard = app_state.config.lock().unwrap();
        let resolved_model = guard.model_path.clone();
        *guard = new_config;
        guard.model_path = resolved_model;
    }
    Ok(())
}

/// Inner pipeline logic. Separated so `run_pipeline` can reset state on any error path.
async fn do_pipeline(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    local: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
    cloud: Arc<Mutex<Option<transcription::cloud::CloudWhisper>>>,
    use_cloud: bool,
) -> Result<String, String> {
    // 1. Atomic claim: reject if another pipeline is already running (TOCTOU-safe).
    {
        let mut rec = app_state.recording.lock().unwrap();
        if *rec != RecordingState::Idle {
            return Ok(String::new());
        }
        *rec = RecordingState::Recording;
    }
    app_handle.emit("state-changed", "recording").map_err(|e| e.to_string())?;
    if let Some(tray) = app_handle.tray_by_id("main") {
        if let Ok(res_dir) = app_handle.path().resource_dir() {
            if let Ok(icon) = tauri::image::Image::from_path(res_dir.join("icons/tray-recording.png")) {
                tray.set_icon(Some(icon)).ok();
            }
        }
    }

    // 2. Capture audio + VAD in a blocking thread (cpal::Stream is !Send).
    let accumulated = tokio::task::spawn_blocking(|| {
        use audio::{AudioCapture, EnergyVad};
        let capture = AudioCapture::start().map_err(|e| e.to_string())?;
        let mut vad = EnergyVad::new(0.01);
        let mut accumulated: Vec<f32> = Vec::new();
        let deadline = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(32));
            let chunk = capture.drain();
            if !chunk.is_empty() {
                let ended = vad.speech_ended(&chunk);
                accumulated.extend_from_slice(&chunk);
                if ended {
                    break;
                }
            }
            if deadline.elapsed().as_secs() > 30 {
                break;
            }
        }
        drop(capture);
        Ok::<Vec<f32>, String>(accumulated)
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Transcribe using the selected cached engine (no reload per call).
    do_set_state("transcribing", app_state, app_handle)?;
    let transcript = tokio::task::spawn_blocking(move || {
        if use_cloud {
            let guard = cloud.lock().unwrap();
            match guard.as_ref() {
                Some(engine) => engine.transcribe(&accumulated).map_err(|e| e.to_string()),
                None => Err("Cloud backend not configured — set an API key in Settings".to_string()),
            }
        } else {
            let guard = local.lock().unwrap();
            match guard.as_ref() {
                Some(engine) => engine.transcribe(&accumulated).map_err(|e| e.to_string()),
                None => Err(
                    "Whisper model not loaded — run scripts/download_model.ps1 first".to_string(),
                ),
            }
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    // 4. Inject text into the focused field.
    if !transcript.is_empty() {
        injection::inject_text(&transcript).map_err(|e| e.to_string())?;
    }

    // 5. Return to idle.
    do_set_state("idle", app_state, app_handle)?;
    Ok(transcript)
}

/// One dictation cycle: record → VAD → transcribe → inject.
/// On any error, resets to Idle so future hotkey triggers are not blocked.
#[tauri::command]
async fn run_pipeline(
    app_state: tauri::State<'_, AppState>,
    whisper_state: tauri::State<'_, WhisperState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let local = whisper_state.local.clone();
    let cloud = whisper_state.cloud.clone();
    let use_cloud = app_state.config.lock().unwrap().use_cloud;
    let outcome = do_pipeline(&app_state, &app_handle, local, cloud, use_cloud).await;
    if outcome.is_err() {
        app_state.set_state(RecordingState::Idle);
        app_handle.emit("state-changed", "idle").ok();
    }
    outcome
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() == ShortcutState::Pressed {
                        app.emit("pipeline-start", ()).ok();
                    }
                })
                .build(),
        )
        .manage(AppState::new())
        .manage(WhisperState {
            local: Arc::new(Mutex::new(None)),
            cloud: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            set_recording_state,
            run_pipeline,
            get_config,
            save_config
        ])
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            // Build the system tray with a Settings/Quit menu.
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Chirp", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Load persisted config from disk (falls back to Config::default()).
            if let Ok(cfg_dir) = app.path().app_config_dir() {
                let cfg_path = cfg_dir.join("config.json");
                if let Ok(loaded) = config::Config::load_from(&cfg_path) {
                    *app.state::<AppState>().config.lock().unwrap() = loaded;
                }
            }

            // Resolve bundled model path at runtime.
            if let Ok(res_dir) = app.path().resource_dir() {
                let bundled = res_dir.join("models/ggml-tiny.en.bin");
                if bundled.exists() {
                    app.state::<AppState>().config.lock().unwrap().model_path = bundled;
                }
            }

            // Pre-load Whisper model into the cache.
            let model_path = app.state::<AppState>().config.lock().unwrap().model_path.clone();
            match transcription::local::LocalWhisper::new(&model_path) {
                Ok(engine) => {
                    *app.state::<WhisperState>().local.lock().unwrap() = Some(engine);
                }
                Err(e) => {
                    eprintln!("warning: could not load whisper model at {model_path:?}: {e}");
                }
            }

            // Pre-build the cloud engine from the (possibly persisted) config.
            {
                let cfg = app.state::<AppState>().config.lock().unwrap().clone();
                *app.state::<WhisperState>().cloud.lock().unwrap() =
                    Some(transcription::cloud::CloudWhisper::new(
                        &cfg.cloud_base_url,
                        &cfg.cloud_api_key,
                    ));
            }

            // Register global hotkeys. Failures here must NOT brick startup —
            // an unregistrable hotkey (e.g. a bare modifier, which muda rejects)
            // should log a warning and leave the app running.
            let config = app.state::<AppState>().config.lock().unwrap().clone();
            match hotkey::HotkeyManager::from_config(&config.hold_hotkey, &config.toggle_hotkey) {
                Ok(hk) => {
                    if let Err(e) = app.global_shortcut().register(hk.hold_shortcut.as_str()) {
                        eprintln!(
                            "warning: failed to register hold hotkey '{}': {e}",
                            hk.hold_shortcut
                        );
                    }
                }
                Err(e) => eprintln!("warning: invalid hotkey config: {e}"),
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Chirp");
}
