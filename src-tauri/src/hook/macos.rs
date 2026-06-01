//! macOS keyboard hook via `CGEventTap`.
//!
//! Modifiers (Right Ctrl etc.) are delivered as `FlagsChanged` events, not
//! key up/down, so we derive press/release by tracking the previous down-state
//! per target keycode. F13-F15 arrive as ordinary `KeyDown`/`KeyUp`. Requires
//! Accessibility permission (`AXIsProcessTrusted`).

use super::{config_key_to_mackey, HookContext, KeyboardHook};
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
static KEY_DOWN: AtomicBool = AtomicBool::new(false);

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

fn fire_start() {
    if !KEY_DOWN.swap(true, Ordering::SeqCst) {
        if let Some(active) = RECORDING_ACTIVE.get() {
            active.store(true, Ordering::SeqCst);
        }
        if let Some(app) = APP_HANDLE.get() {
            let _ = app.emit("hold-start", ());
        }
    }
}

fn fire_stop() {
    KEY_DOWN.store(false, Ordering::SeqCst);
    if let Some(active) = RECORDING_ACTIVE.get() {
        active.store(false, Ordering::SeqCst);
    }
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit("hold-stop", ());
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
        let keycode = config_key_to_mackey(&ctx.target_key)
            .ok_or_else(|| anyhow::anyhow!("unsupported hold key: {}", ctx.target_key))?;
        TARGET_KEY.store(keycode as u32, Ordering::Relaxed);

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
                    if keycode == TARGET_KEY.load(Ordering::Relaxed) {
                        match event_type {
                            CGEventType::KeyDown => fire_start(),
                            CGEventType::KeyUp => fire_stop(),
                            CGEventType::FlagsChanged => {
                                // Modifier: down-edge if not already tracked as down.
                                if KEY_DOWN.load(Ordering::SeqCst) {
                                    fire_stop();
                                } else {
                                    fire_start();
                                }
                            }
                            _ => {}
                        }
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
        if let Some(keycode) = config_key_to_mackey(target_key) {
            TARGET_KEY.store(keycode as u32, Ordering::Relaxed);
        }
    }
}
