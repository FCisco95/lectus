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
    /// How text lands in the focused field: "auto" (SendInput with clipboard
    /// fallback), "sendinput" (typed keystrokes — works in terminals/CLIs),
    /// or "clipboard" (paste — fastest for very long texts).
    pub injection_mode: String,
    /// Gate audio through Silero neural VAD before the whisper encoder:
    /// trims silence/noise and suppresses the hallucinations they cause.
    pub vad_enabled: bool,
    /// Per-app overrides, matched (first hit wins) against the focused app's
    /// executable name at the moment the hotkey lands.
    pub app_profiles: Vec<AppProfile>,
    /// First-run onboarding finished; false shows the setup flow instead of
    /// Settings. Configs saved before this field existed deserialize to false
    /// (container-level serde(default)) — those users see onboarding once,
    /// which doubles as a permissions health-check after the update.
    pub onboarding_completed: bool,
}

/// Overrides applied when dictating into a matching app. `None` = keep the
/// global setting.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppProfile {
    /// Case-insensitive substring of the target exe name (e.g. "code", "slack").
    pub app_match: String,
    pub ai_cleanup_enabled: Option<bool>,
    pub ai_cleanup_tone: Option<String>,
    /// "auto" or an ISO code — e.g. force "en" inside a code editor.
    pub language: Option<String>,
    pub injection_mode: Option<String>,
}

impl Config {
    /// Overlay the first matching app profile onto a copy of the config.
    pub fn with_app_profile(&self, app_exe: Option<&str>) -> Config {
        let mut cfg = self.clone();
        let Some(exe) = app_exe else { return cfg };
        let exe = exe.to_ascii_lowercase();
        if let Some(p) = self
            .app_profiles
            .iter()
            .filter(|p| !p.app_match.trim().is_empty())
            .find(|p| exe.contains(&p.app_match.trim().to_ascii_lowercase()))
        {
            log::info!("app profile matched: {:?} for {exe}", p.app_match);
            if let Some(v) = p.ai_cleanup_enabled {
                cfg.ai_cleanup_enabled = v;
            }
            if let Some(v) = &p.ai_cleanup_tone {
                cfg.ai_cleanup_tone = v.clone();
            }
            if let Some(v) = &p.language {
                cfg.language = v.clone();
            }
            if let Some(v) = &p.injection_mode {
                cfg.injection_mode = v.clone();
            }
        }
        cfg
    }
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
            injection_mode: "auto".into(),
            vad_enabled: true,
            app_profiles: Vec::new(),
            onboarding_completed: false,
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
    fn test_app_profile_overlay() {
        let mut cfg = Config::default();
        cfg.ai_cleanup_enabled = true;
        cfg.app_profiles = vec![AppProfile {
            app_match: "Code".into(),
            ai_cleanup_enabled: Some(false),
            language: Some("en".into()),
            ..Default::default()
        }];

        // Matching app (case-insensitive substring) applies overrides.
        let over = cfg.with_app_profile(Some("code.exe"));
        assert!(!over.ai_cleanup_enabled);
        assert_eq!(over.language, "en");
        // Unset fields keep global values.
        assert_eq!(over.ai_cleanup_tone, cfg.ai_cleanup_tone);

        // Non-matching / unknown app keeps globals.
        assert!(cfg.with_app_profile(Some("chrome.exe")).ai_cleanup_enabled);
        assert!(cfg.with_app_profile(None).ai_cleanup_enabled);

        // Empty match strings never match everything.
        cfg.app_profiles[0].app_match = "  ".into();
        assert!(cfg.with_app_profile(Some("code.exe")).ai_cleanup_enabled);
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
