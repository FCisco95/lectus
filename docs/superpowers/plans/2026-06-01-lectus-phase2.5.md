# Lectus Phase 2.5 — Daily-Driver UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Lectus **v0.3.0** with true hold-to-talk (hold Right Ctrl, release to stop) via a low-level keyboard hook on **Windows (`WH_KEYBOARD_LL`) and macOS (`CGEventTap`)**, a visible pill overlay, an editable hotkey, reliable paste injection, and real Eclectus-parrot branding.

**Architecture:** A new cross-platform `hook/` module installs an OS keyboard hook that, on key-down of the configured target key, sets an `Arc<AtomicBool> recording_active = true` and emits `hold-start`; on key-up it clears the flag and emits `hold-stop`. A Rust-side listener on `hold-start` spawns the existing pipeline directly (no JS round-trip). The capture loop polls `recording_active` every 32 ms and stops on release (or a 30 s safety cap). The pill window shows/hides inside `do_set_state`. The dead `tauri-plugin-global-shortcut` activation path is removed (dependency kept).

**Tech Stack:** Tauri 2, Rust (windows-rs 0.58, core-graphics/core-foundation on macOS), React + TypeScript, internal crate name **`chirp`** (unchanged).

---

## Pre-flight

- Internal crate name stays `chirp` / `chirp_lib`. Do **not** rename.
- Master Eclectus logo is already at `src-tauri/icons/lectus-master.png` (1254×1254, transparent) — used by Task 13.
- Work **Windows-first** within each shared task, then add the macOS path. macOS files are written here but compile/runtime-verified on a Mac (Task 4, Task 9 macOS notes, manual verification).
- Run all `cargo` commands from `src-tauri/`. Run all `npm` commands from the repo root.
- Branch: all work happens on `feat/phase2.5-hold-to-talk` (created in Task 0).

---

## File Structure

| File | Responsibility | Action |
|---|---|---|
| `src-tauri/src/hook/mod.rs` | `KeyboardHook` trait, `HookContext`, cross-platform key-string→native-key maps (unit-tested), `PlatformHook` cfg re-export | Create |
| `src-tauri/src/hook/windows.rs` | `WH_KEYBOARD_LL` install + message pump + callback + `rearm` | Create |
| `src-tauri/src/hook/macos.rs` | `CGEventTap` + Accessibility check + `rearm` | Create |
| `src-tauri/src/lib.rs` | `Activation` state, hook install, `hold-start` listener, capture-loop stop flag, pill show/hide, `save_config` re-arm; remove plugin activation | Modify |
| `src-tauri/src/hotkey.rs` | Old plugin hotkey parser — now off the call path | Modify (`#![allow(dead_code)]`) |
| `src-tauri/src/injection/mod.rs` | Paste with clipboard read-back confirmation | Modify |
| `src-tauri/Cargo.toml` | Win32 + macOS hook features/deps | Modify |
| `src-tauri/tauri.conf.json` | Version bump, bundle tray icons as resources | Modify |
| `src/components/Settings.tsx` | Click-to-capture hotkey control | Modify |
| `src/App.tsx` | Remove frontend `pipeline-start`→`invoke` activation | Modify |
| `package.json` | Version bump | Modify |
| `scripts/gen_tray_icons.ps1` | Generate tray-state PNGs from the master logo | Create |

---

## Task 0: Branch setup

- [ ] **Step 1: Confirm clean tree on master**

Run: `git status --short`
Expected: no output (clean).

- [ ] **Step 2: Create and switch to the feature branch**

```bash
git checkout -b feat/phase2.5-hold-to-talk
```

- [ ] **Step 3: Confirm baseline tests pass**

Run (from `src-tauri/`): `cargo test`
Expected: existing 17 tests pass.

---

## Task 1: Cargo dependencies + hook module scaffold

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/hook/mod.rs`
- Modify: `src-tauri/src/lib.rs:1-6` (module declarations)

- [ ] **Step 1: Add Win32 features and macOS crates to `Cargo.toml`**

Replace the two platform dependency blocks (lines 38-46) with:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.26"
objc = "0.2"
core-graphics = "0.24"
core-foundation = "0.10"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
  "Win32_UI_Input_KeyboardAndMouse",
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_LibraryLoader",
  "Win32_System_Threading",
  "Win32_Foundation",
] }
```

- [ ] **Step 2: Create the (temporarily empty) hook module file**

Create `src-tauri/src/hook/mod.rs`:

```rust
//! Cross-platform low-level keyboard hook for hold-to-talk activation.
//!
//! On key-down of the configured target key the platform hook sets
//! `recording_active = true` and emits `hold-start`; on key-up it clears the
//! flag and emits `hold-stop`. The key-string→native-key maps below are the
//! unit-testable core shared by both platforms.

// Platform implementations + re-export are added in later tasks.
```

- [ ] **Step 3: Declare the module in `lib.rs`**

In `src-tauri/src/lib.rs`, add `mod hook;` to the module list (after `mod config;`):

```rust
mod audio;
mod config;
mod hook;
mod hotkey;
mod injection;
mod state;
mod transcription;
```

- [ ] **Step 4: Verify it still compiles**

Run (from `src-tauri/`): `cargo check`
Expected: compiles (the empty module is allowed; the new Win32 features download/build).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/hook/mod.rs src-tauri/src/lib.rs
git commit -m "build: add Win32 hook features + macOS CGEventTap crates; scaffold hook module"
```

---

## Task 2: Key-string ↔ native-key maps (TDD)

The maps use plain integer literals so they compile and unit-test on any OS.

**Files:**
- Modify: `src-tauri/src/hook/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/src/hook/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_map_known_keys() {
        assert_eq!(config_key_to_vk("RControl"), Some(0xA3));
        assert_eq!(config_key_to_vk("LControl"), Some(0xA2));
        assert_eq!(config_key_to_vk("RShift"), Some(0xA1));
        assert_eq!(config_key_to_vk("LShift"), Some(0xA0));
        assert_eq!(config_key_to_vk("RAlt"), Some(0xA5));
        assert_eq!(config_key_to_vk("LAlt"), Some(0xA4));
        assert_eq!(config_key_to_vk("F13"), Some(0x7C));
        assert_eq!(config_key_to_vk("F14"), Some(0x7D));
        assert_eq!(config_key_to_vk("F15"), Some(0x7E));
    }

    #[test]
    fn vk_map_rejects_junk_and_combos() {
        assert_eq!(config_key_to_vk("NotAKey"), None);
        assert_eq!(config_key_to_vk("Ctrl+Shift+Space"), None);
        assert_eq!(config_key_to_vk(""), None);
    }

    #[test]
    fn mackey_map_known_keys() {
        assert_eq!(config_key_to_mackey("RControl"), Some(62));
        assert_eq!(config_key_to_mackey("LControl"), Some(59));
        assert_eq!(config_key_to_mackey("RShift"), Some(60));
        assert_eq!(config_key_to_mackey("LShift"), Some(56));
        assert_eq!(config_key_to_mackey("RAlt"), Some(61));
        assert_eq!(config_key_to_mackey("LAlt"), Some(58));
        assert_eq!(config_key_to_mackey("F13"), Some(105));
        assert_eq!(config_key_to_mackey("F14"), Some(107));
        assert_eq!(config_key_to_mackey("F15"), Some(113));
    }

    #[test]
    fn mackey_map_rejects_junk() {
        assert_eq!(config_key_to_mackey("NotAKey"), None);
    }

    #[test]
    fn is_supported_matches_vk_map() {
        assert!(is_supported_hold_key("RControl"));
        assert!(!is_supported_hold_key("Ctrl+Shift+Space"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run (from `src-tauri/`): `cargo test --lib hook::tests`
Expected: FAIL — `config_key_to_vk` / `config_key_to_mackey` / `is_supported_hold_key` not found.

- [ ] **Step 3: Implement the maps**

Insert above the `#[cfg(test)]` block in `src-tauri/src/hook/mod.rs`:

```rust
/// Map a Lectus config key string to a Windows virtual-key code (`VK_*`).
/// Returns `None` for unknown keys or combos (only bare modifiers / F13-F15
/// are valid hold-to-talk targets). Defined with plain literals so it compiles
/// and is the canonical key validator on every platform.
pub fn config_key_to_vk(s: &str) -> Option<u32> {
    match s {
        "RControl" => Some(0xA3), // VK_RCONTROL
        "LControl" => Some(0xA2), // VK_LCONTROL
        "RShift" => Some(0xA1),   // VK_RSHIFT
        "LShift" => Some(0xA0),   // VK_LSHIFT
        "RAlt" => Some(0xA5),     // VK_RMENU
        "LAlt" => Some(0xA4),     // VK_LMENU
        "F13" => Some(0x7C),
        "F14" => Some(0x7D),
        "F15" => Some(0x7E),
        _ => None,
    }
}

/// Map a Lectus config key string to a macOS virtual keycode (`kVK_*`).
pub fn config_key_to_mackey(s: &str) -> Option<u16> {
    match s {
        "RControl" => Some(62),
        "LControl" => Some(59),
        "RShift" => Some(60),
        "LShift" => Some(56),
        "RAlt" => Some(61),
        "LAlt" => Some(58),
        "F13" => Some(105),
        "F14" => Some(107),
        "F15" => Some(113),
        _ => None,
    }
}

/// Whether a config string is a valid hold-to-talk target on any platform.
pub fn is_supported_hold_key(s: &str) -> bool {
    config_key_to_vk(s).is_some()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run (from `src-tauri/`): `cargo test --lib hook::tests`
Expected: PASS (5 new tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/hook/mod.rs
git commit -m "feat(hook): cross-platform key-string to native-key maps with tests"
```

---

## Task 3: Hook trait, `HookContext`, and platform re-export

**Files:**
- Modify: `src-tauri/src/hook/mod.rs`

- [ ] **Step 1: Add the trait, context, and cfg re-exports**

Insert at the top of `src-tauri/src/hook/mod.rs` (after the doc comment, before the maps):

```rust
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Everything a platform hook needs to install.
pub struct HookContext {
    pub app: tauri::AppHandle,
    pub recording_active: Arc<AtomicBool>,
    pub target_key: String,
}

/// A platform keyboard hook. Installed once at startup; `rearm` swaps the
/// watched key live (no thread teardown). Cleanup happens on `Drop`.
pub trait KeyboardHook: Sized {
    fn install(ctx: HookContext) -> anyhow::Result<Self>;
    fn rearm(&self, target_key: &str);
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsHook as PlatformHook;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacosHook as PlatformHook;

// Fallback no-op so the crate still compiles on other targets (e.g. Linux CI).
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub struct PlatformHook;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl KeyboardHook for PlatformHook {
    fn install(_ctx: HookContext) -> anyhow::Result<Self> {
        anyhow::bail!("keyboard hook not supported on this platform")
    }
    fn rearm(&self, _target_key: &str) {}
}
```

- [ ] **Step 2: Verify the module compiles (windows.rs/macos.rs not yet present → expected error)**

Run (from `src-tauri/`): `cargo check`
Expected: FAIL on Windows — `file not found for module windows`. This is expected; Task 4 (Windows) resolves it. Proceed.

> Note: do not commit yet — Task 4 produces the matching platform file so the tree compiles before committing.

---

## Task 4: Windows hook implementation (`WH_KEYBOARD_LL`)

Runtime-verified (the callback fires only with real keypresses), but **must `cargo check` clean on Windows**.

**Files:**
- Create: `src-tauri/src/hook/windows.rs`

- [ ] **Step 1: Write the Windows hook**

Create `src-tauri/src/hook/windows.rs`:

```rust
//! Windows low-level keyboard hook (`WH_KEYBOARD_LL`).
//!
//! The hook callback must be `extern "system"` with no captures, so shared
//! state lives in statics. The callback does only atomic stores + an
//! empty-payload `emit` to stay well under the `LowLevelHooksTimeout` (~300 ms),
//! and ALWAYS calls `CallNextHookEx` so the key is never consumed.

use super::{config_key_to_vk, HookContext, KeyboardHook};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static RECORDING_ACTIVE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static TARGET_VK: AtomicU32 = AtomicU32::new(0xA3); // VK_RCONTROL default
static KEY_DOWN: AtomicBool = AtomicBool::new(false); // debounce auto-repeat

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if kb.vkCode == TARGET_VK.load(Ordering::Relaxed) {
            match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    // Ignore OS auto-repeat: only the first down edge counts.
                    if !KEY_DOWN.swap(true, Ordering::SeqCst) {
                        if let Some(active) = RECORDING_ACTIVE.get() {
                            active.store(true, Ordering::SeqCst);
                        }
                        if let Some(app) = APP_HANDLE.get() {
                            let _ = app.emit("hold-start", ());
                        }
                    }
                }
                WM_KEYUP | WM_SYSKEYUP => {
                    KEY_DOWN.store(false, Ordering::SeqCst);
                    if let Some(active) = RECORDING_ACTIVE.get() {
                        active.store(false, Ordering::SeqCst);
                    }
                    if let Some(app) = APP_HANDLE.get() {
                        let _ = app.emit("hold-stop", ());
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
        let _ = APP_HANDLE.set(ctx.app);
        let _ = RECORDING_ACTIVE.set(ctx.recording_active);
        let vk = config_key_to_vk(&ctx.target_key)
            .ok_or_else(|| anyhow::anyhow!("unsupported hold key: {}", ctx.target_key))?;
        TARGET_VK.store(vk, Ordering::Relaxed);

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

// Silence "field never read" — thread_id is read by Drop.
#[allow(dead_code)]
fn _assert_uses_hhook(_: HHOOK) {}
```

- [ ] **Step 2: `cargo check` and fix any windows-rs 0.58 signature mismatches**

Run (from `src-tauri/`): `cargo check`
Expected: clean. **If it fails on nullable-handle args**, apply these known 0.58 alternatives and re-check:
- `GetMessageW(&mut msg, None, 0, 0)` → `GetMessageW(&mut msg, windows::Win32::Foundation::HWND::default(), 0, 0)`
- `CallNextHookEx(None, ...)` → `CallNextHookEx(HHOOK::default(), ...)`
- `HINSTANCE(hmod.0)` → if `HMODULE`/`HINSTANCE` are pointer-wrapped, use `core::mem::transmute(hmod)` or `HINSTANCE(hmod.0 as *mut _)`.
Document whichever form compiles in a one-line comment.

- [ ] **Step 3: Run the full test suite (maps still green, nothing else broke)**

Run (from `src-tauri/`): `cargo test`
Expected: all tests pass (no new unit tests here — hook is runtime-verified).

- [ ] **Step 4: Commit (trait + Windows impl together — tree now compiles)**

```bash
git add src-tauri/src/hook/mod.rs src-tauri/src/hook/windows.rs
git commit -m "feat(hook): KeyboardHook trait + Windows WH_KEYBOARD_LL implementation"
```

---

## Task 5: macOS hook implementation (`CGEventTap`)

**Written here; compiled and runtime-verified on a Mac** (cannot be checked on Windows). Default hold key (Right Ctrl) is a modifier, delivered via `flagsChanged`; F13-F15 arrive via `keyDown`/`keyUp`, so the tap watches all three.

**Files:**
- Create: `src-tauri/src/hook/macos.rs`

- [ ] **Step 1: Write the macOS hook**

Create `src-tauri/src/hook/macos.rs`:

```rust
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
```

- [ ] **Step 2 (on a Mac only): `cargo check` and reconcile core-graphics API**

Run (from `src-tauri/`, on macOS): `cargo check`
Expected: clean. **If `core-graphics` 0.24 differs**, the likely adjustments are the `CGEventTap::new` closure arity, `tap.mach_port.create_runloop_source` path, or `kCFRunLoopCommonModes` import. Reconcile against the installed crate docs and keep the press/release semantics identical.

- [ ] **Step 3: Commit (compiles on Windows because macos.rs is cfg-gated out)**

Run (from `src-tauri/`): `cargo check` (on Windows — confirms macos.rs is excluded and nothing regressed)
Expected: clean.

```bash
git add src-tauri/src/hook/macos.rs
git commit -m "feat(hook): macOS CGEventTap implementation (Mac-verified)"
```

---

## Task 6: Capture-loop stop predicate (TDD) + `Activation` state

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test for `should_continue`**

Add to the bottom of `src-tauri/src/lib.rs` (create a `#[cfg(test)] mod tests` block if absent):

```rust
#[cfg(test)]
mod tests {
    use super::should_continue;

    #[test]
    fn continues_while_active_and_under_cap() {
        assert!(should_continue(true, 0));
        assert!(should_continue(true, 29));
    }

    #[test]
    fn stops_on_release() {
        assert!(!should_continue(false, 0));
    }

    #[test]
    fn stops_at_safety_cap() {
        assert!(!should_continue(true, 30));
        assert!(!should_continue(true, 45));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `src-tauri/`): `cargo test --lib tests::`
Expected: FAIL — `should_continue` not found.

- [ ] **Step 3: Implement `should_continue` and the `Activation` state**

Add near the top of `src-tauri/src/lib.rs` (after the `use` lines), introducing the atomic imports:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the capture loop should keep recording: active (key still held)
/// AND under the 30 s safety cap.
fn should_continue(active: bool, elapsed_secs: u64) -> bool {
    active && elapsed_secs < 30
}

/// Managed activation state: the shared stop flag + the installed hook.
struct Activation {
    recording_active: Arc<AtomicBool>,
    /// Kept alive for the app's lifetime; `rearm` swaps the key live.
    #[allow(dead_code)]
    hook: Mutex<Option<hook::PlatformHook>>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run (from `src-tauri/`): `cargo test --lib tests::`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: should_continue stop predicate + Activation state"
```

---

## Task 7: Rewrite the capture loop to poll the stop flag

**Files:**
- Modify: `src-tauri/src/lib.rs` (the `do_pipeline` capture block + signature)

- [ ] **Step 1: Add the stop flag parameter to `do_pipeline`**

Change the `do_pipeline` signature (currently lines 99-105) to accept the flag:

```rust
async fn do_pipeline(
    app_state: &AppState,
    app_handle: &tauri::AppHandle,
    recording_active: Arc<AtomicBool>,
    local: Arc<Mutex<Option<transcription::local::LocalWhisper>>>,
    cloud: Arc<Mutex<Option<transcription::cloud::CloudWhisper>>>,
    use_cloud: bool,
) -> Result<String, String> {
```

- [ ] **Step 2: Replace the inline "recording" state set with `do_set_state`**

Replace the manual emit + tray-icon block (currently lines 114-121) with a single call so the pill shows on start:

```rust
    // 1b. Enter Recording (emits state-changed, swaps tray icon, shows pill).
    do_set_state("recording", app_state, app_handle)?;
```

- [ ] **Step 3: Replace the VAD-driven capture loop with flag polling**

Replace the `spawn_blocking` capture block (currently lines 124-148) with:

```rust
    // 2. Capture audio in a blocking thread (cpal::Stream is !Send).
    //    Stop when the key is released (recording_active clears) or at 30 s.
    let active = recording_active.clone();
    let accumulated = tokio::task::spawn_blocking(move || {
        use audio::AudioCapture;
        let capture = AudioCapture::start().map_err(|e| e.to_string())?;
        let mut accumulated: Vec<f32> = Vec::new();
        let start = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(32));
            let chunk = capture.drain();
            if !chunk.is_empty() {
                accumulated.extend_from_slice(&chunk);
            }
            if !should_continue(active.load(Ordering::SeqCst), start.elapsed().as_secs()) {
                break;
            }
        }
        drop(capture);
        Ok::<Vec<f32>, String>(accumulated)
    })
    .await
    .map_err(|e| e.to_string())??;
```

> The `EnergyVad` import is dropped from this function. `audio::EnergyVad` stays compiled and used by its own unit tests — leave the type in place.

- [ ] **Step 4: Update the one existing caller `run_pipeline`**

In `run_pipeline` (currently lines 184-198), thread the flag through from `Activation`:

```rust
#[tauri::command]
async fn run_pipeline(
    app_state: tauri::State<'_, AppState>,
    whisper_state: tauri::State<'_, WhisperState>,
    activation: tauri::State<'_, Activation>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let local = whisper_state.local.clone();
    let cloud = whisper_state.cloud.clone();
    let recording_active = activation.recording_active.clone();
    let use_cloud = app_state.config.lock().unwrap().use_cloud;
    // Manual invoke path: behave like a tap (record until the safety cap),
    // since no physical key is being held. Pre-set the flag true.
    recording_active.store(true, Ordering::SeqCst);
    let outcome = do_pipeline(
        &app_state,
        &app_handle,
        recording_active.clone(),
        local,
        cloud,
        use_cloud,
    )
    .await;
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
        do_set_state("idle", &app_state, &app_handle).ok();
    }
    outcome
}
```

- [ ] **Step 5: Verify compile + tests**

Run (from `src-tauri/`): `cargo test`
Expected: compiles; all tests pass. (`run_pipeline` now needs `Activation` managed — added in Task 9. If `cargo check` errors on the missing managed state, proceed to Task 9 before running the app; unit tests still pass.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: hold-to-release capture loop driven by recording_active flag"
```

---

## Task 8: Pill overlay show/hide in `do_set_state`

**Files:**
- Modify: `src-tauri/src/lib.rs` (`do_set_state`)

- [ ] **Step 1: Add pill show/hide to `do_set_state`**

In `do_set_state`, after the tray-icon block and before `Ok(())` (currently around line 44), add:

```rust
    // Show the pill while busy, hide it when idle/error.
    if let Some(pill) = app_handle.get_webview_window("pill") {
        match next {
            RecordingState::Recording | RecordingState::Transcribing => {
                let _ = pill.show();
            }
            _ => {
                let _ = pill.hide();
            }
        }
    }
```

- [ ] **Step 2: Verify compile**

Run (from `src-tauri/`): `cargo check`
Expected: clean (modulo the Task 9 managed-state note).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: show/hide pill overlay in do_set_state"
```

---

## Task 9: Activation rewrite in `run()` — install hook, remove plugin activation, add `hold-start` listener

**Files:**
- Modify: `src-tauri/src/lib.rs` (`run()` builder + setup), `src-tauri/src/config.rs` (default key), `src-tauri/src/hotkey.rs` (dead-code allow)

- [ ] **Step 1: Update the default hold key to a bare modifier**

In `src-tauri/src/config.rs`, change the `hold_hotkey` default (currently line 28) and refresh the comment:

```rust
            // Hold-to-talk default: bare Right Ctrl. Activated via a low-level
            // keyboard hook (Win WH_KEYBOARD_LL / macOS CGEventTap), not the
            // global-shortcut plugin (which can't register a bare modifier).
            hold_hotkey: "RControl".into(),
            // reserved: toggle mode — kept in schema, not wired this phase.
            toggle_hotkey: "F13".into(),
```

- [ ] **Step 2: Silence dead code in the now-unused plugin hotkey parser**

Add as the **first line** of `src-tauri/src/hotkey.rs`:

```rust
#![allow(dead_code)] // off the call path since Phase 2.5 (kept for reference / tests)
```

- [ ] **Step 3: Add the `run_hold_pipeline` helper**

In `src-tauri/src/lib.rs`, add above `run()`:

```rust
/// Run one dictation cycle triggered by the hold-to-talk hook.
/// Pulls managed state from the handle so it can be spawned from an event listener.
async fn run_hold_pipeline(app: tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let whisper = app.state::<WhisperState>();
    let activation = app.state::<Activation>();
    let local = whisper.local.clone();
    let cloud = whisper.cloud.clone();
    let recording_active = activation.recording_active.clone();
    let use_cloud = app_state.config.lock().unwrap().use_cloud;

    let outcome = do_pipeline(
        app_state.inner(),
        &app,
        recording_active.clone(),
        local,
        cloud,
        use_cloud,
    )
    .await;

    // Never leave the flag stuck true (e.g. after the 30 s cap while still held).
    recording_active.store(false, Ordering::SeqCst);
    if outcome.is_err() {
        do_set_state("idle", app_state.inner(), &app).ok();
    }
}
```

- [ ] **Step 4: Remove the global-shortcut plugin activation handler**

In `run()`, delete the entire `.plugin(tauri_plugin_global_shortcut::Builder::new()...build())` block (currently lines 204-213). Leave the dependency in `Cargo.toml`.

- [ ] **Step 5: Manage the `Activation` state**

In `run()`, add a `.manage(Activation { ... })` call alongside the other `.manage(...)` calls:

```rust
        .manage(Activation {
            recording_active: Arc::new(AtomicBool::new(false)),
            hook: Mutex::new(None),
        })
```

- [ ] **Step 6: Replace hotkey registration in `setup` with hook install + listener**

In the `setup` closure, delete the `use tauri_plugin_global_shortcut::GlobalShortcutExt;` line and the entire hotkey-registration block (currently lines 298-312). In its place add:

```rust
            // Install the low-level keyboard hook for hold-to-talk.
            // Non-fatal on failure (matches the startup philosophy): a missing
            // Accessibility grant (macOS) or hook error just disables hold-to-talk.
            let hold_key = {
                let cfg = app.state::<AppState>().config.lock().unwrap();
                if hook::config_key_to_vk(&cfg.hold_hotkey).is_some() {
                    cfg.hold_hotkey.clone()
                } else {
                    "RControl".to_string() // migrate legacy combo configs
                }
            };
            let recording_active = app.state::<Activation>().recording_active.clone();
            match hook::PlatformHook::install(HookContext {
                app: app.handle().clone(),
                recording_active,
                target_key: hold_key,
            }) {
                Ok(installed) => {
                    *app.state::<Activation>().hook.lock().unwrap() = Some(installed);
                }
                Err(e) => eprintln!("warning: keyboard hook not installed: {e}"),
            }

            // Drive the pipeline directly from Rust on hold-start (no JS hop).
            let pipeline_handle = app.handle().clone();
            app.listen("hold-start", move |_event| {
                let h = pipeline_handle.clone();
                tauri::async_runtime::spawn(run_hold_pipeline(h));
            });
```

- [ ] **Step 7: Add the `Listener` and hook trait imports**

At the top of `src-tauri/src/lib.rs`, extend the tauri use to include `Listener`, and bring the hook trait/context into scope (needed for `PlatformHook::install` here and `rearm` in Task 10):

```rust
use tauri::{Emitter, Listener, Manager};
use hook::{HookContext, KeyboardHook};
```

- [ ] **Step 8: Verify compile + tests**

Run (from `src-tauri/`): `cargo test`
Expected: compiles; all unit tests pass.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/config.rs src-tauri/src/hotkey.rs
git commit -m "feat: install keyboard hook + Rust-side hold-start pipeline; remove plugin activation"
```

---

## Task 10: Live re-arm on `save_config`

**Files:**
- Modify: `src-tauri/src/lib.rs` (`save_config`)

- [ ] **Step 1: Add `Activation` param and re-arm after persist**

Change `save_config` to accept `Activation` and re-arm the hook live. Update the signature (currently lines 65-71) and add the re-arm after the in-memory update (before `Ok(())`):

```rust
#[tauri::command]
fn save_config(
    new_config: config::Config,
    app_state: tauri::State<AppState>,
    whisper_state: tauri::State<WhisperState>,
    activation: tauri::State<Activation>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
```

Add before the closing `Ok(())`:

```rust
    // Re-arm the hook to the (possibly new) hold key — no restart needed.
    {
        let key = app_state.config.lock().unwrap().hold_hotkey.clone();
        if let Some(hook) = activation.hook.lock().unwrap().as_ref() {
            hook.rearm(&key);
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Verify compile + tests**

Run (from `src-tauri/`): `cargo test`
Expected: compiles; tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: re-arm keyboard hook live on save_config"
```

---

## Task 11: Editable hotkey capture in Settings

**Files:**
- Modify: `src/components/Settings.tsx`

- [ ] **Step 1: Add the key map and capture state**

In `src/components/Settings.tsx`, add above the `Settings` component:

```tsx
// KeyboardEvent.code → Lectus config key string.
// MUST stay in sync with config_key_to_vk in src-tauri/src/hook/mod.rs.
const CODE_TO_KEY: Record<string, string> = {
  ControlRight: 'RControl',
  ControlLeft: 'LControl',
  ShiftRight: 'RShift',
  ShiftLeft: 'LShift',
  AltRight: 'RAlt',
  AltLeft: 'LAlt',
  F13: 'F13',
  F14: 'F14',
  F15: 'F15',
};
```

- [ ] **Step 2: Add capture state inside the component**

After the existing `const [status, setStatus] = useState('');` line, add:

```tsx
  const [capturing, setCapturing] = useState(false);
```

- [ ] **Step 3: Replace the read-only hotkey field with a click-to-capture control**

Replace the entire "Dictation hotkey" block (currently lines 73-82) with:

```tsx
      <div style={{ marginBottom: 16 }}>
        <label style={{ display: 'block', fontSize: 13, marginBottom: 4 }}>Dictation hotkey</label>
        <button
          type="button"
          onClick={() => { setCapturing(true); setStatus('Press a key…'); }}
          onKeyDown={(e) => {
            if (!capturing) return;
            e.preventDefault();
            const mapped = CODE_TO_KEY[e.code];
            setCapturing(false);
            if (!mapped) {
              setStatus(`Unsupported key (${e.code}). Try Right Ctrl, Shift, Alt, or F13-F15.`);
              return;
            }
            update({ hold_hotkey: mapped });
            setStatus(`Captured: ${mapped} — click Save`);
          }}
          style={{
            width: '100%',
            padding: 6,
            textAlign: 'left',
            background: capturing ? '#fffbe6' : '#fff',
            border: '1px solid #ccc',
            cursor: 'pointer',
          }}
        >
          {capturing ? 'Press a key…' : config.hold_hotkey}
        </button>
        <small style={{ color: '#888' }}>
          Click, then press your hold-to-talk key (hold to talk, release to stop).
        </small>
      </div>
```

- [ ] **Step 4: Verify the frontend builds**

Run (from repo root): `npm run build`
Expected: TypeScript compiles, Vite build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/components/Settings.tsx
git commit -m "feat(ui): click-to-capture editable hold-to-talk hotkey"
```

---

## Task 12: Remove frontend activation listener

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Delete the `pipeline-start` → `invoke` effect**

In `src/App.tsx`, remove the second `useEffect` (currently lines 21-30) that listens for `pipeline-start` and calls `invoke('run_pipeline')`. The pipeline is now Rust-driven.

- [ ] **Step 2: Remove the now-unused `invoke` import**

Delete `import { invoke } from '@tauri-apps/api/core';` (line 4). Keep the `state-changed` listener (it drives the pill).

Resulting `App.tsx` top:

```tsx
import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { Pill } from './components/Pill';
import { Settings } from './components/Settings';
```

- [ ] **Step 3: Verify the frontend builds (no unused-import errors)**

Run (from repo root): `npm run build`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "refactor(ui): drop frontend pipeline-start activation (now Rust-driven)"
```

---

## Task 13: Reliable paste injection (clipboard read-back confirmation)

**Files:**
- Modify: `src-tauri/src/injection/mod.rs`

- [ ] **Step 1: Write the failing test for the verification predicate**

Append to `src-tauri/src/injection/mod.rs` a test module:

```rust
#[cfg(test)]
mod tests {
    use super::clipboard_matches;

    #[test]
    fn matches_when_readback_equals_expected() {
        assert!(clipboard_matches("hello world", Some("hello world".to_string())));
    }

    #[test]
    fn no_match_when_stale_or_empty() {
        assert!(!clipboard_matches("hello world", Some("old text".to_string())));
        assert!(!clipboard_matches("hello world", None));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `src-tauri/`): `cargo test --lib injection::mod::tests`
Expected: FAIL — `clipboard_matches` not found.

- [ ] **Step 3: Implement read-back confirmation and the predicate**

Replace the body of `src-tauri/src/injection/mod.rs` `inject_text` (currently lines 8-26) and add the helper:

```rust
/// Whether the clipboard read-back equals the text we intended to paste.
fn clipboard_matches(expected: &str, readback: Option<String>) -> bool {
    readback.as_deref() == Some(expected)
}

/// Inject text into the currently focused field.
/// Strategy: save prior clipboard → set ours → confirm the OS accepted it via
/// read-back (bounded retry) → simulate paste → restore the prior clipboard.
/// Confirming the write before pasting fixes slow/deferred-read apps (Electron),
/// replacing the old fixed 50 ms guess.
pub fn inject_text(text: &str) -> Result<()> {
    let previous = {
        let mut cb = arboard::Clipboard::new()?;
        cb.get_text().ok()
    };

    clipboard::set_clipboard(text)?;

    // Confirm the clipboard actually holds our text before pasting (≤ ~150 ms).
    let mut confirmed = false;
    for _ in 0..15 {
        let readback = arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_text().ok());
        if clipboard_matches(text, readback) {
            confirmed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !confirmed {
        // Proceed anyway — better to attempt the paste than to drop the text.
        eprintln!("warning: clipboard read-back not confirmed before paste");
    }

    clipboard::simulate_paste()?;

    // Give the target app time to consume the paste, then restore the prior clipboard.
    if let Some(prev) = previous {
        std::thread::sleep(std::time::Duration::from_millis(200));
        clipboard::set_clipboard(&prev).ok();
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run (from `src-tauri/`): `cargo test --lib injection`
Expected: PASS (2 new tests + existing clipboard test).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/injection/mod.rs
git commit -m "fix(injection): confirm clipboard write via read-back before paste"
```

---

## Task 14: Eclectus icons (app + tray)

**Files:**
- Create: `scripts/gen_tray_icons.ps1`
- Modify: `src-tauri/tauri.conf.json` (bundle tray icons as resources)
- Replace: `src-tauri/icons/*` (generated)

- [ ] **Step 1: Generate the app icon bundle from the master**

Run (from repo root): `npm run tauri icon src-tauri/icons/lectus-master.png`
Expected: regenerates `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`, `icon.png`, and the `Square*`/`StoreLogo` set.

- [ ] **Step 2: Create the tray-icon generation script**

Create `scripts/gen_tray_icons.ps1` (System.Drawing — no extra deps; tints state via a corner status dot):

```powershell
# Generates tray-state PNGs from the Eclectus master logo.
# tray-idle = 64x64 logo; recording = + red dot; transcribing = + amber dot.
Add-Type -AssemblyName System.Drawing
$icons = Join-Path $PSScriptRoot '..\src-tauri\icons'
$master = Join-Path $icons 'lectus-master.png'
$src = [System.Drawing.Image]::FromFile($master)

function Save-Tray($name, $dotColor) {
    $bmp = New-Object System.Drawing.Bitmap 64, 64
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($src, 0, 0, 64, 64)
    if ($dotColor) {
        $brush = New-Object System.Drawing.SolidBrush $dotColor
        $g.FillEllipse($brush, 40, 40, 22, 22)
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), 3
        $g.DrawEllipse($pen, 40, 40, 22, 22)
        $brush.Dispose(); $pen.Dispose()
    }
    $bmp.Save((Join-Path $icons $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

Save-Tray 'tray-idle.png' $null
Save-Tray 'tray-recording.png' ([System.Drawing.Color]::FromArgb(255, 230, 50, 50))
Save-Tray 'tray-transcribing.png' ([System.Drawing.Color]::FromArgb(255, 245, 170, 30))
$src.Dispose()
Write-Host 'Tray icons generated.'
```

- [ ] **Step 3: Run the tray-icon script**

Run (from repo root): `powershell -ExecutionPolicy Bypass -File scripts/gen_tray_icons.ps1`
Expected: `tray-idle.png`, `tray-recording.png`, `tray-transcribing.png` written to `src-tauri/icons/`.

> On macOS, regenerate the three tray PNGs with any image tool (64×64, transparent, red/amber status dot for recording/transcribing). The PowerShell script is Windows-only.

- [ ] **Step 4: Bundle the tray icons as resources so they resolve at runtime**

In `src-tauri/tauri.conf.json`, change the `bundle.resources` object (currently lines 53-55) to include the tray icons:

```json
    "resources": {
      "../models/ggml-tiny.en.bin": "models/ggml-tiny.en.bin",
      "icons/tray-idle.png": "icons/tray-idle.png",
      "icons/tray-recording.png": "icons/tray-recording.png",
      "icons/tray-transcribing.png": "icons/tray-transcribing.png"
    }
```

- [ ] **Step 5: Verify the dev build launches with branded icons**

Run (from repo root): `npm run tauri dev`
Expected: app launches; tray shows the Eclectus art (idle). Close after confirming (full hold-to-talk verification is the manual gate below).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/icons scripts/gen_tray_icons.ps1 src-tauri/tauri.conf.json
git commit -m "feat: Eclectus-parrot app + tray icons; bundle tray icons as resources"
```

---

## Task 15: Version bump to 0.3.0

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json`

- [ ] **Step 1: Bump all three version strings**

- `src-tauri/tauri.conf.json` line 4: `"version": "0.3.0"`
- `src-tauri/Cargo.toml` line 3: `version = "0.3.0"`
- `package.json`: set `"version": "0.3.0"`

- [ ] **Step 2: Verify build metadata**

Run (from `src-tauri/`): `cargo build`
Expected: compiles at the new version.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml package.json src-tauri/Cargo.lock
git commit -m "chore: bump version to 0.3.0"
```

> **Do NOT tag `v0.3.0` or merge to master yet.** Tagging happens only after the manual verification gate below passes.

---

## Verification

### Automated (run before the manual gate)

- [ ] Unit tests: from `src-tauri/`, `cargo test` — key maps (`config_key_to_vk` / `config_key_to_mackey`), `should_continue`, `clipboard_matches`, plus existing 17 tests, all green.
- [ ] Frontend: from repo root, `npm run build` — clean TypeScript + Vite build.
- [ ] Release build sanity: from repo root, `npm run tauri build` — produces an installer without console errors.

### Manual runtime gate (PAUSE here for the user — Windows + macOS)

1. **Hold-to-talk:** Hold Right Ctrl → pill shows "Listening…" → speak → release → "Transcribing…" → text injected → pill hides. Release-to-stop feels instant (<100 ms).
2. **Modifier pass-through:** Right Ctrl still works as a normal modifier (Ctrl+C copies) — the hook never consumes the key.
3. **Tap-and-instant-release:** quick tap → no crash, returns to Idle.
4. **Safety cap:** hold > 30 s → recording stops at the cap, transcribes what was captured.
5. **Editable hotkey:** Settings → click the hotkey button → press F13 → Save → F13 now activates live; Right Ctrl no longer does (no restart).
6. **Error path:** bad cloud key or mic failure → returns to Idle; next press still works (flag not stuck).
7. **Paste reliability:** dictate into a slow app (heavy web editor / Electron) → text lands intact.
8. **macOS Accessibility:** first run warns if Accessibility is not granted; after granting + restart, hold-to-talk works.
9. **Release binary:** the `--release` build (with `windows_subsystem="windows"`) runs the hook + pump without a console window.
10. **Branding:** tray shows Eclectus art in idle/recording/transcribing; installer/app icon is branded.

### After the gate passes (do these only on user go-ahead)

- [ ] Tag: `git tag v0.3.0`
- [ ] Merge `feat/phase2.5-hold-to-talk` → `master`
- [ ] Push: `git push origin master --tags`

---

## Notes & Risks

- **Global keyboard hook = AV/privacy optics** (same API family as keyloggers). Lectus acts only on the configured target key and never logs other keys — document this in the README/onboarding.
- **Windows hook callback must stay fast** (`LowLevelHooksTimeout` ~300 ms or the OS silently drops the hook): the callback does only atomic stores + an empty-payload `emit`.
- **windows-rs 0.58 signatures** for `SetWindowsHookExW` / `GetMessageW` / `CallNextHookEx` / `HINSTANCE` may need the nullable-handle adjustments listed in Task 4 Step 2 — driven by `cargo check`.
- **macOS Accessibility permission** is mandatory and cannot be auto-granted; the first-run path warns and disables hold-to-talk gracefully until granted.
- **macOS `core-graphics` 0.24 API** (`CGEventTap::new` closure shape, runloop-source creation) is verified on the Mac in Task 5 Step 2; keep press/release semantics identical when reconciling.
- **`tauri-plugin-global-shortcut`** stays in `Cargo.toml` (harmless) and `hotkey.rs` stays compiled under `#![allow(dead_code)]` for easy revival of combo hotkeys later.
- **`toggle_hotkey`** remains in the `Config` schema (don't break persisted configs) but is intentionally unwired this phase.
```
