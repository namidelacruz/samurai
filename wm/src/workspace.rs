//! Workspaces, kept out of sight: nine of them, reached only by keys (Super+1…9 to go there,
//! Super+Shift+1…9 to send the focused window there). Nothing on screen shows them, and their
//! keys are left out of the shortcut lists, so the desktop still reads as one space; people who
//! know them have them.
//!
//! A window on another workspace is unmapped (like a minimized one, but without the animation and
//! without being "minimized": the taskbar does not list it). New windows open on the workspace
//! you are on.

use crate::wm::Wm;
use x11rb::protocol::xproto::*;

pub const COUNT: u32 = 9;

/// Whether an action belongs to the hidden workspace keys (left out of every list).
pub fn is_hidden_action(action: &str) -> bool {
    action.starts_with("workspace ") || action.starts_with("send ")
}

/// Super+1…9 and Super+Shift+1…9, added to the keys unless you bound those keys yourself.
pub fn default_keys() -> Vec<(String, String)> {
    (1..=COUNT).flat_map(|n| [(format!("Super+{n}"), format!("workspace {n}")), (format!("Super+Shift+{n}"), format!("send {n}"))]).collect()
}

impl Wm {
    fn ws_hide(&mut self, w: Window) {
        let Some(c) = self.client(w).cloned() else { return };
        if c.tucked {
            return; // already unmapped
        }
        if let Some(cm) = self.client_mut(w) {
            cm.ignore_unmaps += 1;
        }
        self.conn.unmap_window(c.win).ok();
        self.conn.unmap_window(c.frame).ok();
    }

    fn ws_show(&mut self, w: Window) {
        let Some(c) = self.client(w).cloned() else { return };
        if c.tucked {
            return; // minimized windows stay minimized
        }
        self.conn.map_window(c.win).ok();
        self.conn.map_window(c.frame).ok();
        self.comp.opening(c.frame);
    }

    /// Go to workspace `n` (1-based).
    pub fn workspace(&mut self, n: u32) {
        let n = n.clamp(1, COUNT);
        if n == self.ws {
            return;
        }
        if self.ov.open {
            self.overview_close(None);
        }
        let (leave, come): (Vec<Window>, Vec<Window>) = (
            self.clients.iter().filter(|c| c.ws == self.ws).map(|c| c.win).collect(),
            self.clients.iter().filter(|c| c.ws == n).map(|c| c.win).collect(),
        );
        for w in leave {
            self.ws_hide(w);
        }
        self.ws = n;
        for w in come {
            self.ws_show(w);
        }
        let a = self.atoms;
        crate::x::set_u32s(&self.conn, self.root, a._NET_CURRENT_DESKTOP, AtomEnum::CARDINAL, &[n - 1]);
        self.retile();
        let next = self.mru.iter().copied().find(|w| self.client(*w).is_some_and(|c| c.ws == n && !c.tucked));
        self.focus(next);
        self.comp.dirty = true;
    }

    /// Send a window to workspace `n`; you stay where you are.
    pub fn send_to(&mut self, w: Window, n: u32) {
        let n = n.clamp(1, COUNT);
        let Some(c) = self.client(w).cloned() else { return };
        if c.ws == n {
            return;
        }
        if let Some(cm) = self.client_mut(w) {
            cm.ws = n;
        }
        let a = self.atoms;
        crate::x::set_u32s(&self.conn, w, a._NET_WM_DESKTOP, AtomEnum::CARDINAL, &[n - 1]);
        if n != self.ws {
            self.ws_hide(w);
            if self.focused == Some(w) {
                let next = self.mru.iter().copied().find(|o| *o != w && self.client(*o).is_some_and(|c| c.ws == self.ws && !c.tucked));
                self.focus(next);
            }
        }
        self.retile();
        self.comp.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hidden_keys() {
        let k = super::default_keys();
        assert_eq!(k.len(), 18);
        assert!(k.iter().all(|(_, a)| super::is_hidden_action(a)));
        assert!(!super::is_hidden_action("move left"));
    }
}
