pub mod vad;
// `vad::EnergyVad` is reachable as `audio::vad::EnergyVad`. It is no longer on
// the dictation path (Phase 2.5 stops on key-release, not VAD silence) but is
// kept for its own tests and possible future use.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream,
}

/// Samples kept in the always-on pre-roll ring: 500 ms @ 16 kHz. Captures the
/// audio spoken just before the hotkey lands so the first word is never clipped.
pub const PREROLL_SAMPLES: usize = 8_000;

/// Always-on capture stream. Maintains a bounded pre-roll ring at all times and
/// additionally accumulates into an unbounded live buffer while a dictation is
/// active. One instance lives for the app's lifetime (rebuilt on device change).
pub struct PersistentCapture {
    ring: Arc<Mutex<VecDeque<f32>>>,
    live: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    device_name: String,
    _stream: cpal::Stream,
}

// cpal::Stream is !Send on some platforms; on Windows (WASAPI) moving it across
// threads is safe in practice and Tauri state requires Send + Sync. The stream
// is never touched after construction — only the Arc'd buffers are.
unsafe impl Send for PersistentCapture {}
unsafe impl Sync for PersistentCapture {}

impl PersistentCapture {
    /// Open the device and start filling the pre-roll ring immediately.
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let device = resolve_input_device(device_name)?;
        let resolved_name = device.name().unwrap_or_default();
        log::info!("pre-roll capture on input device: {resolved_name:?}");

        let ring: Arc<Mutex<VecDeque<f32>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(PREROLL_SAMPLES)));
        let live: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let recording = Arc::new(AtomicBool::new(false));

        let ring_cb = ring.clone();
        let live_cb = live.clone();
        let recording_cb = recording.clone();
        let stream = build_16k_mono_stream(&device, move |resampled| {
            {
                let mut r = ring_cb.lock().unwrap();
                r.extend(resampled.iter().copied());
                let excess = r.len().saturating_sub(PREROLL_SAMPLES);
                if excess > 0 {
                    r.drain(..excess);
                }
            }
            if recording_cb.load(Ordering::Relaxed) {
                live_cb.lock().unwrap().extend_from_slice(resampled);
            }
        })?;
        stream.play()?;

        Ok(Self { ring, live, recording, device_name: resolved_name, _stream: stream })
    }

    /// Begin a dictation: returns the pre-roll snapshot (up to 500 ms of audio
    /// from just before the hotkey) and starts filling the live buffer.
    pub fn begin(&self) -> Vec<f32> {
        self.live.lock().unwrap().clear();
        self.recording.store(true, Ordering::Relaxed);
        self.ring.lock().unwrap().iter().copied().collect()
    }

    /// Drain samples accumulated since the last drain (or `begin`).
    pub fn drain_live(&self) -> Vec<f32> {
        let mut buf = self.live.lock().unwrap();
        std::mem::take(&mut *buf)
    }

    /// End the dictation: stop filling the live buffer and return any tail.
    pub fn end(&self) -> Vec<f32> {
        self.recording.store(false, Ordering::Relaxed);
        self.drain_live()
    }

    /// The device this stream was opened on (for detecting settings changes).
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

/// Resolve a configured device name to a cpal device, falling back to the
/// system default when unset or unplugged.
fn resolve_input_device(device_name: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    match device_name.filter(|n| !n.is_empty()) {
        Some(wanted) => match host
            .input_devices()?
            .find(|d| d.name().map(|n| n == wanted).unwrap_or(false))
        {
            Some(d) => Ok(d),
            None => {
                log::warn!("input device {wanted:?} not found; using system default");
                host.default_input_device()
                    .ok_or_else(|| anyhow::anyhow!("no input device found"))
            }
        },
        None => host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device found")),
    }
}

/// Build an input stream that delivers 16 kHz mono f32 chunks to `on_chunk`,
/// mixing down and linearly resampling from the device's native format.
fn build_16k_mono_stream(
    device: &cpal::Device,
    mut on_chunk: impl FnMut(&[f32]) + Send + 'static,
) -> Result<cpal::Stream> {
    let supported = device.default_input_config()?;
    let native_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;

    let config = cpal::StreamConfig {
        channels: channels as u16,
        sample_rate: supported.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    const TARGET_RATE: u32 = 16_000;

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mono: Vec<f32> = if channels == 1 {
                data.to_vec()
            } else {
                data.chunks(channels)
                    .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                    .collect()
            };

            let resampled = if native_rate == TARGET_RATE {
                mono
            } else {
                let ratio = native_rate as f64 / TARGET_RATE as f64;
                let out_len = (mono.len() as f64 / ratio).ceil() as usize;
                (0..out_len)
                    .map(|i| {
                        let src = i as f64 * ratio;
                        let lo = src.floor() as usize;
                        let hi = (lo + 1).min(mono.len() - 1);
                        let frac = (src - lo as f64) as f32;
                        mono[lo] * (1.0 - frac) + mono[hi] * frac
                    })
                    .collect()
            };

            on_chunk(&resampled);
        },
        |err| eprintln!("audio stream error: {err}"),
        None,
    )?;
    Ok(stream)
}

/// Names of all available input devices, for the settings mic picker.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

impl AudioCapture {
    /// Start capturing mic audio. Resamples to 16kHz mono f32 for Whisper compatibility.
    /// `device_name` of `None`/empty selects the system default input; an unknown
    /// name falls back to the default with a warning (device may be unplugged).
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let device = resolve_input_device(device_name)?;
        log::info!("recording from input device: {:?}", device.name().unwrap_or_default());

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let samples_clone = samples.clone();
        let stream = build_16k_mono_stream(&device, move |resampled| {
            samples_clone.lock().unwrap().extend_from_slice(resampled);
        })?;
        stream.play()?;

        Ok(Self { samples, _stream: stream })
    }

    /// Drain and return all captured samples (16kHz mono f32), clearing the buffer.
    pub fn drain(&self) -> Vec<f32> {
        let mut buf = self.samples.lock().unwrap();
        std::mem::take(&mut *buf)
    }

    /// Returns current number of buffered samples.
    pub fn len(&self) -> usize {
        self.samples.lock().unwrap().len()
    }
}
