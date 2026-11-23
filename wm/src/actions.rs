//! Named actions: what keys, the socket and the overview ask the WM to do.

use crate::status;
use crate::wm::Wm;
use std::io::Write;

pub const ACTIONS: &[(&str, &str)] = &[
    ("overview", "Open or close the overview"),
    ("terminal", "Open a terminal"),
    ("files", "Open the file manager"),
    ("browser", "Open the web browser"),
    ("settings", "Open Settings"),
    ("close", "Close the window"),
    ("fill", "Fill the screen / back"),
    ("fullscreen", "Full screen / back"),
    ("restore", "Back to its own size, or tuck away"),
    ("snap left", "Left half"),
    ("snap right", "Right half"),
    ("snap top-left", "Top-left quarter"),
    ("snap top-right", "Top-right quarter"),
    ("snap bottom-left", "Bottom-left quarter"),
    ("snap bottom-right", "Bottom-right quarter"),
    ("center", "Centre the window"),
    ("tuck", "Tuck the window away"),
    ("untuck", "Bring back the last tucked window"),
    ("focus next", "Next window"),
    ("focus prev", "Previous window"),
    ("lock", "Lock the screen"),
    ("ground", "Switch night / paper"),
    ("screenshot area", "Screenshot of an area"),
    ("screenshot screen", "Screenshot of the screen"),
    ("screenshot window", "Screenshot of the window"),
    ("volume up", "Louder"),
    ("volume down", "Quieter"),
    ("volume mute", "Mute / unmute"),
    ("brightness up", "Brighter"),
    ("brightness down", "Dimmer"),
    ("reload", "Read the settings again"),
    ("quit", "Log out"),
    ("restart", "Restart the window manager (windows stay open)"),
];

impl Wm {
    /// Run an action. Returns what to answer on the socket.
    pub fn run(&mut self, action: &str) -> Result<String, String> {
        let words: Vec<&str> = action.split_whitespace().collect();
        let focused = self.focused;
        let need = |f: Option<u32>| f.ok_or_else(|| "no window has focus".to_string());
        match words.as_slice() {
            ["overview"] => self.overview_toggle(),
            ["terminal"] => status::spawn(&self.cfg.apps.terminal.clone()),
            ["files"] => status::spawn(&self.cfg.apps.files.clone()),
            ["browser"] => status::spawn(&self.cfg.apps.browser.clone()),
            ["settings"] => status::spawn(&self.cfg.apps.settings.clone()),
            ["settings", page] => status::spawn(&format!("{} --page {}", self.cfg.apps.settings, status::quote(page))),
            ["spawn", rest @ ..] if !rest.is_empty() => status::spawn(&rest.join(" ")),
            ["close"] => self.close(need(focused)?),
            ["fill"] => self.toggle_fill(need(focused)?),
            ["fullscreen"] => {
                let w = need(focused)?;
                let on = self.client(w).is_some_and(|c| c.fullscreen);
                self.set_fullscreen(w, !on);
            }
            ["restore"] => self.restore(need(focused)?),
            ["snap", which] => self.snap(need(focused)?, which),
            ["center"] => self.center(need(focused)?),
            ["tuck"] => self.tuck(need(focused)?),
            ["untuck"] => {
                let w = self.mru.iter().copied().find(|w| self.client(*w).is_some_and(|c| c.tucked)).or_else(|| self.clients.iter().rev().find(|c| c.tucked).map(|c| c.win));
                if let Some(w) = w {
                    self.untuck(w);
                    self.focus(Some(w));
                }
            }
            ["focus", dir] => {
                let order: Vec<u32> = self.mru.iter().copied().filter(|w| self.client(*w).is_some_and(|c| !c.tucked)).collect();
                if order.len() > 1 {
                    let w = if *dir == "prev" { order[order.len() - 1] } else { order[1] };
                    self.focus(Some(w));
                }
            }
            ["focus", "window", id] => {
                let w = id.trim_start_matches("0x");
                let w = u32::from_str_radix(w, if id.starts_with("0x") { 16 } else { 10 }).map_err(|_| "not a window id")?;
                self.focus(Some(w));
            }
            ["lock"] => self.lock_now(),
            ["ground"] => status::spawn("samurai ground toggle"),
            ["screenshot", how] => status::spawn(&format!("samurai shot {how}")),
            ["volume", "up"] => { status::spawn("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+"); self.status.refresh(); }
            ["volume", "down"] => { status::spawn("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"); self.status.refresh(); }
            ["volume", "mute"] => { status::spawn("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"); self.status.refresh(); }
            ["brightness", "up"] => { status::spawn("brightnessctl set 5%+"); self.status.refresh(); }
            ["brightness", "down"] => { status::spawn("brightnessctl set 5%-"); self.status.refresh(); }
            ["reload"] => self.reload(),
            ["quit"] => self.running = false,
            ["restart"] => { self.restart = true; self.running = false; }
            ["power", what] => self.power(what),
            ["say", rest @ ..] => self.say(rest.join(" ")),
            ["state"] => return Ok(self.state_json()),
            ["actions"] => return Ok(ACTIONS.iter().map(|(a, d)| format!("{a}\t{d}")).collect::<Vec<_>>().join("\n")),
            ["ping"] => return Ok("pong".into()),
            [] => return Err("say what to do".into()),
            _ => return Err(format!("unknown action: {action}")),
        }
        Ok("ok".into())
    }

    pub fn state_json(&self) -> String {
        let wins: Vec<serde_json::Value> = self
            .clients
            .iter()
            .map(|c| {
                let v = self.visible_rect(c);
                serde_json::json!({
                    "id": c.win, "frame": c.frame, "title": c.title, "class": c.class,
                    "rect": [c.rect.x, c.rect.y, c.rect.w, c.rect.h], "visible": [v.x, v.y, v.w, v.h],
                    "focused": self.focused == Some(c.win), "tucked": c.tucked, "fullscreen": c.fullscreen,
                    "filled": c.filled, "snapped": c.snapped, "strip": c.strip,
                })
            })
            .collect();
        serde_json::json!({
            "windows": wins,
            "monitors": self.monitors.iter().map(|m| [m.x, m.y, m.w, m.h]).collect::<Vec<_>>(),
            "overview": self.ov.open, "query": self.ov.query,
            "ov_zone": if self.ov.zone == 1 { "apps" } else { "windows" }, "ov_sel": self.ov.sel, "ov_cols": self.ov.app_cols, "ov_apps": self.ov.apps.len(), "locked": self.lock.locked,
            "ground": if self.theme.night { "night" } else { "paper" }, "scale": self.scale,
            "config_error": self.cfg_error,
        })
        .to_string()
    }

    pub fn serve(&mut self) {
        let reqs = match &self.ipc { Some(s) => s.accept(), None => return };
        for mut r in reqs {
            if r.line == "lock --wait" {
                self.lock_now();
                let _ = r.stream.write_all(b"locked\n");
                self.lock.waiters.push(r.stream);
                continue;
            }
            let reply = match self.run(&r.line) {
                Ok(s) => s,
                Err(e) => format!("error: {e}"),
            };
            let _ = r.stream.write_all(format!("{reply}\n").as_bytes());
        }
    }
}
