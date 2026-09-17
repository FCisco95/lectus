//! Organic token gate.
//!
//! Lectus is free software for people who hold ORGANIC. There is no account,
//! no subscription and no card: you link a Solana wallet once by signing a
//! nonce (no transfer, no spend), and the app re-reads that wallet's ORGANIC
//! balance in the background.
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

/// ORGANIC (ORG) — SPL Token, 6 decimals, immutable.
pub const ORGANIC_MINT: &str = "DuXugm4oTXrGDopgxgudyhboaf6uUg1GVbJ6jk6qbonk";

/// Dollar value of ORGANIC a wallet must hold to unlock dictation.
pub const FLOOR_USD: f64 = 20.0;

/// How long a wallet that has dropped below the floor (or that we simply
/// cannot reach the chain for) keeps working before dictation locks.
pub const GRACE_DAYS: i64 = 7;

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
    /// ORGANIC held at the last successful check (UI amount, not lamports).
    pub last_balance: f64,
    /// Dollar value of that balance.
    pub last_usd: f64,
    /// ORGANIC/USD price used for it.
    pub last_price: f64,
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
    },
    /// Below the floor (or unverified) but still inside the grace window.
    Grace {
        pubkey: String,
        usd: f64,
        days_left: i64,
    },
    /// Grace spent. Dictation is refused until the wallet is topped back up.
    Locked { pubkey: String, usd: f64 },
}

impl Status {
    /// Whether dictation may run.
    pub fn allows_dictation(&self) -> bool {
        matches!(self, Status::Active { .. } | Status::Grace { .. })
    }
}

/// Derive the gate's verdict from stored facts. Pure — no clock, no network.
pub fn status_of(license: Option<&License>, now_ms: i64) -> Status {
    let Some(l) = license.filter(|l| !l.pubkey.is_empty()) else {
        return Status::Unlinked;
    };

    // Held at or above the floor as of the last successful read.
    if l.last_usd >= FLOOR_USD {
        return Status::Active {
            pubkey: l.pubkey.clone(),
            balance: l.last_balance,
            usd: l.last_usd,
        };
    }

    // Never seen above the floor: no grace to give.
    if l.last_ok_ms <= 0 {
        return Status::Locked {
            pubkey: l.pubkey.clone(),
            usd: l.last_usd,
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
        }
    } else {
        Status::Locked {
            pubkey: l.pubkey.clone(),
            usd: l.last_usd,
        }
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
pub fn apply_check(license: &mut License, balance: f64, price_usd: f64, now_ms: i64) {
    let usd = balance * price_usd;
    license.last_checked_ms = now_ms;
    license.last_balance = balance;
    license.last_price = price_usd;
    license.last_usd = usd;
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
    /// Nonce handed to the browser for the in-flight link attempt.
    pub pending_nonce: Mutex<Option<String>>,
}

impl LicenseState {
    pub fn new(license: Option<License>) -> Self {
        Self {
            license: Mutex::new(license),
            pending_nonce: Mutex::new(None),
        }
    }

    pub fn status(&self, now_ms: i64) -> Status {
        status_of(self.license.lock().unwrap().as_ref(), now_ms)
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
        }
    }

    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn no_license_is_unlinked() {
        assert_eq!(status_of(None, NOW), Status::Unlinked);
    }

    #[test]
    fn empty_pubkey_is_unlinked() {
        let l = License::default();
        assert_eq!(status_of(Some(&l), NOW), Status::Unlinked);
    }

    #[test]
    fn at_or_above_floor_is_active() {
        let l = linked(FLOOR_USD, NOW);
        assert!(matches!(status_of(Some(&l), NOW), Status::Active { .. }));
    }

    #[test]
    fn below_floor_inside_window_is_grace() {
        let l = linked(3.0, NOW - 2 * MS_PER_DAY);
        match status_of(Some(&l), NOW) {
            Status::Grace { days_left, .. } => assert_eq!(days_left, GRACE_DAYS - 2),
            other => panic!("expected grace, got {other:?}"),
        }
    }

    #[test]
    fn below_floor_past_window_is_locked() {
        let l = linked(3.0, NOW - (GRACE_DAYS + 1) * MS_PER_DAY);
        assert!(matches!(status_of(Some(&l), NOW), Status::Locked { .. }));
    }

    #[test]
    fn never_funded_gets_no_grace() {
        let l = linked(0.0, 0);
        assert!(matches!(status_of(Some(&l), NOW), Status::Locked { .. }));
    }

    #[test]
    fn grace_still_dictates_locked_does_not() {
        assert!(status_of(Some(&linked(3.0, NOW)), NOW).allows_dictation());
        assert!(!status_of(Some(&linked(3.0, 0)), NOW).allows_dictation());
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
        apply_check(&mut l, 1_000.0, 0.0025, NOW); // $2.50
        assert_eq!(l.last_ok_ms, 0);
        assert_eq!(l.last_checked_ms, NOW);

        apply_check(&mut l, 100_000.0, 0.0025, NOW + 1); // $250
        assert_eq!(l.last_ok_ms, NOW + 1);
        assert!((l.last_usd - 250.0).abs() < 1e-9);
    }

    #[test]
    fn a_failed_check_never_shortens_grace() {
        // Grace runs from last_ok_ms, which a failed read leaves untouched.
        let mut l = linked(50.0, NOW);
        let before = l.last_ok_ms;
        apply_check(&mut l, 0.0, 0.0025, NOW + MS_PER_DAY);
        assert_eq!(l.last_ok_ms, before);
        assert!(matches!(
            status_of(Some(&l), NOW + MS_PER_DAY),
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
    fn missing_or_corrupt_file_reads_as_unlinked() {
        let dir = tempfile::tempdir().unwrap();
        let path = license_file(dir.path());
        assert!(load_from(&path).is_none());
        std::fs::write(&path, "{ truncated").unwrap();
        assert!(load_from(&path).is_none());
    }
}
