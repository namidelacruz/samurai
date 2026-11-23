//! Snap assist: after a window is snapped to one half (or a quarter), the other windows are
//! offered as small cards in the space left over; pick one (click, or arrows and Enter) and it
//! fills that space. Escape, Super or a click anywhere else lets it go.

use crate::draw::{self, Face};
use crate::wm::{Rect, Wm};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::CURRENT_TIME;

#[derive(Default)]
pub struct Assist {
    pub open: bool,
    /// where the picked window goes ("right", "bottom-left", …) and that place on screen
    pub side: String,
    pub area: Rect,
    pub wins: Vec<Window>,
    pub sel: usize,
    pub hot: Vec<(Rect, Window)>,
    pub hover: Option<usize>,
}

/// The place across from where a window was snapped, or None (fill, or not a snap).
pub fn other_side(which: &str) -> Option<&'static str> {
    Some(match which {
        "left" => "right",
        "right" => "left",
        "top-left" => "bottom-left",
        "bottom-left" => "top-left",
        "top-right" => "bottom-right",
        "bottom-right" => "top-right",
        _ => return None,
    })
}

impl Wm {
    /// Offer the other windows for the rest of the screen after `win` was snapped to `which`.
    pub fn offer_assist(&mut self, win: Window, which: &str) {
        if !self.cfg.windows.snap_assist || self.assist.open || self.ov.open || self.lock.locked || self.tiling_on() {
            return;
        }
        let Some(side) = other_side(which) else { return };
        let Some(c) = self.client(win).cloned() else { return };
        let mon = self.monitor_of(&self.visible_rect(&c));
        let Some(area) = self.snap_area(&mon, side) else { return };
        // nothing to offer if that place is already taken
        if self.clients.iter().any(|o| o.win != win && !o.tucked && o.snapped.as_deref() == Some(side) && mon.overlap(&self.visible_rect(o)) > 0) {
            return;
        }
        let wins: Vec<Window> = self
            .mru
            .iter()
            .copied()
            .filter(|w| *w != win && self.client(*w).is_some_and(|o| !o.fullscreen && o.transient.is_none()))
            .take(9)
            .collect();
        if wins.is_empty() {
            return;
        }
        let grabbed = self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok().and_then(|r| r.reply().ok()).is_some_and(|r| r.status == GrabStatus::SUCCESS);
        if !grabbed {
            return;
        }
        self.conn.grab_pointer(false, self.root, EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, self.cursors.normal, CURRENT_TIME).ok();
        self.assist = Assist { open: true, side: side.to_string(), area, wins, sel: 0, hot: vec![], hover: None };
        self.draw_assist();
    }

    pub fn assist_close(&mut self) {
        if !self.assist.open {
            return;
        }
        self.assist.open = false;
        self.conn.ungrab_keyboard(CURRENT_TIME).ok();
        self.conn.ungrab_pointer(CURRENT_TIME).ok();
        self.comp.drop_layer(&self.conn, "assist");
        self.comp.dirty = true;
        let _ = self.conn.flush();
    }

    fn assist_pick(&mut self, w: Window) {
        let side = self.assist.side.clone();
        self.assist_close();
        if self.client(w).is_some_and(|c| c.tucked) {
            self.untuck(w);
        }
        if let Some(cm) = self.client_mut(w) {
            cm.restore = Some(cm.rect);
        }
        self.snap(w, &side);
        self.focus(Some(w));
    }

    pub fn assist_key(&mut self, sym: u32) {
        let n = self.assist.wins.len();
        match sym {
            crate::x::KS_ESCAPE | crate::x::KS_SUPER_L | crate::x::KS_SUPER_R => self.assist_close(),
            crate::x::KS_RETURN | crate::x::KS_KP_ENTER => {
                if let Some(&w) = self.assist.wins.get(self.assist.sel) {
                    self.assist_pick(w);
                }
            }
            crate::x::KS_LEFT | crate::x::KS_UP => {
                self.assist.sel = (self.assist.sel + n - 1) % n.max(1);
                self.draw_assist();
            }
            crate::x::KS_RIGHT | crate::x::KS_DOWN | crate::x::KS_TAB => {
                self.assist.sel = (self.assist.sel + 1) % n.max(1);
                self.draw_assist();
            }
            _ => self.assist_close(),
        }
    }

    pub fn assist_click(&mut self, x: i32, y: i32) {
        let hit = self.assist.hot.iter().find(|(r, _)| r.contains(x, y)).map(|h| h.1);
        match hit {
            Some(w) => self.assist_pick(w),
            None => self.assist_close(),
        }
    }

    pub fn assist_hover(&mut self, x: i32, y: i32) {
        let h = self.assist.hot.iter().position(|(r, _)| r.contains(x, y));
        if h != self.assist.hover {
            self.assist.hover = h;
            if let Some(i) = h {
                self.assist.sel = i;
            }
            self.draw_assist();
        }
    }

    fn draw_assist(&mut self) {
        let th = self.theme.clone();
        let s = self.scale;
        let a = self.assist.area;
        let Some(mut px) = draw::Px::new(a.w.max(1) as u32, a.h.max(1) as u32) else { return };
        let f = &self.fonts;
        draw::rrect(&mut px, 0.0, 0.0, a.w as f32, a.h as f32, 14.0 * s, th.glass);
        draw::rrect_line(&mut px, 0.5, 0.5, a.w as f32 - 1.0, a.h as f32 - 1.0, 14.0 * s, th.line, 1.0);
        f.draw_centered(&mut px, Face::UiBold, 15.0 * s, a.w as f32 / 2.0, 22.0 * s, th.text, "Fill this side");
        f.draw_centered(&mut px, Face::Ui, 12.0 * s, a.w as f32 / 2.0, 44.0 * s, th.dim, "Pick a window · arrows and Enter · Esc lets it go");
        // cards: icon and title, in as many columns as fit
        let cw = (260.0 * s).min(a.w as f32 - 32.0 * s);
        let ch = 56.0 * s;
        let gap = 10.0 * s;
        let cols = (((a.w as f32 - 32.0 * s + gap) / (cw + gap)).floor() as usize).max(1);
        let n = self.assist.wins.len();
        let rows = n.div_ceil(cols);
        let total_h = rows as f32 * ch + (rows as f32 - 1.0) * gap;
        let total_w = cols.min(n) as f32 * cw + (cols.min(n) as f32 - 1.0) * gap;
        let y0 = (76.0 * s).max((a.h as f32 - total_h) / 2.0);
        let x0 = (a.w as f32 - total_w) / 2.0;
        let mut hot = vec![];
        for (i, w) in self.assist.wins.iter().enumerate() {
            let Some(c) = self.client(*w) else { continue };
            let (col, row) = (i % cols, i / cols);
            let x = x0 + col as f32 * (cw + gap);
            let y = y0 + row as f32 * (ch + gap);
            if y + ch > a.h as f32 - 8.0 * s {
                break;
            }
            let sel = i == self.assist.sel;
            draw::rrect(&mut px, x, y, cw, ch, 11.0 * s, if sel { th.soft } else { th.raised });
            draw::rrect_line(&mut px, x, y, cw, ch, 11.0 * s, if sel { th.accent } else { th.line }, if sel { 1.5 } else { 1.0 });
            let cls = c.class.to_lowercase();
            let icon = self.ov.apps.iter().find(|a| a.id.to_lowercase() == cls || a.id.to_lowercase().ends_with(&format!(".{cls}")) || a.name.to_lowercase() == cls).map(|a| a.icon.clone()).unwrap_or_default();
            if !draw::app_icon(&mut px, &icon, x + 12.0 * s, y + (ch - 30.0 * s) / 2.0, 30.0 * s) {
                draw::icon(&mut px, "apps", x + 17.0 * s, y + (ch - 20.0 * s) / 2.0, 20.0 * s, th.text, 1.6);
            }
            let title = if c.title.is_empty() { c.class.clone() } else { c.title.clone() };
            let t = f.fit(Face::UiBold, 13.0 * s, &title, cw - 64.0 * s);
            f.draw(&mut px, Face::UiBold, 13.0 * s, x + 52.0 * s, y + 10.0 * s, th.text, &t);
            f.draw(&mut px, Face::Ui, 11.5 * s, x + 52.0 * s, y + 29.0 * s, th.dim, &f.fit(Face::Ui, 11.5 * s, &c.class, cw - 64.0 * s));
            hot.push((Rect::new(a.x + x as i32, a.y + y as i32, cw as i32, ch as i32), *w));
        }
        self.assist.hot = hot;
        self.comp.set_layer(&self.conn, "assist", &px, a.x, a.y, 1.0);
        self.comp.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sides() {
        assert_eq!(super::other_side("left"), Some("right"));
        assert_eq!(super::other_side("top-right"), Some("bottom-right"));
        assert_eq!(super::other_side("fill"), None);
    }
}
