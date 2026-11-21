//! Platform-free keyboard logic for the controlled Mac: PC scan codes to Mac key codes,
//! modifier flags, and release of everything held when the link ends.
//!
//! Positional mapping (scan codes, not characters): both computers should use the same layout.
//! Modifiers follow the owner's Input Leap setup: PC Ctrl = Command, Windows key = Control,
//! Alt = Option. Right Ctrl is the dictation key and never reaches this module.
use std::collections::BTreeSet;

pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CONTROL: u64 = 0x0004_0000;
pub const FLAG_OPTION: u64 = 0x0008_0000;
pub const FLAG_COMMAND: u64 = 0x0010_0000;

/// Set-1 scan code (plus the E0 "extended" bit) to a Mac virtual key code.
pub fn mac_keycode(scan: u16, extended: bool) -> Option<u16> {
    let code = if extended {
        match scan {
            0x1C => 0x4C, // keypad Enter
            0x1D => 0x36, // right Ctrl -> right Command
            0x35 => 0x4B, // keypad /
            0x38 => 0x3D, // right Alt (AltGr) -> right Option
            0x47 => 0x73, // Home
            0x48 => 0x7E, // Up
            0x49 => 0x74, // Page Up
            0x4B => 0x7B, // Left
            0x4D => 0x7C, // Right
            0x4F => 0x77, // End
            0x50 => 0x7D, // Down
            0x51 => 0x79, // Page Down
            0x52 => 0x72, // Insert -> Help
            0x53 => 0x75, // forward Delete
            0x5B => 0x3B, // left Windows -> Control
            0x5C => 0x3E, // right Windows -> right Control
            _ => return None,
        }
    } else {
        match scan {
            0x01 => 0x35, // Esc
            0x02 => 0x12,
            0x03 => 0x13,
            0x04 => 0x14,
            0x05 => 0x15,
            0x06 => 0x17,
            0x07 => 0x16,
            0x08 => 0x1A,
            0x09 => 0x1C,
            0x0A => 0x19,
            0x0B => 0x1D,
            0x0C => 0x1B, // - (PT: ')
            0x0D => 0x18, // = (PT: «)
            0x0E => 0x33, // Backspace
            0x0F => 0x30, // Tab
            0x10 => 0x0C, // Q
            0x11 => 0x0D,
            0x12 => 0x0E,
            0x13 => 0x0F,
            0x14 => 0x11,
            0x15 => 0x10,
            0x16 => 0x20,
            0x17 => 0x22,
            0x18 => 0x1F,
            0x19 => 0x23, // P
            0x1A => 0x21,
            0x1B => 0x1E,
            0x1C => 0x24, // Return
            0x1D => 0x37, // left Ctrl -> Command
            0x1E => 0x00, // A
            0x1F => 0x01,
            0x20 => 0x02,
            0x21 => 0x03,
            0x22 => 0x05,
            0x23 => 0x04,
            0x24 => 0x26,
            0x25 => 0x28,
            0x26 => 0x25, // L
            0x27 => 0x29, // PT: ç
            0x28 => 0x27, // PT: º
            0x29 => 0x0A, // key left of 1 (ISO Mac: section key)
            0x2A => 0x38, // left Shift
            0x2B => 0x2A, // PT: ~
            0x2C => 0x06, // Z
            0x2D => 0x07,
            0x2E => 0x08,
            0x2F => 0x09,
            0x30 => 0x0B,
            0x31 => 0x2D,
            0x32 => 0x2E, // M
            0x33 => 0x2B,
            0x34 => 0x2F,
            0x35 => 0x2C,
            0x36 => 0x3C, // right Shift
            0x37 => 0x43, // keypad *
            0x38 => 0x3A, // left Alt -> Option
            0x39 => 0x31, // Space
            0x3B => 0x7A, // F1
            0x3C => 0x78,
            0x3D => 0x63,
            0x3E => 0x76,
            0x3F => 0x60,
            0x40 => 0x61,
            0x41 => 0x62,
            0x42 => 0x64,
            0x43 => 0x65,
            0x44 => 0x6D, // F10
            0x57 => 0x67, // F11
            0x58 => 0x6F, // F12
            0x45 => 0x47, // Num Lock -> keypad Clear
            0x47 => 0x59, // keypad 7
            0x48 => 0x5B,
            0x49 => 0x5C,
            0x4A => 0x4E, // keypad -
            0x4B => 0x56,
            0x4C => 0x57,
            0x4D => 0x58,
            0x4E => 0x45, // keypad +
            0x4F => 0x53,
            0x50 => 0x54,
            0x51 => 0x55,
            0x52 => 0x52, // keypad 0
            0x53 => 0x41, // keypad .
            0x56 => 0x32, // ISO key left of Z (PT: < >)
            _ => return None, // Caps Lock, Scroll Lock, Print Screen, Pause: not forwarded
        }
    };
    Some(code)
}

fn modifier_flag(code: u16) -> Option<u64> {
    match code {
        0x37 | 0x36 => Some(FLAG_COMMAND),
        0x38 | 0x3C => Some(FLAG_SHIFT),
        0x3B | 0x3E => Some(FLAG_CONTROL),
        0x3A | 0x3D => Some(FLAG_OPTION),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyAction {
    pub code: u16,
    pub down: bool,
    /// Modifier flags in force after this event.
    pub flags: u64,
    /// True for Shift/Control/Option/Command: post as a flags-changed event.
    pub modifier: bool,
    pub repeat: bool,
}

#[derive(Default)]
pub struct KeyState {
    held: BTreeSet<u16>,
}

impl KeyState {
    fn flags(&self) -> u64 {
        self.held.iter().filter_map(|&c| modifier_flag(c)).fold(0, |a, f| a | f)
    }

    pub fn handle(&mut self, scan: u16, extended: bool, down: bool) -> Option<KeyAction> {
        let code = mac_keycode(scan, extended)?;
        let repeat = down && self.held.contains(&code);
        if down {
            self.held.insert(code);
        } else if !self.held.remove(&code) {
            return None; // never pressed here: nothing to release
        }
        Some(KeyAction {
            code,
            down,
            flags: self.flags(),
            modifier: modifier_flag(code).is_some(),
            repeat,
        })
    }

    /// Key-ups for everything still held, normal keys first, modifiers last.
    pub fn release_all(&mut self) -> Vec<KeyAction> {
        let held: Vec<u16> = std::mem::take(&mut self.held).into_iter().collect();
        let (mods, keys): (Vec<u16>, Vec<u16>) =
            held.into_iter().partition(|&c| modifier_flag(c).is_some());
        let mut out = Vec::new();
        let mut remaining: Vec<u16> = mods.clone();
        for code in keys {
            out.push(KeyAction {
                code,
                down: false,
                flags: remaining.iter().filter_map(|&c| modifier_flag(c)).fold(0, |a, f| a | f),
                modifier: false,
                repeat: false,
            });
        }
        for code in mods {
            remaining.retain(|&c| c != code);
            out.push(KeyAction {
                code,
                down: false,
                flags: remaining.iter().filter_map(|&c| modifier_flag(c)).fold(0, |a, f| a | f),
                modifier: true,
                repeat: false,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_arrows_map_by_position() {
        assert_eq!(mac_keycode(0x1E, false), Some(0x00)); // A
        assert_eq!(mac_keycode(0x10, false), Some(0x0C)); // Q
        assert_eq!(mac_keycode(0x48, true), Some(0x7E)); // Up arrow
        assert_eq!(mac_keycode(0x48, false), Some(0x5B)); // keypad 8, not Up
    }

    #[test]
    fn ctrl_c_becomes_command_c() {
        let mut k = KeyState::default();
        let ctrl = k.handle(0x1D, false, true).unwrap();
        assert_eq!((ctrl.code, ctrl.modifier, ctrl.flags), (0x37, true, FLAG_COMMAND));
        let c = k.handle(0x2E, false, true).unwrap();
        assert_eq!((c.code, c.flags, c.modifier), (0x08, FLAG_COMMAND, false));
        k.handle(0x2E, false, false).unwrap();
        let up = k.handle(0x1D, false, false).unwrap();
        assert_eq!(up.flags, 0);
    }

    #[test]
    fn held_key_repeats_and_unknown_release_is_dropped() {
        let mut k = KeyState::default();
        assert!(!k.handle(0x1E, false, true).unwrap().repeat);
        assert!(k.handle(0x1E, false, true).unwrap().repeat);
        assert!(k.handle(0x30, false, false).is_none());
    }

    #[test]
    fn unmapped_keys_are_ignored() {
        let mut k = KeyState::default();
        assert!(k.handle(0x46, false, true).is_none()); // Scroll Lock
        assert!(k.handle(0x3A, false, true).is_none()); // Caps Lock
    }

    #[test]
    fn release_all_lets_go_of_keys_then_modifiers() {
        let mut k = KeyState::default();
        k.handle(0x2A, false, true); // Shift
        k.handle(0x1E, false, true); // A
        let out = k.release_all();
        assert_eq!(out.len(), 2);
        assert_eq!((out[0].code, out[0].flags), (0x00, FLAG_SHIFT));
        assert_eq!((out[1].code, out[1].flags, out[1].modifier), (0x38, 0, true));
        assert!(k.release_all().is_empty());
    }
}
