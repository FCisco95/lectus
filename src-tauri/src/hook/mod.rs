//! Cross-platform low-level keyboard hook for hold-to-talk activation.
//!
//! On key-down of the configured target key the platform hook sets
//! `recording_active = true` and emits `hold-start`; on key-up it clears the
//! flag and emits `hold-stop`. The key-string→native-key maps below are the
//! unit-testable core shared by both platforms.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Everything a platform hook needs to install.
pub struct HookContext {
    pub app: tauri::AppHandle,
    pub recording_active: Arc<AtomicBool>,
    pub target_key: String,
    /// "hold" (push-to-talk) or "toggle" (tap on / tap off).
    pub trigger_mode: String,
}

/// Numeric trigger-mode code shared with the platform hooks (which keep it in an
/// atomic). 1 = toggle, 0 = hold.
pub const MODE_HOLD: u32 = 0;
pub const MODE_TOGGLE: u32 = 1;

pub fn mode_code(s: &str) -> u32 {
    if s.eq_ignore_ascii_case("toggle") {
        MODE_TOGGLE
    } else {
        MODE_HOLD
    }
}

/// A platform keyboard hook. Installed once at startup; `rearm` swaps the
/// watched key live (no thread teardown). Cleanup happens on `Drop`.
pub trait KeyboardHook: Sized {
    fn install(ctx: HookContext) -> anyhow::Result<Self>;
    fn rearm(&self, target_key: &str);
    /// Live-swap the trigger mode ("hold"/"toggle") without reinstalling.
    fn set_mode(&self, mode: &str);
    /// Force the internal toggle latch (so click-to-toggle and the physical
    /// toggle key stay in sync). No-op in hold mode.
    fn set_toggle_state(&self, on: bool);
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsHook as PlatformHook;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacosHook as PlatformHook;

// Fallback no-op so the crate still compiles on other targets (e.g. Linux CI).
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub struct PlatformHook;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl KeyboardHook for PlatformHook {
    fn install(_ctx: HookContext) -> anyhow::Result<Self> {
        anyhow::bail!("keyboard hook not supported on this platform")
    }
    fn rearm(&self, _target_key: &str) {}
    fn set_mode(&self, _mode: &str) {}
    fn set_toggle_state(&self, _on: bool) {}
}

/// Map a Lectus config key string to a Windows virtual-key code (`VK_*`).
/// Returns `None` for unknown keys or combos (only bare modifiers / F13-F15
/// are valid hold-to-talk targets). Defined with plain literals so it compiles
/// and is the canonical key validator on every platform.
pub fn config_key_to_vk(s: &str) -> Option<u32> {
    match s {
        "RControl" => Some(0xA3), // VK_RCONTROL
        "LControl" => Some(0xA2), // VK_LCONTROL
        "RShift" => Some(0xA1),   // VK_RSHIFT
        "LShift" => Some(0xA0),   // VK_LSHIFT
        "RAlt" => Some(0xA5),     // VK_RMENU
        "LAlt" => Some(0xA4),     // VK_LMENU
        "F13" => Some(0x7C),
        "F14" => Some(0x7D),
        "F15" => Some(0x7E),
        _ => None,
    }
}

/// Map a Lectus config key string to a macOS virtual keycode (`kVK_*`).
pub fn config_key_to_mackey(s: &str) -> Option<u16> {
    match s {
        "RControl" => Some(62),
        "LControl" => Some(59),
        "RShift" => Some(60),
        "LShift" => Some(56),
        "RAlt" => Some(61),
        "LAlt" => Some(58),
        "F13" => Some(105),
        "F14" => Some(107),
        "F15" => Some(113),
        _ => None,
    }
}

/// Whether a config string is a valid hold-to-talk target on any platform.
pub fn is_supported_hold_key(s: &str) -> bool {
    config_key_to_vk(s).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_map_known_keys() {
        assert_eq!(config_key_to_vk("RControl"), Some(0xA3));
        assert_eq!(config_key_to_vk("LControl"), Some(0xA2));
        assert_eq!(config_key_to_vk("RShift"), Some(0xA1));
        assert_eq!(config_key_to_vk("LShift"), Some(0xA0));
        assert_eq!(config_key_to_vk("RAlt"), Some(0xA5));
        assert_eq!(config_key_to_vk("LAlt"), Some(0xA4));
        assert_eq!(config_key_to_vk("F13"), Some(0x7C));
        assert_eq!(config_key_to_vk("F14"), Some(0x7D));
        assert_eq!(config_key_to_vk("F15"), Some(0x7E));
    }

    #[test]
    fn vk_map_rejects_junk_and_combos() {
        assert_eq!(config_key_to_vk("NotAKey"), None);
        assert_eq!(config_key_to_vk("Ctrl+Shift+Space"), None);
        assert_eq!(config_key_to_vk(""), None);
    }

    #[test]
    fn mackey_map_known_keys() {
        assert_eq!(config_key_to_mackey("RControl"), Some(62));
        assert_eq!(config_key_to_mackey("LControl"), Some(59));
        assert_eq!(config_key_to_mackey("RShift"), Some(60));
        assert_eq!(config_key_to_mackey("LShift"), Some(56));
        assert_eq!(config_key_to_mackey("RAlt"), Some(61));
        assert_eq!(config_key_to_mackey("LAlt"), Some(58));
        assert_eq!(config_key_to_mackey("F13"), Some(105));
        assert_eq!(config_key_to_mackey("F14"), Some(107));
        assert_eq!(config_key_to_mackey("F15"), Some(113));
    }

    #[test]
    fn mackey_map_rejects_junk() {
        assert_eq!(config_key_to_mackey("NotAKey"), None);
    }

    #[test]
    fn is_supported_matches_vk_map() {
        assert!(is_supported_hold_key("RControl"));
        assert!(!is_supported_hold_key("Ctrl+Shift+Space"));
    }

    #[test]
    fn mode_code_maps_toggle_and_hold() {
        assert_eq!(mode_code("toggle"), MODE_TOGGLE);
        assert_eq!(mode_code("Toggle"), MODE_TOGGLE);
        assert_eq!(mode_code("hold"), MODE_HOLD);
        assert_eq!(mode_code("anything-else"), MODE_HOLD);
    }
}
