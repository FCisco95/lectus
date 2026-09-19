//! Launch-at-login path repair, and which `chirp.exe` should win.
//!
//! Two copies exist on a Windows dev machine: the NSIS install
//! (`%LOCALAPPDATA%\Lectus\chirp.exe`) and the cargo target (`C:\lt\release\chirp.exe`).
//! Login, Start Menu, and `tauri-plugin-single-instance` used to always keep the
//! install, so a newer local build never became the running app — the old window
//! just focused. Pick the strictly-newer file (mtime). An older leftover cargo
//! target still loses to a newer NSIS install, which is the original 0.4.0 bug.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Run-key value names used across Lectus versions (`productName` vs crate name).
pub const WINDOWS_VALUE_NAMES: &[&str] = &["Lectus", "chirp"];

/// Prefer the newer of `current_exe` and the NSIS install. Missing timestamps
/// (fake paths in tests) keep the install, matching the old stable-path rule.
pub fn preferred_exe(current_exe: &Path, installed_exe: Option<&Path>) -> PathBuf {
    match installed_exe {
        Some(installed) => pick_newer_path(current_exe, installed, file_mtime),
        None => current_exe.to_path_buf(),
    }
}

/// `current` wins only when it is strictly newer; otherwise `installed`.
pub fn pick_newer_path(
    current: &Path,
    installed: &Path,
    mtime: impl Fn(&Path) -> Option<SystemTime>,
) -> PathBuf {
    match (mtime(current), mtime(installed)) {
        (Some(c), Some(i)) if c > i => current.to_path_buf(),
        _ => installed.to_path_buf(),
    }
}

fn file_mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// If a second-instance launch is a different, strictly-newer `chirp.exe`,
/// return that path so the running process can hand off.
pub fn takeover_exe(running_exe: &Path, incoming_args: &[String]) -> Option<PathBuf> {
    takeover_exe_with(running_exe, incoming_args, file_mtime)
}

pub fn takeover_exe_with(
    running_exe: &Path,
    incoming_args: &[String],
    mtime: impl Fn(&Path) -> Option<SystemTime>,
) -> Option<PathBuf> {
    let incoming = PathBuf::from(incoming_args.first()?);
    if !incoming.is_absolute() {
        return None;
    }
    let name = incoming.file_name()?.to_string_lossy();
    if !name.eq_ignore_ascii_case("chirp.exe") {
        return None;
    }
    if paths_eq(&incoming, running_exe) {
        return None;
    }
    match (mtime(&incoming), mtime(running_exe)) {
        (Some(i), Some(r)) if i > r => Some(incoming),
        _ => None,
    }
}

/// Start `exe` after this process is gone, so it does not bounce off the
/// still-held single-instance mutex. Windows only.
#[cfg(windows)]
pub fn spawn_replacing(exe: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    let quoted = format!("\"{}\"", exe.display());
    let script = format!("timeout /T 1 /NOBREAK >nul & start \"\" {quoted}");
    let _ = std::process::Command::new("cmd")
        .args(["/C", &script])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn();
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
        // Fake paths have no mtime, so the install (stable path) still wins.
        let current = PathBuf::from(r"C:\lt\release\chirp.exe");
        let installed = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        assert_eq!(
            preferred_exe(&current, Some(&installed)),
            installed
        );
    }

    #[test]
    fn pick_newer_path_prefers_strictly_newer_current() {
        let current = Path::new(r"C:\lt\release\chirp.exe");
        let installed = Path::new(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let t0 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let t1 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2);
        let mtime = |p: &Path| {
            if p == current {
                Some(t1)
            } else {
                Some(t0)
            }
        };
        assert_eq!(pick_newer_path(current, installed, mtime), current);
    }

    #[test]
    fn pick_newer_path_keeps_installed_when_equal_or_older() {
        let current = Path::new(r"C:\lt\release\chirp.exe");
        let installed = Path::new(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let t0 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let mtime = |_p: &Path| Some(t0);
        assert_eq!(pick_newer_path(current, installed, mtime), installed);
    }

    #[test]
    fn takeover_skips_same_path_and_older_incoming() {
        let running = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let cargo = PathBuf::from(r"C:\lt\release\chirp.exe");
        let t_old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let t_new = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2);
        let older_cargo = |p: &Path| {
            if p == cargo.as_path() {
                Some(t_old)
            } else {
                Some(t_new)
            }
        };
        assert_eq!(
            takeover_exe_with(
                &running,
                &[running.to_string_lossy().into()],
                |_| Some(t_new)
            ),
            None
        );
        assert_eq!(
            takeover_exe_with(&running, &[cargo.to_string_lossy().into()], older_cargo),
            None
        );
    }

    #[test]
    fn takeover_hands_off_to_newer_different_exe() {
        let running = PathBuf::from(r"C:\Users\me\AppData\Local\Lectus\chirp.exe");
        let cargo = PathBuf::from(r"C:\lt\release\chirp.exe");
        let t_old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        let t_new = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2);
        let mtime = |p: &Path| {
            if p == cargo.as_path() {
                Some(t_new)
            } else {
                Some(t_old)
            }
        };
        assert_eq!(
            takeover_exe_with(&running, &[cargo.to_string_lossy().into()], mtime),
            Some(cargo)
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
