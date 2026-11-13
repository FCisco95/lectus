//! Explicit optional-model agreement, separate from Lectus Terms and cloud choices.
//! Receipts stay on this device and are not proof of an independent legal review.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::Mutex};
use tauri::Emitter;

const ID: &str = "gemma-3";
const VERSION: &str = "terms-2026-04-01_policy-2024-02-21";
const TERMS: &str = include_str!("../../legal/model-agreements/gemma-3-terms.txt");
const POLICY: &str = include_str!("../../legal/model-agreements/gemma-3-policy.txt");
static WRITE: Mutex<()> = Mutex::new(());
static CHANGE: Mutex<()> = Mutex::new(());

#[derive(Serialize, Clone)]
pub struct ModelLicenseStatus {
    pub model_id: &'static str,
    pub title: &'static str,
    pub version: &'static str,
    pub fingerprint: String,
    pub accepted: bool,
    pub terms: &'static str,
    pub policy: &'static str,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    model_id: String,
    version: String,
    fingerprint: String,
    accepted_at_ms: i64,
}

fn fingerprint() -> String {
    let mut hash = Sha256::new();
    for value in [ID, VERSION, TERMS, POLICY] {
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    format!("{:x}", hash.finalize())
}
fn path() -> PathBuf { crate::config::Config::app_support_dir().join("model-license-gemma-3.json") }
fn read_receipt(path: &Path) -> Option<Receipt> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).ok()?.take(8193).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 8192 { return None; }
    serde_json::from_slice(&bytes).ok()
}
fn status_at(path: &Path) -> ModelLicenseStatus {
    let fingerprint = fingerprint();
    let accepted = read_receipt(path).is_some_and(|r| {
        r.model_id == ID && r.version == VERSION && r.fingerprint == fingerprint && r.accepted_at_ms > 0
    });
    ModelLicenseStatus { model_id: ID, title: "Gemma 3 model agreement", version: VERSION,
        fingerprint, accepted, terms: TERMS, policy: POLICY }
}
pub fn ensure_accepted() -> anyhow::Result<()> {
    if !status_at(&path()).accepted {
        anyhow::bail!("Review and accept the Gemma model agreement in AI cleanup before downloading or using this optional model.");
    }
    Ok(())
}
fn accept_at(path: &Path, requested_fingerprint: &str, now_ms: i64) -> Result<ModelLicenseStatus, String> {
    let _guard = WRITE.lock().map_err(|_| "Could not save the model agreement.")?;
    if requested_fingerprint != fingerprint() || now_ms <= 0 {
        return Err("The model agreement changed. Read the current documents and try again.".into());
    }
    let receipt = Receipt { model_id: ID.into(), version: VERSION.into(),
        fingerprint: requested_fingerprint.into(), accepted_at_ms: now_ms };
    let save = || -> anyhow::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let temporary = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    };
    save().map_err(|_| "Could not save the model agreement. The previous acceptance state is unchanged.".to_string())?;
    Ok(status_at(path))
}
fn revoke_at(path: &Path) -> Result<ModelLicenseStatus, String> {
    let _guard = WRITE.lock().map_err(|_| "Could not change the model agreement.")?;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Could not revoke the model agreement. Try again.".into()),
    }
    Ok(status_at(path))
}
#[tauri::command]
pub fn get_cleanup_model_license() -> ModelLicenseStatus { status_at(&path()) }
fn change_and_notify(
    change: impl FnOnce() -> Result<ModelLicenseStatus, String>,
    notify: impl FnOnce(&ModelLicenseStatus),
) -> Result<ModelLicenseStatus, String> {
    // Keep notification order identical to receipt mutation order across windows.
    let _guard = CHANGE.lock().map_err(|_| "Could not change the model agreement.")?;
    let status = change()?;
    notify(&status);
    Ok(status)
}
#[tauri::command]
pub fn accept_cleanup_model_license(fingerprint: String, app: tauri::AppHandle) -> Result<ModelLicenseStatus, String> {
    change_and_notify(
        || accept_at(&path(), &fingerprint, crate::now_millis()),
        |status| { let _ = app.emit("cleanup-license-changed", status); },
    )
}
#[tauri::command]
pub fn revoke_cleanup_model_license(app: tauri::AppHandle) -> Result<ModelLicenseStatus, String> {
    change_and_notify(
        || revoke_at(&path()),
        |status| { let _ = app.emit("cleanup-license-changed", status); },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_changes_cannot_overtake_a_pending_notification() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipt.json");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let events = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            let accept_path = &path;
            let event_log = &events;
            let accepted = scope.spawn(move || change_and_notify(
                || accept_at(accept_path, &fingerprint(), 1234),
                |status| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    event_log.lock().unwrap().push(status.accepted);
                },
            ));
            entered_rx.recv().unwrap();
            assert!(CHANGE.try_lock().is_err(), "notification must hold the mutation guard");
            let revoked = scope.spawn(|| change_and_notify(
                || revoke_at(&path),
                |status| events.lock().unwrap().push(status.accepted),
            ));
            release_tx.send(()).unwrap();
            accepted.join().unwrap().unwrap();
            revoked.join().unwrap().unwrap();
        });
        assert_eq!(*events.lock().unwrap(), vec![true, false]);
        assert!(!status_at(&path).accepted);
    }
    #[test]
    fn no_implicit_acceptance_and_explicit_receipt_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipt.json");
        assert!(!status_at(&path).accepted);
        assert!(!path.exists(), "reading status must not create a receipt");
        assert!(accept_at(&path, &fingerprint(), 1234).unwrap().accepted);
        assert!(status_at(&path).accepted);
        assert!(!revoke_at(&path).unwrap().accepted);
        assert!(!revoke_at(&path).unwrap().accepted);
    }
    #[test]
    fn changed_version_content_and_corrupt_receipts_require_review() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipt.json");
        assert!(accept_at(&path, "old-document", 1234).is_err());
        assert!(!path.exists());
        for field in ["model_id", "version", "fingerprint", "accepted_at_ms"] {
            accept_at(&path, &fingerprint(), 1234).unwrap();
            let mut receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            receipt[field] = if field == "accepted_at_ms" { serde_json::json!(0) } else { serde_json::json!("old") };
            std::fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
            assert!(!status_at(&path).accepted, "{field}");
        }
        std::fs::write(&path, b"invalid json").unwrap();
        assert!(!status_at(&path).accepted);
    }
    #[test]
    fn persistence_failure_does_not_claim_acceptance_or_destroy_previous_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipt.json");
        accept_at(&path, &fingerprint(), 1234).unwrap();
        let before = std::fs::read(&path).unwrap();
        std::fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(accept_at(&path, &fingerprint(), 5678).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let directory = dir.path().join("not-a-file");
        std::fs::create_dir(&directory).unwrap();
        assert!(revoke_at(&directory).is_err());
        assert!(directory.is_dir());
    }
}
