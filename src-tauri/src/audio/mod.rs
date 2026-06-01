pub mod vad;
pub use vad::EnergyVad;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream,
}

impl AudioCapture {
    /// Start capturing mic audio. Resamples to 16kHz mono f32 for Whisper compatibility.
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device found"))?;

        // Query native config instead of hardcoding 16kHz
        let supported = device.default_input_config()?;
        let native_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;

        let config = cpal::StreamConfig {
            channels: channels as u16,
            sample_rate: supported.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let samples_clone = samples.clone();
        const TARGET_RATE: u32 = 16_000;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Mix down to mono if stereo
                let mono: Vec<f32> = if channels == 1 {
                    data.to_vec()
                } else {
                    data.chunks(channels)
                        .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                        .collect()
                };

                // Resample to 16kHz using linear interpolation
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

                samples_clone.lock().unwrap().extend_from_slice(&resampled);
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )?;
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
