//! Test capture for the first-run wizard.
//!
//! The wizard lets the user check the microphone and say a word before setup
//! finishes. This module owns that capture. It never touches the hotkey
//! pipeline, the pre-roll stream or text injection, keeps audio in memory only
//! and writes no file.

use crate::{transcription, worker, Activation, AppState, WhisperState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// 15 seconds of 16 kHz mono audio. Older samples are dropped.
const MAX_SAMPLES: usize = 15 * 16_000;
/// Anything shorter than 0.3 seconds is treated as nothing recorded.
const MIN_SAMPLES: usize = 16_000 * 3 / 10;
const TICK: Duration = Duration::from_millis(50);

#[derive(Serialize)]
pub struct WizardTranscript {
    pub text: String,
    pub language: Option<String>,
    pub audio_ms: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
struct WizardMicError { session: Option<String>, message: String }

/// Managed state: the shared sample buffer, the running flag and the capture thread.
pub struct WizardCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl WizardCapture {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
            failure: Arc::new(Mutex::new(None)),
        }
    }

    fn stop(&self) {
        let _ = self.take_for_transcription();
    }

    fn take_for_transcription(&self) -> Result<Vec<f32>, String> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        match self.failure.lock().unwrap().take() {
            Some(message) => Err(message),
            None => Ok(samples),
        }
    }
}

/// Append `chunk` to `buf`, keeping only the newest `cap` samples.
pub fn push_capped(buf: &mut Vec<f32>, chunk: &[f32], cap: usize) {
    if chunk.len() >= cap {
        buf.clear();
        buf.extend_from_slice(&chunk[chunk.len() - cap..]);
        return;
    }
    buf.extend_from_slice(chunk);
    let excess = buf.len().saturating_sub(cap);
    if excess > 0 {
        buf.drain(..excess);
    }
}

/// Map a root-mean-square amplitude to a 0 to 1 meter level.
pub fn level_from_rms(rms: f32) -> f32 {
    (rms * 6.0).clamp(0.0, 1.0)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Start the test capture. A second call while it runs does nothing.
#[tauri::command]
pub fn wizard_mic_start(
    app: tauri::AppHandle,
    capture: tauri::State<WizardCapture>,
    device: Option<String>,
    session: Option<String>,
) -> Result<(), String> {
    if capture.running.load(Ordering::SeqCst) {
        return Ok(());
    }
    if app.state::<Activation>().recording_active.load(Ordering::SeqCst) {
        return Err("A dictation is in progress.".into());
    }
    // Reap a thread that ended by itself before starting a new one.
    if let Some(handle) = capture.thread.lock().unwrap().take() {
        let _ = handle.join();
    }
    capture.buffer.lock().unwrap().clear();
    *capture.failure.lock().unwrap() = None;

    let buffer = capture.buffer.clone();
    let running = capture.running.clone();
    let failure = capture.failure.clone();
    running.store(true, Ordering::SeqCst);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let emitter = app.clone();
    let handle = std::thread::Builder::new()
        .name("wizard-capture".into())
        .spawn(move || {
            // The cpal stream is not Send, so it is created, polled and dropped here.
            let error_emitter = emitter.clone();
            let error_running = running.clone();
            let audio = match crate::audio::AudioCapture::start_with_error_handler(device.as_deref(), move |message| {
                *failure.lock().unwrap() = Some(message.clone());
                error_running.store(false, Ordering::SeqCst);
                let _ = error_emitter.emit("wizard-mic-error", WizardMicError { session: session.clone(), message });
            }) {
                Ok(audio) => {
                    let _ = ready_tx.send(Ok(()));
                    audio
                }
                Err(e) => {
                    running.store(false, Ordering::SeqCst);
                    let _ = ready_tx.send(Err(format!("Could not open the microphone: {e}")));
                    return;
                }
            };
            while running.load(Ordering::SeqCst) {
                std::thread::sleep(TICK);
                let chunk = audio.drain();
                let level = level_from_rms(rms(&chunk));
                push_capped(&mut buffer.lock().unwrap(), &chunk, MAX_SAMPLES);
                let _ = emitter.emit("wizard-audio-level", level);
            }
            let tail = audio.stop_and_drain();
            push_capped(&mut buffer.lock().unwrap(), &tail, MAX_SAMPLES);
            let _ = emitter.emit("wizard-audio-level", 0.0f32);
        })
        .map_err(|e| format!("Could not start the microphone test: {e}"))?;

    match ready_rx.recv() {
        Ok(Ok(())) => {
            *capture.thread.lock().unwrap() = Some(handle);
            Ok(())
        }
        Ok(Err(message)) => {
            let _ = handle.join();
            Err(message)
        }
        Err(_) => {
            capture.running.store(false, Ordering::SeqCst);
            let _ = handle.join();
            Err("The microphone test stopped unexpectedly.".into())
        }
    }
}

/// Stop the capture, drop its audio and zero the meter.
#[tauri::command]
pub fn wizard_mic_stop(app: tauri::AppHandle, capture: tauri::State<WizardCapture>) {
    capture.stop();
    let _ = app.emit("wizard-audio-level", 0.0f32);
}

/// Transcribe what the capture heard, then clear it. `raw` returns the
/// recognizer's text untouched; otherwise sound-alikes, spellings and rules
/// are applied like a normal dictation.
#[tauri::command]
pub async fn wizard_transcribe(
    app: tauri::AppHandle,
    capture: tauri::State<'_, WizardCapture>,
    raw: bool,
) -> Result<WizardTranscript, String> {
    let samples = capture.take_for_transcription()?;
    let _ = app.emit("wizard-audio-level", 0.0f32);
    let local = app.state::<WhisperState>().local.clone();
    if local.lock().unwrap().is_none() {
        return Err("The speech model is not ready yet.".into());
    }
    if samples.len() < MIN_SAMPLES {
        return Err("Nothing was recorded.".into());
    }
    let cfg = app.state::<AppState>().config.lock().unwrap().clone();
    let mut opts = transcription::TranscribeOptions::from_config(&cfg.language, &cfg.dictionary_words);
    if cfg.vad_enabled {
        if let Ok(path) = transcription::model::model_path(&app, transcription::model::VAD_MODEL_NAME) {
            if path.exists() {
                opts.vad_model_path = path.to_str().map(String::from);
            }
        }
    }
    let worker = app.state::<worker::TranscribeWorker>().inner().clone();
    let audio_ms = samples.len() as u64 * 1000 / 16_000;

    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        // Through the shared worker so this never overlaps a dictation.
        let (text, language) = worker.run(move || {
            let mut guard = local.lock().unwrap();
            match guard.as_mut() {
                Some(engine) => engine.transcribe(&samples, &opts).map_err(|e| e.to_string()),
                None => Err("The speech model is not ready yet.".to_string()),
            }
        })?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let text = if raw {
            text
        } else {
            use transcription::dictionary::{
                apply_dictionary_spellings, apply_rules, apply_sound_alikes, effective_language,
            };
            let rule_language = effective_language(&cfg.language, language.as_deref());
            let text = apply_sound_alikes(&text, &cfg.sound_alikes, rule_language.as_deref());
            let text = apply_dictionary_spellings(&text, &cfg.dictionary_words);
            apply_rules(&text, &cfg.replacement_rules)
        };
        Ok(WizardTranscript { text: text.trim().to_string(), language, audio_ms, elapsed_ms })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finishing_joins_the_capture_and_keeps_its_final_tail() {
        let capture = WizardCapture::new();
        capture.running.store(true, Ordering::SeqCst);
        capture.buffer.lock().unwrap().push(1.0);
        let running = capture.running.clone();
        let buffer = capture.buffer.clone();
        *capture.thread.lock().unwrap() = Some(std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) { std::thread::yield_now(); }
            buffer.lock().unwrap().push(2.0);
        }));
        assert_eq!(capture.take_for_transcription().unwrap(), vec![1.0, 2.0]);
        assert!(!capture.running.load(Ordering::SeqCst));
        assert!(capture.thread.lock().unwrap().is_none());
        assert!(capture.buffer.lock().unwrap().is_empty());
    }

    #[test]
    fn failed_capture_discards_partial_audio_and_reports_the_error_once() {
        let capture = WizardCapture::new();
        capture.buffer.lock().unwrap().push(1.0);
        *capture.failure.lock().unwrap() = Some("Fixture input disconnected".into());
        assert_eq!(capture.take_for_transcription().unwrap_err(), "Fixture input disconnected");
        assert!(capture.take_for_transcription().unwrap().is_empty());
    }

    #[test]
    fn push_capped_keeps_the_newest_samples() {
        let mut buf = vec![1.0, 2.0, 3.0];
        push_capped(&mut buf, &[4.0, 5.0], 4);
        assert_eq!(buf, vec![2.0, 3.0, 4.0, 5.0]);
        push_capped(&mut buf, &[6.0], 4);
        assert_eq!(buf, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn push_capped_handles_a_chunk_larger_than_the_cap() {
        let mut buf = vec![1.0];
        push_capped(&mut buf, &[2.0, 3.0, 4.0, 5.0, 6.0], 3);
        assert_eq!(buf, vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn level_from_rms_scales_and_clamps() {
        assert_eq!(level_from_rms(0.0), 0.0);
        assert_eq!(level_from_rms(1.0), 1.0);
        assert!((level_from_rms(0.05) - 0.3).abs() < 1e-6);
        assert_eq!(level_from_rms(-0.5), 0.0);
    }

    #[test]
    fn rms_of_silence_and_a_constant_signal() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms(&[0.0; 100]), 0.0);
        assert!((rms(&[0.5; 100]) - 0.5).abs() < 1e-6);
    }
}
