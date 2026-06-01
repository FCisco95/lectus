pub mod clipboard;

use anyhow::Result;

/// Inject text into the currently focused text field.
/// Strategy: set clipboard → small delay → simulate paste → restore clipboard.
/// Phase 2 will add AXUIElement direct injection as the primary path.
pub fn inject_text(text: &str) -> Result<()> {
    let previous = {
        let mut cb = arboard::Clipboard::new()?;
        cb.get_text().ok()
    };

    clipboard::set_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    clipboard::simulate_paste()?;

    // FIXME(phase-2): fixed delay doesn't confirm paste was consumed — slow apps
    // (e.g. Electron) may receive stale clipboard if they defer the read past 200ms.
    if let Some(prev) = previous {
        std::thread::sleep(std::time::Duration::from_millis(200));
        clipboard::set_clipboard(&prev).ok();
    }

    Ok(())
}
