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
    /// Model IDs are independent because speech and text endpoints accept different models.
    pub cloud_transcription_model: String,
    pub cloud_cleanup_model: String,
    /// Output translation is separate from recognition language and cleanup.
    /// "same" keeps the recognized language; otherwise an explicit target code.
    pub output_language: String,
    /// "local", "cloud", "chatgpt" or "claude". Never switches on failure.
    pub translation_engine: String,
    pub translation_model: String,
    pub cloud_translation_model: String,
    /// Explicit acknowledgement for audio/text processing by the configured
    /// cloud provider. Reset when its base URL changes; not a legal waiver.
    pub cloud_processing_acknowledged: bool,
    /// Separate acknowledgement for sending transcripts through the installed
    /// Claude CLI and its account/provider configuration.
    pub claude_processing_acknowledged: bool,
    pub chatgpt_processing_acknowledged: bool,
    /// Shared by cleanup and translation; empty selects the official default.
    pub chatgpt_model: String,
    pub claude_model: String,
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
    /// Language-scoped fixes for a word heard as a different real word
    /// (e.g. "myself" → "Mycel" in Portuguese). Applied before spellings.
    pub sound_alikes: Vec<SoundAlike>,
    /// Whether to run an AI cleanup pass (punctuation, casing, filler removal).
    pub ai_cleanup_enabled: bool,
    /// "local", "groq" (configured API), "chatgpt" or "claude" (subscriptions).
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
    /// UI theme: "system" (follow OS), "light", or "dark".
    pub theme: String,
    /// Mute (or duck) system playback while the dictation key is held so
    /// background audio does not drown out your own thoughts. Restored on
    /// release, error, idle, and process exit. Default on.
    pub mute_while_dictating: bool,
    /// Name shown in the Home greeting. Empty means derive it from the OS
    /// account (`identity::os_display_name`). Older configs without the key
    /// load as empty via the container-level `serde(default)`.
    pub display_name: String,
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
    /// Settings that authorize or shape text leaving the device. Cosmetic and
    /// hardware choices do not cancel text generation; edits to either global
    /// settings or per-app profiles invalidate preflight work before dispatch.
    pub fn text_processing_changed(&self, next: &Config) -> bool {
        self.cloud_processing_acknowledged != next.cloud_processing_acknowledged
            || self.claude_processing_acknowledged != next.claude_processing_acknowledged
            || self.chatgpt_processing_acknowledged != next.chatgpt_processing_acknowledged
            || self.cloud_base_url != next.cloud_base_url
            || self.cloud_api_key != next.cloud_api_key
            || self.cloud_cleanup_model != next.cloud_cleanup_model
            || self.chatgpt_model != next.chatgpt_model
            || self.claude_model != next.claude_model
            || self.ai_cleanup_enabled != next.ai_cleanup_enabled
            || self.ai_cleanup_engine != next.ai_cleanup_engine
            || self.ai_cleanup_tone != next.ai_cleanup_tone
            || self.language != next.language
            || self.dictionary_words != next.dictionary_words
            || self.replacement_rules != next.replacement_rules
            || self.sound_alikes != next.sound_alikes
            || self.app_profiles != next.app_profiles
            || self.output_language != next.output_language
            || self.translation_engine != next.translation_engine
            || self.translation_model != next.translation_model
            || self.cloud_translation_model != next.cloud_translation_model
    }

    /// Re-check the current authority before an earlier dictation snapshot can
    /// send data. Feature-specific enablement is checked by the caller too.
    pub fn allows_cloud_snapshot(&self, snapshot: &Config) -> bool {
        self.cloud_processing_acknowledged && snapshot.cloud_processing_acknowledged
            && self.cloud_base_url == snapshot.cloud_base_url
            && self.cloud_api_key == snapshot.cloud_api_key
    }

    /// Reapply the original target's current profile, never the app focused
    /// after recording. A global off/profile on selection remains authorized,
    /// while removing the profile or changing its settings revokes the snapshot.
    pub fn allows_cleanup_snapshot(&self, snapshot: &Config, original_app: Option<&str>) -> bool {
        let current = self.with_app_profile(original_app);
        current.ai_cleanup_enabled && snapshot.ai_cleanup_enabled
            && current.ai_cleanup_engine == snapshot.ai_cleanup_engine
            && current.ai_cleanup_tone == snapshot.ai_cleanup_tone
            && current.language == snapshot.language
            && current.dictionary_words == snapshot.dictionary_words
            && current.sound_alikes == snapshot.sound_alikes
            && match snapshot.ai_cleanup_engine.as_str() {
                "local" => true,
                "groq" => current.allows_cloud_snapshot(snapshot)
                    && current.cloud_cleanup_model == snapshot.cloud_cleanup_model,
                "chatgpt" => current.allows_subscription_snapshot(snapshot, "chatgpt"),
                "claude" => current.allows_subscription_snapshot(snapshot, "claude"),
                _ => false,
            }
    }

    pub fn allows_translation_snapshot(&self, snapshot: &Config, original_app: Option<&str>) -> bool {
        let current = self.with_app_profile(original_app);
        current.output_language != "same"
            && current.output_language == snapshot.output_language
            && current.translation_engine == snapshot.translation_engine
            && current.dictionary_words == snapshot.dictionary_words
            && current.sound_alikes == snapshot.sound_alikes
            && match snapshot.translation_engine.as_str() {
                "local" => current.translation_model == snapshot.translation_model,
                "cloud" => current.allows_cloud_snapshot(snapshot)
                    && current.cloud_translation_model == snapshot.cloud_translation_model,
                "chatgpt" => current.allows_subscription_snapshot(snapshot, "chatgpt"),
                "claude" => current.allows_subscription_snapshot(snapshot, "claude"),
                _ => false,
            }
    }

    fn allows_subscription_snapshot(&self, snapshot: &Config, provider: &str) -> bool {
        match provider {
            "chatgpt" => self.chatgpt_processing_acknowledged && snapshot.chatgpt_processing_acknowledged
                && self.chatgpt_model == snapshot.chatgpt_model,
            "claude" => self.claude_processing_acknowledged && snapshot.claude_processing_acknowledged
                && self.claude_model == snapshot.claude_model,
            _ => false,
        }
    }

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

/// "When the recognizer writes `heard`, write `word` instead", only in the
/// listed languages. An empty language list never fires.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SoundAlike {
    pub heard: String,
    pub word: String,
    pub languages: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/ggml-tiny.en.bin"),
            use_cloud: false,
            cloud_base_url: "https://api.groq.com/openai/v1".into(),
            cloud_api_key: String::new(),
            cloud_transcription_model: "whisper-large-v3-turbo".into(),
            cloud_cleanup_model: "openai/gpt-oss-120b".into(),
            output_language: "same".into(),
            translation_engine: "local".into(),
            translation_model: "Qwen3-4B-Q4_K_M.gguf".into(),
            cloud_translation_model: "openai/gpt-oss-120b".into(),
            cloud_processing_acknowledged: false,
            claude_processing_acknowledged: false,
            chatgpt_processing_acknowledged: false,
            chatgpt_model: String::new(),
            claude_model: String::new(),
            // Hold-to-talk default: bare Right Ctrl. Activated via a low-level
            // keyboard hook (Win WH_KEYBOARD_LL / macOS CGEventTap), not the
            // global-shortcut plugin (which can't register a bare modifier).
            hold_hotkey: "RControl".into(),
            toggle_hotkey: "F13".into(),
            pill_x: 100,
            pill_y: 100,
            language: "auto".into(),
            // base, not tiny: cheapest model with correct PT on the M4 bench
            // (tiny mis-hears "quinta-feira"; base fixes it at +60–120 ms).
            // See docs/benchmarks/2026-08-01-model-bench-m4.md.
            model_name: "ggml-base.bin".into(),
            trigger_mode: "hold".into(),
            dictionary_words: Vec::new(),
            replacement_rules: Vec::new(),
            sound_alikes: Vec::new(),
            ai_cleanup_enabled: false,
            ai_cleanup_engine: "claude".into(),
            ai_cleanup_tone: "neutral".into(),
            input_device: String::new(),
            injection_mode: "auto".into(),
            vad_enabled: true,
            app_profiles: Vec::new(),
            onboarding_completed: false,
            theme: "system".into(),
            mute_while_dictating: true,
            display_name: String::new(),
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
        // All writers share the same atomic-replace temporary path.
        static SAVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _save = SAVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Atomic write: a crash mid-save must not leave a truncated config
        // that would deserialize as an error and silently fall back to
        // defaults (including `model_name`) on the next launch.
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Overlay a Settings-form snapshot without clobbering fields the form
    /// does not own. The Settings webview stays mounted while hidden, so its
    /// copy of `model_name` / `model_path` / pill position can be stale
    /// relative to `select_model` and drag-to-move. Writing that snapshot
    /// straight to disk is what reset the chosen model after a reboot.
    ///
    /// `onboarding_completed` is owned by the dedicated completion command:
    /// generic settings saves can neither bypass nor undo setup completion.
    pub fn apply_ui_update(&mut self, mut incoming: Config) {
        incoming.model_name = self.model_name.clone();
        incoming.model_path = self.model_path.clone();
        incoming.pill_x = self.pill_x;
        incoming.pill_y = self.pill_y;
        incoming.onboarding_completed = self.onboarding_completed;
        *self = incoming;
    }

    /// Platform app-support dir (`%APPDATA%\ai.organic.lectus` on Windows).
    /// Used to load config before Tauri windows exist, so the first
    /// `get_config` invoke cannot return defaults and flash onboarding.
    pub fn app_support_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("ai.organic.lectus")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("Library/Application Support/ai.organic.lectus")
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/ai.organic.lectus")
        }
    }

    pub fn config_file() -> PathBuf {
        Self::app_support_dir().join("config.json")
    }

    pub fn load_or_default() -> Self {
        Self::load_from(&Self::config_file()).unwrap_or_else(|_| Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn text_processing_changes_invalidate_preflight_but_cosmetic_updates_do_not() {
        let current = Config::default();
        let policies: &[fn(&mut Config)] = &[
            |c| c.chatgpt_processing_acknowledged = true,
            |c| c.claude_processing_acknowledged = true,
            |c| c.cloud_processing_acknowledged = true,
            |c| c.chatgpt_model = "selected".into(),
            |c| c.claude_model = "sonnet".into(),
            |c| c.cloud_base_url = "https://other.example/v1".into(),
            |c| c.cloud_api_key = "fixture".into(),
            |c| c.cloud_cleanup_model = "other".into(),
            |c| c.ai_cleanup_enabled = true,
            |c| c.ai_cleanup_engine = "chatgpt".into(),
            |c| c.ai_cleanup_tone = "formal".into(),
            |c| c.language = "pt".into(),
            |c| c.dictionary_words.push("Lectus".into()),
            |c| c.replacement_rules.push(ReplacementRule { from: "raw".into(), to: "fixed".into(), case_sensitive: false }),
            |c| c.app_profiles.push(AppProfile { app_match: "editor".into(), ai_cleanup_enabled: Some(true), ..AppProfile::default() }),
            |c| c.output_language = "pt".into(),
            |c| c.translation_engine = "claude".into(),
            |c| c.translation_model = "different.gguf".into(),
            |c| c.cloud_translation_model = "different".into(),
        ];
        for (index, update) in policies.iter().enumerate() {
            let mut changed = current.clone();
            update(&mut changed);
            assert!(current.text_processing_changed(&changed), "policy {index} did not invalidate");
            assert!(changed.text_processing_changed(&current), "policy {index} revocation did not invalidate");
        }
        let mut cosmetic = current.clone();
        cosmetic.pill_x += 1;
        cosmetic.theme = "dark".into();
        cosmetic.display_name = "Display name".into();
        cosmetic.input_device = "Other microphone".into();
        cosmetic.model_name = "other-speech.bin".into();
        cosmetic.onboarding_completed = true;
        assert!(!current.text_processing_changed(&cosmetic));
        assert!(!current.text_processing_changed(&current));
    }

    #[test]
    fn subscription_settings_migrate_without_enabling_or_granting_consent() {
        let old: Config = serde_json::from_str(r#"{"claude_processing_acknowledged":true}"#).unwrap();
        assert!(!old.chatgpt_processing_acknowledged);
        assert!(old.chatgpt_model.is_empty() && old.claude_model.is_empty());
        assert!(old.claude_processing_acknowledged);
        assert!(!old.ai_cleanup_enabled);
        assert_eq!(old.output_language, "same");
        let custom = Config { chatgpt_model: "model-a".into(), claude_model: "sonnet".into(),
            chatgpt_processing_acknowledged: true, ..old };
        assert_eq!(serde_json::from_str::<Config>(&serde_json::to_string(&custom).unwrap()).unwrap(), custom);
    }

    #[test]
    fn cleanup_authority_uses_original_profile_and_revokes_changed_selection() {
        for provider in ["local", "groq", "chatgpt", "claude"] {
            let current = Config { ai_cleanup_enabled: false, ai_cleanup_engine: provider.into(),
                cloud_processing_acknowledged: true, claude_processing_acknowledged: true,
                chatgpt_processing_acknowledged: true,
                app_profiles: vec![AppProfile { app_match: "editor".into(), ai_cleanup_enabled: Some(true),
                    ai_cleanup_tone: Some("formal".into()), ..AppProfile::default() }], ..Config::default() };
            let snapshot = current.with_app_profile(Some("editor.exe"));
            assert!(current.allows_cleanup_snapshot(&snapshot, Some("editor.exe")), "{provider}");
            assert!(!current.allows_cleanup_snapshot(&snapshot, Some("other.exe")));
            let mut changed = current.clone();
            changed.app_profiles.clear();
            assert!(!changed.allows_cleanup_snapshot(&snapshot, Some("editor.exe")));
            changed = current.clone();
            changed.ai_cleanup_engine = "different".into();
            assert!(!changed.allows_cleanup_snapshot(&snapshot, Some("editor.exe")));
            changed = current.clone();
            changed.app_profiles[0].ai_cleanup_tone = Some("casual".into());
            assert!(!changed.allows_cleanup_snapshot(&snapshot, Some("editor.exe")));
        }
    }

    #[test]
    fn subscription_snapshot_requires_both_consents_and_same_model_for_each_task() {
        for provider in ["chatgpt", "claude"] {
            let snapshot = Config { ai_cleanup_enabled: true, ai_cleanup_engine: provider.into(),
                output_language: "en".into(), translation_engine: provider.into(),
                chatgpt_processing_acknowledged: true, claude_processing_acknowledged: true, ..Config::default() };
            assert!(snapshot.allows_cleanup_snapshot(&snapshot, None));
            assert!(snapshot.allows_translation_snapshot(&snapshot, None));
            let mut changed = snapshot.clone();
            if provider == "chatgpt" { changed.chatgpt_processing_acknowledged = false; }
            else { changed.claude_processing_acknowledged = false; }
            assert!(!changed.allows_cleanup_snapshot(&snapshot, None));
            assert!(!snapshot.allows_cleanup_snapshot(&changed, None));
            assert!(!changed.allows_translation_snapshot(&snapshot, None));
            assert!(!snapshot.allows_translation_snapshot(&changed, None));
            changed = snapshot.clone();
            if provider == "chatgpt" { changed.chatgpt_model = "new-model".into(); }
            else { changed.claude_model = "opus".into(); }
            assert!(!changed.allows_cleanup_snapshot(&snapshot, None));
            assert!(!changed.allows_translation_snapshot(&snapshot, None));
            changed = snapshot.clone();
            changed.output_language = "pt".into();
            assert!(!changed.allows_translation_snapshot(&snapshot, None));
            changed.output_language = "same".into();
            assert!(!changed.allows_translation_snapshot(&snapshot, None));
        }
    }

    #[test]
    fn cloud_snapshot_authority_requires_current_acknowledgement_and_same_credentials() {
        let snapshot = Config { cloud_processing_acknowledged: true, cloud_api_key: "fixture-a".into(), ..Config::default() };
        let mut current = snapshot.clone();
        assert!(current.allows_cloud_snapshot(&snapshot));
        current.cloud_base_url = "https://other.example/v1".into();
        assert!(!current.allows_cloud_snapshot(&snapshot));
        current = snapshot.clone();
        current.cloud_api_key = "fixture-b".into();
        assert!(!current.allows_cloud_snapshot(&snapshot));
        current = snapshot.clone();
        current.cloud_processing_acknowledged = false;
        assert!(!current.allows_cloud_snapshot(&snapshot));
        assert!(!snapshot.allows_cloud_snapshot(&current));
    }

    #[test]
    fn translation_migrates_disabled_and_preserves_independent_provider_choices() {
        let old: Config = serde_json::from_str(r#"{"use_cloud":true,"language":"zh"}"#).unwrap();
        assert_eq!(old.output_language, "same");
        assert_eq!(old.translation_engine, "local");
        assert_eq!(old.translation_model, "Qwen3-4B-Q4_K_M.gguf");
        assert_eq!(old.cloud_translation_model, "openai/gpt-oss-120b");
        assert!(!old.cloud_processing_acknowledged);
        assert!(!old.claude_processing_acknowledged);
        let custom = Config { output_language: "zh-Hant".into(), translation_engine: "cloud".into(),
            cloud_translation_model: "custom-translator".into(), use_cloud: false, ..old };
        let restored: Config = serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(restored, custom);
        assert!(!restored.use_cloud);
    }

    #[test]
    fn cloud_models_migrate_old_configs_and_preserve_custom_ids() {
        let old: Config = serde_json::from_str(r#"{"use_cloud":true}"#).unwrap();
        assert_eq!(old.cloud_transcription_model, "whisper-large-v3-turbo");
        assert_eq!(old.cloud_cleanup_model, "openai/gpt-oss-120b");
        let custom = Config { cloud_transcription_model: "custom-speech".into(), cloud_cleanup_model: "custom-chat".into(), ..old };
        let restored: Config = serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(restored, custom);
    }

    #[test]
    fn concurrent_saves_leave_one_complete_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::thread::scope(|scope| {
            for i in 0..12 {
                let path = &path;
                scope.spawn(move || Config { display_name: format!("writer-{i}"), ..Config::default() }.save_to(path).unwrap());
            }
        });
        assert!(Config::load_from(&path).unwrap().display_name.starts_with("writer-"));
        assert!(!path.with_extension("json.tmp").exists());
    }

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
        assert!(loaded.mute_while_dictating);
    }

    #[test]
    fn mute_while_dictating_defaults_on() {
        assert!(Config::default().mute_while_dictating);
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"use_cloud": false}"#).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert!(loaded.mute_while_dictating);
    }

    #[test]
    fn display_name_defaults_empty_and_roundtrips() {
        assert!(Config::default().display_name.is_empty());
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        // A config from before the field existed: empty, not an error.
        std::fs::write(&path, r#"{"theme": "dark"}"#).unwrap();
        assert!(Config::load_from(&path).unwrap().display_name.is_empty());
        let mut cfg = Config::default();
        cfg.display_name = "João".into();
        cfg.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap().display_name, "João");
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

    #[test]
    fn test_save_is_atomic_no_tmp_left() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save_to(&path).unwrap();
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.model_name, Config::default().model_name);
    }

    #[test]
    fn test_ui_update_does_not_clobber_selected_model_or_pill() {
        let mut live = Config::default();
        live.model_name = "ggml-large-v3-turbo.bin".into();
        live.model_path = PathBuf::from(r"C:\models\ggml-large-v3-turbo.bin");
        live.pill_x = 1533;
        live.pill_y = 1309;
        live.theme = "dark".into();

        // Stale Settings snapshot still has the previous model and default pill.
        let mut incoming = Config::default();
        incoming.theme = "light".into();
        incoming.dictionary_words = vec!["Mycel".into()];
        incoming.model_name = "ggml-base.bin".into();
        incoming.model_path = PathBuf::from("models/ggml-tiny.en.bin");

        incoming.mute_while_dictating = false;
        live.apply_ui_update(incoming);

        assert_eq!(live.model_name, "ggml-large-v3-turbo.bin");
        assert_eq!(live.model_path, PathBuf::from(r"C:\models\ggml-large-v3-turbo.bin"));
        assert_eq!(live.pill_x, 1533);
        assert_eq!(live.pill_y, 1309);
        assert_eq!(live.theme, "light");
        assert_eq!(live.dictionary_words, vec!["Mycel".to_string()]);
        assert!(!live.mute_while_dictating);
        assert!(!live.onboarding_completed);
    }

    #[test]
    fn test_ui_update_does_not_revert_onboarding() {
        let mut live = Config::default();
        live.onboarding_completed = true;
        live.dictionary_words = vec!["Mycel".into(), "Claude".into()];
        live.model_name = "ggml-large-v3-turbo.bin".into();

        // Stale Onboarding/Settings snapshot captured before disk load.
        let incoming = Config::default();
        assert!(!incoming.onboarding_completed);

        live.apply_ui_update(incoming);

        assert!(
            live.onboarding_completed,
            "finishing a stale onboarding form must not reset the completed flag"
        );
        assert_eq!(live.model_name, "ggml-large-v3-turbo.bin");
    }

    #[test]
    fn test_ui_update_cannot_complete_onboarding() {
        let mut live = Config::default();
        let mut incoming = Config::default();
        incoming.onboarding_completed = true;
        live.apply_ui_update(incoming);
        assert!(!live.onboarding_completed);
    }

    #[test]
    fn test_load_or_default_reads_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut saved = Config::default();
        saved.model_name = "ggml-large-v3-turbo.bin".into();
        saved.dictionary_words = vec!["Mycel".into()];
        saved.onboarding_completed = true;
        saved.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.model_name, "ggml-large-v3-turbo.bin");
        assert_eq!(loaded.dictionary_words, vec!["Mycel".to_string()]);
        assert!(loaded.onboarding_completed);
    }
}
