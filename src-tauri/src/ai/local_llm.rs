//! In-process local cleanup LLM (llama.cpp via `llama-cpp-2`).
//!
//! Runs Gemma 3 270M (Q4_K_M GGUF, ~253 MB) fully offline — the free-tier
//! counterpart to the cloud cleanup engines. The model loads lazily on first
//! use (or at startup warmup) and stays resident; a fresh context is created
//! per call (cheap at this model size, and it keeps the engine stateless).

use anyhow::{anyhow, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// GGUF filename in `<app_data>/models/` (same dir as the whisper models).
/// Gemma 3 1B: the 270M variant was measured too weak (barely cleaned EN,
/// translated PT); 1B cleans well at ~120-140 ms warm on the RTX 3080.
pub const CLEANUP_MODEL_NAME: &str = "gemma-3-1b-it-Q4_K_M.gguf";
/// Approximate download size, for the settings UI.
pub const CLEANUP_MODEL_SIZE_MB: u32 = 769;

pub fn cleanup_model_url() -> String {
    format!("https://huggingface.co/unsloth/gemma-3-1b-it-GGUF/resolve/main/{CLEANUP_MODEL_NAME}")
}

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
/// Resident model, keyed by the path it was loaded from so a re-download or
/// path change reloads it.
static MODEL: Mutex<Option<(PathBuf, LlamaModel)>> = Mutex::new(None);

fn backend() -> Result<&'static LlamaBackend> {
    if BACKEND.get().is_none() {
        // dynamic-backends build: register the GGML backend DLLs
        // (ggml-vulkan.dll, ggml-cpu-*.dll) before backend init. Dev builds
        // find them at the compile-time dir; shipped builds next to the exe.
        let compile_time_dir = llama_cpp_2::llama_backend::BACKENDS_DIR
            .map(Path::new)
            .filter(|p| p.exists());
        match compile_time_dir {
            Some(_) => llama_cpp_2::llama_backend::load_backends(),
            None => {
                if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
                    llama_cpp_2::llama_backend::load_backends_from_path(&dir);
                }
            }
        }
        let b = LlamaBackend::init().map_err(|e| anyhow!("llama backend init failed: {e}"))?;
        let _ = BACKEND.set(b);
    }
    BACKEND.get().ok_or_else(|| anyhow!("llama backend unavailable"))
}

/// Load the model into the resident slot if it isn't already. Blocking
/// (~0.5 s); call off the UI thread. Used by the startup warmup too.
pub fn ensure_loaded(model_path: &Path) -> Result<()> {
    let mut guard = MODEL.lock().unwrap();
    if matches!(&*guard, Some((p, _)) if p == model_path) {
        return Ok(());
    }
    if !model_path.exists() {
        return Err(anyhow!("local cleanup model not downloaded"));
    }
    let t = std::time::Instant::now();
    let params = LlamaModelParams::default().with_n_gpu_layers(1_000_000); // all layers on GPU
    let model = LlamaModel::load_from_file(backend()?, model_path, &params)
        .map_err(|e| anyhow!("cleanup model load failed: {e}"))?;
    log::info!("cleanup LLM loaded in {} ms", t.elapsed().as_millis());
    *guard = Some((model_path.to_path_buf(), model));
    Ok(())
}

/// Gemma 3 instruction-tuned chat template around our cleanup instruction.
/// Instruction-only, no few-shot: cross-language examples made small models
/// TRANSLATE the transcript into the example's language (measured in the B2
/// bench) — worse than under-cleaning.
fn build_prompt(text: &str, system: &str) -> String {
    format!(
        "<start_of_turn>user\n{system}\nNever translate — keep the transcript's own language.\n\nTranscript:\n{text}<end_of_turn>\n<start_of_turn>model\n"
    )
}

/// Load the model AND run a one-token generation so the Vulkan pipelines
/// compile now instead of on the first real dictation (~19 s cold otherwise).
pub fn warmup(model_path: &Path) -> Result<()> {
    ensure_loaded(model_path)?;
    let _ = cleanup_local("ok", "Repeat the transcript unchanged.", model_path);
    Ok(())
}

/// Clean `text` with the resident local model. Greedy decoding for
/// deterministic output (the model card's temp 1.0 is for chat, not cleanup).
pub fn cleanup_local(text: &str, system: &str, model_path: &Path) -> Result<String> {
    ensure_loaded(model_path)?;
    let guard = MODEL.lock().unwrap();
    let (_, model) = guard.as_ref().ok_or_else(|| anyhow!("cleanup model not loaded"))?;

    let prompt = build_prompt(text, system);
    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| anyhow!("tokenize failed: {e}"))?;

    // Cleanup output is roughly input-sized; leave generous headroom.
    let max_out = (tokens.len() * 2 + 64).min(1024);
    let n_ctx = (tokens.len() + max_out + 8).max(512) as u32;

    let mut ctx = model
        .new_context(
            backend()?,
            LlamaContextParams::default().with_n_ctx(NonZeroU32::new(n_ctx)),
        )
        .map_err(|e| anyhow!("context creation failed: {e}"))?;

    let t_start = std::time::Instant::now();
    let mut batch = LlamaBatch::new(tokens.len().max(64), 1);
    let last = tokens.len() - 1;
    for (i, tok) in tokens.iter().enumerate() {
        batch
            .add(*tok, i as i32, &[0], i == last)
            .map_err(|e| anyhow!("batch add failed: {e}"))?;
    }
    ctx.decode(&mut batch).map_err(|e| anyhow!("prompt decode failed: {e}"))?;

    let mut sampler = LlamaSampler::greedy();
    let mut out = String::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_pos = tokens.len() as i32;
    let mut ttft_ms: Option<u128> = None;

    for _ in 0..max_out {
        let next = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(next);
        if ttft_ms.is_none() {
            ttft_ms = Some(t_start.elapsed().as_millis());
        }
        if model.is_eog_token(next) {
            break;
        }
        let piece = model
            .token_to_piece(next, &mut decoder, false, None)
            .unwrap_or_default();
        out.push_str(&piece);

        batch.clear();
        batch
            .add(next, n_pos, &[0], true)
            .map_err(|e| anyhow!("batch add failed: {e}"))?;
        ctx.decode(&mut batch).map_err(|e| anyhow!("decode failed: {e}"))?;
        n_pos += 1;
    }

    log::info!(
        "cleanup LLM: ttft {} ms, total {} ms ({} prompt tokens)",
        ttft_ms.unwrap_or(0),
        t_start.elapsed().as_millis(),
        tokens.len()
    );

    let cleaned = out.trim().trim_end_matches("<end_of_turn>").trim().to_string();
    if cleaned.is_empty() {
        return Err(anyhow!("local cleanup returned empty output"));
    }
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B2 measurement: local cleanup quality + latency on real dictation-shaped
    /// input. Run: cargo test --release bench_cleanup -- --ignored --nocapture
    #[test]
    #[ignore = "benchmark — requires the downloaded cleanup model"]
    fn bench_cleanup() {
        let appdata = std::env::var("APPDATA").expect("APPDATA not set");
        // LECTUS_CLEANUP_MODEL overrides the file name for A/B-ing candidates.
        let file = std::env::var("LECTUS_CLEANUP_MODEL")
            .unwrap_or_else(|_| CLEANUP_MODEL_NAME.to_string());
        let model_path = std::path::PathBuf::from(appdata)
            .join("ai.organic.lectus")
            .join("models")
            .join(file);
        assert!(model_path.exists(), "cleanup model not downloaded at {model_path:?}");

        // Same system prompt the pipeline builds, incl. the language pin that
        // stops small models from translating instead of cleaning.
        let cases = [
            ("um so basically i think we should uh move the meeting to thursday because like the client isnt available on wednesday", "en"),
            ("ok testing one two three this is a short dictation latency benchmark", "en"),
            ("então tipo eu acho que a gente devia mudar a reunião pra quinta porque o cliente não tá disponível na quarta", "pt"),
        ];
        for (i, (text, lang)) in cases.iter().enumerate() {
            let system = crate::ai::system_prompt("neutral", Some(lang));
            let t = std::time::Instant::now();
            let out = cleanup_local(text, &system, &model_path).expect("cleanup failed");
            println!(
                "case {i} ({lang}): {} ms\n  in:  {text}\n  out: {out}\n",
                t.elapsed().as_millis()
            );
        }
    }

    #[test]
    fn prompt_uses_gemma_chat_template() {
        let p = build_prompt("hello world", "clean this up");
        assert!(p.starts_with("<start_of_turn>user\n"));
        assert!(p.ends_with("<start_of_turn>model\n"));
        assert!(p.contains("hello world"));
    }
}
