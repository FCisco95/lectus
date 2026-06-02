use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub model_path: PathBuf,
    pub use_cloud: bool,
    pub cloud_base_url: String,
    pub cloud_api_key: String,
    pub hold_hotkey: String,
    pub toggle_hotkey: String,
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
            // reserved: toggle mode — kept in schema, not wired this phase.
            toggle_hotkey: "F13".into(),
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
    fn test_save_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.json");
        let cfg = Config::default();
        cfg.save_to(&path).unwrap();
        assert!(path.exists());
    }
}
