//! Mac side of Lectus Share: turns the PC's mouse messages into Core Graphics events.
//! Needs the Accessibility permission Lectus already asks for. M1 uses the main display only.
use super::link::{Msg, Session, PING_EVERY};
use super::keys::{KeyAction, KeyState};
use super::pointer::{Bounds, PointerAction, RemotePointer};
use core_graphics::display::CGDisplay;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// core-graphics 0.24 gates the scroll constructor behind a feature; call the C API directly.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventCreateScrollWheelEvent2(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> *mut c_void;
    fn CGEventPost(tap: u32, event: *mut c_void);
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}
const SCROLL_UNIT_LINE: u32 = 1;
const HID_EVENT_TAP: u32 = 0;

static POINTER: Mutex<Option<RemotePointer>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static KEYS: Mutex<Option<KeyState>> = Mutex::new(None);

fn main_bounds() -> Bounds {
    let r = CGDisplay::main().bounds();
    Bounds {
        x: r.origin.x,
        y: r.origin.y,
        w: r.size.width.max(2.0),
        h: r.size.height.max(2.0),
    }
}

fn cursor_location() -> Option<CGPoint> {
    let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    CGEvent::new(src).ok().map(|e| e.location())
}

fn post(action: PointerAction) {
    let source = || CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok();
    match action {
        PointerAction::Move { x, y, drag } => {
            let (kind, button) = match drag {
                None => (CGEventType::MouseMoved, CGMouseButton::Left),
                Some(0) => (CGEventType::LeftMouseDragged, CGMouseButton::Left),
                Some(1) => (CGEventType::RightMouseDragged, CGMouseButton::Right),
                Some(_) => (CGEventType::OtherMouseDragged, CGMouseButton::Center),
            };
            if let Some(src) = source() {
                if let Ok(e) = CGEvent::new_mouse_event(src, kind, CGPoint::new(x, y), button) {
                    e.post(CGEventTapLocation::HID);
                }
            }
        }
        PointerAction::Button { x, y, b, down, clicks } => {
            let (kind, button) = match (b, down) {
                (0, true) => (CGEventType::LeftMouseDown, CGMouseButton::Left),
                (0, false) => (CGEventType::LeftMouseUp, CGMouseButton::Left),
                (1, true) => (CGEventType::RightMouseDown, CGMouseButton::Right),
                (1, false) => (CGEventType::RightMouseUp, CGMouseButton::Right),
                (_, true) => (CGEventType::OtherMouseDown, CGMouseButton::Center),
                (_, false) => (CGEventType::OtherMouseUp, CGMouseButton::Center),
            };
            if let Some(src) = source() {
                if let Ok(e) = CGEvent::new_mouse_event(src, kind, CGPoint::new(x, y), button) {
                    e.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, clicks.max(1));
                    e.post(CGEventTapLocation::HID);
                }
            }
        }
        PointerAction::Scroll { y, x } => unsafe {
            let e = CGEventCreateScrollWheelEvent2(std::ptr::null(), SCROLL_UNIT_LINE, 2, y, x, 0);
            if !e.is_null() {
                CGEventPost(HID_EVENT_TAP, e);
                CFRelease(e);
            }
        },
        PointerAction::Exit { .. } => {}
    }
}

fn post_key(a: KeyAction) {
    let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
        return;
    };
    if let Ok(e) = CGEvent::new_keyboard_event(src, a.code, a.down) {
        if a.modifier {
            e.set_type(CGEventType::FlagsChanged);
        }
        e.set_flags(CGEventFlags::from_bits_truncate(a.flags));
        if a.repeat {
            e.set_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT, 1);
        }
        e.post(CGEventTapLocation::HID);
    }
}

fn release_keys() {
    let ups = KEYS
        .lock()
        .unwrap()
        .as_mut()
        .map(|k| k.release_all())
        .unwrap_or_default();
    ups.into_iter().for_each(post_key);
}

/// Lets go of every key and button Share pressed. Safe to call at any time.
pub fn release_all() {
    release_keys();
    let actions = POINTER
        .lock()
        .unwrap()
        .as_mut()
        .map(|p| p.release_all())
        .unwrap_or_default();
    actions.into_iter().for_each(post);
}

/// Runs one PC connection. A newer connection ends this one. Returns why it ended.
pub fn run_session(mut session: Session, stop: &Arc<AtomicBool>) -> String {
    let mine = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    release_all();
    POINTER
        .lock()
        .unwrap()
        .get_or_insert_with(|| RemotePointer::new(main_bounds()));
    let mut last_ping = Instant::now();
    let reason = loop {
        if stop.load(Ordering::SeqCst) {
            break "Share stopped".to_string();
        }
        if GENERATION.load(Ordering::SeqCst) != mine {
            session.close();
            return "replaced by a new connection".to_string();
        }
        let msg = match session.recv() {
            Ok(m) => m,
            Err(e) => break format!("{e:#}"),
        };
        // Answer before handling, so a long burst of keys never starves the PC's watchdog.
        if last_ping.elapsed() >= PING_EVERY {
            if let Err(e) = session.send(Msg::Ping) {
                break e.to_string();
            }
            last_ping = Instant::now();
        }
        if let Msg::Key { s, e, down } = msg {
            let action = KEYS
                .lock()
                .unwrap()
                .get_or_insert_with(KeyState::default)
                .handle(s, e, down);
            if let Some(a) = action {
                post_key(a);
            }
            continue;
        }
        if matches!(msg, Msg::Leave) {
            release_keys();
        }
        let actions = {
            let mut guard = POINTER.lock().unwrap();
            let pointer = guard.get_or_insert_with(|| RemotePointer::new(main_bounds()));
            if matches!(msg, Msg::Enter { .. }) {
                pointer.set_bounds(main_bounds());
                log::info!("share: mouse arrived from the PC");
            }
            if matches!(msg, Msg::Move { .. } | Msg::Button { .. }) {
                if let Some(at) = cursor_location() {
                    pointer.sync(at.x, at.y);
                }
            }
            pointer.handle(msg, Instant::now())
        };
        let mut exit = None;
        for action in actions {
            if let PointerAction::Exit { y } = action {
                exit = Some(y);
            } else {
                post(action);
            }
        }
        if let Some(y) = exit {
            release_keys();
            log::info!("share: mouse back to the PC");
            if let Err(e) = session.send(Msg::Left { y }) {
                break e.to_string();
            }
        }
    };
    if GENERATION.load(Ordering::SeqCst) == mine {
        release_all();
    }
    session.close();
    reason
}
