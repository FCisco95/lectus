use anyhow::Result;
use arboard::Clipboard;

/// Write text to the system clipboard.
pub fn set_clipboard(text: &str) -> Result<()> {
    let mut cb = Clipboard::new()?;
    cb.set_text(text.to_string())?;
    Ok(())
}

/// Simulate the paste keyboard shortcut for the current OS.
/// macOS: Cmd+V  |  Windows/Linux: Ctrl+V
pub fn simulate_paste() -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default())?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Direction::Press)?;
    let click_result = enigo.key(Key::Unicode('v'), Direction::Click);
    // Release modifier unconditionally — a stuck Ctrl/Cmd is visible to the user.
    enigo.key(modifier, Direction::Release).ok();
    click_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_clipboard_text() {
        let result = set_clipboard("chirp test content 12345");
        assert!(result.is_ok(), "clipboard write failed: {:?}", result);

        let mut cb = arboard::Clipboard::new().unwrap();
        let read_back = cb.get_text().unwrap();
        assert_eq!(read_back, "chirp test content 12345");
    }
}
