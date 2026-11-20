//! Platform-free cursor logic for the controlled computer, so it is tested on every OS.
//! The Mac side turns the returned actions into Core Graphics events.
use super::link::Msg;
use std::time::{Duration, Instant};

const DOUBLE_CLICK: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_SLOP: f64 = 4.0;
const WHEEL_NOTCH: i32 = 120;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerAction {
    /// Move the cursor; `drag` is the button held, if any.
    Move { x: f64, y: f64, drag: Option<u8> },
    Button { x: f64, y: f64, b: u8, down: bool, clicks: i64 },
    /// Line units. Positive `y` scrolls up, positive `x` scrolls left (Core Graphics order).
    Scroll { y: i32, x: i32 },
    /// Tell the PC the cursor crossed the right edge at this height (0 top, 1 bottom).
    Exit { y: f64 },
}

pub struct RemotePointer {
    bounds: Bounds,
    x: f64,
    y: f64,
    active: bool,
    held: [bool; 3],
    last_down: Option<(u8, Instant, f64, f64)>,
    clicks: i64,
    wheel: (i32, i32),
}

impl RemotePointer {
    pub fn new(bounds: Bounds) -> Self {
        Self {
            bounds,
            x: bounds.x + bounds.w / 2.0,
            y: bounds.y + bounds.h / 2.0,
            active: false,
            held: [false; 3],
            last_down: None,
            clicks: 0,
            wheel: (0, 0),
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }

    /// Follows the real cursor, which the Mac's own trackpad may have moved.
    pub fn sync(&mut self, x: f64, y: f64) {
        if self.active {
            self.x = x.clamp(self.bounds.x, self.right() - 1.0);
            self.y = y.clamp(self.bounds.y, self.bounds.y + self.bounds.h - 1.0);
        }
    }

    fn right(&self) -> f64 {
        self.bounds.x + self.bounds.w
    }

    fn drag(&self) -> Option<u8> {
        (0..3u8).find(|&b| self.held[b as usize])
    }

    /// Releases every held button at the current position and stops following the PC.
    pub fn release_all(&mut self) -> Vec<PointerAction> {
        let mut out = Vec::new();
        for b in 0..3u8 {
            if std::mem::take(&mut self.held[b as usize]) {
                out.push(PointerAction::Button {
                    x: self.x,
                    y: self.y,
                    b,
                    down: false,
                    clicks: self.clicks.max(1),
                });
            }
        }
        self.active = false;
        self.wheel = (0, 0);
        out
    }

    pub fn handle(&mut self, msg: Msg, now: Instant) -> Vec<PointerAction> {
        match msg {
            Msg::Enter { y } => {
                self.active = true;
                self.x = self.right() - 2.0;
                self.y = self.bounds.y + y.clamp(0.0, 1.0) * (self.bounds.h - 1.0);
                vec![PointerAction::Move { x: self.x, y: self.y, drag: None }]
            }
            Msg::Leave => self.release_all(),
            _ if !self.active => Vec::new(),
            Msg::Move { dx, dy } => {
                let nx = self.x + dx as f64;
                self.y = (self.y + dy as f64).clamp(self.bounds.y, self.bounds.y + self.bounds.h - 1.0);
                if nx >= self.right() && dx > 0 {
                    let frac = (self.y - self.bounds.y) / (self.bounds.h - 1.0).max(1.0);
                    let mut out = self.release_all();
                    out.push(PointerAction::Exit { y: frac });
                    return out;
                }
                self.x = nx.clamp(self.bounds.x, self.right() - 1.0);
                vec![PointerAction::Move { x: self.x, y: self.y, drag: self.drag() }]
            }
            Msg::Button { b, down } if b < 3 => {
                let i = b as usize;
                if down {
                    self.clicks = match self.last_down {
                        Some((lb, t, lx, ly))
                            if lb == b
                                && now.duration_since(t) <= DOUBLE_CLICK
                                && (self.x - lx).abs() <= DOUBLE_CLICK_SLOP
                                && (self.y - ly).abs() <= DOUBLE_CLICK_SLOP =>
                        {
                            self.clicks + 1
                        }
                        _ => 1,
                    };
                    self.last_down = Some((b, now, self.x, self.y));
                    self.held[i] = true;
                } else if !std::mem::take(&mut self.held[i]) {
                    return Vec::new();
                }
                vec![PointerAction::Button { x: self.x, y: self.y, b, down, clicks: self.clicks }]
            }
            Msg::Wheel { dx, dy } => {
                self.wheel.0 += dx;
                self.wheel.1 += dy;
                let (lx, ly) = (self.wheel.0 / WHEEL_NOTCH, self.wheel.1 / WHEEL_NOTCH);
                self.wheel.0 -= lx * WHEEL_NOTCH;
                self.wheel.1 -= ly * WHEEL_NOTCH;
                if lx == 0 && ly == 0 {
                    return Vec::new();
                }
                vec![PointerAction::Scroll { y: ly, x: -lx }]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Bounds = Bounds { x: 0.0, y: 0.0, w: 1470.0, h: 956.0 };

    fn entered() -> (RemotePointer, Instant) {
        let mut p = RemotePointer::new(SCREEN);
        let now = Instant::now();
        p.handle(Msg::Enter { y: 0.5 }, now);
        (p, now)
    }

    #[test]
    fn enter_places_the_cursor_at_the_right_edge_at_the_same_height() {
        let mut p = RemotePointer::new(SCREEN);
        let a = p.handle(Msg::Enter { y: 0.5 }, Instant::now());
        assert_eq!(a, vec![PointerAction::Move { x: 1468.0, y: 477.5, drag: None }]);
        assert!(p.active());
    }

    #[test]
    fn ignores_input_until_entered() {
        let mut p = RemotePointer::new(SCREEN);
        assert!(p.handle(Msg::Move { dx: -5, dy: 0 }, Instant::now()).is_empty());
        assert!(p.handle(Msg::Button { b: 0, down: true }, Instant::now()).is_empty());
    }

    #[test]
    fn moves_are_clamped_to_the_screen() {
        let (mut p, now) = entered();
        let a = p.handle(Msg::Move { dx: -5000, dy: -5000 }, now);
        assert_eq!(a, vec![PointerAction::Move { x: 0.0, y: 0.0, drag: None }]);
    }

    #[test]
    fn crossing_the_right_edge_releases_buttons_and_hands_back() {
        let (mut p, now) = entered();
        p.handle(Msg::Button { b: 0, down: true }, now);
        let a = p.handle(Msg::Move { dx: 10, dy: 0 }, now);
        assert_eq!(a.len(), 2);
        assert!(matches!(a[0], PointerAction::Button { b: 0, down: false, .. }));
        assert!(matches!(a[1], PointerAction::Exit { y } if (y - 0.5).abs() < 0.01));
        assert!(!p.active());
    }

    #[test]
    fn drag_reports_the_held_button() {
        let (mut p, now) = entered();
        p.handle(Msg::Button { b: 1, down: true }, now);
        let a = p.handle(Msg::Move { dx: -10, dy: 0 }, now);
        assert!(matches!(a[0], PointerAction::Move { drag: Some(1), .. }));
    }

    #[test]
    fn quick_second_click_in_place_is_a_double_click() {
        let (mut p, now) = entered();
        p.handle(Msg::Button { b: 0, down: true }, now);
        p.handle(Msg::Button { b: 0, down: false }, now);
        let a = p.handle(Msg::Button { b: 0, down: true }, now + Duration::from_millis(200));
        assert!(matches!(a[0], PointerAction::Button { clicks: 2, .. }));
        let slow = p.handle(Msg::Button { b: 0, down: false }, now);
        assert!(matches!(slow[0], PointerAction::Button { clicks: 2, down: false, .. }));
        let a = p.handle(Msg::Button { b: 0, down: true }, now + Duration::from_secs(2));
        assert!(matches!(a[0], PointerAction::Button { clicks: 1, .. }));
    }

    #[test]
    fn release_without_press_is_dropped() {
        let (mut p, now) = entered();
        assert!(p.handle(Msg::Button { b: 0, down: false }, now).is_empty());
    }

    #[test]
    fn wheel_accumulates_partial_notches() {
        let (mut p, now) = entered();
        assert!(p.handle(Msg::Wheel { dx: 0, dy: 60 }, now).is_empty());
        assert_eq!(p.handle(Msg::Wheel { dx: 0, dy: 60 }, now), vec![PointerAction::Scroll { y: 1, x: 0 }]);
        assert_eq!(p.handle(Msg::Wheel { dx: 240, dy: -120 }, now), vec![PointerAction::Scroll { y: -1, x: -2 }]);
    }

    #[test]
    fn leave_releases_and_stops() {
        let (mut p, now) = entered();
        p.handle(Msg::Button { b: 2, down: true }, now);
        let a = p.handle(Msg::Leave, now);
        assert!(matches!(a[0], PointerAction::Button { b: 2, down: false, .. }));
        assert!(!p.active());
    }
}
