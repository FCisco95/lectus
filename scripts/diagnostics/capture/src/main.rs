//! Opens only the configured, enumerated microphone for two seconds. No WAV,
//! transcript, key events, volume writes, persistent receipts or app startup.
#[allow(dead_code)]
#[path = "../../../../src-tauri/src/audio/mod.rs"]
mod audio;
#[allow(dead_code)]
#[path = "../../../../src-tauri/src/playback.rs"]
mod playback;
mod watch;

use playback::OutputVolume;
use serde::Deserialize;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct InputConfig { input_device: String }

#[derive(Default)]
struct Stats { samples: usize, square_sum: f64, peak: f32, non_finite: usize }
impl Stats {
    fn consume(&mut self, samples: Vec<f32>) {
        self.samples += samples.len();
        for sample in samples {
            if sample.is_finite() {
                self.square_sum += f64::from(sample) * f64::from(sample);
                self.peak = self.peak.max(sample.abs());
            } else { self.non_finite += 1; }
        }
    }
    fn rms(&self) -> f64 {
        if self.samples == 0 { 0.0 } else { (self.square_sum / self.samples as f64).sqrt() }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--watch-seconds" || arg.starts_with("--watch-seconds=")) {
        return watch::run();
    }
    let config_path = std::path::PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?)
        .join("ai.organic.lectus/config.json");
    // Deserialize only the required field. Never print the config or secrets.
    let config: InputConfig = serde_json::from_reader(std::fs::File::open(config_path)?)?;
    if config.input_device.trim().is_empty() { return Err("No explicitly configured microphone; refusing automatic fallback".into()); }
    if !audio::list_input_devices().iter().any(|name| name == &config.input_device) {
        return Err("Configured microphone is absent or cannot be enumerated; no capture was opened".into());
    }
    let before = playback::PlatformOutput.get().map_err(std::io::Error::other)?;
    println!("configured_device_enumerated=true volume_before_percent={:.2} muted_before={}", before.volume * 100.0, before.muted);
    let capture = audio::PersistentCapture::start(Some(&config.input_device))?;
    // Also catch a removal/default-fallback race between enumeration and open.
    if capture.device_name() != config.input_device {
        drop(capture);
        return Err("Input changed while opening; fallback capture immediately discarded".into());
    }
    let opened = Instant::now();
    std::thread::sleep(Duration::from_millis(500));
    let mut preroll = Stats::default();
    preroll.consume(capture.begin());
    let mut live = Stats::default();
    while opened.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(50));
        live.consume(capture.drain_live());
    }
    live.consume(capture.end());
    drop(capture);
    let open_ms = opened.elapsed().as_millis();
    let after = playback::PlatformOutput.get().map_err(std::io::Error::other)?;
    println!("capture_open_ms={open_ms} preroll_samples={} live_samples={} live_audio_ms={:.1} rms={:.8} peak={:.8} non_finite={}", preroll.samples, live.samples, live.samples as f64 / 16.0, live.rms(), live.peak, preroll.non_finite + live.non_finite);
    println!("volume_after_percent={:.2} muted_after={} volume_changed_observed={} audio_saved=false transcription_attempted=false injection_attempted=false", after.volume * 100.0, after.muted, before != after);
    if preroll.samples == 0 || live.samples == 0 || preroll.non_finite + live.non_finite != 0 {
        return Err("Capture supplied missing or non-finite samples; this is not a successful live-capture proof".into());
    }
    println!("real_capture_completed=true scope=shared_microphone_capture_only");
    Ok(())
}
