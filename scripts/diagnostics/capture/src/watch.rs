//! Human-coordinated physical unplug/replug observation. No audio is saved,
//! no inputs are synthesized and no installed app or OS setting is modified.
use crate::{audio, playback, Stats};
use cpal::traits::{DeviceTrait, HostTrait};
use playback::OutputVolume;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct InputConfig { input_device: String }

fn sha(bytes: &[u8]) -> String { format!("{:X}", Sha256::digest(bytes)) }
fn file_sha(path: &Path) -> Result<String, Box<dyn Error>> { Ok(sha(&std::fs::read(path)?)) }
fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn present(name: &str) -> Result<bool, Box<dyn Error>> {
    // Unlike the UI convenience enumeration, distinguish enumeration failure
    // from physical absence; an API error must never prove unplugging.
    let host = cpal::default_host();
    let names = host.input_devices()?.map(|device| device.name()).collect::<Result<Vec<_>, _>>()?;
    let count = names.iter().filter(|candidate| candidate.as_str() == name).count();
    if count > 1 { return Err("Configured input name is ambiguous; refusing another endpoint".into()); }
    Ok(count == 1)
}
fn check_config(path: &Path, original: &[u8]) -> Result<(), Box<dyn Error>> {
    if std::fs::read(path)? != original { return Err("Configuration changed; human observation aborted".into()); }
    Ok(())
}
fn volume() -> Value {
    match playback::PlatformOutput.get_bound() {
        Ok((snapshot, endpoint)) => json!({
            "available": true, "volume": snapshot.volume, "muted": snapshot.muted,
            "endpoint_id_sha256": endpoint.as_deref().map(|s| sha(s.as_bytes()))
        }),
        Err(_) => json!({"available": false}),
    }
}
fn stats(stats: &Stats) -> Value {
    json!({"frames_16khz_mono":stats.samples,"audio_ms":stats.samples as f64 / 16.0,
        "rms":stats.rms(),"peak":stats.peak,"non_finite":stats.non_finite})
}
fn fresh_capture(name: &str) -> Result<audio::PersistentCapture, Box<dyn Error>> {
    if !present(name)? { return Err("Configured input is absent; no default capture opened".into()); }
    let capture = audio::PersistentCapture::start(Some(name))?;
    if capture.device_name() != name || !present(name)? {
        drop(capture);
        return Err("Configured input changed while opening; fallback stream discarded".into());
    }
    Ok(capture)
}
fn observe(capture: &audio::PersistentCapture, duration: Duration, config_path: &Path,
    config_bytes: &[u8], name: &str) -> Result<Stats, Box<dyn Error>> {
    let started = Instant::now();
    let mut result = Stats::default();
    while started.elapsed() < duration {
        std::thread::sleep(Duration::from_millis(250));
        check_config(config_path, config_bytes)?;
        if !present(name)? { return Err("Configured input disappeared during the measurement window".into()); }
        result.consume(capture.drain_live());
    }
    result.consume(capture.drain_live());
    Ok(result)
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut watch_seconds = None;
    let mut receipt_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--watch-seconds" => watch_seconds = Some(args.next().ok_or("Missing watch duration")?.parse::<u64>()?),
            "--receipt" => receipt_path = Some(PathBuf::from(args.next().ok_or("Missing receipt path")?)),
            _ if arg.starts_with("--watch-seconds=") => watch_seconds = Some(arg[16..].parse::<u64>()?),
            _ => return Err("Unknown diagnostic argument".into()),
        }
    }
    let seconds = watch_seconds.ok_or("Missing --watch-seconds")?;
    if !(15..=90).contains(&seconds) { return Err("Watch duration must be 15 to 90 seconds".into()); }
    let destination = receipt_path.ok_or("--receipt is required so physical observations have durable evidence")?;
    if destination.exists() { return Err("Receipt already exists; refusing to overwrite previous evidence".into()); }
    let config_path = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?)
        .join("ai.organic.lectus/config.json");
    let config_bytes = std::fs::read(&config_path)?;
    let config: InputConfig = serde_json::from_slice(&config_bytes)?;
    if config.input_device.trim().is_empty() { return Err("No explicitly configured input; refusing default-device fallback".into()); }
    if !present(&config.input_device)? { return Err("Configured input absent before baseline; nothing opened".into()); }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let started_ms = now_ms();
    let mut receipt = json!({
        "schema":1,"kind":"human_physical_microphone_unplug_replug_production_capture",
        "outcome":"pending","started_timestamp_ms":started_ms,"watch_seconds":seconds,
        "config_sha256_before":sha(&config_bytes),"input_name_sha256":sha(config.input_device.as_bytes()),
        "diagnostic_binary_sha256":file_sha(&std::env::current_exe()?)?,
        "production_audio_source_sha256":file_sha(&root.join("src-tauri/src/audio/mod.rs"))?,
        "production_playback_source_sha256":file_sha(&root.join("src-tauri/src/playback.rs"))?,
        "diagnostic_main_source_sha256":file_sha(&root.join("scripts/diagnostics/capture/src/main.rs"))?,
        "diagnostic_watch_source_sha256":file_sha(&root.join("scripts/diagnostics/capture/src/watch.rs"))?,
        "volume_before":volume(),"absence_observed":false,"return_observed":false,
        "audio_saved":false,"synthetic_input":false,"os_device_change_by_helper":false,
        "configuration_written":false,"installed_app_started_stopped_or_replaced":false,
        "identity_limit":"Input is matched by its explicitly configured unique name, not a stable hardware endpoint ID.",
        "scope":"Standalone process uses production PersistentCapture source; installed app recovery and full dictation are not exercised."
    });
    let result = (|| -> Result<(), Box<dyn Error>> {
        let capture = fresh_capture(&config.input_device)?;
        std::thread::sleep(Duration::from_millis(500));
        check_config(&config_path, &config_bytes)?;
        let preroll_frames = capture.begin().len();
        let baseline = observe(&capture, Duration::from_secs(2), &config_path, &config_bytes, &config.input_device)?;
        receipt["baseline_preroll_frames"] = json!(preroll_frames);
        receipt["baseline"] = stats(&baseline);
        if baseline.samples == 0 || baseline.non_finite != 0 { return Err("Baseline did not deliver finite live frames; physical removal test not ready".into()); }
        println!("{}", json!({"stage":"ready_for_human_unplug_replug","timestamp_ms":now_ms(),
            "baseline_frames":baseline.samples,"watch_seconds":seconds,
            "warning":"Physical removal also affects installed Lectus; it may require reopening the same installed executable afterwards. This helper never restarts it."}));
        std::io::stdout().flush()?;
        let watch_started = Instant::now();
        let mut absence = false;
        let mut during_watch = Stats::default();
        while watch_started.elapsed() < Duration::from_secs(seconds) {
            std::thread::sleep(Duration::from_millis(250));
            check_config(&config_path, &config_bytes)?;
            let is_present = present(&config.input_device)?;
            during_watch.consume(capture.drain_live());
            if !is_present && !absence {
                absence = true;
                receipt["absence_observed"] = json!(true);
                receipt["absent_timestamp_ms"] = json!(now_ms());
                println!("{}", json!({"stage":"configured_input_absent","timestamp_ms":now_ms()}));
                std::io::stdout().flush()?;
            } else if is_present && absence {
                receipt["return_observed"] = json!(true);
                receipt["returned_timestamp_ms"] = json!(now_ms());
                receipt["during_watch"] = stats(&during_watch);
                println!("{}", json!({"stage":"configured_input_returned_observing_original_stream","timestamp_ms":now_ms()}));
                std::io::stdout().flush()?;
                // Only post-return live frames count. No stale pre-roll is used.
                let mut original = observe(&capture, Duration::from_secs(2), &config_path, &config_bytes, &config.input_device)?;
                let tail = capture.end();
                receipt["original_tail_frames_after_measurement"] = json!(tail.len());
                original.consume(tail);
                receipt["original_stream_after_return"] = stats(&original);
                drop(capture); // Original failure evidence is recorded before opening another stream.
                let reopened = fresh_capture(&config.input_device)?;
                std::thread::sleep(Duration::from_millis(500));
                check_config(&config_path, &config_bytes)?;
                drop(reopened.begin());
                let fresh = observe(&reopened, Duration::from_secs(2), &config_path, &config_bytes, &config.input_device)?;
                receipt["fresh_stream_after_return"] = stats(&fresh);
                let _ = reopened.end();
                drop(reopened);
                receipt["outcome"] = json!(if original.non_finite != 0 || fresh.non_finite != 0 { "invalid_non_finite_samples" }
                    else if original.samples == 0 && fresh.samples > 0 { "reproduced_original_stream_stalled_fresh_capture_live" }
                    else if original.samples > 0 && fresh.samples > 0 { "original_and_fresh_streams_live_after_return" }
                    else { "inconclusive_fresh_capture_not_live" });
                return Ok(());
            }
        }
        let _ = capture.end();
        drop(capture);
        receipt["during_watch"] = stats(&during_watch);
        receipt["outcome"] = json!(if absence {"not_accepted_input_did_not_return_within_window"} else {"not_accepted_no_physical_absence_observed"});
        Ok(())
    })();
    if result.is_err() {
        receipt["outcome"] = json!("aborted_read_only_safety_or_capture_check");
        // Generic reason only: native error strings may contain device names.
        receipt["error"] = json!("A configuration, presence, identity, capture, or I/O safety check failed; no settings or installed processes were changed.");
    }
    receipt["finished_timestamp_ms"] = json!(now_ms());
    receipt["config_sha256_after"] = json!(file_sha(&config_path).ok());
    receipt["volume_after"] = volume();
    receipt["volume_changed_observed"] = json!(receipt["volume_before"] != receipt["volume_after"]);
    if let Some(parent) = destination.parent() { std::fs::create_dir_all(parent)?; }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&destination)?;
    file.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
    file.sync_all()?;
    println!("{}", json!({"stage":"finished","outcome":receipt["outcome"],"receipt_written":true,
        "audio_saved":false,"synthetic_input":false,"installed_app_restarted":false}));
    if result.is_err() { return Err("Physical capture observation aborted; see privacy-limited receipt".into()); }
    Ok(())
}
