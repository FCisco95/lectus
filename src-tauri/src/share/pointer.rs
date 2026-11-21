//! Platform-free cursor logic for the receiving computer, so it is tested on every OS.
//! The Mac side turns the returned actions into Core Graphics events.
use super::link::{Edge, Msg};
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
    /// Tell the main computer the cursor crossed back, at `at` along the shared edge.
    Exit { at: f64 },
}

pub struct RemotePointer {
    bounds: Bounds,
    x: f64,
    y: f64,
    active: bool,
    /// The edge of this screen that faces the main computer.
    edge: Edge,
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
            edge: Edge::Right,
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

    fn right(&self) -> f64 {
        self.bounds.x + self.bounds.w
    }

    fn bottom(&self) -> f64 {
        self.bounds.y + self.bounds.h
    }

    /// Follows the real cursor, which the Mac's own trackpad may have moved.
    pub fn sync(&mut self, x: f64, y: f64) {
        if self.active {
            self.x = x.clamp(self.bounds.x, self.right() - 1.0);
            self.y = y.clamp(self.bounds.y, self.bottom() - 1.0);
        }
    }

    fn drag(&self) -> Option<u8> {
        (0..3u8).find(|&b| self.held[b as usize])
    }

    fn frac(v: f64, start: f64, len: f64) -> f64 {
        ((v - start) / (len - 1.0).max(1.0)).clamp(0.0, 1.0)
    }

    /// Releases every held button at the current position and stops following the main computer.
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
        let b = self.bounds;
        match msg {
            Msg::Enter { e, at } => {
                let at = at.clamp(0.0, 1.0);
                self.active = true;
                self.edge = e;
                let along_y = b.y + at * (b.h - 1.0);
                let along_x = b.x + at * (b.w - 1.0);
                (self.x, self.y) = match e {
                    Edge::Right => (self.right() - 2.0, along_y),
                    Edge::Left => (b.x + 1.0, along_y),
                    Edge::Top => (along_x, b.y + 1.0),
                    Edge::Bottom => (along_x, self.bottom() - 2.0),
                };
                vec![PointerAction::Move { x: self.x, y: self.y, drag: None }]
            }
            Msg::Leave => self.release_all(),
            _ if !self.active => Vec::new(),
            Msg::Move { dx, dy } => {
                let (nx, ny) = (self.x + dx as f64, self.y + dy as f64);
                let crossed = match self.edge {
                    Edge::Right => dx > 0 && nx >= self.right(),
                    Edge::Left => dx < 0 && nx < b.x,
                    Edge::Top => dy < 0 && ny < b.y,
                    Edge::Bottom => dy > 0 && ny >= self.bottom(),
                };
                self.x = nx.clamp(b.x, self.right() - 1.0);
                self.y = ny.clamp(b.y, self.bottom() - 1.0);
                if crossed {
                    let at = match self.edge {
                        Edge::Left | Edge::Right => Self::frac(self.y, b.y, b.h),
                        Edge::Top | Edge::Bottom => Self::frac(self.x, b.x, b.w),
                    };
                    let mut out = self.release_all();
                    out.push(PointerAction::Exit { at });
                    return out;
                }
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

    fn entered_at(e: Edge) -> (RemotePointer, Instant) {
        let mut p = RemotePointer::new(SCREEN);
        let now = Instant::now();
        p.handle(Msg::Enter { e, at: 0.5 }, now);
        (p, now)
    }

    fn entered() -> (RemotePointer, Instant) {
        entered_at(Edge::Right)
    }

    #[test]
    fn enter_places_the_cursor_on_the_facing_edge() {
        let mut p = RemotePointer::new(SCREEN);
        let now = Instant::now();
        assert_eq!(
            p.handle(Msg::Enter { e: Edge::Right, at: 0.5 }, now),
            vec![PointerAction::Move { x: 1468.0, y: 477.5, drag: None }]
        );
        assert_eq!(
            p.handle(Msg::Enter { e: Edge::Top, at: 0.0 }, now),
            vec![PointerAction::Move { x: 0.0, y: 1.0, drag: None }]
        );
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
    fn crossing_the_facing_edge_releases_buttons_and_hands_back() {
        let (mut p, now) = entered();
        p.handle(Msg::Button { b: 0, down: true }, now);
        let a = p.handle(Msg::Move { dx: 10, dy: 0 }, now);
        assert_eq!(a.len(), 2);
        assert!(matches!(a[0], PointerAction::Button { b: 0, down: false, .. }));
        assert!(matches!(a[1], PointerAction::Exit { at } if (at - 0.5).abs() < 0.01));
        assert!(!p.active());
    }

    #[test]
    fn other_edges_do_not_hand_back() {
        let (mut p, now) = entered_at(Edge::Bottom);
        let a = p.handle(Msg::Move { dx: 5000, dy: 0 }, now);
        assert!(matches!(a[0], PointerAction::Move { .. }));
        let a = p.handle(Msg::Move { dx: 0, dy: 50 }, now);
        assert!(matches!(a[0], PointerAction::Exit { at } if (at - 1.0).abs() < 0.01));
    }

    #[test]
    fn left_edge_hands_back_when_moving_left() {
        let (mut p, now) = entered_at(Edge::Left);
        assert!(matches!(p.handle(Msg::Move { dx: -2, dy: 0 }, now)[0], PointerAction::Exit { .. }));
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
        p.handle(Msg::Button { b: 0, down: false }, now);
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
