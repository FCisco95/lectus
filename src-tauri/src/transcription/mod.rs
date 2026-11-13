pub mod cloud;
pub mod dictionary;
pub mod local;
pub mod model;
pub use cloud::CloudWhisper;
pub use local::LocalWhisper;

use anyhow::Result;

/// Per-call transcription options, derived from the user's config.
#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    /// `None` = auto-detect the spoken language; `Some("en"/"pt"/…)` forces it.
    pub language: Option<String>,
    /// Optional context/vocabulary bias — fed to whisper as `initial_prompt`
    /// locally and as the `prompt` field for the cloud API.
    pub initial_prompt: Option<String>,
    /// The dictionary words behind `initial_prompt`, so the local engine can
    /// rebuild the prompt with an exact token count.
    pub vocabulary: Vec<String>,
    /// Path to a Silero VAD ggml model. When set, whisper.cpp gates frames
    /// through neural VAD before the encoder — trims silence/noise and
    /// suppresses the hallucinations they trigger. `None` = VAD off.
    pub vad_model_path: Option<String>,
}

impl TranscribeOptions {
    /// Build options from a config language string ("auto" → detect) and the
    /// dictionary words. The prompt uses a rough character-based token estimate;
    /// the local engine replaces it with an exact count.
    pub fn from_config(language: &str, vocabulary: &[String]) -> Self {
        let language = match language.trim() {
            "" | "auto" => None,
            other => Some(other.to_string()),
        };
        let initial_prompt = dictionary::bias_prompt_within(
            vocabulary,
            dictionary::PROMPT_TOKEN_BUDGET,
            |s| s.len() / 3,
        )
        .map(|(prompt, _left_out)| prompt);
        Self { language, initial_prompt, vocabulary: vocabulary.to_vec(), vad_model_path: None }
    }
}

