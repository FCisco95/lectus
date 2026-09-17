//! Transcription history: the last N dictations, persisted to `history.json` in
//! the app data dir. Capped and clearable. Writes are atomic (temp + rename) so
//! a crash mid-write can't corrupt the store.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Keep at most this many entries (newest last); older ones are dropped.
/// Home's "this week" stats read this list, so the cap has to survive a busy day.
pub const MAX_ENTRIES: usize = 500;

/// Rolling window used by Home's weekly counts.
pub const WEEK_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HistoryStats {
    pub dictations_7d: usize,
    pub words_7d: usize,
    pub dictations_total: usize,
    pub words_total: usize,
}

/// Whitespace-separated tokens. Empty / whitespace-only text is zero words.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Counts over `entries`. `now_ms` is Unix epoch milliseconds.
pub fn stats(entries: &[HistoryEntry], now_ms: i64) -> HistoryStats {
    let cutoff = now_ms.saturating_sub(WEEK_MS);
    let mut out = HistoryStats {
        dictations_total: entries.len(),
        ..HistoryStats::default()
    };
    for e in entries {
        let words = word_count(&e.text);
        out.words_total += words;
        if e.timestamp >= cutoff {
            out.dictations_7d += 1;
            out.words_7d += words;
        }
    }
    out
}

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

    #[test]
    fn word_count_splits_on_whitespace() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   "), 0);
        assert_eq!(word_count("Mycel"), 1);
        assert_eq!(word_count("hold the key and talk"), 5);
        assert_eq!(word_count("  two\nlines  here "), 3);
    }

    #[test]
    fn stats_empty_is_zeros() {
        let s = stats(&[], 1_000_000);
        assert_eq!(s, HistoryStats::default());
    }

    #[test]
    fn stats_counts_only_the_rolling_week() {
        let now = 1_700_000_000_000;
        let week_ago = now - WEEK_MS;
        let entries = vec![
            entry("one two", week_ago - 1),
            entry("three four five", week_ago),
            entry("six", now),
        ];
        let s = stats(&entries, now);
        assert_eq!(s.dictations_total, 3);
        assert_eq!(s.words_total, 6);
        // timestamp == cutoff is in-window; older than cutoff is not.
        assert_eq!(s.dictations_7d, 2);
        assert_eq!(s.words_7d, 4);
    }
}
