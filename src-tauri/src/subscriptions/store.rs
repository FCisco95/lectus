//! Native credential store with small chunks for Windows Credential Manager limits.
//! The manifest is written last, so token rotation is atomic across all fields.
use super::*;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    thread,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_at: u64,
    pub scope: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Registration {
    pub client_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub tokens: Option<Tokens>,
}
#[derive(Default, Serialize, Deserialize)]
pub(super) struct Vault {
    pub host_id: String,
    pub registration: Registration,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    id: String,
    chunks: usize,
}
pub(super) fn random() -> Result<String> {
    use base64::Engine;
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|_| anyhow!("Secure random generation is unavailable."))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}
pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(super) fn lock(lease: Lease) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root()?.join("chatgpt.lock"))
        .map_err(|_| anyhow!("Cannot lock subscription credentials."))?;
    let start = Instant::now();
    loop {
        lease.check()?;
        if file.try_lock_exclusive().is_ok() {
            return Ok(file);
        }
        if start.elapsed() > Duration::from_secs(35) {
            bail!("Another Lectus process is using this subscription. Try again.");
        }
        thread::sleep(Duration::from_millis(50));
    }
}
fn entry(name: &str) -> Result<keyring::Entry> {
    if !cfg!(any(target_os = "windows", target_os = "macos")) {
        bail!("Native subscription credential storage is supported on Windows and macOS.");
    }
    keyring::Entry::new("com.organic.lectus.subscriptions", name)
        .map_err(|_| anyhow!("Native credential storage is unavailable."))
}
trait Credentials {
    fn get(&self, name: &str) -> Result<Option<String>>;
    fn set(&self, name: &str, value: &str) -> Result<()>;
    fn delete(&self, name: &str);
}
struct Native;
impl Credentials for Native {
    fn get(&self, name: &str) -> Result<Option<String>> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => {
                bail!("Cannot read native subscription credentials. Unlock your system key store.")
            }
        }
    }
    fn set(&self, name: &str, value: &str) -> Result<()> {
        entry(name)?.set_password(value).map_err(|_| {
            anyhow!("Cannot save native subscription credentials. Unlock your system key store.")
        })
    }
    fn delete(&self, name: &str) {
        if let Ok(entry) = entry(name) {
            let _ = entry.delete_credential();
        }
    }
}
fn manifest(storage: &impl Credentials) -> Result<Option<Manifest>> {
    match storage.get("chatgpt.active")? {
        Some(value) => {
            let parsed: Manifest = serde_json::from_str(&value)
                .map_err(|_| anyhow!("Subscription credentials need a fresh sign-in."))?;
            if parsed.chunks == 0
                || parsed.chunks > 100
                || !parsed
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                bail!("Subscription credentials need a fresh sign-in.");
            }
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}
pub(super) fn load() -> Result<Vault> {
    load_from(&Native)
}
fn load_from(storage: &impl Credentials) -> Result<Vault> {
    let Some(manifest) = manifest(storage)? else {
        return Ok(Vault::default());
    };
    let mut encoded = String::new();
    for i in 0..manifest.chunks {
        encoded.push_str(
            &storage
                .get(&format!("chatgpt.{}.{}", manifest.id, i))?
                .ok_or_else(|| anyhow!("Subscription credentials need a fresh sign-in."))?,
        );
        if encoded.len() > 90_000 {
            bail!("Subscription credentials are invalid.");
        }
    }
    // ASCII base64 avoids splitting Unicode and limits each OS credential to 1800 UTF-16 bytes.
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| anyhow!("Subscription credentials are invalid."))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("Subscription credentials need a fresh sign-in."))
}
pub(super) fn save(vault: &Vault) -> Result<()> {
    save_to(vault, &Native)
}
fn save_to(vault: &Vault, storage: &impl Credentials) -> Result<()> {
    use base64::Engine;
    let old = manifest(storage)?;
    let bytes = serde_json::to_vec(vault)
        .map_err(|_| anyhow!("Cannot protect subscription credentials."))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    if encoded.len() > 90_000 {
        bail!("Subscription credential size exceeds the native storage limit.");
    }
    let id = random()?;
    let chunks = encoded.as_bytes().chunks(900).collect::<Vec<_>>();
    for (i, chunk) in chunks.iter().enumerate() {
        storage.set(
            &format!("chatgpt.{id}.{i}"),
            std::str::from_utf8(chunk).expect("base64 ASCII"),
        )?;
    }
    storage.set(
        "chatgpt.active",
        &serde_json::to_string(&Manifest {
            id,
            chunks: chunks.len(),
        })
        .map_err(|_| anyhow!("Cannot save subscription credentials."))?,
    )?;
    if let Some(old) = old {
        for i in 0..old.chunks {
            storage.delete(&format!("chatgpt.{}.{}", old.id, i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};
    #[derive(Default)]
    struct FixtureStore {
        entries: RefCell<HashMap<String, String>>,
        writes_left: RefCell<Option<usize>>,
    }
    impl Credentials for FixtureStore {
        fn get(&self, name: &str) -> Result<Option<String>> {
            Ok(self.entries.borrow().get(name).cloned())
        }
        fn set(&self, name: &str, value: &str) -> Result<()> {
            if let Some(left) = self.writes_left.borrow_mut().as_mut() {
                if *left == 0 {
                    bail!("fixture storage failure");
                }
                *left -= 1;
            }
            assert!(
                value.encode_utf16().count() * 2 <= 2560,
                "must fit Windows native credential limit"
            );
            self.entries.borrow_mut().insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) {
            self.entries.borrow_mut().remove(name);
        }
    }
    #[test]
    fn failed_rotation_keeps_old_complete_native_record_and_success_replaces_all_fields() {
        let storage = FixtureStore::default();
        let mut vault = Vault {
            host_id: "stable-host".into(),
            registration: Registration {
                client_id: "oaiapp_fixture".into(),
                subject: "person".into(),
                email: Some("pessoa-é@example.test".into()),
                tokens: Some(Tokens {
                    access_token: "old-access".repeat(150),
                    refresh_token: "old-refresh".repeat(150),
                    id_token: "old-id".repeat(150),
                    scope: "old-scope".into(),
                    expires_at: 10,
                }),
            },
        };
        save_to(&vault, &storage).unwrap();
        vault.registration.tokens = Some(Tokens {
            access_token: "new-access".repeat(150),
            refresh_token: "new-refresh".repeat(150),
            id_token: "new-id".repeat(150),
            scope: "new-scope".into(),
            expires_at: 20,
        });
        *storage.writes_left.borrow_mut() = Some(2);
        assert!(save_to(&vault, &storage).is_err());
        let old = load_from(&storage).unwrap();
        let tokens = old.registration.tokens.unwrap();
        assert_eq!(tokens.access_token, "old-access".repeat(150));
        assert_eq!(tokens.refresh_token, "old-refresh".repeat(150));
        assert_eq!(tokens.id_token, "old-id".repeat(150));
        assert_eq!(tokens.expires_at, 10);
        *storage.writes_left.borrow_mut() = None;
        save_to(&vault, &storage).unwrap();
        let new = load_from(&storage).unwrap();
        assert_eq!(new.host_id, "stable-host");
        assert_eq!(new.registration.email.unwrap(), "pessoa-é@example.test");
        let tokens = new.registration.tokens.unwrap();
        assert_eq!(tokens.refresh_token, "new-refresh".repeat(150));
        assert_eq!(tokens.access_token, "new-access".repeat(150));
        assert_eq!(tokens.id_token, "new-id".repeat(150));
        assert_eq!(tokens.expires_at, 20);
        assert_eq!(tokens.scope, "new-scope");
        vault.registration.tokens = None;
        save_to(&vault, &storage).unwrap();
        let disconnected = load_from(&storage).unwrap();
        assert_eq!(disconnected.registration.client_id, "oaiapp_fixture");
        assert!(disconnected.registration.tokens.is_none());
    }
}
