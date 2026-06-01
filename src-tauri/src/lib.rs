mod audio;
mod config;
mod hotkey;
mod injection;
mod state;
mod transcription;

use state::{AppState, RecordingState};
use tauri::{Emitter, Manager};

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

/// One dictation cycle: record until VAD silence → transcribe → inject text.
#[tauri::command]
async fn run_pipeline(
    app_state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // 1. Start recording
    do_set_state("recording", &app_state, &app_handle)?;

    // 2. Capture audio + VAD in a blocking thread (AudioCapture / cpal::Stream is !Send)
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
                break; // 30s safety cap
            }
        }
        drop(capture); // stop the CPAL stream
        Ok::<Vec<f32>, String>(accumulated)
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Transcribe (blocking — CPU heavy)
    do_set_state("transcribing", &app_state, &app_handle)?;
    let config = app_state.config.lock().unwrap().clone();
    let transcript = tokio::task::spawn_blocking(move || {
        let engine = transcription::local::LocalWhisper::new(&config.model_path)
            .map_err(|e| e.to_string())?;
        engine.transcribe(&accumulated).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // 4. Inject text into focused field
    if !transcript.is_empty() {
        injection::inject_text(&transcript).map_err(|e| e.to_string())?;
    }

    // 5. Return to idle
    do_set_state("idle", &app_state, &app_handle)?;
    Ok(transcript)
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
        .invoke_handler(tauri::generate_handler![set_recording_state, run_pipeline])
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
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
