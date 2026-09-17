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

/// One calendar day, for the streak's day buckets.
pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HistoryStats {
    pub dictations_7d: usize,
    pub words_7d: usize,
    pub dictations_total: usize,
    pub words_total: usize,
    /// Consecutive local days with a dictation, ending today or yesterday.
    pub streak_days: u32,
}

/// Whitespace-separated tokens. Empty / whitespace-only text is zero words.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Counts over `entries`. `now_ms` is Unix epoch milliseconds; `utc_offset_ms`
/// is the user's local offset from UTC (positive east), which only the
/// streak's day boundaries care about.
pub fn stats(entries: &[HistoryEntry], now_ms: i64, utc_offset_ms: i64) -> HistoryStats {
    let cutoff = now_ms.saturating_sub(WEEK_MS);
    let mut out = HistoryStats {
        dictations_total: entries.len(),
        streak_days: streak_days(entries, now_ms, utc_offset_ms),
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

/// Consecutive local calendar days with at least one dictation, counted
/// back from today. A streak that last ran yesterday still counts — the
/// number must not read 0 before the day's first dictation — while a gap of
/// a full day ends it. Derived from the timestamps already in history.json,
/// so it works for entries recorded before this existed. Entries dated after
/// `now_ms` are ignored.
pub fn streak_days(entries: &[HistoryEntry], now_ms: i64, utc_offset_ms: i64) -> u32 {
    let local_day = |ms: i64| (ms + utc_offset_ms).div_euclid(DAY_MS);
    let today = local_day(now_ms);
    let mut days: Vec<i64> = entries
        .iter()
        .map(|e| local_day(e.timestamp))
        .filter(|d| *d <= today)
        .collect();
    days.sort_unstable();
    days.dedup();
    let Some(&latest) = days.last() else {
        return 0;
    };
    if latest < today - 1 {
        return 0;
    }
    let mut streak = 1;
    let mut cursor = latest;
    for &day in days.iter().rev().skip(1) {
        if day != cursor - 1 {
            break;
        }
        streak += 1;
        cursor = day;
    }
    streak
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
        let s = stats(&[], 1_000_000, 0);
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
        let s = stats(&entries, now, 0);
        assert_eq!(s.dictations_total, 3);
        assert_eq!(s.words_total, 6);
        // timestamp == cutoff is in-window; older than cutoff is not.
        assert_eq!(s.dictations_7d, 2);
        assert_eq!(s.words_7d, 4);
    }

    // ---- streak ----
    // `NOON` is 12:00 UTC on some day; with offset 0 the local day is the
    // UTC day, so ±DAY_MS moves exactly one calendar day.
    const NOON: i64 = 1_700_000_000_000 - (1_700_000_000_000 % DAY_MS) + DAY_MS / 2;

    #[test]
    fn streak_empty_is_zero() {
        assert_eq!(streak_days(&[], NOON, 0), 0);
    }

    #[test]
    fn streak_single_day_today_is_one() {
        let entries = vec![entry("a", NOON - 3_600_000), entry("b", NOON)];
        assert_eq!(streak_days(&entries, NOON, 0), 1);
    }

    #[test]
    fn streak_counts_consecutive_days() {
        let entries = vec![
            entry("three days ago", NOON - 3 * DAY_MS),
            entry("two days ago", NOON - 2 * DAY_MS),
            entry("yesterday", NOON - DAY_MS),
            entry("today", NOON),
        ];
        assert_eq!(streak_days(&entries, NOON, 0), 4);
    }

    #[test]
    fn streak_breaks_on_a_gap() {
        // Today and three days ago, nothing in between: the run is today only.
        let entries = vec![entry("old", NOON - 3 * DAY_MS), entry("today", NOON)];
        assert_eq!(streak_days(&entries, NOON, 0), 1);
        // A gap further back cuts the run at the gap, not at zero.
        let entries = vec![
            entry("five days ago", NOON - 5 * DAY_MS),
            entry("two days ago", NOON - 2 * DAY_MS),
            entry("yesterday", NOON - DAY_MS),
            entry("today", NOON),
        ];
        assert_eq!(streak_days(&entries, NOON, 0), 3);
    }

    #[test]
    fn streak_ending_yesterday_still_counts() {
        // Nothing yet today: the streak must not read 0 before the first
        // dictation of the day.
        let entries = vec![entry("two days ago", NOON - 2 * DAY_MS), entry("yesterday", NOON - DAY_MS)];
        assert_eq!(streak_days(&entries, NOON, 0), 2);
        // Last dictation the day before yesterday: the streak is over.
        let entries = vec![entry("two days ago", NOON - 2 * DAY_MS)];
        assert_eq!(streak_days(&entries, NOON, 0), 0);
    }

    #[test]
    fn streak_uses_local_midnight_not_utc() {
        // Two entries 90 minutes apart straddling local midnight in UTC+1:
        // 23:30 local yesterday (22:30 UTC) and 00:30 local today (23:30 UTC
        // yesterday). In UTC they share a day; locally they are two.
        let midnight_utc = NOON - DAY_MS / 2; // 00:00 UTC today
        let one_hour = 3_600_000;
        let entries = vec![
            entry("23:30 local yesterday", midnight_utc - 90 * 60_000),
            entry("00:30 local today", midnight_utc - 30 * 60_000),
        ];
        let now = midnight_utc + one_hour; // 02:00 local today
        assert_eq!(streak_days(&entries, now, one_hour), 2);
        assert_eq!(streak_days(&entries, now, 0), 1);
    }

    #[test]
    fn streak_ignores_entries_from_the_future() {
        let entries = vec![entry("clock skew", NOON + 3 * DAY_MS), entry("today", NOON)];
        assert_eq!(streak_days(&entries, NOON, 0), 1);
    }

    #[test]
    fn stats_carries_the_streak() {
        let entries = vec![entry("yesterday", NOON - DAY_MS), entry("today", NOON)];
        assert_eq!(stats(&entries, NOON, 0).streak_days, 2);
    }
}
