//! Model provisioning — download, validate, and enumerate Whisper models.
//!
//! Models live in `<app_data>/models/`. Any model from the ggerganov/whisper.cpp
//! HuggingFace repo can be used; the filename determines the download URL.

use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

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
        description: "Fastest CPU model. Good accuracy for clear speech. Supports Portuguese and 99 other languages.",
    },
    ModelInfo {
        name: "ggml-base.bin",
        display_name: "Base",
        size_mb: 142,
        multilingual: true,
        description: "Balanced speed and accuracy. ~2× slower than Tiny on CPU. Good for accented speech.",
    },
    ModelInfo {
        name: "ggml-small.bin",
        display_name: "Small",
        size_mb: 466,
        multilingual: true,
        description: "Higher accuracy, slower. Recommended if you have a GPU (Vulkan/CUDA) or can wait 5–10 s.",
    },
    ModelInfo {
        name: "ggml-medium.bin",
        display_name: "Medium",
        size_mb: 1530,
        multilingual: true,
        description: "High accuracy for non-English languages. GPU strongly recommended.",
    },
    ModelInfo {
        name: "ggml-large-v3-turbo.bin",
        display_name: "Large v3 Turbo",
        size_mb: 1620,
        multilingual: true,
        description: "Best multilingual accuracy (recommended for Portuguese) with near-realtime speed on a GPU.",
    },
];

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

/// True if the model file is already present and plausibly complete (> 1 MB).
pub fn is_downloaded(app: &AppHandle, model_name: &str) -> bool {
    match model_path(app, model_name) {
        Ok(p) => p.exists() && std::fs::metadata(&p).map(|m| m.len() > 1_000_000).unwrap_or(false),
        Err(_) => false,
    }
}

/// Ensure the model is present, downloading if needed.
/// Emits `model-download-progress` (f64 0..1), then `model-ready` (model_name)
/// on success or `model-download-failed` (String) on error. Blocking — call off
/// the UI thread.
pub fn ensure_model(app: &AppHandle, model_name: &str) -> Result<PathBuf> {
    let dest = model_path(app, model_name)?;
    if is_downloaded(app, model_name) {
        return Ok(dest);
    }
    let url = model_url(model_name);
    match download_with_progress(app, &url, &dest) {
        Ok(()) => {
            let _ = app.emit("model-ready", model_name.to_string());
            Ok(dest)
        }
        Err(e) => {
            let _ = app.emit("model-download-failed", e.to_string());
            Err(e)
        }
    }
}

/// Stream a download to a temp file, emitting progress, then atomically rename.
fn download_with_progress(app: &AppHandle, url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let mut resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("model download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let tmp = dest.with_extension("part");
    let mut file = std::fs::File::create(&tmp)?;
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    let mut last_emit = 0.0_f64;
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        if total > 0 {
            let frac = downloaded as f64 / total as f64;
            if frac - last_emit >= 0.01 {
                last_emit = frac;
                let _ = app.emit("model-download-progress", frac);
            }
        }
    }
    file.flush()?;
    drop(file);
    std::fs::rename(&tmp, dest)?;
    let _ = app.emit("model-download-progress", 1.0_f64);
    Ok(())
}
