mod audio;
mod config;
mod hotkey;
mod injection;
mod state;
mod transcription;

use state::{AppState, RecordingState};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

/// Cached Whisper engine. Loaded once at startup to avoid per-call disk I/O (~1–3 s overhead).
struct WhisperState(Arc<Mutex<Option<transcription::local::LocalWhisper>>>);

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

/// Inner pipeline logic. Separated so `run_pipeline` can reset state on any error path.
async fn do_pipeline(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    whisper_arc: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
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

    // 3. Transcribe using the cached engine (no model reload on each call).
    do_set_state("transcribing", app_state, app_handle)?;
    let transcript = tokio::task::spawn_blocking(move || {
        let guard = whisper_arc.lock().unwrap();
        match guard.as_ref() {
            Some(engine) => engine.transcribe(&accumulated).map_err(|e| e.to_string()),
            None => Err("Whisper model not loaded — run scripts/download_model.ps1 first".to_string()),
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
    let whisper_arc = whisper_state.0.clone();
    let outcome = do_pipeline(&app_state, &app_handle, whisper_arc).await;
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
        .manage(WhisperState(Arc::new(Mutex::new(None))))
        .invoke_handler(tauri::generate_handler![set_recording_state, run_pipeline])
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

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
                    *app.state::<WhisperState>().0.lock().unwrap() = Some(engine);
                }
                Err(e) => {
                    eprintln!("warning: could not load whisper model at {model_path:?}: {e}");
                }
            }

            // Register global hotkeys.
            let config = app.state::<AppState>().config.lock().unwrap().clone();
            let hk = hotkey::HotkeyManager::from_config(
                &config.hold_hotkey,
                &config.toggle_hotkey,
            )?;
            app.global_shortcut().register(hk.hold_shortcut.as_str())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Chirp");
}
