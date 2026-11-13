//! Model provisioning — download, validate, and enumerate Whisper models.
//!
//! Models live in `<app_data>/models/`. Supported downloads have a pinned
//! publisher checksum; incomplete or unverified bytes are never made ready.

use anyhow::{anyhow, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "../model_download.rs"]
mod transport;

#[derive(Clone, Default, PartialEq, serde::Serialize)]
pub struct DownloadStatus {
    pub downloading: bool,
    pub progress: f64,
    pub error: Option<String>,
    pub paused: bool,
    pub can_pause: bool,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub message: Option<String>,
}

#[derive(Default)]
struct Downloads {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    status: Mutex<HashMap<String, DownloadStatus>>,
    verified: Mutex<HashMap<PathBuf, VerifiedFile>>,
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[derive(Clone)]
struct VerifiedFile {
    size: u64,
    modified: SystemTime,
    sha256: String,
}

impl Downloads {
    fn lock_for(&self, path: &Path) -> Arc<Mutex<()>> {
        self.locks.lock().unwrap().entry(path.to_owned()).or_default().clone()
    }
}

static DOWNLOADS: LazyLock<Downloads> = LazyLock::new(Downloads::default);

pub fn download_status(name: &str) -> DownloadStatus {
    DOWNLOADS.status.lock().unwrap().get(name).cloned().unwrap_or_default()
}

/// Restores paused progress after an app restart without making .part usable.
pub fn download_status_for(app: &AppHandle, name: &str) -> DownloadStatus {
    let mut status = download_status(name);
    if !status.downloading {
        if let Ok(path) = model_path(app, name) {
            let (bytes, total) = transport::partial_status(&path);
            if bytes > 0 {
                status.paused = true;
                status.downloaded_bytes = bytes;
                status.total_bytes = total;
                status.progress = total.filter(|n| *n > 0).map(|n| (bytes as f64 / n as f64).min(0.99)).unwrap_or(0.0);
                if status.error.is_none() && status.message.is_none() {
                    status.message = Some("Saved download available. Resume to finish and verify it.".into());
                }
            }
        }
    }
    status
}

pub fn cancel_download(name: &str) -> Result<()> {
    let active = DOWNLOADS.active.lock().unwrap();
    let token = active.get(name).ok_or_else(|| anyhow!("This model has no active download."))?;
    token.store(true, Ordering::Release);
    Ok(())
}

/// Publisher LFS SHA256 values retrieved from the official repository APIs on
/// 2026-10-21. These describe exact bytes, independently of mutable main URLs.
pub fn model_checksum(name: &str) -> Option<&'static str> {
    Some(match name {
        crate::translation::LOCAL_MODEL_NAME => crate::translation::LOCAL_MODEL_SHA256,
        "ggml-tiny.bin" => "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        "ggml-base.bin" => "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        "ggml-small.bin" => "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        "ggml-medium.bin" => "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        "ggml-large-v3-turbo.bin" => "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        "ggml-large-v3-turbo-q5_0.bin" => "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        "ggml-large-v3.bin" => "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        "ggml-silero-v5.1.2.bin" => "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
        "gemma-3-1b-it-Q4_K_M.gguf" => "8270790f3ab69fdfe860b7b64008d9a19986d8df7e407bb018184caa08798ebd",
        _ => return None,
    })
}

fn report_download(app: &AppHandle, name: &str, status: DownloadStatus) {
    {
        let mut current = DOWNLOADS.status.lock().unwrap();
        if current.get(name) == Some(&status) { return; }
        current.insert(name.to_string(), status.clone());
    }
    let _ = app.emit("model-download-state", serde_json::json!({"name": name, "status": status}));
}

/// A short-lived reservation, acquired before spawning a loader. Failed loads
/// release it too, leaving the existing engine available for retry.
#[derive(Default)]
pub struct LoadGate(Mutex<Option<String>>);

pub struct LoadPermit<'a>(&'a LoadGate);

impl LoadGate {
    pub const fn new() -> Self { Self(Mutex::new(None)) }

    pub fn begin(&self, name: &str) -> Result<LoadPermit<'_>> {
        let mut current = self.0.lock().unwrap();
        if let Some(name) = current.as_ref() {
            return Err(anyhow!("Model {name} is still loading. Wait for it to finish."));
        }
        *current = Some(name.to_owned());
        Ok(LoadPermit(self))
    }

    pub fn current(&self) -> Option<String> { self.0.lock().unwrap().clone() }
}

impl Drop for LoadPermit<'_> {
    fn drop(&mut self) { *self.0.0.lock().unwrap() = None; }
}

/// Bench-only twin of Tauri's `app_data_dir` (benches run without an
/// AppHandle): `%APPDATA%\ai.organic.lectus` on Windows,
/// `~/Library/Application Support/ai.organic.lectus` on macOS.
#[cfg(test)]
pub(crate) fn bench_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    let root = PathBuf::from(std::env::var("APPDATA").expect("APPDATA not set"));
    #[cfg(target_os = "macos")]
    let root = PathBuf::from(std::env::var("HOME").expect("HOME not set"))
        .join("Library/Application Support");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = PathBuf::from(std::env::var("HOME").expect("HOME not set")).join(".local/share");
    root.join("ai.organic.lectus")
}

/// A downloadable Whisper model.
#[derive(Clone, serde::Serialize)]
pub struct ModelInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    /// Approximate download size in MB.
    pub size_mb: u32,
    pub multilingual: bool,
    pub description: &'static str,
}

pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "ggml-tiny.bin",
        display_name: "Tiny",
        size_mb: 75,
        multilingual: true,
        description: "Smallest listed recognition model. Lower storage and memory needs; evaluate accuracy with your own speech.",
    },
    ModelInfo {
        name: "ggml-base.bin",
        display_name: "Base",
        size_mb: 142,
        multilingual: true,
        description: "Larger than Tiny, with higher storage and memory needs. Recognition quality and speed depend on your speech and hardware.",
    },
    ModelInfo {
        name: "ggml-small.bin",
        display_name: "Small",
        size_mb: 466,
        multilingual: true,
        description: "More capacity than Base for difficult speech. Uses more memory; speed depends on your hardware.",
    },
    ModelInfo {
        name: "ggml-medium.bin",
        display_name: "Medium",
        size_mb: 1530,
        multilingual: true,
        description: "Larger multilingual recognition model. Needs more memory; GPU acceleration is recommended.",
    },
    ModelInfo {
        name: "ggml-large-v3-turbo.bin",
        display_name: "Large v3 Turbo",
        size_mb: 1620,
        multilingual: true,
        description: "Balances multilingual recognition and speed. GPU acceleration is recommended for this larger model.",
    },
    ModelInfo {
        name: "ggml-large-v3-turbo-q5_0.bin",
        display_name: "Large v3 Turbo · Compact",
        size_mb: 574,
        multilingual: true,
        description: "Quantized Turbo with a smaller download and lower memory use. Compression can affect recognition accuracy.",
    },
    ModelInfo {
        name: "ggml-large-v3.bin",
        display_name: "Large v3",
        size_mb: 3095,
        multilingual: true,
        description: "Full-size multilingual model for accuracy-focused use. Needs more memory and takes longer than Turbo.",
    },
];

/// Silero neural VAD model (whisper.cpp built-in VAD support). Tiny (~0.9 MB),
/// auto-downloaded at startup.
pub const VAD_MODEL_NAME: &str = "ggml-silero-v5.1.2.bin";

pub fn vad_model_url() -> String {
    format!("https://huggingface.co/ggml-org/whisper-vad/resolve/main/{VAD_MODEL_NAME}")
}

fn model_url(model_name: &str) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}"
    )
}

/// `<app_data>/models`, created if missing.
pub fn models_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?.join("models");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Where the named model lives once downloaded (may not exist yet).
pub fn model_path(app: &AppHandle, model_name: &str) -> Result<PathBuf> {
    Ok(models_dir(app)?.join(model_name))
}

/// True only for a known model whose publisher digest has been verified.
/// Successful checks are cached against file metadata; .part is never ready.
pub fn is_downloaded(app: &AppHandle, model_name: &str) -> bool {
    match model_path(app, model_name) {
        Ok(p) => {
            let Some(expected) = model_checksum(model_name) else { return false };
            let lock = DOWNLOADS.lock_for(&p);
            // Status queries must not block behind a multi-gigabyte download.
            let Ok(_operation) = lock.try_lock() else { return cached_verification(&p, expected) };
            p.is_file() && verify_cached(&p, expected).is_ok()
        },
        Err(_) => false,
    }
}

/// Ensure the model is present, downloading if needed.
/// Emits `model-download-progress` (f64 0..1), then `model-ready` (model_name)
/// on success or `model-download-failed` (String) on error. Blocking — call off
/// the UI thread.
pub fn ensure_model(app: &AppHandle, model_name: &str) -> Result<PathBuf> {
    ensure_file_from_url(app, &model_url(model_name), model_name)
}

/// Ensure a catalogued model is present, downloading from `url` if needed.
/// Unknown checksums fail closed. Same events as `ensure_model`.
pub fn ensure_file_from_url(app: &AppHandle, url: &str, file_name: &str) -> Result<PathBuf> {
    let expected = model_checksum(file_name).ok_or_else(|| anyhow!("This model has no approved checksum; download was refused safely."))?;
    ensure_verified_file_from_url(app, url, file_name, expected)
}

/// Explicit download with integrity validation before the file becomes usable.
/// Existing files are checked too; a mere filename/size is never sufficient.
pub fn ensure_verified_file_from_url(app: &AppHandle, url: &str, name: &str, expected_sha: &str) -> Result<PathBuf> {
    validate_expected_sha(expected_sha)?;
    let dest = model_path(app, name)?;
    let lock = DOWNLOADS.lock_for(&dest);
    let _operation = reserve_download(&lock)?;
    if cached_verification(&dest, expected_sha) { return Ok(dest); }
    report_download(app, name, DownloadStatus { downloading: true, ..Default::default() });
    let result = if dest.exists() {
        report_download(app, name, DownloadStatus { downloading: true, progress: 0.99,
            message: Some("Verifying the saved model…".into()), ..Default::default() });
        verify_cached(&dest, expected_sha)
    } else {
        download_with_progress(app, url, &dest, Some(expected_sha))
    };
    match result {
        Ok(()) => {
            report_download(app, name, DownloadStatus { progress: 1.0, ..Default::default() });
            let _ = app.emit("model-ready", name.to_string());
            Ok(dest)
        }
        Err(error) => {
            let voluntarily_paused = error.is::<transport::Paused>();
            let mut status = download_status(name);
            let (bytes, total) = transport::partial_status(&dest);
            status.downloading = false;
            status.can_pause = false;
            status.downloaded_bytes = bytes;
            status.total_bytes = total;
            status.progress = total.filter(|n| *n > 0).map(|n| (bytes as f64 / n as f64).min(0.99)).unwrap_or(0.0);
            status.paused = bytes > 0;
            status.error = if voluntarily_paused { None } else { Some(error.to_string()) };
            status.message = Some(error.to_string());
            report_download(app, name, status);
            if !voluntarily_paused { let _ = app.emit("model-download-failed", error.to_string()); }
            Err(error)
        }
    }
}

/// Download requests must never queue behind an active transfer: a queued
/// duplicate could restart it immediately after the user pauses it.
fn reserve_download(lock: &Mutex<()>) -> Result<std::sync::MutexGuard<'_, ()>> {
    lock.try_lock().map_err(|_| anyhow!("A download or verification is already running for this model. Wait for it to finish."))
}

/// Verify an installed model without any network access. The successful digest
/// is cached against path, size, modification time and expected digest. Every
/// controlled download shares this file lock and invalidates the cache.
pub fn verify_model_file(app: &AppHandle, name: &str, expected_sha: &str) -> Result<PathBuf> {
    validate_expected_sha(expected_sha)?;
    let dest = model_path(app, name)?;
    let lock = DOWNLOADS.lock_for(&dest);
    let _operation = lock.lock().unwrap();
    if cached_verification(&dest, expected_sha) { return Ok(dest); }
    if !dest.is_file() { return Err(anyhow!("The selected model is not downloaded.")); }
    report_download(app, name, DownloadStatus { downloading: true, progress: 0.99, message: Some("Verifying model…".into()), ..Default::default() });
    match verify_cached(&dest, expected_sha) {
        Ok(()) => {
            report_download(app, name, DownloadStatus { progress: 1.0, ..Default::default() });
            Ok(dest)
        }
        Err(error) => {
            report_download(app, name, DownloadStatus { error: Some(error.to_string()), ..Default::default() });
            Err(error)
        }
    }
}

/// Verify catalogued model bytes before any resident-cache reuse or load.
/// Unknown filenames are left to the explicit experimental benchmark path;
/// they do not gain a catalogued/verified availability claim from this helper.
pub fn verify_known_model_path(path: &Path) -> Result<()> {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let expected = AVAILABLE_MODELS.iter().map(|model| model.name)
        .chain([VAD_MODEL_NAME, crate::ai::local_llm::CLEANUP_MODEL_NAME, crate::translation::LOCAL_MODEL_NAME])
        .find(|known| known.eq_ignore_ascii_case(name))
        .and_then(model_checksum);
    let Some(expected) = expected else { return Ok(()) };
    let lock = DOWNLOADS.lock_for(path);
    let _operation = lock.lock().unwrap();
    if !path.is_file() { return Err(anyhow!("The selected model is not downloaded.")); }
    verify_cached(path, expected)
}

fn validate_expected_sha(expected: &str) -> Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(anyhow!("The expected model checksum is invalid."));
    }
    Ok(())
}

fn cached_verification(path: &Path, expected: &str) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else { return false };
    let Ok(modified) = metadata.modified() else { return false };
    DOWNLOADS.verified.lock().unwrap().get(path).is_some_and(|verified|
        metadata.is_file() && verified.size == metadata.len() && verified.modified == modified
            && verified.sha256.eq_ignore_ascii_case(expected))
}

/// Caller holds the destination's operation lock, including during publication.
fn verify_cached(path: &Path, expected: &str) -> Result<()> {
    if cached_verification(path, expected) { return Ok(()); }
    DOWNLOADS.verified.lock().unwrap().remove(path);
    let verified = hash_and_validate(path, expected)?;
    DOWNLOADS.verified.lock().unwrap().insert(path.to_owned(), verified);
    Ok(())
}

fn hash_and_validate(path: &Path, expected: &str) -> Result<VerifiedFile> {
    validate_expected_sha(expected)?;
    let mut file = std::fs::File::open(path)?;
    let before = file.metadata()?;
    let modified = before.modified()?;
    if !before.is_file() { return Err(anyhow!("The model path is not a regular file.")); }
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    let after = std::fs::metadata(path)?;
    if after.len() != before.len() || after.modified()? != modified {
        return Err(anyhow!("The model file changed during verification. Try again."));
    }
    let actual = format!("{:x}", hasher.finalize());
    drop(file);
    if !actual.eq_ignore_ascii_case(expected) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static QUARANTINE_ID: AtomicU64 = AtomicU64::new(0);
        let quarantine = path.with_extension(format!("invalid-{}-{}", std::process::id(), QUARANTINE_ID.fetch_add(1, Ordering::Relaxed)));
        std::fs::rename(path, quarantine)
            .map_err(|_| anyhow!("Model checksum mismatch. The invalid file could not be quarantined; close any model task and retry."))?;
        return Err(anyhow!("Model checksum mismatch. The file was quarantined; download the model again."));
    }
    Ok(VerifiedFile { size: before.len(), modified, sha256: actual })
}

fn publish_download(tmp: &Path, dest: &Path, expected_sha: Option<&str>) -> Result<()> {
    // The destination operation lock spans hashing and rename. No readiness
    // event can fire, and no verifier can pass, while only .part exists.
    let verified = expected_sha.map(|expected| hash_and_validate(tmp, expected)).transpose()?;
    publish_verified(tmp, dest, verified)
}

fn publish_verified(tmp: &Path, dest: &Path, verified: Option<VerifiedFile>) -> Result<()> {
    std::fs::rename(tmp, dest)?;
    let mut cache = DOWNLOADS.verified.lock().unwrap();
    cache.remove(dest);
    if let Some(verified) = verified { cache.insert(dest.to_owned(), verified); }
    Ok(())
}

/// Stream a download to a temp file, emitting progress, then atomically rename.
fn download_with_progress(app: &AppHandle, url: &str, dest: &Path, expected_sha: Option<&str>) -> Result<()> {
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let expected = expected_sha.ok_or_else(|| anyhow!("A model checksum is required."))?;
    let cancel = Arc::new(AtomicBool::new(false));
    DOWNLOADS.active.lock().unwrap().insert(name.to_owned(), cancel.clone());
    let (bytes, total) = transport::partial_status(dest);
    report_download(app, name, DownloadStatus { downloading: true, can_pause: true,
        downloaded_bytes: bytes, total_bytes: total,
        progress: total.filter(|n| *n > 0).map(|n| (bytes as f64 / n as f64).min(0.99)).unwrap_or(0.0),
        message: Some("Connecting to the model server…".into()), ..Default::default() });
    let tmp = dest.with_extension("part");
    DOWNLOADS.verified.lock().unwrap().remove(dest);
    let mut last_emit = -1.0_f64;
    let mut last_message = String::new();
    let result = transport::transfer(url, dest, expected, &cancel, |downloaded, total, message| {
        let frac = (downloaded as f64 / total as f64).min(0.99);
        if frac - last_emit >= 0.005 || last_message != message {
            last_emit = frac;
            last_message = message.to_owned();
            let _ = app.emit("model-download-progress", frac);
            report_download(app, name, DownloadStatus { downloading: true, progress: frac,
                can_pause: true, downloaded_bytes: downloaded, total_bytes: Some(total),
                message: Some(message.to_owned()), ..Default::default() });
        }
    }).and_then(|()| {
        let verified = hash_and_validate(&tmp, expected)?;
        // Serialize the publication decision with cancellation. Once this lock
        // is acquired cancellation either wins, or observes a finished download.
        let mut active = DOWNLOADS.active.lock().unwrap();
        if cancel.load(Ordering::Acquire) { return Err(transport::Paused.into()); }
        publish_verified(&tmp, dest, Some(verified))?;
        active.remove(name);
        drop(active);
        transport::remove_receipt(dest);
        let _ = app.emit("model-download-progress", 1.0_f64);
        Ok(())
    });
    DOWNLOADS.active.lock().unwrap().remove(name);
    result
}

fn validate_download_size(downloaded: u64, expected: u64) -> Result<()> {
    if downloaded <= 100_000 || (expected > 0 && downloaded != expected) {
        return Err(anyhow!("Incomplete model download ({downloaded} bytes). Please retry."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_model_uses_its_existing_publisher_integrity_policy() {
        // A ready translation file must not disappear from status merely
        // because it belongs to a different model family than Whisper.
        assert_eq!(model_checksum(crate::translation::LOCAL_MODEL_NAME), Some(crate::translation::LOCAL_MODEL_SHA256));
        assert!(validate_expected_sha(model_checksum(crate::translation::LOCAL_MODEL_NAME).unwrap()).is_ok());
    }

    #[test]
    fn legacy_known_models_cannot_bypass_integrity_by_path_or_filename_case() {
        let dir = tempfile::tempdir().unwrap();
        for name in [crate::ai::local_llm::CLEANUP_MODEL_NAME.to_owned(), crate::translation::LOCAL_MODEL_NAME.to_ascii_uppercase()] {
            let path = dir.path().join(name);
            std::fs::write(&path, vec![7; 200_000]).unwrap();
            assert!(verify_known_model_path(&path).unwrap_err().to_string().contains("checksum"));
            assert!(!path.exists(), "Invalid legacy model must be quarantined before load");
        }
    }

    #[test]
    fn duplicate_download_cannot_queue_and_restart_a_paused_transfer() {
        use std::sync::{atomic::AtomicUsize, mpsc};
        use std::time::Duration;
        let lock = Arc::new(Mutex::new(()));
        let active = reserve_download(&lock).unwrap();
        let started = Arc::new(AtomicUsize::new(1));
        let duplicate_lock = lock.clone();
        let duplicate_started = started.clone();
        let (result_tx, result_rx) = mpsc::channel();
        let duplicate = std::thread::spawn(move || {
            let result = reserve_download(&duplicate_lock).map(|_permit| {
                duplicate_started.fetch_add(1, Ordering::SeqCst);
            });
            result_tx.send(result.is_err()).unwrap();
        });
        // The duplicate must reject before the first operation releases its
        // lock. Blocking here would reproduce the unwanted queued restart.
        assert!(result_rx.recv_timeout(Duration::from_secs(1)).unwrap());
        drop(active); // Represents the first operation completing its pause.
        duplicate.join().unwrap();
        assert_eq!(started.load(Ordering::SeqCst), 1);
        // A new, explicit request after the pause remains permitted.
        let _explicit_resume = reserve_download(&lock).unwrap();
        started.fetch_add(1, Ordering::SeqCst);
        assert_eq!(started.load(Ordering::SeqCst), 2);
    }

    /// Opt-in network test; uses a disposable directory, never app data.
    #[test]
    #[ignore = "requires official HuggingFace connectivity; isolated 885 KB VAD download"]
    fn live_vad_download_cancel_resume_and_verify() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(VAD_MODEL_NAME);
        let expected = model_checksum(VAD_MODEL_NAME).unwrap();
        let cancel = AtomicBool::new(false);
        let result = transport::transfer(&vad_model_url(), &dest, expected, &cancel, |done, _, _| {
            if done > 0 { cancel.store(true, Ordering::Release); }
        });
        assert!(result.unwrap_err().to_string().contains("paused"));
        let saved = std::fs::metadata(dest.with_extension("part")).unwrap().len();
        assert!(saved > 0 && saved <= 885098);
        assert!(!dest.exists());
        cancel.store(false, Ordering::Release);
        let mut resumed = None;
        transport::transfer(&vad_model_url(), &dest, expected, &cancel, |done, total, _| {
            resumed.get_or_insert(done);
            assert_eq!(total, 885098);
        }).unwrap();
        assert_eq!(resumed, Some(saved));
        publish_download(&dest.with_extension("part"), &dest, Some(expected)).unwrap();
        transport::remove_receipt(&dest);
        assert!(cached_verification(&dest, expected));
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 885098);
        println!("Official VAD download: paused at {saved}/885098 bytes; resumed at {}; SHA256 {expected}; temporary destination only", resumed.unwrap());
    }

    #[test]
    fn verified_download_is_published_only_after_its_digest_matches() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("translation.gguf");
        let partial = dest.with_extension("part");
        let bytes = vec![7u8; 200_000];
        let expected = format!("{:x}", Sha256::digest(&bytes));
        std::fs::write(&partial, &bytes).unwrap();
        let lock = DOWNLOADS.lock_for(&dest);
        let _operation = lock.lock().unwrap();
        assert!(!dest.exists());
        assert!(!cached_verification(&dest, &expected));
        publish_download(&partial, &dest, Some(&expected)).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), bytes);
        assert!(!partial.exists());
        assert!(cached_verification(&dest, &expected));
    }

    #[test]
    fn checksum_mismatch_quarantines_partial_and_never_publishes_it() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("translation.gguf");
        let partial = dest.with_extension("part");
        std::fs::write(&partial, vec![7u8; 200_000]).unwrap();
        let lock = DOWNLOADS.lock_for(&dest);
        let _operation = lock.lock().unwrap();
        let expected = format!("{:x}", Sha256::digest(b"different model"));
        assert!(publish_download(&partial, &dest, Some(&expected)).is_err());
        assert!(!dest.exists() && !partial.exists());
        assert!(!cached_verification(&dest, &expected));
        assert!(std::fs::read_dir(dir.path()).unwrap().any(|entry|
            entry.unwrap().file_name().to_string_lossy().contains("invalid-")));
    }

    #[test]
    fn cached_verification_is_bound_to_file_metadata_and_expected_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("translation.gguf");
        let bytes = vec![1u8; 200_000];
        let expected = format!("{:x}", Sha256::digest(&bytes));
        std::fs::write(&path, &bytes).unwrap();
        let lock = DOWNLOADS.lock_for(&path);
        let _operation = lock.lock().unwrap();
        verify_cached(&path, &expected).unwrap();
        assert!(cached_verification(&path, &expected));
        assert!(!cached_verification(&path, &"0".repeat(64)));
        // A changed installed file cannot retain the previous success receipt.
        std::fs::write(&path, vec![2u8; 200_001]).unwrap();
        assert!(!cached_verification(&path, &expected));
        assert!(verify_cached(&path, &expected).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn verification_waits_for_the_same_file_publication_lock() {
        use std::sync::mpsc;
        use std::time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("translation.gguf");
        let partial = dest.with_extension("part");
        let bytes = vec![9u8; 200_000];
        let expected = format!("{:x}", Sha256::digest(&bytes));
        std::fs::write(&partial, bytes).unwrap();
        let lock = DOWNLOADS.lock_for(&dest);
        let writer = lock.lock().unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let reader_dest = dest.clone();
        let reader_expected = expected.clone();
        let reader = std::thread::spawn(move || {
            let lock = DOWNLOADS.lock_for(&reader_dest);
            started_tx.send(()).unwrap();
            let _verification = lock.lock().unwrap();
            done_tx.send(verify_cached(&reader_dest, &reader_expected).is_ok()).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(30)).is_err());
        publish_download(&partial, &dest, Some(&expected)).unwrap();
        drop(writer);
        assert!(done_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        reader.join().unwrap();
    }

    #[test]
    fn model_load_reservation_rejects_competing_selection_and_releases_on_error() {
        let gate = LoadGate::new();
        let first = gate.begin("large").unwrap();
        assert!(gate.begin("small").is_err());
        assert_eq!(gate.current().as_deref(), Some("large"));
        drop(first); // Covers the early-return/error path as well as success.
        let _next = gate.begin("small").unwrap();
        assert_eq!(gate.current().as_deref(), Some("small"));
    }

    #[test]
    fn duplicate_download_requests_share_a_lock_but_different_models_do_not() {
        let downloads = Downloads::default();
        let first = downloads.lock_for(Path::new("models/base.bin"));
        let duplicate = downloads.lock_for(Path::new("models/base.bin"));
        let other = downloads.lock_for(Path::new("models/small.bin"));
        let _writing_part_file = first.lock().unwrap();
        assert!(duplicate.try_lock().is_err());
        assert!(other.try_lock().is_ok());
    }

    #[test]
    fn incomplete_or_error_page_downloads_are_not_promoted() {
        assert!(validate_download_size(200, 200).is_err());
        assert!(validate_download_size(200_000, 300_000).is_err());
        assert!(validate_download_size(200_000, 200_000).is_ok());
        assert!(validate_download_size(200_000, 0).is_ok());
    }
}
