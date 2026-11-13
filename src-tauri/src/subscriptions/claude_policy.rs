//! Safe mode preserves OAuth, but cannot suppress managed hooks/provider env.
//! Reject documented policy sources instead of bypassing organization policy.
use super::*;
use std::path::{Path, PathBuf};

const POLICY_ERROR: &str = "Claude managed policy was detected or cannot be excluded. Lectus cannot guarantee text-only execution without managed hooks/provider overrides. Use an unmanaged personal Pro/Max login or another engine; Lectus does not change your policy.";

fn absent(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => bail!(POLICY_ERROR),
    }
}
fn no_policy_files(system_dirs: &[PathBuf], config: &Path) -> Result<()> {
    for root in system_dirs {
        for name in [
            "managed-settings.json",
            "managed-settings.d",
            "managed-mcp.json",
        ] {
            absent(&root.join(name))?;
        }
    }
    // Only presence is inspected, never remote policy contents or credentials.
    absent(&config.join("remote-settings.json"))?;
    Ok(())
}

#[cfg(windows)]
fn no_os_policy() -> Result<()> {
    use windows::{
        core::w,
        Win32::{
            Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND},
            System::Registry::*,
        },
    };
    // Check both registry views; any policy key (including unreadable) denies.
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let mut key = HKEY::default();
            let result = unsafe {
                RegOpenKeyExW(
                    hive,
                    w!("SOFTWARE\\Policies\\ClaudeCode"),
                    0,
                    KEY_READ | view,
                    &mut key,
                )
            };
            if result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND {
                continue;
            }
            if result.is_ok() {
                unsafe {
                    let _ = RegCloseKey(key);
                }
            }
            bail!(POLICY_ERROR);
        }
    }
    Ok(())
}
#[cfg(target_os = "macos")]
fn no_os_policy() -> Result<()> {
    use core_foundation::{base::TCFType, string::CFString};
    // Query preference key presence only. Do not inspect values or execute a
    // managed helper. Check all documented user/host domains conservatively.
    extern "C" {
        fn CFPreferencesCopyKeyList(
            app: *const std::ffi::c_void,
            user: *const std::ffi::c_void,
            host: *const std::ffi::c_void,
        ) -> *const std::ffi::c_void;
        fn CFArrayGetCount(array: *const std::ffi::c_void) -> isize;
        fn CFRelease(value: *const std::ffi::c_void);
        static kCFPreferencesAnyUser: *const std::ffi::c_void;
        static kCFPreferencesCurrentUser: *const std::ffi::c_void;
        static kCFPreferencesAnyHost: *const std::ffi::c_void;
        static kCFPreferencesCurrentHost: *const std::ffi::c_void;
    }
    let app = CFString::new("com.anthropic.claudecode");
    unsafe {
        for user in [kCFPreferencesAnyUser, kCFPreferencesCurrentUser] {
            for host in [kCFPreferencesAnyHost, kCFPreferencesCurrentHost] {
                let keys = CFPreferencesCopyKeyList(app.as_concrete_TypeRef().cast(), user, host);
                if !keys.is_null() {
                    let count = CFArrayGetCount(keys);
                    CFRelease(keys);
                    if count != 0 {
                        bail!(POLICY_ERROR);
                    }
                }
            }
        }
    }
    Ok(())
}
#[cfg(not(any(windows, target_os = "macos")))]
fn no_os_policy() -> Result<()> {
    bail!("Claude subscription isolation is supported on Windows and macOS.")
}

pub(super) fn ensure_unmanaged(config: &Path) -> Result<()> {
    let mut dirs = Vec::new();
    #[cfg(windows)]
    {
        dirs.push(PathBuf::from("C:\\Program Files\\ClaudeCode"));
        for variable in ["ProgramFiles", "ProgramW6432"] {
            if let Some(root) = std::env::var_os(variable) {
                dirs.push(PathBuf::from(root).join("ClaudeCode"));
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Library/Application Support/ClaudeCode"));
    }
    no_policy_files(&dirs, config)?;
    no_os_policy()
}

pub(super) fn ensure_personal_subscription(value: &serde_json::Value) -> Result<()> {
    if !matches!(value["subscriptionType"].as_str(), Some("pro" | "max")) {
        bail!("Lectus currently supports unmanaged personal Claude Pro/Max subscriptions only. Team/Enterprise remote policy cannot be excluded, so no text is sent. Use ChatGPT or another engine for this account.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_policy_presence_and_remote_cache_fail_closed_without_reading_values() {
        let fixture = tempfile::tempdir().unwrap();
        let system = fixture.path().join("system");
        let config = fixture.path().join("isolated");
        std::fs::create_dir_all(&system).unwrap();
        std::fs::create_dir_all(&config).unwrap();
        assert!(no_policy_files(&[system.clone()], &config).is_ok());
        for name in ["managed-settings.json", "managed-mcp.json"] {
            std::fs::write(
                system.join(name),
                b"never parse or disclose this secret fixture",
            )
            .unwrap();
            let error = no_policy_files(&[system.clone()], &config)
                .unwrap_err()
                .to_string();
            assert!(!error.contains("secret fixture"));
            std::fs::remove_file(system.join(name)).unwrap();
        }
        std::fs::create_dir(system.join("managed-settings.d")).unwrap();
        assert!(no_policy_files(&[system.clone()], &config).is_err());
        std::fs::remove_dir(system.join("managed-settings.d")).unwrap();
        std::fs::write(config.join("remote-settings.json"), b"{}").unwrap();
        assert!(no_policy_files(&[system], &config).is_err());
    }
    #[test]
    fn organizational_plan_is_not_an_attestation_of_policy_isolation() {
        for plan in ["team", "enterprise", "unknown", ""] {
            assert!(
                ensure_personal_subscription(&serde_json::json!({"subscriptionType":plan}))
                    .is_err()
            );
        }
        assert!(
            ensure_personal_subscription(&serde_json::json!({"subscriptionType":"max"})).is_ok()
        );
    }
}
