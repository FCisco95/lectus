//! Tauri integration for the local upload-only API. This adapter never captures
//! audio, injects text, writes history, downloads models or calls a cloud engine.
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

use crate::{api, config, transcription, AppState, RecordingState, WhisperState};

#[derive(Default)]
pub struct ApiServiceState {
    service: Mutex<Option<api::Service>>,
    initialization_error: Mutex<Option<String>>,
}

struct NativeBackend(tauri::AppHandle);

fn error(code: &str, status: u16, message: &str) -> api::ApiError {
    api::ApiError { code: code.into(), status, message: message.into() }
}

impl api::Backend for NativeBackend {
    fn allowed(&self) -> bool {
        self.0.state::<AppState>().config.lock().unwrap().onboarding_completed
            && !crate::legal::requires_acceptance()
            && self.0.state::<crate::license::LicenseState>().status(crate::now_millis()).allows_dictation()
    }

    fn busy(&self) -> bool {
        *self.0.state::<AppState>().recording.lock().unwrap() != RecordingState::Idle
            || crate::MODEL_LOAD.current().is_some()
    }

    fn ready(&self) -> bool {
        self.0.state::<WhisperState>().local.try_lock()
            .map(|engine| engine.is_some()).unwrap_or(false)
    }

    fn transcribe(&self, samples: Vec<f32>, language: Option<String>, cleanup: bool) -> Result<api::Transcript, api::ApiError> {
        let app = self.0.clone();
        let worker = app.state::<crate::worker::TranscribeWorker>().inner().clone();
        // Check again on the inference thread: a dictation may have begun while
        // a caller uploaded its WAV or waited for the worker to accept this job.
        worker.run(move || {
            let backend = NativeBackend(app.clone());
            if !backend.allowed() { return Err(error("locked", 403, "Membership is required.")); }
            if backend.busy() { return Err(error("busy", 503, "Dictation or model loading is active. Try again when idle.")); }
            let cfg = app.state::<AppState>().config.lock().unwrap().clone();
            let language = language.as_deref().unwrap_or(&cfg.language).trim();
            if language != "auto" && !language.is_empty()
                && (language.len() > 8 || !language.bytes().all(|b| b.is_ascii_alphabetic()) || whisper_rs::get_lang_id(language).is_none()) {
                return Err(error("bad_language", 400, "Choose a supported Whisper language code, or auto."));
            }
            let mut opts = transcription::TranscribeOptions::from_config(
                language, transcription::dictionary::bias_prompt(&cfg.dictionary_words),
            );
            if cfg.vad_enabled {
                let path = transcription::model::model_path(&app, transcription::model::VAD_MODEL_NAME)
                    .map_err(|_| error("engine_unavailable", 503, "Local model directory is unavailable."))?;
                if path.is_file() { opts.vad_model_path = path.to_str().map(str::to_owned); }
            }
            let duration_ms = samples.len() as u64 * 1000 / 16_000;
            let start = std::time::Instant::now();
            let state = app.state::<WhisperState>();
            let (text, detected_language) = {
                let mut local = state.local.lock().unwrap();
                let engine = local.as_mut().ok_or_else(|| error("engine_unavailable", 503, "Download and select a local model in Settings."))?;
                engine.transcribe(&samples, &opts)
                    .map_err(|_| error("transcription_failed", 500, "The local model could not transcribe this audio."))?
            };
            let rule_language = transcription::dictionary::effective_language(language, detected_language.as_deref());
            let mut text = transcription::dictionary::apply_sound_alikes(&text, &cfg.sound_alikes, rule_language.as_deref());
            text = transcription::dictionary::apply_dictionary_spellings(&text, &cfg.dictionary_words);
            text = transcription::dictionary::apply_rules(&text, &cfg.replacement_rules);
            // The API's local-only guarantee overrides the old spec's ambiguous
            // cleanup paragraph. Cloud and CLI engines are never called here.
            if cleanup && cfg.ai_cleanup_enabled && cfg.ai_cleanup_engine == "local" {
                if let Ok(path) = transcription::model::model_path(&app, crate::ai::local_llm::CLEANUP_MODEL_NAME) {
                    if path.is_file() {
                        if let Ok(cleaned) = crate::ai::cleanup(&text, &cfg, Some(&path), detected_language.as_deref()) {
                            text = cleaned;
                        }
                    }
                }
            }
            Ok(api::Transcript {
                text: text.trim().to_string(), language: detected_language, duration_ms,
                transcribe_ms: start.elapsed().as_millis() as u64, engine: "local".into(),
            })
        })
    }
}

fn notify(app: &tauri::AppHandle, status: api::Status) {
    let tooltip = if status.busy && !status.running {
        "Lectus — finishing current API request".into()
    } else if status.running {
        if status.busy { format!("Lectus — API processing (port {})", status.port) }
        else { format!("Lectus — API on (port {})", status.port) }
    } else { "Lectus".into() };
    if let Some(tray) = app.tray_by_id("main") { let _ = tray.set_tooltip(Some(tooltip)); }
    let _ = app.emit("api-activity", status);
}

pub fn initialize(app: &tauri::AppHandle) {
    let notification_app = app.clone();
    let result = api::Service::new(
        config::Config::app_support_dir().join("api.json"),
        Arc::new(NativeBackend(app.clone())),
        Arc::new(move |status| notify(&notification_app, status)),
    );
    let state = app.state::<ApiServiceState>();
    match result {
        Ok(service) => {
            // A busy port is an API error, never a reason to crash dictation.
            if let Err(e) = service.start() { log::warn!("Local API could not start: {}", e.message); }
            notify(app, service.status());
            *state.service.lock().unwrap() = Some(service);
        }
        Err(_) => {
            *state.initialization_error.lock().unwrap() = Some("Local API settings could not be loaded. Check api.json in the app data folder.".into());
        }
    }
}

pub fn stop(app: &tauri::AppHandle) {
    if let Some(service) = app.state::<ApiServiceState>().service.lock().unwrap().as_ref() { service.stop(); }
}

fn with_service<T>(state: &ApiServiceState, action: impl FnOnce(&api::Service) -> Result<T, String>) -> Result<T, String> {
    let guard = state.service.lock().unwrap();
    match guard.as_ref() {
        Some(service) => action(service),
        None => Err(state.initialization_error.lock().unwrap().clone().unwrap_or_else(|| "Local API is initializing. Try again.".into())),
    }
}

#[tauri::command]
pub fn get_api_status(state: tauri::State<ApiServiceState>) -> Result<api::Status, String> {
    with_service(&state, |service| Ok(service.status()))
}

#[tauri::command]
pub fn save_api_settings(settings: api::Settings, state: tauri::State<ApiServiceState>) -> Result<api::Status, String> {
    with_service(&state, |service| service.configure(settings).map_err(|e| e.message))
}

#[tauri::command]
pub fn get_api_token(state: tauri::State<ApiServiceState>) -> Result<String, String> {
    with_service(&state, |service| Ok(service.token()))
}

#[tauri::command]
pub fn regenerate_api_token(state: tauri::State<ApiServiceState>) -> Result<String, String> {
    with_service(&state, |service| service.regenerate_token().map_err(|e| e.message))
}
