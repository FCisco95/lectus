//! Windows low-level keyboard hook (`WH_KEYBOARD_LL`).
//!
//! The hook callback must be `extern "system"` with no captures, so shared
//! state lives in statics. The callback does only atomic stores + an
//! empty-payload `emit` to stay well under the `LowLevelHooksTimeout` (~300 ms),
//! and ALWAYS calls `CallNextHookEx` so the key is never consumed.

use super::{config_combo_to_vks, mode_code, HookContext, KeyboardHook, MODE_TOGGLE};
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
static TARGET_VK2: AtomicU32 = AtomicU32::new(0); // second chord key; 0 = single-key mode
static K1_DOWN: AtomicBool = AtomicBool::new(false);
static K2_DOWN: AtomicBool = AtomicBool::new(false);
static CHORD_ACTIVE: AtomicBool = AtomicBool::new(false); // all target keys currently held
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
        let (t1, t2) = (
            TARGET_VK.load(Ordering::Relaxed),
            TARGET_VK2.load(Ordering::Relaxed),
        );
        let is_down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let matched = if kb.vkCode == t1 {
            K1_DOWN.store(is_down, Ordering::SeqCst);
            true
        } else if t2 != 0 && kb.vkCode == t2 {
            K2_DOWN.store(is_down, Ordering::SeqCst);
            true
        } else {
            false
        };
        if matched {
            // Chord edge detection: auto-repeat re-stores true, producing no
            // edge because CHORD_ACTIVE is already set.
            let all_down = K1_DOWN.load(Ordering::SeqCst)
                && (t2 == 0 || K2_DOWN.load(Ordering::SeqCst));
            let toggle = TRIGGER_MODE.load(Ordering::Relaxed) == MODE_TOGGLE;
            if all_down {
                if !CHORD_ACTIVE.swap(true, Ordering::SeqCst) {
                    if toggle {
                        // Flip the latch: tap to start, tap again to stop.
                        let now_on = !TOGGLE_ON.fetch_xor(true, Ordering::SeqCst);
                        signal(now_on);
                    } else {
                        signal(true);
                    }
                }
            } else if CHORD_ACTIVE.swap(false, Ordering::SeqCst) {
                // In toggle mode the release is ignored; the latch decides.
                if !toggle {
                    signal(false);
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Reset per-key and chord state plus the recording flag.
fn clear_press_state() {
    K1_DOWN.store(false, Ordering::SeqCst);
    K2_DOWN.store(false, Ordering::SeqCst);
    CHORD_ACTIVE.store(false, Ordering::SeqCst);
    if let Some(active) = RECORDING_ACTIVE.get() {
        active.store(false, Ordering::SeqCst);
    }
}

pub struct WindowsHook {
    thread_id: u32,
}

/// Spawn the message-pump thread that owns the WH_KEYBOARD_LL hook. The hook
/// must be installed on, and events delivered to, the thread running the pump.
/// Returns the pump's thread id (used to post WM_QUIT for teardown).
fn spawn_hook_thread() -> anyhow::Result<u32> {
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
        Ok(Ok(thread_id)) => Ok(thread_id),
        Ok(Err(e)) => Err(anyhow::anyhow!("SetWindowsHookExW failed: {e}")),
        Err(_) => Err(anyhow::anyhow!("hook thread died during install")),
    }
}

impl KeyboardHook for WindowsHook {
    fn install(ctx: HookContext) -> anyhow::Result<Self> {
        // Installed exactly once at startup; live key changes go through
        // `rearm` (atomic store), never a second install. The `set` calls
        // therefore never fail in practice — discard is intentional.
        let _ = APP_HANDLE.set(ctx.app);
        let _ = RECORDING_ACTIVE.set(ctx.recording_active);
        let (vk1, vk2) = config_combo_to_vks(&ctx.target_key)
            .ok_or_else(|| anyhow::anyhow!("unsupported hold key: {}", ctx.target_key))?;
        TARGET_VK.store(vk1, Ordering::Relaxed);
        TARGET_VK2.store(vk2.unwrap_or(0), Ordering::Relaxed);
        TRIGGER_MODE.store(mode_code(&ctx.trigger_mode), Ordering::Relaxed);

        let thread_id = spawn_hook_thread()?;
        Ok(WindowsHook { thread_id })
    }

    fn rearm(&self, target_key: &str) {
        // Live key swap is a pair of atomic stores — no thread teardown.
        if let Some((vk1, vk2)) = config_combo_to_vks(target_key) {
            TARGET_VK.store(vk1, Ordering::Relaxed);
            TARGET_VK2.store(vk2.unwrap_or(0), Ordering::Relaxed);
            // Clear stale press-state: the old key may still be physically held
            // during the swap, so the new key starts from a clean slate.
            clear_press_state();
        }
    }

    fn set_mode(&self, mode: &str) {
        TRIGGER_MODE.store(mode_code(mode), Ordering::Relaxed);
        // Reset latches so the new mode starts clean (never stuck recording).
        TOGGLE_ON.store(false, Ordering::SeqCst);
        clear_press_state();
    }

    fn set_toggle_state(&self, on: bool) {
        // Keep the physical toggle key in sync with click-to-toggle / pipeline end.
        TOGGLE_ON.store(on, Ordering::SeqCst);
    }

    fn reinstall(&mut self) {
        // Tear down the old pump (also unhooks) and install a fresh hook.
        // Statics (target keys, mode, latches) carry over untouched; only the
        // OS-level hook registration is renewed.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        match spawn_hook_thread() {
            Ok(id) => {
                self.thread_id = id;
                clear_press_state();
                log::debug!("keyboard hook reinstalled (watchdog)");
            }
            Err(e) => log::error!("keyboard hook reinstall failed: {e}"),
        }
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

