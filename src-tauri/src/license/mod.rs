//! Organic token gate.
//!
//! Lectus is free software for people who hold ORGANIC or Mycel. There is no
//! account, no subscription and no card: you link a Solana wallet once by
//! signing a nonce (no transfer, no spend), and the app re-reads that wallet's
//! ORGANIC and Mycel balances in the background. Either token at the dollar
//! floor is enough.
//!
//! This is a membership check, not DRM. The binary is public and patchable;
//! the gate exists so "Organic holders get Lectus" is true, not to fight a
//! determined attacker.
//!
//! The floor is denominated in **dollars**, not tokens, so the token count is
//! derived at every check from the live price. That keeps the ask stable for
//! holders while ORGANIC moves.

pub mod chain;
pub mod commands;
pub mod connect;
pub mod verify;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A mint that can satisfy the membership floor on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateToken {
    pub mint: &'static str,
    pub ticker: &'static str,
    pub name: &'static str,
}

/// ORGANIC (ORG) — SPL Token, 6 decimals, immutable.
pub const ORGANIC: GateToken = GateToken {
    mint: "DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk",
    ticker: "ORG",
    name: "ORGANIC",
};

/// Mycel (MYCEL) — SPL Token, 6 decimals. Either this or ORGANIC clears the gate.
pub const MYCEL: GateToken = GateToken {
    mint: "HudkzEWpcUnTYFZMMcbNdwk1S5Am26J2SyEh4NfFworg",
    ticker: "MYCEL",
    name: "Mycel",
};

pub const GATE_TOKENS: &[GateToken] = &[ORGANIC, MYCEL];

/// Dollar value of ORGANIC *or* Mycel a wallet must hold to unlock dictation.
pub const FLOOR_USD: f64 = 20.0;

/// How long a wallet that has dropped below the floor (or that we simply
/// cannot reach the chain for) keeps working before dictation locks.
pub const GRACE_DAYS: i64 = 7;

/// Free days every installation gets from the end of first-run setup, with or
/// without a wallet. Separate from [`GRACE_DAYS`], which only protects a
/// wallet that was once above the floor.
pub const TRIAL_DAYS: i64 = 7;

/// Background re-check cadence. Deliberately slow: the wallet is linked once
/// and must never pop up again during normal use.
pub const RECHECK_INTERVAL_MS: i64 = 12 * 60 * 60 * 1000;

const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// The linked wallet plus the last thing the chain told us about it.
///
/// Stored in its own `license.json` rather than in `Config`: the Settings
/// webview round-trips the whole config through `apply_ui_update`, and a stale
/// snapshot from a hidden window must never be able to clobber the link.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct License {
    /// Base58 Solana public key that signed the link nonce.
    pub pubkey: String,
    pub linked_at_ms: i64,
    /// Last moment the wallet was observed at or above the floor. Grace is
    /// measured from here, so an unreachable RPC never locks anyone out on
    /// its own.
    pub last_ok_ms: i64,
    /// Last moment a check completed, successfully or not.
    pub last_checked_ms: i64,
    /// Tokens held at the last successful check (UI amount, not lamports).
    pub last_balance: f64,
    /// Dollar value of that balance.
    pub last_usd: f64,
    /// Token/USD price used for it.
    pub last_price: f64,
    /// Ticker of the mint that produced `last_usd` (`ORG` or `MYCEL`). Empty
    /// in license.json written before Mycel was added; treated as ORG.
    pub last_ticker: String,
}

/// What the gate currently permits. `Active` and `Grace` both dictate.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Status {
    /// No wallet linked yet.
    Unlinked,
    Active {
        pubkey: String,
        balance: f64,
        usd: f64,
        ticker: String,
    },
    /// Below the floor (or unverified) but still inside the grace window.
    Grace {
        pubkey: String,
        usd: f64,
        days_left: i64,
        ticker: String,
    },
    /// Grace spent. Dictation is refused until the wallet is topped back up.
    Locked {
        pubkey: String,
        usd: f64,
        ticker: String,
    },
    /// Inside the free trial and not otherwise allowed. `pubkey` is set when a
    /// wallet is linked but has not (yet) cleared the floor.
    Trial {
        days_left: i64,
        pubkey: Option<String>,
    },
}

impl Status {
    /// Whether dictation may run.
    pub fn allows_dictation(&self) -> bool {
        matches!(self, Status::Active { .. } | Status::Grace { .. } | Status::Trial { .. })
    }
}

/// Derive the gate's verdict from stored facts. Pure — no clock, no network.
///
/// The wallet's own verdict wins when it allows dictation. Otherwise a trial
/// that started less than [`TRIAL_DAYS`] ago allows it. A clock earlier than
/// the trial start counts as day 0.
pub fn status_of(license: Option<&License>, trial_started_ms: Option<i64>, now_ms: i64) -> Status {
    let wallet = wallet_status(license, now_ms);
    if wallet.allows_dictation() {
        return wallet;
    }
    if let Some(start) = trial_started_ms {
        let elapsed = now_ms.saturating_sub(start).max(0);
        let trial_ms = TRIAL_DAYS * MS_PER_DAY;
        if elapsed < trial_ms {
            let days_left = (trial_ms - elapsed + MS_PER_DAY - 1) / MS_PER_DAY;
            let pubkey = license
                .map(|l| l.pubkey.clone())
                .filter(|k| !k.is_empty());
            return Status::Trial { days_left, pubkey };
        }
    }
    wallet
}

fn wallet_status(license: Option<&License>, now_ms: i64) -> Status {
    let Some(l) = license.filter(|l| !l.pubkey.is_empty()) else {
        return Status::Unlinked;
    };

    let ticker = display_ticker(&l.last_ticker);

    // Held at or above the floor as of the last successful read.
    if l.last_usd >= FLOOR_USD {
        return Status::Active {
            pubkey: l.pubkey.clone(),
            balance: l.last_balance,
            usd: l.last_usd,
            ticker,
        };
    }

    // Never seen above the floor: no grace to give.
    if l.last_ok_ms <= 0 {
        return Status::Locked {
            pubkey: l.pubkey.clone(),
            usd: l.last_usd,
            ticker,
        };
    }

    let elapsed = now_ms.saturating_sub(l.last_ok_ms);
    let grace_ms = GRACE_DAYS * MS_PER_DAY;
    if elapsed < grace_ms {
        // Round up so the last partial day still reads as "1 day left".
        let days_left = (grace_ms - elapsed + MS_PER_DAY - 1) / MS_PER_DAY;
        Status::Grace {
            pubkey: l.pubkey.clone(),
            usd: l.last_usd,
            days_left,
            ticker,
        }
    } else {
        Status::Locked {
            pubkey: l.pubkey.clone(),
            usd: l.last_usd,
            ticker,
        }
    }
}

fn display_ticker(stored: &str) -> String {
    if stored.is_empty() {
        ORGANIC.ticker.to_string()
    } else {
        stored.to_string()
    }
}

/// ORGANIC needed to clear the floor at a given price. `None` when the price
/// feed is unusable — callers must not treat that as "zero tokens needed".
pub fn tokens_required(price_usd: f64) -> Option<f64> {
    if price_usd.is_finite() && price_usd > 0.0 {
        Some(FLOOR_USD / price_usd)
    } else {
        None
    }
}

/// Whether enough time has passed to re-read the chain.
pub fn is_check_due(license: &License, now_ms: i64) -> bool {
    now_ms.saturating_sub(license.last_checked_ms) >= RECHECK_INTERVAL_MS
}

/// Fold a completed balance read into the stored record.
pub fn apply_check(license: &mut License, balance: f64, price_usd: f64, ticker: &str, now_ms: i64) {
    let usd = balance * price_usd;
    license.last_checked_ms = now_ms;
    license.last_balance = balance;
    license.last_price = price_usd;
    license.last_usd = usd;
    license.last_ticker = ticker.to_string();
    if usd >= FLOOR_USD {
        license.last_ok_ms = now_ms;
    }
}

pub fn license_file(dir: &Path) -> PathBuf {
    dir.join("license.json")
}

pub fn load_from(path: &Path) -> Option<License> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// `trial.json` lives beside `license.json` and survives link and unlink.
pub fn trial_file(dir: &Path) -> PathBuf {
    dir.join("trial.json")
}

#[derive(Serialize, Deserialize)]
struct TrialRecord {
    started_ms: i64,
}

/// The stored trial start. A missing, corrupt or non-positive file is absent.
pub fn load_trial(path: &Path) -> Option<i64> {
    let text = std::fs::read_to_string(path).ok()?;
    let record: TrialRecord = serde_json::from_str(&text).ok()?;
    (record.started_ms > 0).then_some(record.started_ms)
}

/// Record the trial start once. Returns the existing start when one is valid
/// and never overwrites it.
pub fn start_trial_if_absent(path: &Path, now_ms: i64) -> anyhow::Result<i64> {
    if let Some(existing) = load_trial(path) {
        return Ok(existing);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&TrialRecord { started_ms: now_ms })?)?;
    std::fs::rename(&tmp, path)?;
    Ok(now_ms)
}

pub fn save_to(path: &Path, license: &License) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Same atomic-rename discipline as Config: a truncated license.json would
    // read back as "unlinked" and lock a paying holder out of their own app.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(license)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// In-memory holder of the link, shared with Tauri commands.
pub struct LicenseState {
    pub license: Mutex<Option<License>>,
    /// When the free trial began (ms since epoch), if it has.
    pub trial_started_ms: Mutex<Option<i64>>,
    /// Nonce handed to the browser for the in-flight link attempt.
    pub pending_nonce: Mutex<Option<String>>,
}

impl LicenseState {
    pub fn new(license: Option<License>, trial_started_ms: Option<i64>) -> Self {
        Self {
            license: Mutex::new(license),
            trial_started_ms: Mutex::new(trial_started_ms),
            pending_nonce: Mutex::new(None),
        }
    }

    pub fn status(&self, now_ms: i64) -> Status {
        let trial = *self.trial_started_ms.lock().unwrap();
        status_of(self.license.lock().unwrap().as_ref(), trial, now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked(usd: f64, last_ok_ms: i64) -> License {
        License {
            pubkey: "Cisco1111111111111111111111111111111111111".into(),
            linked_at_ms: 0,
            last_ok_ms,
            last_checked_ms: 0,
            last_balance: 10_000.0,
            last_usd: usd,
            last_price: 0.0025,
            last_ticker: "ORG".into(),
        }
    }

    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn no_license_is_unlinked() {
        assert_eq!(status_of(None, None, NOW), Status::Unlinked);
    }

    #[test]
    fn empty_pubkey_is_unlinked() {
        let l = License::default();
        assert_eq!(status_of(Some(&l), None, NOW), Status::Unlinked);
    }

    #[test]
    fn at_or_above_floor_is_active() {
        let l = linked(FLOOR_USD, NOW);
        assert!(matches!(status_of(Some(&l), None, NOW), Status::Active { .. }));
    }

    #[test]
    fn below_floor_inside_window_is_grace() {
        let l = linked(3.0, NOW - 2 * MS_PER_DAY);
        match status_of(Some(&l), None, NOW) {
            Status::Grace { days_left, .. } => assert_eq!(days_left, GRACE_DAYS - 2),
            other => panic!("expected grace, got {other:?}"),
        }
    }

    #[test]
    fn below_floor_past_window_is_locked() {
        let l = linked(3.0, NOW - (GRACE_DAYS + 1) * MS_PER_DAY);
        assert!(matches!(status_of(Some(&l), None, NOW), Status::Locked { .. }));
    }

    #[test]
    fn never_funded_gets_no_grace() {
        let l = linked(0.0, 0);
        assert!(matches!(status_of(Some(&l), None, NOW), Status::Locked { .. }));
    }

    #[test]
    fn grace_still_dictates_locked_does_not() {
        assert!(status_of(Some(&linked(3.0, NOW)), None, NOW).allows_dictation());
        assert!(!status_of(Some(&linked(3.0, 0)), None, NOW).allows_dictation());
        assert!(!Status::Unlinked.allows_dictation());
    }

    #[test]
    fn tokens_required_tracks_price() {
        // $20 floor at the 2026-09-17 price of $0.00252.
        let n = tokens_required(0.0025220802712995127).unwrap();
        assert!((n - 7929.9).abs() < 1.0, "got {n}");
    }

    #[test]
    fn tokens_required_rejects_unusable_price() {
        assert!(tokens_required(0.0).is_none());
        assert!(tokens_required(-1.0).is_none());
        assert!(tokens_required(f64::NAN).is_none());
    }

    #[test]
    fn apply_check_marks_ok_only_above_floor() {
        let mut l = linked(0.0, 0);
        apply_check(&mut l, 1_000.0, 0.0025, "ORG", NOW); // $2.50
        assert_eq!(l.last_ok_ms, 0);
        assert_eq!(l.last_checked_ms, NOW);
        assert_eq!(l.last_ticker, "ORG");

        apply_check(&mut l, 100_000.0, 0.0002, "MYCEL", NOW + 1); // $20
        assert_eq!(l.last_ok_ms, NOW + 1);
        assert_eq!(l.last_ticker, "MYCEL");
        assert!((l.last_usd - 20.0).abs() < 1e-9);
    }

    #[test]
    fn a_failed_check_never_shortens_grace() {
        // Grace runs from last_ok_ms, which a failed read leaves untouched.
        let mut l = linked(50.0, NOW);
        let before = l.last_ok_ms;
        apply_check(&mut l, 0.0, 0.0025, "ORG", NOW + MS_PER_DAY);
        assert_eq!(l.last_ok_ms, before);
        assert!(matches!(
            status_of(Some(&l), None, NOW + MS_PER_DAY),
            Status::Grace { .. }
        ));
    }

    #[test]
    fn recheck_is_due_only_after_the_interval() {
        let mut l = linked(50.0, NOW);
        l.last_checked_ms = NOW;
        assert!(!is_check_due(&l, NOW + RECHECK_INTERVAL_MS - 1));
        assert!(is_check_due(&l, NOW + RECHECK_INTERVAL_MS));
    }

    #[test]
    fn license_roundtrips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = license_file(dir.path());
        let l = linked(42.0, NOW);
        save_to(&path, &l).unwrap();
        assert_eq!(load_from(&path).unwrap(), l);
    }

    #[test]
    fn old_license_without_ticker_still_loads() {
        let json = r#"{
            "pubkey":"Cisco1111111111111111111111111111111111111",
            "linked_at_ms":1,
            "last_ok_ms":1,
            "last_checked_ms":1,
            "last_balance":1000.0,
            "last_usd":20.0,
            "last_price":0.02
        }"#;
        let l: License = serde_json::from_str(json).unwrap();
        assert_eq!(l.last_ticker, "");
        match status_of(Some(&l), None, NOW) {
            Status::Active { ticker, .. } => assert_eq!(ticker, "ORG"),
            other => panic!("expected active, got {other:?}"),
        }
    }

    #[test]
    fn gate_accepts_organic_or_mycel() {
        assert_eq!(ORGANIC.mint, "DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk");
        assert_eq!(GATE_TOKENS.len(), 2);
        assert_eq!(MYCEL.mint, "HudkzEWpcUnTYFZMMcbNdwk1S5Am26J2SyEh4NfFworg");
        assert_eq!(MYCEL.ticker, "MYCEL");
    }

    #[test]
    fn missing_or_corrupt_file_reads_as_unlinked() {
        let dir = tempfile::tempdir().unwrap();
        let path = license_file(dir.path());
        assert!(load_from(&path).is_none());
        std::fs::write(&path, "{ truncated").unwrap();
        assert!(load_from(&path).is_none());
    }

    #[test]
    fn unlinked_with_fresh_trial_is_trial_for_seven_days() {
        assert_eq!(
            status_of(None, Some(NOW), NOW),
            Status::Trial { days_left: 7, pubkey: None }
        );
        assert!(status_of(None, Some(NOW), NOW).allows_dictation());
    }

    #[test]
    fn trial_days_round_up_like_grace() {
        let started = NOW - 6 * MS_PER_DAY - MS_PER_DAY / 2; // 6.5 days ago
        assert_eq!(
            status_of(None, Some(started), NOW),
            Status::Trial { days_left: 1, pubkey: None }
        );
    }

    #[test]
    fn trial_ends_exactly_after_seven_days() {
        let started = NOW - TRIAL_DAYS * MS_PER_DAY;
        assert_eq!(status_of(None, Some(started), NOW), Status::Unlinked);
        assert!(!status_of(None, Some(started), NOW).allows_dictation());
    }

    #[test]
    fn linked_but_never_funded_wallet_keeps_the_trial() {
        let l = linked(0.0, 0);
        let status = status_of(Some(&l), Some(NOW - MS_PER_DAY), NOW);
        assert_eq!(
            status,
            Status::Trial { days_left: 6, pubkey: Some(l.pubkey.clone()) }
        );
        assert!(status.allows_dictation());
    }

    #[test]
    fn active_and_grace_wallets_ignore_the_trial() {
        let trial = Some(NOW - MS_PER_DAY);
        assert!(matches!(status_of(Some(&linked(FLOOR_USD, NOW)), trial, NOW), Status::Active { .. }));
        assert!(matches!(status_of(Some(&linked(3.0, NOW)), trial, NOW), Status::Grace { .. }));
    }

    #[test]
    fn clock_earlier_than_trial_start_counts_as_day_zero() {
        assert_eq!(
            status_of(None, Some(NOW + 3 * MS_PER_DAY), NOW),
            Status::Trial { days_left: 7, pubkey: None }
        );
    }

    #[test]
    fn expired_trial_falls_back_to_the_wallet_verdict() {
        let started = NOW - 8 * MS_PER_DAY;
        assert!(matches!(
            status_of(Some(&linked(0.0, 0)), Some(started), NOW),
            Status::Locked { .. }
        ));
    }

    #[test]
    fn trial_start_is_written_once_and_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = trial_file(dir.path());
        assert_eq!(load_trial(&path), None);
        assert_eq!(start_trial_if_absent(&path, NOW).unwrap(), NOW);
        assert_eq!(start_trial_if_absent(&path, NOW + 5 * MS_PER_DAY).unwrap(), NOW);
        assert_eq!(load_trial(&path), Some(NOW));
    }

    #[test]
    fn corrupt_or_non_positive_trial_file_counts_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = trial_file(dir.path());
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(load_trial(&path), None);
        std::fs::write(&path, r#"{"started_ms":-5}"#).unwrap();
        assert_eq!(load_trial(&path), None);
        std::fs::write(&path, r#"{"started_ms":0}"#).unwrap();
        assert_eq!(load_trial(&path), None);
        // A corrupt file is replaced by a fresh start rather than blocking it.
        assert_eq!(start_trial_if_absent(&path, NOW).unwrap(), NOW);
    }

    #[test]
    fn license_state_reports_the_trial() {
        let state = LicenseState::new(None, Some(NOW));
        assert_eq!(state.status(NOW), Status::Trial { days_left: 7, pubkey: None });
        *state.trial_started_ms.lock().unwrap() = None;
        assert_eq!(state.status(NOW), Status::Unlinked);
    }
}
