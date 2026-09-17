//! Launch-at-login path repair.
//!
//! The autostart plugin registers `std::env::current_exe()`. Enabling it from a
//! local cargo-target copy (`C:\lt\release\chirp.exe`) leaves that leftover on
//! the HKCU Run key forever, so a reboot starts 0.4.0 even after the NSIS
//! 0.5.0 install. Single-instance then keeps the old process when the user
//! clicks the 0.5.0 shortcut.

use std::path::{Path, PathBuf};

/// Run-key value names used across Lectus versions (`productName` vs crate name).
pub const WINDOWS_VALUE_NAMES: &[&str] = &["Lectus", "chirp"];

/// Prefer the NSIS-installed binary when it exists.
pub fn preferred_exe(current_exe: &Path, installed_exe: Option<&Path>) -> PathBuf {
    installed_exe.unwrap_or(current_exe).to_path_buf()
}

/// True for the cargo-target leftover that used to be the Windows deploy path.
pub fn is_leftover_exe(path: &Path) -> bool {
    let s = normalize(path);
    s.contains(r"\lt\release\chirp.exe")
}

/// HKCU Run value that starts the installed app silently at login.
pub fn desired_run_value(exe: &Path) -> String {
    let p = exe.to_string_lossy();
    if p.contains(' ') {
        format!("\"{p}\" --autostart")
    } else {
        format!("{p} --autostart")
    }
}

/// Path portion of a Run-key command (`C:\...\chirp.exe --autostart` → exe).
pub fn exe_from_run_value(value: &str) -> PathBuf {
    let v = value.trim();
    if let Some(rest) = v.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return PathBuf::from(&rest[..end]);
        }
    }
    PathBuf::from(v.split_whitespace().next().unwrap_or(v))
}

/// If an existing Run-key value should be rewritten, return the path to write.
pub fn replacement_path(
    existing_value: &Path,
    installed: Option<&Path>,
    current_exe: &Path,
) -> Option<PathBuf> {
    let target = preferred_exe(current_exe, installed);
    if paths_eq(existing_value, &target) {
        return None;
    }
    let is_chirp = existing_value
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("chirp.exe"));
    if is_chirp && (is_leftover_exe(existing_value) || installed.is_some()) {
        return Some(target);
    }
    None
}

/// Full REG_SZ to write, or None if `existing` already matches.
pub fn replacement_value(
    existing: &str,
    installed: Option<&Path>,
    current_exe: &Path,
) -> Option<String> {
    let existing_exe = exe_from_run_value(existing);
    let target = replacement_path(&existing_exe, installed, current_exe)
        .unwrap_or_else(|| preferred_exe(current_exe, installed));
    let desired = desired_run_value(&target);
    if normalize(Path::new(existing.trim())) == normalize(Path::new(&desired)) {
        return None;
    }
    let is_chirp = existing_exe
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("chirp.exe"));
    if is_chirp && (is_leftover_exe(&existing_exe) || installed.is_some() || !existing.contains("--autostart"))
    {
        return Some(desired);
    }
    None
}

fn normalize(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_matches('"')
        .to_ascii_lowercase()
}

fn paths_eq(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// `%LOCALAPPDATA%\Lectus\chirp.exe` when that file exists.
pub fn installed_windows_exe() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let p = PathBuf::from(local).join("Lectus").join("chirp.exe");
    p.is_file().then_some(p)
}

/// Rewrite leftover HKCU Run values so login starts the installed app.
///
/// `plugin_enabled` is the plugin's own idea of launch-at-login. Orphan keys
/// from older app names are repaired even when the plugin reports disabled,
/// because those orphans are what actually launch on boot.
#[cfg(windows)]
pub fn repair_windows_run_keys(plugin_enabled: bool) {
    let current = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("autostart repair: current_exe failed: {e}");
            return;
        }
    };
    let installed = installed_windows_exe();
    let target = preferred_exe(&current, installed.as_deref());

    for name in WINDOWS_VALUE_NAMES {
        match read_run_value(name) {
            Ok(Some(existing)) => {
                if let Some(next) = replacement_value(&existing, installed.as_deref(), &current)
                {
                    if let Err(e) = write_run_command(name, &next) {
                        log::warn!("autostart repair: failed to rewrite {name}: {e}");
                    } else {
                        log::info!("autostart repair: {name} -> {next}");
                    }
                }
            }
            Ok(None) => {
                // Plugin thinks autostart is on but this name was never written
                // (name mismatch across versions). Fill the Lectus key so boot
                // starts the installed binary silently.
                if plugin_enabled && *name == "Lectus" {
                    let cmd = desired_run_value(&target);
                    if let Err(e) = write_run_command(name, &cmd) {
                        log::warn!("autostart repair: failed to write {name}: {e}");
                    }
                }
            }
            Err(e) => log::warn!("autostart repair: query {name} failed: {e}"),
        }
    }
}

#[cfg(windows)]
fn read_run_value(name: &str) -> std::io::Result<Option<String>> {
    let output = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            name,
        ])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // REG_SZ    C:\path\chirp.exe
    for line in stdout.lines() {
        if let Some(idx) = line.find("REG_SZ") {
            let value = line[idx + "REG_SZ".len()..].trim();
            if !value.is_empty() {
                return Ok(Some(value.to_string()));
            }
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn write_run_command(name: &str, value: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            name,
            "/t",
            "REG_SZ",
            "/d",
            value,
            "/f",
        ])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn prefers_installed_over_current() {
        let current = PathBuf::from(r"C:\lt\release\chirp.exe");
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        assert_eq!(
            preferred_exe(&current, Some(&installed)),
            installed
        );
    }

    #[test]
    fn falls_back_to_current_when_not_installed() {
        let current = PathBuf::from(r"C:\lt\release\chirp.exe");
        assert_eq!(preferred_exe(&current, None), current);
    }

    #[test]
    fn leftover_path_is_detected() {
        assert!(is_leftover_exe(Path::new(r"C:\lt\release\chirp.exe")));
        assert!(is_leftover_exe(Path::new(r"c:/lt/release/chirp.exe")));
        assert!(!is_leftover_exe(Path::new(
            r"C:\Users\me\AppData\Local\Lectus\chirp.exe"
        )));
    }

    #[test]
    fn rewrites_leftover_run_value() {
        let leftover = PathBuf::from(r"C:\lt\release\chirp.exe");
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let next = replacement_path(&leftover, Some(&installed), &leftover);
        assert_eq!(next.as_deref(), Some(installed.as_path()));
    }

    #[test]
    fn leaves_correct_installed_path_alone() {
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        assert_eq!(
            replacement_path(&installed, Some(&installed), &installed),
            None
        );
    }

    #[test]
    fn desired_run_value_appends_autostart_flag() {
        let exe = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        assert_eq!(
            desired_run_value(&exe),
            r"C:\Users\me\AppData\Local\Lectus\chirp.exe --autostart"
        );
    }

    #[test]
    fn desired_run_value_quotes_paths_with_spaces() {
        let exe = PathBuf::from(r"C:\Program Files\Lectus\chirp.exe");
        assert_eq!(
            desired_run_value(&exe),
            r#""C:\Program Files\Lectus\chirp.exe" --autostart"#
        );
    }

    #[test]
    fn exe_from_run_value_strips_flag() {
        assert_eq!(
            exe_from_run_value(r"C:\Users\me\AppData\Local\Lectus\chirp.exe --autostart"),
            PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe")
        );
        assert_eq!(
            exe_from_run_value(r#""C:\Program Files\Lectus\chirp.exe" --autostart"#),
            PathBuf::from(r"C:\Program Files\Lectus\chirp.exe")
        );
    }

    #[test]
    fn rewrites_installed_path_missing_autostart_flag() {
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let next = replacement_value(
            r"C:\Users\me\AppData\Local\Lectus\chirp.exe",
            Some(&installed),
            &installed,
        );
        assert_eq!(
            next.as_deref(),
            Some(r"C:\Users\me\AppData\Local\Lectus\chirp.exe --autostart")
        );
    }

    #[test]
    fn leaves_correct_autostart_command_alone() {
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        assert_eq!(
            replacement_value(
                r"C:\Users\me\AppData\Local\Lectus\chirp.exe --autostart",
                Some(&installed),
                &installed,
            ),
            None
        );
    }
}
