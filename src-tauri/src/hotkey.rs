#![allow(dead_code)] // off the call path since Phase 2.5 (kept for reference / tests)
use anyhow::{anyhow, Result};

/// Normalize a config hotkey string to tauri-plugin-global-shortcut format.
///
/// Supported bare keys: RControl, LControl, F13, F14, F15, Space
/// Supported combos: "Modifier+Key" using Meta/Ctrl/Shift/Alt prefixes
///
/// Returns a string the plugin's register() accepts.
pub fn parse_hotkey_str(s: &str) -> Result<String> {
    let parts: Vec<&str> = s.split('+').collect();
    let key_str = parts.last().unwrap();

    // Validate key
    let key_code = match *key_str {
        "RControl" | "RCtrl" => "ControlRight",
        "LControl" | "LCtrl" => "ControlLeft",
        "F13" => "F13",
        "F14" => "F14",
        "F15" => "F15",
        "Space" => "Space",
        other => return Err(anyhow!("unknown key: {other}")),
    };

    // Validate modifiers
    let mut mod_parts: Vec<&str> = Vec::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        match *part {
            "Meta" | "Cmd" => mod_parts.push("META"),
            "Ctrl" | "Control" => mod_parts.push("CTRL"),
            "Shift" => mod_parts.push("SHIFT"),
            "Alt" | "Option" => mod_parts.push("ALT"),
            other => return Err(anyhow!("unknown modifier: {other}")),
        }
    }

    if mod_parts.is_empty() {
        Ok(key_code.to_string())
    } else {
        mod_parts.push(key_code);
        Ok(mod_parts.join("+"))
    }
}

/// Holds validated shortcut strings for the two hotkey actions.
/// Registration against the app handle happens in the Tauri setup callback (Task 10).
pub struct HotkeyManager {
    pub hold_shortcut: String,
    pub toggle_shortcut: String,
}

impl HotkeyManager {
    pub fn from_config(hold_str: &str, toggle_str: &str) -> Result<Self> {
        Ok(Self {
            hold_shortcut: parse_hotkey_str(hold_str)?,
            toggle_shortcut: parse_hotkey_str(toggle_str)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey_str_rcontrol() {
        let s = parse_hotkey_str("RControl").unwrap();
        assert_eq!(s, "ControlRight");
    }

    #[test]
    fn test_parse_hotkey_str_f13() {
        let s = parse_hotkey_str("F13").unwrap();
        assert_eq!(s, "F13");
    }

    #[test]
    fn test_parse_invalid_hotkey_returns_err() {
        let result = parse_hotkey_str("NotAKey");
        assert!(result.is_err());
    }

    #[test]
    fn test_hotkey_manager_stores_shortcuts() {
        let mgr = HotkeyManager::from_config("RControl", "F13").unwrap();
        assert_eq!(mgr.hold_shortcut, "ControlRight");
        assert_eq!(mgr.toggle_shortcut, "F13");
    }
}
