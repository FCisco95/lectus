//! Native Win32 `SendInput` text injection.
//!
//! Sends each UTF-16 code unit as a `KEYEVENTF_UNICODE` key event, which the
//! target app receives as `WM_CHAR` regardless of keyboard layout. This works
//! in terminals and CLIs where clipboard paste fails, and cannot race the
//! clipboard (the failure mode behind "pastes clipboard instead of spoken
//! text" bugs in other dictation apps).

use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_RETURN,
};

fn key_event(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Down+up pair for one UTF-16 code unit. Surrogate pairs arrive as two
/// consecutive units, which Windows reassembles into the full code point.
fn push_unicode(inputs: &mut Vec<INPUT>, unit: u16) {
    inputs.push(key_event(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE));
    inputs.push(key_event(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
}

/// Down+up pair for a virtual key (used for Enter, which many controls only
/// accept as a real key press rather than a unicode CR).
fn push_vk(inputs: &mut Vec<INPUT>, vk: VIRTUAL_KEY) {
    inputs.push(key_event(vk, 0, KEYBD_EVENT_FLAGS(0)));
    inputs.push(key_event(vk, 0, KEYEVENTF_KEYUP));
}

/// Type `text` into the focused field via one batched `SendInput` call.
pub fn inject_via_sendinput(text: &str) -> Result<()> {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.len() * 2);
    let mut buf = [0u16; 2];
    for ch in text.chars() {
        match ch {
            '\r' => {} // swallow; '\n' handles the newline
            '\n' => push_vk(&mut inputs, VK_RETURN),
            _ => {
                for unit in ch.encode_utf16(&mut buf) {
                    push_unicode(&mut inputs, *unit);
                }
            }
        }
    }
    if inputs.is_empty() {
        return Ok(());
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        // Partial injection: typically the input queue was blocked (UIPI —
        // e.g. an elevated window has focus). Caller falls back to clipboard.
        anyhow::bail!("SendInput injected {sent}/{} events", inputs.len());
    }
    Ok(())
}
