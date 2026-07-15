use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;

/// Persisted app configuration.
///
/// `#[serde(default)]` is applied at the container level so that loading an
/// older `config.json` that predates a newly-added field still succeeds: any
/// missing key falls back to the value from `Config::default()` rather than
/// erroring out. This keeps config additions backward-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub model_path: PathBuf,
    pub use_cloud: bool,
    pub cloud_base_url: String,
    pub cloud_api_key: String,
    pub hold_hotkey: String,
    pub toggle_hotkey: String,
    /// Last on-screen position of the always-visible pill, in physical pixels.
    /// Persisted so the pill reappears where the user dragged it.
    pub pill_x: i32,
    pub pill_y: i32,
    /// Spoken language: "auto" (detect) or an ISO code like "en"/"pt".
    pub language: String,
    /// Filename of the multilingual local model, downloaded on first run into
    /// the app data dir (e.g. "ggml-base.bin"). The bundled English-only
    /// "ggml-tiny.en.bin" is used as an offline fallback until this arrives.
    pub model_name: String,
    /// How recording is triggered: "hold" (push-to-talk) or "toggle" (tap on,
    /// tap off). In toggle mode the configured key flips recording each press.
    pub trigger_mode: String,
    /// Custom vocabulary fed to the recognizer as a bias hint so names/jargon
    /// are spelled correctly (whisper `initial_prompt` / cloud `prompt`).
    pub dictionary_words: Vec<String>,
    /// Exact find/replace rules applied to the transcript after recognition,
    /// in listed order (e.g. "lectus" → "Lectus", "at gmail" → "@gmail").
    pub replacement_rules: Vec<ReplacementRule>,
    /// Whether to run an AI cleanup pass (punctuation, casing, filler removal).
    pub ai_cleanup_enabled: bool,
    /// Which engine powers cleanup: "claude" (Claude Code CLI, uses the user's
    /// subscription, no API tokens) or "groq" (the configured cloud endpoint).
    pub ai_cleanup_engine: String,
    /// Desired tone for the cleanup pass: "neutral", "formal", "casual", …
    pub ai_cleanup_tone: String,
    /// Input device name for recording; empty string = system default.
    pub input_device: String,
}

/// A single exact find/replace rule applied to the transcript post-recognition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ReplacementRule {
    pub from: String,
    pub to: String,
    pub case_sensitive: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/ggml-tiny.en.bin"),
            use_cloud: false,
            cloud_base_url: "https://api.groq.com/openai/v1".into(),
            cloud_api_key: String::new(),
            // Hold-to-talk default: bare Right Ctrl. Activated via a low-level
            // keyboard hook (Win WH_KEYBOARD_LL / macOS CGEventTap), not the
            // global-shortcut plugin (which can't register a bare modifier).
            hold_hotkey: "RControl".into(),
            toggle_hotkey: "F13".into(),
            pill_x: 100,
            pill_y: 100,
            language: "auto".into(),
            model_name: "ggml-tiny.bin".into(),
            trigger_mode: "hold".into(),
            dictionary_words: Vec::new(),
            replacement_rules: Vec::new(),
            ai_cleanup_enabled: false,
            ai_cleanup_engine: "claude".into(),
            ai_cleanup_tone: "neutral".into(),
            input_device: String::new(),
        }
    }
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_default_config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = Config::default();
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn test_missing_file_returns_default() {
        let path = PathBuf::from("/nonexistent/path/config.json");
        let cfg = Config::load_from(&path).unwrap();
        assert_eq!(cfg.use_cloud, false);
        assert!(!cfg.model_path.as_os_str().is_empty());
    }

    #[test]
    fn test_partial_config_loads_defaults() {
        // A config file written by an older build that only knows about a
        // subset of fields must still load, with the rest defaulted.
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"use_cloud": true}"#).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.use_cloud, true);
        // Every other field falls back to Default.
        let defaults = Config::default();
        assert_eq!(loaded.hold_hotkey, defaults.hold_hotkey);
        assert_eq!(loaded.cloud_base_url, defaults.cloud_base_url);
        assert_eq!(loaded.model_path, defaults.model_path);
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.json");
        let cfg = Config::default();
        cfg.save_to(&path).unwrap();
        assert!(path.exists());
    }
}
