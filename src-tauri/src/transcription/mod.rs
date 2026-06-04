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
}

impl TranscribeOptions {
    /// Build options from a config language string ("auto" → detect) and an
    /// optional dictionary bias prompt.
    pub fn from_config(language: &str, initial_prompt: Option<String>) -> Self {
        let language = match language.trim() {
            "" | "auto" => None,
            other => Some(other.to_string()),
        };
        let initial_prompt = initial_prompt.filter(|p| !p.trim().is_empty());
        Self { language, initial_prompt }
    }
}

