pub mod clipboard;

use anyhow::Result;

/// Whether the clipboard read-back equals the text we intended to paste.
fn clipboard_matches(expected: &str, readback: Option<String>) -> bool {
    readback.as_deref() == Some(expected)
}

/// Inject text into the currently focused field.
/// Strategy: save prior clipboard → set ours → confirm the OS accepted it via
/// read-back (bounded retry) → simulate paste → restore the prior clipboard.
/// Confirming the write before pasting fixes slow/deferred-read apps (Electron),
/// replacing the old fixed 50 ms guess.
pub fn inject_text(text: &str) -> Result<()> {
    let previous = {
        let mut cb = arboard::Clipboard::new()?;
        cb.get_text().ok()
    };

    clipboard::set_clipboard(text)?;

    // Confirm the clipboard actually holds our text before pasting (≤ ~150 ms).
    let mut confirmed = false;
    for _ in 0..15 {
        let readback = arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_text().ok());
        if clipboard_matches(text, readback) {
            confirmed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !confirmed {
        // Proceed anyway — better to attempt the paste than to drop the text.
        eprintln!("warning: clipboard read-back not confirmed before paste");
    }

    clipboard::simulate_paste()?;

    // Give the target app time to consume the paste, then restore the prior clipboard.
    if let Some(prev) = previous {
        std::thread::sleep(std::time::Duration::from_millis(200));
        clipboard::set_clipboard(&prev).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::clipboard_matches;

    #[test]
    fn matches_when_readback_equals_expected() {
        assert!(clipboard_matches("hello world", Some("hello world".to_string())));
    }

    #[test]
    fn no_match_when_stale_or_empty() {
        assert!(!clipboard_matches("hello world", Some("old text".to_string())));
        assert!(!clipboard_matches("hello world", None));
    }
}
