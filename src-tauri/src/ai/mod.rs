//! Optional AI cleanup of a raw transcript: fix punctuation/casing, drop filler
//! words ("um", "uh"), and apply a tone. Two interchangeable engines:
//!
//! * **claude** — shells out to the Claude Code CLI (`claude -p`). This runs on
//!   the user's Claude subscription (Max/Pro), so it costs no API tokens. It
//!   needs the `claude` binary installed and logged in; if absent we return the
//!   input unchanged so dictation still works.
//! * **groq** — calls the configured OpenAI-compatible chat endpoint. Fast and
//!   cheap, needs network + an API key.
//!
//! Cleanup is best-effort: any failure returns the original text via the caller.

pub mod local_llm;

use crate::config::Config;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Hard ceiling on the Claude CLI call so a hung process never blocks dictation.
const CLAUDE_TIMEOUT: Duration = Duration::from_secs(20);
/// Default Groq chat model for cleanup.
const GROQ_CLEANUP_MODEL: &str = "llama-3.3-70b-versatile";

/// Clean up `text` using the engine selected in `cfg`. On any error the caller
/// falls back to the original transcript. `local_model_path` locates the GGUF
/// for the fully-offline "local" engine (None disables that engine).
/// `language` is whisper's detected/forced ISO code — pinned in the prompt so
/// the model cleans in place instead of translating (small local models
/// translated pt→en without it, measured in the B2 bench).
pub fn cleanup(
    text: &str,
    cfg: &Config,
    local_model_path: Option<&Path>,
    language: Option<&str>,
) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }
    let system = system_prompt(&cfg.ai_cleanup_tone, language);
    match cfg.ai_cleanup_engine.as_str() {
        "groq" => cleanup_groq(text, cfg, &system),
        "local" => {
            let path = local_model_path
                .ok_or_else(|| anyhow!("local cleanup model path not available"))?;
            local_llm::cleanup_local(text, &system, path)
        }
        // Default to the Claude CLI path.
        _ => cleanup_claude(text, &system),
    }
}

/// System instruction shared by all engines. Written in the transcript's own
/// language where we have a translation: an English instruction block makes
/// small local models answer in English — i.e. translate the transcript —
/// no matter how loudly a "never translate" line says otherwise (measured).
pub(crate) fn system_prompt(tone: &str, language: Option<&str>) -> String {
    let tone = if tone.trim().is_empty() { "neutral" } else { tone.trim() };
    match language {
        Some("pt") => format!(
            "Você é uma ferramenta de limpeza de ditado. Corrija pontuação, \
             capitalização e espaçamento, e remova vícios de linguagem (hum, é, \
             tipo, né). Mantenha as palavras e o significado de quem fala; não \
             adicione nem resuma. NÃO traduza: a resposta deve permanecer em \
             português. Tom: {tone}. Responda APENAS com o texto limpo, sem \
             preâmbulo, aspas ou observações."
        ),
        Some(l) if l != "en" => format!(
            "You are a dictation cleanup tool. Fix punctuation, capitalization, and \
             spacing, and remove filler words. Keep the speaker's wording and \
             meaning; do not add or summarize. Use a {tone} tone. The transcript is \
             in language code '{l}' — the cleaned text MUST stay in that language; \
             never translate. Output ONLY the cleaned text, with no preamble, \
             quotes, or notes."
        ),
        _ => format!(
            "You are a dictation cleanup tool. Fix punctuation, capitalization, and \
             spacing, and remove filler words (um, uh, like, you know). Keep the \
             speaker's wording and meaning; do not add or summarize. Use a {tone} \
             tone. Output ONLY the cleaned text, with no preamble, quotes, or notes."
        ),
    }
}

/// Locate the Claude Code CLI on PATH, accounting for Windows shims.
pub fn claude_binary() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["claude.cmd", "claude.exe", "claude"]
    } else {
        &["claude"]
    };
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in names {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// Whether the Claude CLI is available (surfaced in Settings).
pub fn claude_available() -> bool {
    claude_binary().is_some()
}

fn cleanup_claude(text: &str, system: &str) -> Result<String> {
    let bin = claude_binary().ok_or_else(|| {
        anyhow!("Claude CLI not found on PATH — install Claude Code or pick the Groq engine")
    })?;
    let prompt = format!("{system}\n\nTranscript:\n{text}\n\nCleaned text:");

    // The prompt goes through stdin, not argv: on Windows the CLI resolves to a
    // .cmd shim, and Rust refuses to spawn batch files with argv containing
    // quotes or newlines ("batch file arguments are invalid", CVE-2024-24576).
    let mut child = Command::new(&bin)
        .arg("-p")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("failed to launch claude: {e}"))?;
    {
        use std::io::Write;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("claude stdin unavailable"))?;
        stdin.write_all(prompt.as_bytes())?;
        // Dropping stdin closes the pipe so the CLI sees EOF and runs.
    }

    // Bounded wait so a stuck CLI can't freeze the pipeline.
    let start = Instant::now();
    loop {
        if let Some(_status) = child.try_wait()? {
            break;
        }
        if start.elapsed() > CLAUDE_TIMEOUT {
            let _ = child.kill();
            return Err(anyhow!("claude cleanup timed out"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("claude exited with error: {}", err.trim()));
    }
    let cleaned = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if cleaned.is_empty() {
        return Err(anyhow!("claude returned empty output"));
    }
    Ok(cleaned)
}

fn cleanup_groq(text: &str, cfg: &Config, system: &str) -> Result<String> {
    if cfg.cloud_api_key.trim().is_empty() {
        return Err(anyhow!("Groq cleanup needs an API key (set it in Settings)"));
    }
    let url = format!("{}/chat/completions", cfg.cloud_base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": GROQ_CLEANUP_MODEL,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": text }
        ]
    });

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()?;
    let resp = client
        .post(&url)
        .bearer_auth(cfg.cloud_api_key.trim())
        .json(&body)
        .send()?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(anyhow!("groq cleanup failed ({status}): {body}"));
    }
    let parsed: ChatResponse = resp.json()?;
    let cleaned = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();
    if cleaned.is_empty() {
        return Err(anyhow!("groq returned empty output"));
    }
    Ok(cleaned)
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(serde::Deserialize)]
struct ChatMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_includes_tone() {
        assert!(system_prompt("formal", None).contains("formal"));
        // Empty tone falls back to neutral.
        assert!(system_prompt("", None).contains("neutral"));
    }

    #[test]
    fn system_prompt_pins_detected_language() {
        // Portuguese gets a fully localized instruction block.
        let pt = system_prompt("neutral", Some("pt"));
        assert!(pt.contains("NÃO traduza"));
        // Other non-English languages get the English block with a pin.
        let es = system_prompt("neutral", Some("es"));
        assert!(es.contains("'es'") && es.contains("never translate"));
        // English/unknown gets no pin.
        assert!(!system_prompt("neutral", None).contains("never translate"));
        assert!(!system_prompt("neutral", Some("en")).contains("never translate"));
    }

    #[test]
    fn cleanup_passthrough_on_empty() {
        let cfg = Config::default();
        assert_eq!(cleanup("   ", &cfg, None, None).unwrap(), "   ");
    }
}
