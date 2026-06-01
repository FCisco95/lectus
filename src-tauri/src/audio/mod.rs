pub mod vad;
pub use vad::EnergyVad;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream, // underscore keeps the stream alive without using it
}

impl AudioCapture {
    /// Start capturing mic audio at 16kHz mono f32.
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device found"))?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Fixed(512),
        };

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let samples_clone = samples.clone();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                samples_clone.lock().unwrap().extend_from_slice(data);
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )?;
        stream.play()?;

        Ok(Self { samples, _stream: stream })
    }

    /// Drain and return all captured samples, clearing the internal buffer.
    pub fn drain(&self) -> Vec<f32> {
        let mut buf = self.samples.lock().unwrap();
        std::mem::take(&mut *buf)
    }

    /// Returns current number of buffered samples.
    pub fn len(&self) -> usize {
        self.samples.lock().unwrap().len()
    }
}
