//! Transcription history: the last N dictations, persisted to `history.json` in
//! the app data dir. Capped and clearable. Writes are atomic (temp + rename) so
//! a crash mid-write can't corrupt the store.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Keep at most this many entries (newest last); older ones are dropped.
pub const MAX_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub text: String,
    /// Unix epoch milliseconds when the dictation completed.
    pub timestamp: i64,
    /// The language used, when known (`None` for auto-detect).
    pub language: Option<String>,
}

/// `<app_data>/history.json`.
fn history_path(dir: &Path) -> PathBuf {
    dir.join("history.json")
}

/// Load the history list from `dir` (empty if missing or unreadable).
pub fn load(dir: &Path) -> Vec<HistoryEntry> {
    let path = history_path(dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Append an entry, enforce the cap, and persist atomically.
pub fn append(dir: &Path, entry: HistoryEntry) -> Result<Vec<HistoryEntry>> {
    let mut entries = load(dir);
    entries.push(entry);
    if entries.len() > MAX_ENTRIES {
        let overflow = entries.len() - MAX_ENTRIES;
        entries.drain(0..overflow);
    }
    save(dir, &entries)?;
    Ok(entries)
}

/// Clear all history.
pub fn clear(dir: &Path) -> Result<()> {
    save(dir, &[])
}

/// Write the list atomically: serialize to a `.tmp` sibling, then rename over
/// the target so readers never see a half-written file.
fn save(dir: &Path, entries: &[HistoryEntry]) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = history_path(dir);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(entries)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn entry(text: &str, ts: i64) -> HistoryEntry {
        HistoryEntry { text: text.into(), timestamp: ts, language: None }
    }

    #[test]
    fn append_and_load_roundtrips() {
        let dir = tempdir().unwrap();
        append(dir.path(), entry("hello", 1)).unwrap();
        append(dir.path(), entry("world", 2)).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].text, "hello");
        assert_eq!(loaded[1].text, "world");
    }

    #[test]
    fn caps_at_max_entries_dropping_oldest() {
        let dir = tempdir().unwrap();
        for i in 0..(MAX_ENTRIES as i64 + 10) {
            append(dir.path(), entry(&format!("e{i}"), i)).unwrap();
        }
        let loaded = load(dir.path());
        assert_eq!(loaded.len(), MAX_ENTRIES);
        // Oldest (e0..e9) dropped; newest retained.
        assert_eq!(loaded.first().unwrap().text, "e10");
        assert_eq!(loaded.last().unwrap().text, format!("e{}", MAX_ENTRIES + 9));
    }

    #[test]
    fn clear_empties_the_store() {
        let dir = tempdir().unwrap();
        append(dir.path(), entry("x", 1)).unwrap();
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn load_missing_is_empty() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_empty());
    }
}
