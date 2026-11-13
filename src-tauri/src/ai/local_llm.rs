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

fn load_model_backends(dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // Whisper links another ggml statically. Calling the Rust ggml symbol
        // may populate Whisper's registry, leaving llama.dll's registry empty.
        // Resolve the loader from llama's shared ggml.dll explicitly.
        use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let path = std::ffi::CString::new(dir.to_string_lossy().as_bytes())?;
        unsafe {
            let module = GetModuleHandleW(windows::core::w!("ggml.dll"))
                .map_err(|_| anyhow!("The local text-model runtime DLL is unavailable"))?;
            let symbol = GetProcAddress(module, windows::core::s!("ggml_backend_load_all_from_path"))
                .ok_or_else(|| anyhow!("The local text-model runtime cannot load its backends"))?;
            let loader: unsafe extern "C" fn(*const std::ffi::c_char) = std::mem::transmute(symbol);
            loader(path.as_ptr());
        }
    }
    #[cfg(not(windows))]
    llama_cpp_2::llama_backend::load_backends_from_path(dir);
    Ok(())
}

fn backend() -> Result<&'static LlamaBackend> {
    if BACKEND.get().is_none() {
        // dynamic-backends build: register the GGML backend DLLs
        // (ggml-vulkan.dll, ggml-cpu-*.dll) before backend init. Dev builds
        // find them at the compile-time dir; shipped builds next to the exe.
        let compile_time_dir = llama_cpp_2::llama_backend::BACKENDS_DIR
            .map(Path::new)
            .filter(|p| p.exists());
        match compile_time_dir {
            Some(dir) => load_model_backends(dir)?,
            None => {
                // Shipped-build search order: next to the exe, then the Windows
                // installer's backends/ dir (tauri.windows.conf.json), then the macOS .app bundle's Resources/backends dir
                // (tauri.macos.conf.json ships the .so modules there).
                if let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
                    let candidates = [
                        exe_dir.clone(),
                        exe_dir.join("backends"),
                        exe_dir.join("../Resources/backends"),
                    ];
                    for dir in candidates {
                        if dir.exists() {
                            load_model_backends(&dir)?;
                        }
                    }
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
    load_into_slot(&mut guard, model_path)
}

fn load_into_slot(slot: &mut Option<(PathBuf, LlamaModel)>, model_path: &Path) -> Result<()> {
    if model_path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.eq_ignore_ascii_case(CLEANUP_MODEL_NAME)) {
        crate::model_license::ensure_accepted()?;
    }
    // Catalogued legacy files need the same integrity guarantee as downloads.
    // Check before reusing the resident slot too: replacing a path on disk must
    // never silently retain a ready state for invalid model bytes.
    crate::transcription::model::verify_known_model_path(model_path)?;
    if matches!(slot.as_ref(), Some((p, _)) if p == model_path) {
        return Ok(());
    }
    if !model_path.exists() {
        return Err(anyhow!("Local text model is not downloaded. Download it in Settings first."));
    }
    let t = std::time::Instant::now();
    let params = LlamaModelParams::default().with_n_gpu_layers(1_000_000); // all layers on GPU
    let model = LlamaModel::load_from_file(backend()?, model_path, &params)
        .map_err(|e| anyhow!("cleanup model load failed: {e}"))?;
    log::info!("cleanup LLM loaded in {} ms", t.elapsed().as_millis());
    *slot = Some((model_path.to_path_buf(), model));
    Ok(())
}

/// Chat template selected from the model filename so cleanup candidates can be
/// A/B-ed fairly (each instruct model only follows its own format).
/// Instruction-only, no few-shot: cross-language examples made small models
/// TRANSLATE the transcript into the example's language (measured in the B2
/// bench) — worse than under-cleaning.
fn build_prompt(text: &str, system: &str, model_file: &str) -> String {
    build_task_prompt(text, system, model_file, Task::Cleanup)
}

#[derive(Clone, Copy)]
enum Task { Cleanup, Translation }

fn build_task_prompt(text: &str, system: &str, model_file: &str, task: Task) -> String {
    let f = model_file.to_ascii_lowercase();
    let instruction = match task {
        Task::Cleanup => format!("{system}\nNever translate — keep the transcript's own language."),
        Task::Translation => system.to_string(),
    };
    if f.contains("qwen") {
        // ChatML (Qwen3). ` /no_think` in the user turn plus an empty-<think>
        // assistant prefill disables Qwen3's thinking mode (llama.cpp-friendly;
        // any stray <think> block is stripped from the output as safety).
        format!(
            "<|im_start|>system\n{instruction}<|im_end|>\n<|im_start|>user\nTranscript:\n{text} /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        )
    } else if f.contains("granite") {
        // Granite 4.0 chat template.
        format!(
            "<|start_of_role|>system<|end_of_role|>{instruction}<|end_of_text|>\n<|start_of_role|>user<|end_of_role|>Transcript:\n{text}<|end_of_text|>\n<|start_of_role|>assistant<|end_of_role|>"
        )
    } else {
        if !f.contains("gemma") {
            log::warn!("build_prompt: unknown model family for {model_file:?}, falling back to Gemma template");
        }
        // Gemma 3 has no system role — instruction rides in the user turn.
        format!(
            "<start_of_turn>user\n{instruction}\n\nTranscript:\n{text}<end_of_turn>\n<start_of_turn>model\n"
        )
    }
}

/// Strip trailing end-of-turn/special tokens and any `<think>...</think>`
/// block a reasoning model may emit despite thinking being disabled.
fn clean_output(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if let Some(end) = s.find("</think>") {
        // Drop everything through the closing tag (covers a missing opening tag too).
        s = s[end + "</think>".len()..].trim().to_string();
    }
    for tok in ["<end_of_turn>", "<|im_end|>", "<|end_of_text|>", "<|endoftext|>"] {
        s = s.trim_end_matches(tok).trim().to_string();
    }
    s
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
    generate_local(text, system, model_path, Task::Cleanup)
}

pub fn translate_local(text: &str, system: &str, model_path: &Path) -> Result<String> {
    generate_local(text, system, model_path, Task::Translation)
}

const MAX_INPUT_CHARS: usize = 12_000;
const MAX_PROMPT_TOKENS: usize = 4096;
const MAX_OUTPUT_BYTES: usize = 32 * 1024;

fn generate_local(text: &str, system: &str, model_path: &Path, task: Task) -> Result<String> {
    if text.chars().count() > MAX_INPUT_CHARS {
        return Err(anyhow!("Text is too long for the local text model. Split it into shorter dictations."));
    }
    // Keep model choice and inference in one critical section. A translation
    // model must not replace the cleanup model between load and generation.
    let mut guard = MODEL.lock().unwrap();
    load_into_slot(&mut guard, model_path)?;
    let (_, model) = guard.as_ref().ok_or_else(|| anyhow!("cleanup model not loaded"))?;

    let model_file = model_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(CLEANUP_MODEL_NAME);
    let prompt = build_task_prompt(text, system, model_file, task);
    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| anyhow!("tokenize failed: {e}"))?;

    if tokens.is_empty() || tokens.len() > MAX_PROMPT_TOKENS {
        return Err(anyhow!("Text exceeds the local model input limit. Split it into shorter dictations."));
    }
    // Translation can expand text, particularly across scripts. Neither task
    // returns a silently truncated result when its generation budget runs out.
    let max_out = match task {
        Task::Cleanup => (tokens.len() * 2 + 64).min(1024),
        Task::Translation => (tokens.len() * 3 + 128).min(2048),
    };
    let n_ctx = (tokens.len() + max_out + 8).max(512) as u32;

    let mut ctx = model
        .new_context(
            backend()?,
            LlamaContextParams::default().with_n_ctx(NonZeroU32::new(n_ctx))
                .with_n_batch(tokens.len().max(512) as u32),
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
    let mut ended = false;

    for _ in 0..max_out {
        if t_start.elapsed() > std::time::Duration::from_secs(45) {
            return Err(anyhow!("Local text generation timed out. Try a shorter dictation."));
        }
        let next = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(next);
        if ttft_ms.is_none() {
            ttft_ms = Some(t_start.elapsed().as_millis());
        }
        if model.is_eog_token(next) {
            ended = true;
            break;
        }
        let piece = model
            .token_to_piece(next, &mut decoder, false, None)
            .map_err(|_| anyhow!("Could not decode the local model response."))?;
        out.push_str(&piece);
        if out.len() > MAX_OUTPUT_BYTES {
            return Err(anyhow!("Local text generation exceeded its output limit."));
        }

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

    validate_completion(ended, &out)?;
    let cleaned = clean_output(&out);
    if cleaned.is_empty() {
        return Err(anyhow!("local cleanup returned empty output"));
    }
    Ok(cleaned)
}

fn validate_completion(ended: bool, output: &str) -> Result<()> {
    if !ended { return Err(anyhow!("Local text generation was incomplete. Try a shorter dictation or another model.")); }
    if output.contains("<think>") && !output.contains("</think>") {
        return Err(anyhow!("The local model did not finish its response."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_prompt_does_not_inherit_cleanup_language_lock() {
        let model = "Qwen3-4B-Q4_K_M.gguf";
        let cleanup = build_task_prompt("Olá João 123", "Clean this text.", model, Task::Cleanup);
        let translation = build_task_prompt("Olá João 123", "Translate into 简体中文.", model, Task::Translation);
        assert!(cleanup.contains("Never translate"));
        assert!(!translation.contains("Never translate"));
        assert!(translation.contains("Translate into 简体中文."));
        assert!(translation.contains("Olá João 123"));
        assert!(translation.contains("/no_think"));
    }

    #[test]
    fn rejects_truncated_or_unfinished_generation_instead_of_returning_partial_text() {
        assert!(validate_completion(false, "partial translation").is_err());
        assert!(validate_completion(true, "<think>unfinished").is_err());
        assert!(validate_completion(true, "你好 João 123").is_ok());
    }

    #[test]
    fn oversized_local_input_fails_before_loading_a_model() {
        let result = translate_local(&"字".repeat(MAX_INPUT_CHARS + 1), "Translate.", Path::new("not-downloaded.gguf"));
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    /// B2 measurement: local cleanup quality + latency on real dictation-shaped
    /// input. Run: cargo test --release bench_cleanup -- --ignored --nocapture
    #[test]
    #[ignore = "benchmark — requires the downloaded cleanup model"]
    fn bench_cleanup() {
        // Show the `cleanup LLM: ttft ...` log lines in the bench output.
        let _ = env_logger::builder()
            .is_test(false)
            .filter_level(log::LevelFilter::Info)
            .try_init();
        // LECTUS_CLEANUP_MODEL overrides the file name for A/B-ing candidates.
        let file = std::env::var("LECTUS_CLEANUP_MODEL")
            .unwrap_or_else(|_| CLEANUP_MODEL_NAME.to_string());
        let model_path = crate::transcription::model::bench_app_data_dir()
            .join("models")
            .join(&file);
        assert!(model_path.exists(), "cleanup model not downloaded at {model_path:?}");

        // Same system prompt the pipeline builds, incl. the language pin that
        // stops small models from translating instead of cleaning.
        let cases = [
            ("um so basically i think we should uh move the meeting to thursday because like the client isnt available on wednesday", "en"),
            ("ok testing one two three this is a short dictation latency benchmark", "en"),
            ("então tipo eu acho que a gente devia mudar a reunião pra quinta porque o cliente não tá disponível na quarta", "pt"),
        ];
        println!("\n=== cleanup bench: {file} ===");
        for (i, (text, lang)) in cases.iter().enumerate() {
            let system = crate::ai::system_prompt("neutral", Some(lang));
            // Untimed warmup (loads model on first case, compiles GPU pipelines).
            let _ = cleanup_local(text, &system, &model_path).expect("warmup failed");
            let mut times = Vec::new();
            let mut out = String::new();
            for _ in 0..3 {
                let t = std::time::Instant::now();
                out = cleanup_local(text, &system, &model_path).expect("cleanup failed");
                times.push(t.elapsed().as_millis());
            }
            let best = *times.iter().min().unwrap();
            println!(
                "case {i} ({lang}): runs {times:?} ms | best {best} ms\n  in:  {text}\n  out: {out}\n"
            );
        }
    }

    #[test]
    fn prompt_uses_gemma_chat_template() {
        let p = build_prompt("hello world", "clean this up", "gemma-3-1b-it-Q4_K_M.gguf");
        assert!(p.starts_with("<start_of_turn>user\n"));
        assert!(p.ends_with("<start_of_turn>model\n"));
        assert!(p.contains("hello world"));
    }

    #[test]
    fn prompt_uses_chatml_for_qwen_with_no_think() {
        let p = build_prompt("hello world", "clean this up", "Qwen_Qwen3-1.7B-Q4_K_M.gguf");
        assert!(p.starts_with("<|im_start|>system\n"));
        assert!(p.contains(" /no_think<|im_end|>"));
        assert!(p.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
        assert!(p.contains("hello world"));
    }

    #[test]
    fn prompt_uses_granite_template() {
        let p = build_prompt("hello world", "clean this up", "granite-4.0-1b-Q4_K_M.gguf");
        assert!(p.starts_with("<|start_of_role|>system<|end_of_role|>"));
        assert!(p.ends_with("<|start_of_role|>assistant<|end_of_role|>"));
        assert!(p.contains("hello world"));
    }

    #[test]
    fn unknown_model_falls_back_to_gemma_template() {
        let p = build_prompt("hi", "sys", "mystery-model.gguf");
        assert!(p.starts_with("<start_of_turn>user\n"));
    }

    #[test]
    fn clean_output_strips_think_and_special_tokens() {
        assert_eq!(clean_output("<think>\nreasoning\n</think>\n\nHello.<|im_end|>"), "Hello.");
        assert_eq!(clean_output("Hello.<end_of_turn>"), "Hello.");
        assert_eq!(clean_output("Olá.<|end_of_text|>"), "Olá.");
        assert_eq!(clean_output("  plain  "), "plain");
    }
}
