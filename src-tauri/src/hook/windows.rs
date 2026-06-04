//! Windows low-level keyboard hook (`WH_KEYBOARD_LL`).
//!
//! The hook callback must be `extern "system"` with no captures, so shared
//! state lives in statics. The callback does only atomic stores + an
//! empty-payload `emit` to stay well under the `LowLevelHooksTimeout` (~300 ms),
//! and ALWAYS calls `CallNextHookEx` so the key is never consumed.

use super::{config_key_to_vk, mode_code, HookContext, KeyboardHook, MODE_TOGGLE};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static RECORDING_ACTIVE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static TARGET_VK: AtomicU32 = AtomicU32::new(0xA3); // VK_RCONTROL default
static KEY_DOWN: AtomicBool = AtomicBool::new(false); // debounce auto-repeat
static TRIGGER_MODE: AtomicU32 = AtomicU32::new(0); // 0 = hold, 1 = toggle
static TOGGLE_ON: AtomicBool = AtomicBool::new(false); // latched state in toggle mode

/// Set recording active/inactive and emit the matching start/stop event.
/// Called from the hook callback — keeps the work minimal (atomic + emit).
fn signal(start: bool) {
    if let Some(active) = RECORDING_ACTIVE.get() {
        active.store(start, Ordering::SeqCst);
    }
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(if start { "hold-start" } else { "hold-stop" }, ());
    }
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if kb.vkCode == TARGET_VK.load(Ordering::Relaxed) {
            let toggle = TRIGGER_MODE.load(Ordering::Relaxed) == MODE_TOGGLE;
            match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    // Ignore OS auto-repeat: only the first down edge counts.
                    if !KEY_DOWN.swap(true, Ordering::SeqCst) {
                        if toggle {
                            // Flip the latch: tap to start, tap again to stop.
                            let now_on = !TOGGLE_ON.fetch_xor(true, Ordering::SeqCst);
                            signal(now_on);
                        } else {
                            signal(true);
                        }
                    }
                }
                WM_KEYUP | WM_SYSKEYUP => {
                    KEY_DOWN.store(false, Ordering::SeqCst);
                    // In toggle mode the release is ignored; the latch decides.
                    if !toggle {
                        signal(false);
                    }
                }
                _ => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub struct WindowsHook {
    thread_id: u32,
}

impl KeyboardHook for WindowsHook {
    fn install(ctx: HookContext) -> anyhow::Result<Self> {
        // Installed exactly once at startup; live key changes go through
        // `rearm` (atomic store), never a second install. The `set` calls
        // therefore never fail in practice — discard is intentional.
        let _ = APP_HANDLE.set(ctx.app);
        let _ = RECORDING_ACTIVE.set(ctx.recording_active);
        let vk = config_key_to_vk(&ctx.target_key)
            .ok_or_else(|| anyhow::anyhow!("unsupported hold key: {}", ctx.target_key))?;
        TARGET_VK.store(vk, Ordering::Relaxed);
        TRIGGER_MODE.store(mode_code(&ctx.trigger_mode), Ordering::Relaxed);

        // The hook must be installed on, and events delivered to, the thread
        // running the message pump. Spawn that thread and report install
        // success/failure back over a channel.
        let (tx, rx) = std::sync::mpsc::channel::<Result<u32, String>>();
        std::thread::spawn(move || unsafe {
            let hmod = match GetModuleHandleW(None) {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    return;
                }
            };
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                HINSTANCE(hmod.0),
                0,
            );
            match hook {
                Ok(h) => {
                    let _ = tx.send(Ok(GetCurrentThreadId()));
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    let _ = UnhookWindowsHookEx(h);
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        });

        match rx.recv() {
            Ok(Ok(thread_id)) => Ok(WindowsHook { thread_id }),
            Ok(Err(e)) => Err(anyhow::anyhow!("SetWindowsHookExW failed: {e}")),
            Err(_) => Err(anyhow::anyhow!("hook thread died during install")),
        }
    }

    fn rearm(&self, target_key: &str) {
        // Live key swap is a single atomic store — no thread teardown.
        if let Some(vk) = config_key_to_vk(target_key) {
            TARGET_VK.store(vk, Ordering::Relaxed);
            // Clear stale press-state: the old key may still be physically held
            // during the swap, so the new key starts from a clean slate.
            KEY_DOWN.store(false, Ordering::SeqCst);
            if let Some(active) = RECORDING_ACTIVE.get() {
                active.store(false, Ordering::SeqCst);
            }
        }
    }

    fn set_mode(&self, mode: &str) {
        TRIGGER_MODE.store(mode_code(mode), Ordering::Relaxed);
        // Reset latches so the new mode starts clean (never stuck recording).
        TOGGLE_ON.store(false, Ordering::SeqCst);
        KEY_DOWN.store(false, Ordering::SeqCst);
        if let Some(active) = RECORDING_ACTIVE.get() {
            active.store(false, Ordering::SeqCst);
        }
    }

    fn set_toggle_state(&self, on: bool) {
        // Keep the physical toggle key in sync with click-to-toggle / pipeline end.
        TOGGLE_ON.store(on, Ordering::SeqCst);
    }
}

impl Drop for WindowsHook {
    fn drop(&mut self) {
        // Break the pump's GetMessageW loop so UnhookWindowsHookEx runs.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

