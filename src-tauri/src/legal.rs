//! Versioned, local terms acceptance. Draft documents cannot be accepted.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

const TERMS: &str = include_str!("../../legal/TERMS.md");
const PRIVACY: &str = include_str!("../../legal/PRIVACY.md");
const MANIFEST: &str = include_str!("../../legal/manifest.json");

#[derive(Deserialize)]
struct Publisher { kind: String, legal_name: String, country: String, support_email: String, privacy_email: String }
#[derive(Deserialize)]
struct Manifest { terms_version: String, privacy_version: String, publication_approved: bool, identity_disclosures_reviewed: bool, third_party_notices_reviewed: bool, publisher: Publisher }
#[derive(Serialize)]
pub struct LegalStatus {
    pub ready: bool,
    pub terms_version: String,
    pub privacy_version: String,
    pub terms_hash: String,
    pub privacy_hash: String,
    pub accepted: bool,
    pub terms: &'static str,
    pub privacy: &'static str,
}
#[derive(Serialize, Deserialize)]
struct Receipt { terms_version: String, terms_hash: String, privacy_version: String, accepted_at_ms: i64 }

fn hash(text: &str) -> String { format!("{:x}", Sha256::digest(text.as_bytes())) }
fn valid_contact(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else { return false };
    !local.is_empty() && !domain.contains('@') && domain.contains('.')
        && !domain.starts_with('.') && !domain.ends_with('.') && !domain.contains("..")
        && !value.chars().any(char::is_whitespace)
}
fn manifest_ready(m: &Manifest, terms: &str, privacy: &str) -> bool {
    m.publication_approved && m.identity_disclosures_reviewed && m.third_party_notices_reviewed
        && matches!(m.publisher.kind.as_str(), "individual" | "company")
        && !m.publisher.legal_name.trim().is_empty() && !m.publisher.country.trim().is_empty()
        && valid_contact(&m.publisher.support_email) && valid_contact(&m.publisher.privacy_email)
        && !m.terms_version.trim().is_empty() && !m.privacy_version.trim().is_empty()
        && !m.terms_version.to_ascii_lowercase().contains("draft") && !m.privacy_version.to_ascii_lowercase().contains("draft")
        && !terms.to_ascii_lowercase().contains("not approved for distribution") && !privacy.to_ascii_lowercase().contains("factual draft")
}
fn receipt_matches(receipt: Option<&Receipt>, version: &str, fingerprint: &str) -> bool {
    receipt.is_some_and(|r| r.terms_version == version && r.terms_hash == fingerprint && r.accepted_at_ms > 0)
}
fn status_at(path: &Path) -> LegalStatus {
    let manifest: Manifest = serde_json::from_str(MANIFEST).expect("bundled legal manifest must parse");
    let terms_hash = hash(TERMS);
    let receipt: Option<Receipt> = std::fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let ready = manifest_ready(&manifest, TERMS, PRIVACY);
    let accepted = ready && receipt_matches(receipt.as_ref(), &manifest.terms_version, &terms_hash);
    LegalStatus { ready, accepted, terms_hash, privacy_hash: hash(PRIVACY), terms_version: manifest.terms_version,
        privacy_version: manifest.privacy_version, terms: TERMS, privacy: PRIVACY }
}
fn receipt_path() -> std::path::PathBuf { crate::config::Config::app_support_dir().join("legal-acceptance.json") }
fn save_receipt(path: &Path, receipt: &Receipt) -> Result<(), String> {
    use std::io::Write;
    let save = || -> anyhow::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let temp = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(receipt)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(temp, path)?;
        Ok(())
    };
    save().map_err(|_| "Could not save acceptance. The previous receipt is unchanged; try again.".into())
}
pub fn requires_acceptance() -> bool { let s = status_at(&receipt_path()); s.ready && !s.accepted }
pub fn ensure_accepted() -> Result<(), String> {
    let s = status_at(&receipt_path());
    if !s.ready { return Err("Legal documents are still being prepared. This build is not ready for new-user distribution.".into()); }
    if !s.accepted { return Err("Review and accept the current Terms before completing setup.".into()); }
    Ok(())
}
#[tauri::command]
pub fn get_legal_status() -> LegalStatus { status_at(&receipt_path()) }
#[tauri::command]
pub fn open_legal_notices(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let folder = app.path().resource_dir().map_err(|_| "Could not find the app resources.")?.join("legal/third-party");
    if !folder.is_dir() { return Err("Bundled third-party notices were not found in this development build.".into()); }
    #[cfg(windows)]
    let result = {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe").arg(&folder).creation_flags(0x08000000).spawn()
    };
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&folder).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(&folder).spawn();
    result.map(|_| ()).map_err(|_| "Could not open the third-party notices folder.".into())
}
#[tauri::command]
pub fn accept_legal_terms(terms_version: String, terms_hash: String, app: tauri::AppHandle) -> Result<LegalStatus, String> {
    static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _write = WRITE.lock().map_err(|_| "Could not lock legal receipt.")?;
    let path = receipt_path();
    let current = status_at(&path);
    if !current.ready { return Err("Draft terms cannot be accepted.".into()); }
    if terms_version != current.terms_version || terms_hash != current.terms_hash { return Err("The Terms changed. Read the current version and try again.".into()); }
    let receipt = Receipt { terms_version, terms_hash, privacy_version: current.privacy_version, accepted_at_ms: crate::now_millis() };
    save_receipt(&path, &receipt)?;
    let status = status_at(&path);
    use tauri::Manager;
    let cfg = app.state::<crate::AppState>().config.lock().unwrap().clone();
    if status.accepted && cfg.onboarding_completed {
        std::thread::spawn(move || crate::restart_preroll(&app, &cfg.input_device));
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
        if let (Some(target), Some(values)) = (base.as_object_mut(), patch.as_object()) {
            for (key, value) in values { merge(target.entry(key.clone()).or_insert(serde_json::Value::Null), value); }
        } else { *base = patch.clone(); }
    }
    #[test] fn native_and_packaging_use_the_same_readiness_cases() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/legal-readiness.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let mut manifest = fixture["manifest"].clone();
            merge(&mut manifest, &case["patch"]);
            let terms = case["terms"].as_str().unwrap_or("Final test terms");
            let privacy = case["privacy"].as_str().unwrap_or("Final test privacy");
            let ready = serde_json::from_value::<Manifest>(manifest).map(|m| manifest_ready(&m, terms, privacy)).unwrap_or(false);
            assert_eq!(ready, case["ready"].as_bool().unwrap(), "{}", case["name"]);
        }
    }
    #[test] fn failed_acceptance_write_preserves_previous_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("acceptance.json");
        let receipt = Receipt { terms_version: "v1".into(), terms_hash: hash("test terms"), privacy_version: "p1".into(), accepted_at_ms: 1 };
        save_receipt(&path, &receipt).unwrap();
        let before = std::fs::read(&path).unwrap();
        std::fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(save_receipt(&path, &Receipt { accepted_at_ms: 2, ..receipt }).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    #[test] fn reading_legal_status_never_records_acceptance() {
        let dir = tempfile::tempdir().unwrap();
        let status = status_at(&dir.path().join("receipt.json"));
        assert!(!status.accepted);
        assert_eq!(status.terms_hash.len(), 64);
        assert!(!dir.path().join("receipt.json").exists());
    }
    #[test] fn document_hash_changes_with_content() { assert_ne!(hash("version one"), hash("version two")); }
    #[test] fn acceptance_requires_current_version_content_and_time() {
        let receipt = Receipt { terms_version: "v1".into(), terms_hash: hash("first terms"), privacy_version: "p1".into(), accepted_at_ms: 1234 };
        assert!(receipt_matches(Some(&receipt), "v1", &hash("first terms")));
        assert!(!receipt_matches(Some(&receipt), "v2", &hash("first terms")));
        assert!(!receipt_matches(Some(&receipt), "v1", &hash("changed terms")));
        assert!(!receipt_matches(None, "v1", &hash("first terms")));
        assert!(!receipt_matches(Some(&Receipt { accepted_at_ms: 0, ..receipt }), "v1", &hash("first terms")));
    }
}
