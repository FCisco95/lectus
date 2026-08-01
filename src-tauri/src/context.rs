//! Focused-app detection for per-app profiles.
//!
//! Captured at the moment the hotkey lands — that is the app the text will be
//! injected into, so its profile governs tone/cleanup/language for this
//! dictation.

/// Lower-cased executable name of the foreground window's process, e.g.
/// "code.exe", "chrome.exe". `None` when it can't be determined.
#[cfg(target_os = "windows")]
pub fn foreground_app() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/'])
            .next()
            .map(|s| s.to_ascii_lowercase())
    }
}

/// Lower-cased localized name of the frontmost app, e.g. "visual studio code",
/// "slack". Windows yields "code.exe" — profiles match by case-insensitive
/// substring, so a rule like "code" hits on both platforms.
#[cfg(target_os = "macos")]
// objc 0.2's msg_send! internals probe cfg(cargo-clippy), tripping the
// unexpected_cfgs lint on modern rustc — noise, not a real problem.
#[allow(unexpected_cfgs)]
pub fn foreground_app() -> Option<String> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    // NSWorkspace getters are safe off the main thread; the pipeline calls
    // this from a tauri worker.
    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: *mut Object = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let name: *mut Object = msg_send![app, localizedName];
        if name.is_null() {
            return None;
        }
        let utf8: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if utf8.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr(utf8)
                .to_string_lossy()
                .to_ascii_lowercase(),
        )
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn foreground_app() -> Option<String> {
    None
}
