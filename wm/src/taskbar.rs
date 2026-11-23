//! The taskbar for minimized windows. It is not on screen: it appears when called up (Super+T,
//! `samurai-wm msg taskbar`, or the button in the overview), on the monitor you are on, as a grid
//! of the minimized (tucked) windows' icons. Click or Enter brings one back; right-click offers
//! Restore and Close; drag an icon onto another to reorder. Escape, the same key, a click
//! outside, or bringing a window back hides it again.

use crate::draw::{self, Face};
use crate::wm::{Rect, Wm};
use x11rb::protocol::xproto::*;
use x11rb::CURRENT_TIME;

#[derive(Default)]
pub struct Taskbar {
    pub open: bool,
    pub sel: usize,
    pub area: Rect,
    /// (cell, window) for clicks
    pub hot: Vec<(Rect, Window)>,
    /// the two buttons of the right-click menu: (rect, restore?, window)
    pub menu_hot: Vec<(Rect, bool, Window)>,
    pub menu: Option<Window>,
    pub hover: Option<usize>,
    /// the order you arranged by dragging (windows not in it come after, by the chosen sort)
    pub order: Vec<Window>,
    pub drag_from: Option<Window>,
}

impl Wm {
    /// The minimized windows, in the order the taskbar shows them.
    pub fn taskbar_windows(&self) -> Vec<Window> {
        let mut v: Vec<Window> = self.clients.iter().filter(|c| c.tucked).map(|c| c.win).collect();
        match self.cfg.taskbar.sort.as_str() {
            "name" => v.sort_by_key(|w| self.client(*w).map(|c| c.title.to_lowercase()).unwrap_or_default()),
            "app" => v.sort_by_key(|w| self.client(*w).map(|c| (c.class.to_lowercase(), c.title.to_lowercase())).unwrap_or_default()),
            _ => v.sort_by_key(|w| self.mru.iter().position(|m| m == w).unwrap_or(usize::MAX)),
        }
        // your own arrangement first
        let mut out: Vec<Window> = self.taskbar.order.iter().copied().filter(|w| v.contains(w)).collect();
        let rest: Vec<Window> = v.into_iter().filter(|w| !out.contains(w)).collect();
        out.extend(rest);
        out
    }

    pub fn taskbar_toggle(&mut self) {
        if self.taskbar.open { self.taskbar_close() } else { self.taskbar_open() }
    }

    pub fn taskbar_open(&mut self) {
        if !self.cfg.taskbar.on || self.taskbar.open || self.lock.locked {
            return;
        }
        if self.ov.open {
            self.overview_close(None);
        }
        let grabbed = self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok().and_then(|r| r.reply().ok()).is_some_and(|r| r.status == GrabStatus::SUCCESS);
        if !grabbed {
            return;
        }
        self.conn.grab_pointer(false, self.root, EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, self.cursors.normal, CURRENT_TIME).ok();
        self.taskbar.open = true;
        self.taskbar.sel = 0;
        self.taskbar.menu = None;
        self.taskbar.drag_from = None;
        self.draw_taskbar();
    }

    pub fn taskbar_close(&mut self) {
        if !self.taskbar.open {
            return;
        }
        self.taskbar.open = false;
        self.taskbar.menu = None;
        self.conn.ungrab_keyboard(CURRENT_TIME).ok();
        self.conn.ungrab_pointer(CURRENT_TIME).ok();
        self.comp.drop_layer(&self.conn, "taskbar");
        self.comp.dirty = true;
    }

    fn taskbar_restore(&mut self, w: Window) {
        self.taskbar_close();
        self.untuck(w);
        self.focus(Some(w));
        self.raise(w);
    }

    pub fn taskbar_key(&mut self, sym: u32) {
        let wins = self.taskbar_windows();
        let n = wins.len();
        let cols = (self.cfg.taskbar.columns.max(1) as usize).min(n.max(1));
        match sym {
            crate::x::KS_ESCAPE | crate::x::KS_SUPER_L | crate::x::KS_SUPER_R => {
                if self.taskbar.menu.is_some() { self.taskbar.menu = None; self.draw_taskbar(); } else { self.taskbar_close(); }
            }
            crate::x::KS_RETURN | crate::x::KS_KP_ENTER => {
                if let Some(&w) = wins.get(self.taskbar.sel) { self.taskbar_restore(w); } else { self.taskbar_close(); }
            }
            crate::x::KS_DELETE => {
                if let Some(&w) = wins.get(self.taskbar.sel) { self.close(w); self.draw_taskbar(); }
            }
            crate::x::KS_LEFT => { self.taskbar.sel = self.taskbar.sel.saturating_sub(1); self.draw_taskbar(); }
            crate::x::KS_RIGHT | crate::x::KS_TAB => { if self.taskbar.sel + 1 < n { self.taskbar.sel += 1; } self.draw_taskbar(); }
            crate::x::KS_UP => { self.taskbar.sel = self.taskbar.sel.saturating_sub(cols); self.draw_taskbar(); }
            crate::x::KS_DOWN => { if self.taskbar.sel + cols < n { self.taskbar.sel += cols; } self.draw_taskbar(); }
            _ => {
                // the key that opened it closes it again (Super+T: the T arrives with Super held)
                self.taskbar_close();
            }
        }
    }

    pub fn taskbar_press(&mut self, x: i32, y: i32, button: u8) {
        if let Some((_, restore, w)) = self.taskbar.menu_hot.iter().find(|(r, _, _)| r.contains(x, y)).copied() {
            self.taskbar.menu = None;
            if restore { self.taskbar_restore(w); } else { self.close(w); self.draw_taskbar(); }
            return;
        }
        let hit = self.taskbar.hot.iter().find(|(r, _)| r.contains(x, y)).map(|h| h.1);
        match (hit, button) {
            (Some(w), 3) => { self.taskbar.menu = Some(w); self.draw_taskbar(); }
            (Some(w), 1) => { self.taskbar.drag_from = Some(w); }
            (None, _) if self.taskbar.area.contains(x, y) => { self.taskbar.menu = None; self.draw_taskbar(); }
            _ => self.taskbar_close(),
        }
    }

    pub fn taskbar_release(&mut self, x: i32, y: i32) {
        let Some(from) = self.taskbar.drag_from.take() else { return };
        let to = self.taskbar.hot.iter().find(|(r, _)| r.contains(x, y)).map(|h| h.1);
        match to {
            Some(t) if t == from => self.taskbar_restore(from),
            Some(t) => {
                // dropped on another icon: it takes that place
                let mut order = self.taskbar_windows();
                order.retain(|w| *w != from);
                let at = order.iter().position(|w| *w == t).unwrap_or(order.len());
                order.insert(at, from);
                self.taskbar.order = order;
                self.draw_taskbar();
            }
            None => {}
        }
    }

    pub fn taskbar_hover(&mut self, x: i32, y: i32) {
        let h = self.taskbar.hot.iter().position(|(r, _)| r.contains(x, y));
        if h != self.taskbar.hover {
            self.taskbar.hover = h;
            if let Some(i) = h { self.taskbar.sel = i; }
            self.draw_taskbar();
        }
    }

    /// Where the bar goes on monitor `m` for a bar of w×h.
    fn taskbar_place(&self, m: Rect, w: i32, h: i32) -> Rect {
        let pad = (24.0 * self.scale) as i32;
        let wa = self.workarea(&m);
        let (x, y) = match self.cfg.taskbar.position.as_str() {
            "top" => (wa.cx() - w / 2, wa.y + pad),
            "center" => (wa.cx() - w / 2, wa.cy() - h / 2),
            "bottom-left" => (wa.x + pad, wa.y + wa.h - h - pad),
            "bottom-right" => (wa.x + wa.w - w - pad, wa.y + wa.h - h - pad),
            "top-left" => (wa.x + pad, wa.y + pad),
            "top-right" => (wa.x + wa.w - w - pad, wa.y + pad),
            _ => (wa.cx() - w / 2, wa.y + wa.h - h - pad),
        };
        Rect::new(x, y, w, h)
    }

    pub fn draw_taskbar(&mut self) {
        if !self.taskbar.open {
            return;
        }
        let th = self.theme.clone();
        let s = self.scale;
        let tb = self.cfg.taskbar.clone();
        let wins = self.taskbar_windows();
        let n = wins.len();
        if self.taskbar.sel >= n.max(1) { self.taskbar.sel = n.saturating_sub(1); }
        let isz = tb.icon as f32 * s;
        let gap = tb.spacing as f32 * s;
        let label_h = if tb.labels { 20.0 * s } else { 0.0 };
        let cell_w = (isz + 28.0 * s).max(if tb.labels { 88.0 * s } else { 0.0 });
        let cell_h = isz + 16.0 * s + label_h;
        let cols = (tb.columns.max(1) as usize).min(n.max(1));
        let max_cells = cols * tb.rows.max(1) as usize;
        let shown = n.min(max_cells);
        let rows = shown.div_ceil(cols).max(1);
        let pad = 14.0 * s;
        let head = 26.0 * s;
        let (w, h) = if n == 0 {
            (260.0 * s, 64.0 * s)
        } else {
            (2.0 * pad + cols as f32 * cell_w + (cols as f32 - 1.0) * gap, 2.0 * pad + head + rows as f32 * cell_h + (rows as f32 - 1.0) * gap)
        };
        let m = self.pointer_monitor();
        let area = self.taskbar_place(m, w.ceil() as i32, h.ceil() as i32);
        self.taskbar.area = area;
        let Some(mut px) = draw::Px::new(area.w.max(1) as u32, area.h.max(1) as u32) else { return };
        let f = &self.fonts;
        draw::rrect(&mut px, 0.0, 0.0, area.w as f32, area.h as f32, 16.0 * s, th.glass);
        draw::rrect_line(&mut px, 0.5, 0.5, area.w as f32 - 1.0, area.h as f32 - 1.0, 16.0 * s, th.line, 1.0);
        let mut hot = vec![];
        let mut menu_hot = vec![];
        if n == 0 {
            f.draw_centered(&mut px, Face::Ui, 13.0 * s, area.w as f32 / 2.0, area.h as f32 / 2.0 - 8.0 * s, th.dim, "Nothing is minimized");
        } else {
            f.draw(&mut px, Face::UiBold, 11.0 * s, pad + 2.0 * s, pad - 2.0 * s, th.dim, &format!("MINIMIZED  ·  {n}"));
            if n > shown {
                f.draw_right(&mut px, Face::Ui, 11.0 * s, area.w as f32 - pad, pad - 2.0 * s, th.dim, &format!("+{} more (Settings: rows)", n - shown));
            }
            for (i, wv) in wins.iter().take(shown).enumerate() {
                let Some(c) = self.client(*wv) else { continue };
                let (col, row) = (i % cols, i / cols);
                let x = pad + col as f32 * (cell_w + gap);
                let y = pad + head + row as f32 * (cell_h + gap);
                let sel = i == self.taskbar.sel;
                if sel {
                    draw::rrect(&mut px, x, y, cell_w, cell_h, 12.0 * s, th.soft);
                    draw::rrect_line(&mut px, x, y, cell_w, cell_h, 12.0 * s, th.accent, 1.5);
                }
                let cls = c.class.to_lowercase();
                let icon = self.ov.apps.iter().find(|a| a.id.to_lowercase() == cls || a.id.to_lowercase().ends_with(&format!(".{cls}")) || a.name.to_lowercase() == cls).map(|a| a.icon.clone()).unwrap_or_default();
                let ix = x + (cell_w - isz) / 2.0;
                let iy = y + 8.0 * s;
                if !draw::app_icon(&mut px, &icon, ix, iy, isz) {
                    draw::rrect(&mut px, ix, iy, isz, isz, 12.0 * s, th.raised);
                    draw::rrect_line(&mut px, ix, iy, isz, isz, 12.0 * s, th.line, 1.0);
                    draw::icon(&mut px, "apps", ix + isz * 0.3, iy + isz * 0.3, isz * 0.4, th.text, 1.6);
                }
                if tb.labels {
                    let title = if c.title.is_empty() { c.class.clone() } else { c.title.clone() };
                    let t = f.fit(Face::Ui, 11.5 * s, &title, cell_w - 8.0 * s);
                    f.draw_centered(&mut px, Face::Ui, 11.5 * s, x + cell_w / 2.0, iy + isz + 5.0 * s, th.text, &t);
                }
                let cell = Rect::new(area.x + x as i32, area.y + y as i32, cell_w as i32, cell_h as i32);
                hot.push((cell, *wv));
                if self.taskbar.menu == Some(*wv) {
                    // right-click: Restore / Close, over the icon
                    let bw = cell_w - 12.0 * s;
                    let bh = 24.0 * s;
                    for (k, (label, restore)) in [("Restore", true), ("Close", false)].iter().enumerate() {
                        let by = y + 8.0 * s + k as f32 * (bh + 4.0 * s);
                        draw::rrect(&mut px, x + 6.0 * s, by, bw, bh, 7.0 * s, if *restore { th.accent } else { th.raised });
                        draw::rrect_line(&mut px, x + 6.0 * s, by, bw, bh, 7.0 * s, th.line, 1.0);
                        f.draw_centered(&mut px, Face::UiBold, 11.5 * s, x + cell_w / 2.0, by + 5.0 * s, if *restore { th.on_accent } else { th.text }, label);
                        menu_hot.push((Rect::new(area.x + (x + 6.0 * s) as i32, area.y + by as i32, bw as i32, bh as i32), *restore, *wv));
                    }
                }
            }
        }
        self.taskbar.hot = hot;
        self.taskbar.menu_hot = menu_hot;
        self.comp.set_layer(&self.conn, "taskbar", &px, area.x, area.y, 1.0);
        self.comp.dirty = true;
    }
}
