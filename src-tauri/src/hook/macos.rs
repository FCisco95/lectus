//! macOS keyboard hook via `CGEventTap`.
//!
//! Modifiers (Right Ctrl etc.) are delivered as `FlagsChanged` events, not
//! key up/down, so we derive press/release by tracking the previous down-state
//! per target keycode. F13-F15 arrive as ordinary `KeyDown`/`KeyUp`. Requires
//! Accessibility permission (`AXIsProcessTrusted`).

use super::{config_combo_to_mackeys, mode_code, HookContext, KeyboardHook, MODE_TOGGLE};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    EventField,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static RECORDING_ACTIVE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static TARGET_KEY: AtomicU32 = AtomicU32::new(62); // kVK_RightControl default
static TARGET_KEY2: AtomicU32 = AtomicU32::new(u32::MAX); // second chord key; MAX = single-key mode
static K1_DOWN: AtomicBool = AtomicBool::new(false);
static K2_DOWN: AtomicBool = AtomicBool::new(false);
static CHORD_ACTIVE: AtomicBool = AtomicBool::new(false); // all target keys currently held
static TRIGGER_MODE: AtomicU32 = AtomicU32::new(0); // 0 = hold, 1 = toggle
static TOGGLE_ON: AtomicBool = AtomicBool::new(false); // latched state in toggle mode

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// Set recording active/inactive and emit the matching start/stop event.
fn signal(start: bool) {
    if let Some(active) = RECORDING_ACTIVE.get() {
        active.store(start, Ordering::SeqCst);
    }
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(if start { "hold-start" } else { "hold-stop" }, ());
    }
}

/// Re-evaluate the chord after a target key changed state. Auto-repeat
/// produces no edge because CHORD_ACTIVE is already latched.
fn evaluate_chord() {
    let t2 = TARGET_KEY2.load(Ordering::Relaxed);
    let all_down =
        K1_DOWN.load(Ordering::SeqCst) && (t2 == u32::MAX || K2_DOWN.load(Ordering::SeqCst));
    let toggle = TRIGGER_MODE.load(Ordering::Relaxed) == MODE_TOGGLE;
    if all_down {
        if !CHORD_ACTIVE.swap(true, Ordering::SeqCst) {
            if toggle {
                let now_on = !TOGGLE_ON.fetch_xor(true, Ordering::SeqCst);
                signal(now_on);
            } else {
                signal(true);
            }
        }
    } else if CHORD_ACTIVE.swap(false, Ordering::SeqCst) && !toggle {
        signal(false);
    }
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

pub struct MacosHook;

impl KeyboardHook for MacosHook {
    fn install(ctx: HookContext) -> anyhow::Result<Self> {
        // Prompt + verify Accessibility (mandatory; cannot be auto-granted).
        if !unsafe { AXIsProcessTrusted() } {
            eprintln!(
                "Lectus needs Accessibility permission for hold-to-talk. \
                 Grant it in System Settings → Privacy & Security → Accessibility, then restart."
            );
            anyhow::bail!("accessibility permission not granted");
        }

        let _ = APP_HANDLE.set(ctx.app);
        let _ = RECORDING_ACTIVE.set(ctx.recording_active);
        let (k1, k2) = config_combo_to_mackeys(&ctx.target_key)
            .ok_or_else(|| anyhow::anyhow!("unsupported hold key: {}", ctx.target_key))?;
        TARGET_KEY.store(k1 as u32, Ordering::Relaxed);
        TARGET_KEY2.store(k2.map(|k| k as u32).unwrap_or(u32::MAX), Ordering::Relaxed);
        TRIGGER_MODE.store(mode_code(&ctx.trigger_mode), Ordering::Relaxed);

        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            let tap = CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                vec![
                    CGEventType::KeyDown,
                    CGEventType::KeyUp,
                    CGEventType::FlagsChanged,
                ],
                |_proxy, event_type, event| {
                    let keycode =
                        event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u32;
                    let down_flag = if keycode == TARGET_KEY.load(Ordering::Relaxed) {
                        Some(&K1_DOWN)
                    } else if keycode == TARGET_KEY2.load(Ordering::Relaxed) {
                        Some(&K2_DOWN)
                    } else {
                        None
                    };
                    if let Some(flag) = down_flag {
                        let is_down = match event_type {
                            CGEventType::KeyDown => true,
                            CGEventType::KeyUp => false,
                            // Modifier toggled: if currently tracked down it's
                            // a release edge, otherwise a press edge.
                            CGEventType::FlagsChanged => !flag.load(Ordering::SeqCst),
                            _ => return None,
                        };
                        flag.store(is_down, Ordering::SeqCst);
                        evaluate_chord();
                    }
                    None
                },
            );

            match tap {
                Ok(tap) => {
                    let loop_source = match tap.mach_port.create_runloop_source(0) {
                        Ok(s) => s,
                        Err(_) => {
                            let _ = tx.send(Err("failed to create runloop source".into()));
                            return;
                        }
                    };
                    let current = CFRunLoop::get_current();
                    unsafe {
                        current.add_source(&loop_source, kCFRunLoopCommonModes);
                    }
                    tap.enable();
                    let _ = tx.send(Ok(()));
                    CFRunLoop::run_current();
                }
                Err(_) => {
                    let _ = tx.send(Err("failed to create event tap".into()));
                }
            }
        });

        match rx.recv() {
            Ok(Ok(())) => Ok(MacosHook),
            Ok(Err(e)) => Err(anyhow::anyhow!("CGEventTap install failed: {e}")),
            Err(_) => Err(anyhow::anyhow!("hook thread died during install")),
        }
    }

    fn rearm(&self, target_key: &str) {
        if let Some((k1, k2)) = config_combo_to_mackeys(target_key) {
            TARGET_KEY.store(k1 as u32, Ordering::Relaxed);
            TARGET_KEY2.store(k2.map(|k| k as u32).unwrap_or(u32::MAX), Ordering::Relaxed);
            // Clear stale press-state: the old key may still be physically held
            // during the swap, so the new key starts from a clean slate.
            clear_press_state();
        }
    }

    fn set_mode(&self, mode: &str) {
        TRIGGER_MODE.store(mode_code(mode), Ordering::Relaxed);
        TOGGLE_ON.store(false, Ordering::SeqCst);
        clear_press_state();
    }

    fn set_toggle_state(&self, on: bool) {
        TOGGLE_ON.store(on, Ordering::SeqCst);
    }
}
