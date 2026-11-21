//! Main-computer side of Lectus Link (Windows): low-level mouse and keyboard hooks on their own
//! message-pump thread. While the other computer is active the mouse is swallowed and forwarded; the cursor stays parked
//! in the middle of the primary screen and the motion is read as its offset from there.
//!
//! The hook callbacks only update atomics and queue a message, to stay far below the
//! `LowLevelHooksTimeout`. This module never touches the dictation keyboard hook.
use super::link::{Edge, Msg, Session, PING_EVERY, SILENCE_LIMIT};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_SCROLL;
use windows::Win32::UI::WindowsAndMessaging::{LLKHF_EXTENDED, LLKHF_INJECTED};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, GetSystemMetrics,
    PostThreadMessageW, SetCursorPos, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    HC_ACTION, KBDLLHOOKSTRUCT, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, SM_CXSCREEN,
    SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

/// The cursor must push against the edge this long before the other computer takes over.
const EDGE_DWELL_MS: u64 = 50;

static OUT: Mutex<Option<Sender<Msg>>> = Mutex::new(None);
static LINKED: AtomicBool = AtomicBool::new(false);
static REMOTE: AtomicBool = AtomicBool::new(false);
static EDGE_SINCE: AtomicU64 = AtomicU64::new(0);
static PARK_X: AtomicI32 = AtomicI32::new(0);
static PARK_Y: AtomicI32 = AtomicI32::new(0);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
/// Keys whose key-down went to the Mac (bit = scan code | 0x100 if extended). Their key-up
/// follows; a key-up for a key pressed before the mouse crossed goes to the PC instead.
static FORWARDED: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
/// Only the newest session may clear the shared sender when it ends.
static SESSION: AtomicU64 = AtomicU64::new(0);
static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
/// Where the other computer is: 0 left, 1 right, 2 above, 3 below.
static SIDE: AtomicU8 = AtomicU8::new(0);
type AckSender = std::sync::mpsc::SyncSender<Option<String>>;
static PENDING: Mutex<Option<HashMap<[u8; 16], AckSender>>> = Mutex::new(None);

pub fn set_side(edge: Edge) {
    SIDE.store(edge as u8, Ordering::SeqCst);
}

fn side() -> Edge {
    match SIDE.load(Ordering::SeqCst) {
        1 => Edge::Right,
        2 => Edge::Top,
        3 => Edge::Bottom,
        _ => Edge::Left,
    }
}

fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

fn queue(msg: Msg) {
    if let Some(tx) = OUT.lock().unwrap().as_ref() {
        let _ = tx.send(msg);
    }
}

struct Virtual {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn virtual_screen() -> Virtual {
    unsafe {
        Virtual {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        }
    }
}

fn at_edge(x: i32, y: i32) -> bool {
    let v = virtual_screen();
    match side() {
        Edge::Left => x <= v.x,
        Edge::Right => x >= v.x + v.w - 1,
        Edge::Top => y <= v.y,
        Edge::Bottom => y >= v.y + v.h - 1,
    }
}

/// Hands the mouse and keyboard to the other computer. Called on the hook thread.
fn go_remote(x: i32, y: i32) {
    let v = virtual_screen();
    let frac = match side() {
        Edge::Left | Edge::Right => (y - v.y) as f64 / (v.h - 1).max(1) as f64,
        Edge::Top | Edge::Bottom => (x - v.x) as f64 / (v.w - 1).max(1) as f64,
    }
    .clamp(0.0, 1.0);
    let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXSCREEN) / 2, GetSystemMetrics(SM_CYSCREEN) / 2) };
    PARK_X.store(cx, Ordering::SeqCst);
    PARK_Y.store(cy, Ordering::SeqCst);
    REMOTE.store(true, Ordering::SeqCst);
    unsafe {
        let _ = SetCursorPos(cx, cy);
    }
    log::info!("share: mouse to the other computer at {frac:.3}");
    queue(Msg::Enter { e: side().opposite(), at: frac });
}

/// Gives the mouse back to this PC: on the shared edge at `frac` along it, or parked (None).
fn come_back(frac: Option<f64>) {
    if !REMOTE.swap(false, Ordering::SeqCst) {
        return;
    }
    // The Mac releases what it holds; later key-ups belong to the PC again.
    FORWARDED.iter().for_each(|w| w.store(0, Ordering::SeqCst));
    EDGE_SINCE.store(0, Ordering::SeqCst);
    let v = virtual_screen();
    unsafe {
        match frac {
            Some(f) => {
                let f = f.clamp(0.0, 1.0);
                let along_y = v.y + (f * (v.h - 1) as f64).round() as i32;
                let along_x = v.x + (f * (v.w - 1) as f64).round() as i32;
                let (x, y) = match side() {
                    Edge::Left => (v.x + 2, along_y),
                    Edge::Right => (v.x + v.w - 3, along_y),
                    Edge::Top => (along_x, v.y + 2),
                    Edge::Bottom => (along_x, v.y + v.h - 3),
                };
                let _ = SetCursorPos(x, y);
            }
            None => {
                let _ = SetCursorPos(PARK_X.load(Ordering::SeqCst), PARK_Y.load(Ordering::SeqCst));
            }
        }
    }
    log::info!("share: mouse back on the PC");
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let injected = ms.flags & LLMHF_INJECTED != 0;
        let kind = wparam.0 as u32;
        if REMOTE.load(Ordering::SeqCst) && !injected {
            let msg = match kind {
                WM_MOUSEMOVE => {
                    let dx = ms.pt.x - PARK_X.load(Ordering::Relaxed);
                    let dy = ms.pt.y - PARK_Y.load(Ordering::Relaxed);
                    (dx != 0 || dy != 0).then_some(Msg::Move { dx, dy })
                }
                WM_LBUTTONDOWN => Some(Msg::Button { b: 0, down: true }),
                WM_LBUTTONUP => Some(Msg::Button { b: 0, down: false }),
                WM_RBUTTONDOWN => Some(Msg::Button { b: 1, down: true }),
                WM_RBUTTONUP => Some(Msg::Button { b: 1, down: false }),
                WM_MBUTTONDOWN => Some(Msg::Button { b: 2, down: true }),
                WM_MBUTTONUP => Some(Msg::Button { b: 2, down: false }),
                WM_MOUSEWHEEL => Some(Msg::Wheel { dx: 0, dy: (ms.mouseData >> 16) as i16 as i32 }),
                WM_MOUSEHWHEEL => Some(Msg::Wheel { dx: (ms.mouseData >> 16) as i16 as i32, dy: 0 }),
                WM_XBUTTONDOWN | WM_XBUTTONUP => None,
                _ => None,
            };
            if let Some(msg) = msg {
                queue(msg);
            }
            return LRESULT(1); // swallowed: the PC cursor stays parked
        }
        if kind == WM_MOUSEMOVE && !injected && LINKED.load(Ordering::SeqCst) {
            if at_edge(ms.pt.x, ms.pt.y) {
                let since = EDGE_SINCE.load(Ordering::SeqCst);
                let now = now_ms();
                if since == 0 {
                    EDGE_SINCE.store(now, Ordering::SeqCst);
                } else if now - since >= EDGE_DWELL_MS {
                    EDGE_SINCE.store(0, Ordering::SeqCst);
                    go_remote(ms.pt.x, ms.pt.y);
                    return LRESULT(1);
                }
            } else {
                EDGE_SINCE.store(0, Ordering::SeqCst);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Sets or clears a key's bit; returns whether it was set before.
fn forwarded(slot: usize, set: bool) -> bool {
    let (word, bit) = (&FORWARDED[slot / 64], 1u64 << (slot % 64));
    let before = if set {
        word.fetch_or(bit, Ordering::SeqCst)
    } else {
        word.fetch_and(!bit, Ordering::SeqCst)
    };
    before & bit != 0
}

/// While the Mac is active every key goes to the Mac, except: Scroll Lock (takes the mouse
/// back), the dictation key (stays with dictation on this PC), injected keys, and key-ups of
/// keys that were already down before the mouse crossed.
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && REMOTE.load(Ordering::SeqCst) {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        if kb.vkCode == VK_SCROLL.0 as u32 {
            if down {
                queue(Msg::Leave);
                come_back(None);
            }
            return LRESULT(1);
        }
        let (d1, d2) = crate::hook::dictation_vks();
        let injected = kb.flags.0 & LLKHF_INJECTED.0 != 0;
        if injected || kb.vkCode == d1 || (d2 != 0 && kb.vkCode == d2) {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        if kb.scanCode & 0x200 != 0 {
            return LRESULT(1); // the fake left Ctrl that AltGr adds
        }
        let extended = kb.flags.0 & LLKHF_EXTENDED.0 != 0;
        let scan = (kb.scanCode & 0xFF) as u16;
        let slot = scan as usize | if extended { 0x100 } else { 0 };
        if down {
            forwarded(slot, true);
        } else if !forwarded(slot, false) {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        queue(Msg::Key { s: scan, e: extended, down });
        return LRESULT(1);
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Sends a finished dictation over the live link and waits for the other computer to confirm
/// the insertion. Never retried: once sent, a missing answer may still mean it was inserted.
pub fn send_text(text: &str) -> Result<()> {
    let id = crate::remote_dictation::transport::random::<16>()?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    PENDING.lock().unwrap().get_or_insert_with(HashMap::new).insert(id, tx);
    let queued = match OUT.lock().unwrap().as_ref() {
        Some(out) => out.send(Msg::Text { id, text: text.to_owned() }).is_ok(),
        None => false,
    };
    let result = if !queued {
        Err(anyhow!("The other computer is out of reach. Your text is saved in Dictations."))
    } else {
        match rx.recv_timeout(Duration::from_secs(6)) {
            Ok(None) => Ok(()),
            Ok(Some(error)) => Err(anyhow!("Other computer: {error}. Your text is saved in Dictations.")),
            Err(_) => Err(anyhow!("Delivery was not confirmed; check the other computer before trying again. Your text is saved in Dictations.")),
        }
    };
    if let Some(m) = PENDING.lock().unwrap().as_mut() {
        m.remove(&id);
    }
    result
}

/// See [`super::follow`].
pub fn follow() -> Option<bool> {
    if HOOK_THREAD.load(Ordering::SeqCst) == 0 {
        return None;
    }
    Some(LINKED.load(Ordering::SeqCst) && REMOTE.load(Ordering::SeqCst))
}

/// Installs both hooks on a dedicated pump thread. Idempotent.
pub fn install() -> Result<()> {
    if HOOK_THREAD.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }
    let (tx, rx) = channel::<Result<u32, String>>();
    std::thread::spawn(move || unsafe {
        let hmod = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0);
        let keys = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0);
        let (mouse, keys) = match (mouse, keys) {
            (Ok(m), Ok(k)) => (m, k),
            (m, k) => {
                if let Ok(m) = m {
                    let _ = UnhookWindowsHookEx(m);
                }
                if let Ok(k) = k {
                    let _ = UnhookWindowsHookEx(k);
                }
                let _ = tx.send(Err("Windows refused the mouse hook".into()));
                return;
            }
        };
        let _ = tx.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(mouse);
        let _ = UnhookWindowsHookEx(keys);
    });
    let id = rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| anyhow!("The mouse hook did not start"))?
        .map_err(|e| anyhow!(e))?;
    HOOK_THREAD.store(id, Ordering::SeqCst);
    Ok(())
}

pub fn uninstall() {
    LINKED.store(false, Ordering::SeqCst);
    come_back(None);
    *OUT.lock().unwrap() = None;
    let id = HOOK_THREAD.swap(0, Ordering::SeqCst);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// Sums consecutive moves so a slow link never builds a backlog of tiny steps.
fn coalesce(first: Msg, rx: &Receiver<Msg>) -> (Msg, Option<Msg>) {
    let Msg::Move { mut dx, mut dy } = first else {
        return (first, None);
    };
    while let Ok(next) = rx.try_recv() {
        match next {
            Msg::Move { dx: x, dy: y } => {
                dx += x;
                dy += y;
            }
            other => return (Msg::Move { dx, dy }, Some(other)),
        }
    }
    (Msg::Move { dx, dy }, None)
}

/// Runs one connected session until the link drops or Link stops. Returns why it ended.
pub fn run_session(session: Session, stop: &Arc<AtomicBool>) -> String {
    let mut reader = match session.split() {
        Ok(r) => r,
        Err(e) => return e.to_string(),
    };
    let mut writer = session;
    let mine = SESSION.fetch_add(1, Ordering::SeqCst) + 1;
    let (tx, rx) = channel();
    *OUT.lock().unwrap() = Some(tx);
    LINKED.store(true, Ordering::SeqCst);
    let heard = Arc::new(AtomicU64::new(now_ms()));
    let alive = Arc::new(AtomicBool::new(true));
    {
        let heard = heard.clone();
        let alive = alive.clone();
        std::thread::spawn(move || {
            while alive.load(Ordering::SeqCst) {
                match reader.recv() {
                    Ok(msg) => {
                        heard.store(now_ms(), Ordering::SeqCst);
                        match msg {
                            Msg::Left { at } => come_back(Some(at)),
                            Msg::Ack { id, error } => {
                                let waiter = PENDING.lock().unwrap().as_mut().and_then(|m| m.remove(&id));
                                if let Some(tx) = waiter {
                                    let _ = tx.send(error);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }
            alive.store(false, Ordering::SeqCst);
        });
    }
    let mut last_ping = Instant::now() - PING_EVERY;
    let mut pending: Option<Msg> = None;
    let reason = loop {
        if stop.load(Ordering::SeqCst) {
            let _ = writer.send(Msg::Leave);
            break "Link stopped".to_string();
        }
        if !alive.load(Ordering::SeqCst) {
            break "link closed".to_string();
        }
        if now_ms() - heard.load(Ordering::SeqCst) > SILENCE_LIMIT.as_millis() as u64 {
            break "no answer for 3 s".to_string();
        }
        if last_ping.elapsed() >= PING_EVERY {
            if let Err(e) = writer.send(Msg::Ping) {
                break e.to_string();
            }
            last_ping = Instant::now();
        }
        let next = match pending.take() {
            Some(m) => m,
            None => match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(m) => m,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break "Link stopped".to_string(),
            },
        };
        let (msg, rest) = coalesce(next, &rx);
        pending = rest;
        if let Err(e) = writer.send(msg) {
            break e.to_string();
        }
    };
    alive.store(false, Ordering::SeqCst);
    if SESSION.load(Ordering::SeqCst) == mine {
        LINKED.store(false, Ordering::SeqCst);
        *OUT.lock().unwrap() = None;
        // After a drop the other computer is unreachable: show the cursor here again.
        come_back(None);
    }
    writer.close();
    reason
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_sums_moves_and_keeps_the_next_message() {
        let (tx, rx) = channel();
        tx.send(Msg::Move { dx: 2, dy: 3 }).unwrap();
        tx.send(Msg::Button { b: 0, down: true }).unwrap();
        tx.send(Msg::Move { dx: 9, dy: 9 }).unwrap();
        let (m, rest) = coalesce(Msg::Move { dx: 1, dy: 1 }, &rx);
        assert_eq!(m, Msg::Move { dx: 3, dy: 4 });
        assert_eq!(rest, Some(Msg::Button { b: 0, down: true }));
        assert_eq!(rx.try_recv().unwrap(), Msg::Move { dx: 9, dy: 9 });
    }

    #[test]
    fn coalesce_passes_other_messages_untouched() {
        let (_tx, rx) = channel::<Msg>();
        assert_eq!(coalesce(Msg::Ping, &rx), (Msg::Ping, None));
    }
}
