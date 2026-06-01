mod audio;
mod config;
mod hook;
mod hotkey;
mod injection;
mod state;
mod transcription;

use state::{AppState, RecordingState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Listener, Manager};
use hook::{HookContext, KeyboardHook};

/// Whether the capture loop should keep recording: active (key still held)
/// AND under the 30 s safety cap.
fn should_continue(active: bool, elapsed_secs: u64) -> bool {
    active && elapsed_secs < 30
}

/// Managed activation state: the shared stop flag + the installed hook.
struct Activation {
    recording_active: Arc<AtomicBool>,
    /// Kept alive for the app's lifetime; `rearm` swaps the key live.
    #[allow(dead_code)]
    hook: Mutex<Option<hook::PlatformHook>>,
}

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

    // Show the pill while busy, hide it when idle/error.
    if let Some(pill) = app_handle.get_webview_window("pill") {
        match next {
            RecordingState::Recording | RecordingState::Transcribing => {
                let _ = pill.show();
            }
            _ => {
                let _ = pill.hide();
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
    activation: tauri::State<Activation>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if new_config.use_cloud && new_config.cloud_api_key.trim().is_empty() {
        return Err("Cannot enable cloud transcription without an API key".to_string());
    }

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

    // Re-arm the hook to the (possibly new) hold key — no restart needed.
    {
        let key = app_state.config.lock().unwrap().hold_hotkey.clone();
        if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
            hook.rearm(&key);
        }
    }
    Ok(())
}

/// Inner pipeline logic. Separated so `run_pipeline` can reset state on any error path.
async fn do_pipeline(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    recording_active: Arc<AtomicBool>,
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
    // 1b. Enter Recording (emits state-changed, swaps tray icon, shows pill).
    do_set_state("recording", app_state, app_handle)?;

    // 2. Capture audio in a blocking thread (cpal::Stream is !Send).
    //    Stop when the key is released (recording_active clears) or at 30 s.
    let active = recording_active.clone();
    let accumulated = tokio::task::spawn_blocking(move || {
        use audio::AudioCapture;
        let capture = AudioCapture::start().map_err(|e| e.to_string())?;
        let mut accumulated: Vec<f32> = Vec::new();
        let start = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(32));
            let chunk = capture.drain();
            if !chunk.is_empty() {
                accumulated.extend_from_slice(&chunk);
            }
            if !should_continue(active.load(Ordering::SeqCst), start.elapsed().as_secs()) {
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
    activation: tauri::State<'_, Activation>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let local = whisper_state.local.clone();
    let cloud = whisper_state.cloud.clone();
    let recording_active = activation.recording_active.clone();
    let use_cloud = app_state.config.lock().unwrap().use_cloud;
    // Manual invoke path: behave like a tap (record until the safety cap),
    // since no physical key is being held. Pre-set the flag true.
    recording_active.store(true, Ordering::SeqCst);
    let outcome = do_pipeline(
        &app_state,
        &app_handle,
        recording_active.clone(),
        local,
        cloud,
        use_cloud,
    )
    .await;
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
        do_set_state("idle", &app_state, &app_handle).ok();
    }
    outcome
}

/// Run one dictation cycle triggered by the hold-to-talk hook.
/// Pulls managed state from the handle so it can be spawned from an event listener.
async fn run_hold_pipeline(app: tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let whisper = app.state::<WhisperState>();
    let activation = app.state::<Activation>();
    let local = whisper.local.clone();
    let cloud = whisper.cloud.clone();
    let recording_active = activation.recording_active.clone();
    let use_cloud = app_state.config.lock().unwrap().use_cloud;

    let outcome = do_pipeline(
        app_state.inner(),
        &app,
        recording_active.clone(),
        local,
        cloud,
        use_cloud,
    )
    .await;

    // Never leave the flag stuck true (e.g. after the 30 s cap while still held).
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
        do_set_state("idle", app_state.inner(), &app).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .manage(WhisperState {
            local: Arc::new(Mutex::new(None)),
            cloud: Arc::new(Mutex::new(None)),
        })
        .manage(Activation {
            recording_active: Arc::new(AtomicBool::new(false)),
            hook: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            set_recording_state,
            run_pipeline,
            get_config,
            save_config
        ])
        .setup(|app| {
            // Build the system tray with a Settings/Quit menu.
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Lectus", true, None::<&str>)?;
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

            // Keep the settings window alive across closes: hide instead of destroy,
            // so the tray "Settings" item can reopen it for the whole session.
            if let Some(w) = app.get_webview_window("settings") {
                let w_for_event = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_for_event.hide();
                    }
                });
            }

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

            // Install the low-level keyboard hook for hold-to-talk.
            // Non-fatal on failure (matches the startup philosophy): a missing
            // Accessibility grant (macOS) or hook error just disables hold-to-talk.
            let hold_key = {
                let app_state = app.state::<AppState>();
                let cfg = app_state.config.lock().unwrap();
                if hook::config_key_to_vk(&cfg.hold_hotkey).is_some() {
                    cfg.hold_hotkey.clone()
                } else {
                    "RControl".to_string() // migrate legacy combo configs
                }
            };
            let recording_active = app.state::<Activation>().recording_active.clone();
            match hook::PlatformHook::install(HookContext {
                app: app.handle().clone(),
                recording_active,
                target_key: hold_key,
            }) {
                Ok(installed) => {
                    *app.state::<Activation>().hook.lock().unwrap() = Some(installed);
                }
                Err(e) => eprintln!("warning: keyboard hook not installed: {e}"),
            }

            // Drive the pipeline directly from Rust on hold-start (no JS hop).
            let pipeline_handle = app.handle().clone();
            app.listen("hold-start", move |_event| {
                let h = pipeline_handle.clone();
                tauri::async_runtime::spawn(run_hold_pipeline(h));
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Lectus");
}

#[cfg(test)]
mod tests {
    use super::should_continue;

    #[test]
    fn continues_while_active_and_under_cap() {
        assert!(should_continue(true, 0));
        assert!(should_continue(true, 29));
    }

    #[test]
    fn stops_on_release() {
        assert!(!should_continue(false, 0));
    }

    #[test]
    fn stops_at_safety_cap() {
        assert!(!should_continue(true, 30));
        assert!(!should_continue(true, 45));
    }
}
