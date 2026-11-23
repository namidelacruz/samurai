//! Tiling, Hyprland's default way ("dwindle"): the first window takes the whole screen; each new
//! one splits the space of the last in two, side by side when that space is wider than tall,
//! one above the other when it is taller. Windows keep their order; moving one swaps it with its
//! neighbour in that direction.
//!
//! What tiles: ordinary windows. What floats: dialogs, utility and splash windows, windows that
//! belong to another (transient), fixed-size windows, anything filled, full screen or tucked away,
//! and any window you float yourself (Super+V). Dragging or resizing a tiled window floats it.
//! With tiling off (Settings › Windows), every window floats and keeps where it is.

use crate::wm::{Client, Kind, Rect, Wm};
use x11rb::protocol::xproto::Window;

/// Split `area` for `n` windows, dwindle-style, with `gap` between them.
pub fn dwindle(area: Rect, n: usize, gap: i32) -> Vec<Rect> {
    let mut out = Vec::with_capacity(n);
    let mut r = area;
    for i in 0..n {
        if i + 1 == n {
            out.push(r);
            break;
        }
        if r.w >= r.h {
            let w1 = (r.w - gap) / 2;
            out.push(Rect::new(r.x, r.y, w1, r.h));
            r = Rect::new(r.x + w1 + gap, r.y, r.w - w1 - gap, r.h);
        } else {
            let h1 = (r.h - gap) / 2;
            out.push(Rect::new(r.x, r.y, r.w, h1));
            r = Rect::new(r.x, r.y + h1 + gap, r.w, r.h - h1 - gap);
        }
    }
    out
}

impl Wm {
    pub fn tiling_on(&self) -> bool {
        self.cfg.windows.tiling
    }

    /// Whether this window takes part in the tiling now.
    pub fn is_tiled(&self, c: &Client) -> bool {
        self.tiling_on()
            && !c.floating
            && !c.tucked
            && !c.fullscreen
            && !c.filled
            && c.kind == Kind::Normal
            && c.ws == self.ws
            && c.transient.is_none()
            && !(c.min.0 > 0 && c.min == c.max)
    }

    /// Lay out the tiled windows of every monitor again (after a window came, went, was tucked,
    /// floated, filled, or a screen changed). Windows glide to their new places.
    pub fn retile(&mut self) {
        if !self.tiling_on() {
            return;
        }
        // keep the order list in step with the windows that exist
        let live: Vec<Window> = self.clients.iter().map(|c| c.win).collect();
        self.tile_order.retain(|w| live.contains(w));
        for c in &self.clients {
            if !self.tile_order.contains(&c.win) {
                self.tile_order.push(c.win);
            }
        }
        let gap = self.cfg.windows.gap;
        for mon in self.monitors.clone() {
            let wins: Vec<Window> = self
                .tile_order
                .iter()
                .copied()
                .filter(|w| self.client(*w).is_some_and(|c| self.is_tiled(c) && self.monitor_of(&self.visible_rect(c)) == mon))
                .collect();
            if wins.is_empty() {
                continue;
            }
            let area = self.workarea(&mon).inset(gap);
            for (w, r) in wins.iter().zip(dwindle(area, wins.len(), gap)) {
                let Some(c) = self.client(*w).cloned() else { continue };
                let to = self.client_rect_for(&c, r);
                if let Some(cm) = self.client_mut(*w) {
                    cm.snapped = None;
                    cm.restore = None;
                }
                if c.rect != to {
                    self.move_to(*w, to);
                }
            }
        }
    }

    /// Move a tiled window one step: it swaps places with its neighbour in that direction.
    /// Returns false for a floating window (the caller then snaps it as before).
    pub fn tile_move(&mut self, w: Window, dir: &str) -> bool {
        if !self.client(w).is_some_and(|c| self.is_tiled(c)) {
            return false;
        }
        let Some(other) = self.neighbour(w, dir).filter(|o| self.client(*o).is_some_and(|c| self.is_tiled(c))) else {
            return true; // already at that edge: nothing to do, and no snapping either
        };
        let (Some(a), Some(b)) = (self.tile_order.iter().position(|x| *x == w), self.tile_order.iter().position(|x| *x == other)) else { return true };
        self.tile_order.swap(a, b);
        self.retile();
        true
    }

    /// Super+V: float this window (it comes forward at a comfortable size), or tile it again.
    pub fn toggle_float(&mut self, w: Window) {
        if !self.tiling_on() {
            self.say("Tiling is off (Settings › Windows): every window floats already.".into());
            return;
        }
        let Some(c) = self.client(w).cloned() else { return };
        // a tiled window floats; a window you floated tiles again
        let now_floating = self.is_tiled(&c);
        if let Some(cm) = self.client_mut(w) {
            cm.floating = now_floating;
        }
        if now_floating {
            // a comfortable size in the middle of its screen
            let mon = self.monitor_of(&self.visible_rect(&c));
            let wa = self.workarea(&mon);
            let (w2, h2) = ((wa.w as f32 * 0.6) as i32, (wa.h as f32 * 0.66) as i32);
            let area = Rect::new(wa.cx() - w2 / 2, wa.cy() - h2 / 2, w2, h2);
            let to = self.client_rect_for(&c, area);
            self.move_to(w, to);
            self.raise(w);
        } else {
            self.tile_order.retain(|x| *x != w);
            self.tile_order.push(w);
        }
        self.retile();
        self.say(if now_floating { "Floating".into() } else { "Tiled".into() });
    }

    /// A tiled window that is dragged or resized by hand floats from then on.
    pub fn float_on_drag(&mut self, w: Window) {
        if self.client(w).is_some_and(|c| self.is_tiled(c)) {
            if let Some(cm) = self.client_mut(w) {
                cm.floating = true;
            }
            self.retile();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dwindle_splits_the_longer_side() {
        let a = Rect::new(0, 0, 1000, 600);
        assert_eq!(dwindle(a, 1, 10), vec![a]);
        let two = dwindle(a, 2, 10);
        assert_eq!(two, vec![Rect::new(0, 0, 495, 600), Rect::new(505, 0, 495, 600)]);
        let three = dwindle(a, 3, 10);
        assert_eq!(three[1], Rect::new(505, 0, 495, 295));
        assert_eq!(three[2], Rect::new(505, 305, 495, 295));
        // every pixel of the area is used, nothing overlaps
        let area: i64 = dwindle(a, 5, 0).iter().map(|r| r.w as i64 * r.h as i64).sum();
        assert_eq!(area, 600_000);
    }
}
