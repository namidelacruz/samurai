//! The overview (Super): every open window laid out on the screen, search across windows,
//! apps, settings and commands, the quick panel, and the shelf of tucked-away windows. It is the
//! whole of Samurai's "shell": there is no taskbar and there are no workspaces.

use crate::draw::{self, Face};
use crate::theme::Rgba;
use crate::status::{self, Snapshot};
use crate::wm::{Rect, Wm};
use crate::x;
use std::collections::HashMap;
use std::time::Instant;
use x11rb::protocol::xproto::*;
use x11rb::CURRENT_TIME;

#[derive(Clone, Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub terminal: bool,
    pub keywords: String,
    pub categories: String,
    /// Icon= from its .desktop file: a theme name or a path
    pub icon: String,
}

#[derive(Clone, Debug)]
pub enum Act {
    Window(Window),
    App(usize),
    Setting(String),
    Run(String),
    Copy(String),
    Tile(&'static str),
    Power(&'static str),
    Settings,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub group: &'static str,
    pub icon: &'static str,
    pub title: String,
    pub sub: String,
    pub act: Act,
}

#[derive(Clone, Debug)]
pub enum Hot {
    Card(Window),
    Item(usize),
    App(usize),
    Tile(&'static str),
    Slider(&'static str, Rect),
    Power(&'static str),
    Shelf(Window),
    Settings,
    /// a row that opens or closes its list ("wifi", "bt")
    Expand(&'static str),
    Net(usize),
    Bt(usize),
    PassField,
    PassGo,
    /// the footer of an open list: its Settings page
    ListSettings(&'static str),
    /// the "n minimized" button: the taskbar
    Taskbar,
    Blank,
}

#[derive(Default)]
pub struct Overview {
    pub open: bool,
    pub closing: bool,
    pub start: Option<Instant>,
    pub t: f32,
    pub query: String,
    pub sel: usize,
    pub rects: HashMap<Window, Rect>,
    pub order: Vec<Window>,
    pub relayout: bool,
    pub dirty: bool,
    pub apps: Vec<App>,
    pub settings_index: Vec<(String, String, String)>,
    pub items: Vec<Item>,
    pub hot: Vec<(Rect, Hot)>,
    pub hover: Option<usize>,
    pub mon: Rect,
    pub snap: Snapshot,
    pub drag_slider: Option<&'static str>,
    pub zone: u8,
    /// the open list in the quick panel: "wifi" or "bt"
    pub expand: Option<&'static str>,
    /// a network waiting for its password, and what has been typed (never drawn, only dots)
    pub pass_for: Option<String>,
    pub pass: String,
    /// what the last network or Bluetooth action said
    pub note: Option<(String, bool)>,
    /// the apps: first row shown, and the grid's shape as last drawn (for the arrow keys)
    pub app_scroll: usize,
    pub app_cols: usize,
    pub app_rows: usize,
}

/// The quick panel's width: compact, but wide enough for a network name.
pub const PANEL_W: f32 = 284.0;

fn data_dirs() -> Vec<std::path::PathBuf> {
    let mut v = vec![];
    let home = crate::config::home();
    v.push(std::env::var("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| home.join(".local/share")));
    for d in std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into()).split(':') {
        if !d.is_empty() {
            v.push(d.into());
        }
    }
    v
}

fn strip_codes(exec: &str) -> String {
    let mut out = vec![];
    for w in exec.split_whitespace() {
        if w.len() == 2 && w.starts_with('%') {
            continue;
        }
        out.push(w.replace("%%", "%"));
    }
    out.join(" ")
}

impl Overview {
    pub fn load_apps(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut apps = vec![];
        for d in data_dirs() {
            let Ok(rd) = std::fs::read_dir(d.join("applications")) else { continue };
            let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "desktop")).collect();
            files.sort();
            for p in files {
                let id = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if !seen.insert(id.clone()) {
                    continue; // the user's own copy wins
                }
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                let mut in_main = false;
                let (mut name, mut exec, mut kw, mut cats, mut icon) = (String::new(), String::new(), String::new(), String::new(), String::new());
                let (mut hidden, mut term, mut is_app) = (false, false, false);
                for line in text.lines() {
                    let l = line.trim();
                    if l.starts_with('[') {
                        in_main = l == "[Desktop Entry]";
                        continue;
                    }
                    if !in_main {
                        continue;
                    }
                    let Some((k, v)) = l.split_once('=') else { continue };
                    match k.trim() {
                        "Name" => name = v.trim().to_string(),
                        "Exec" => exec = strip_codes(v.trim()),
                        "Keywords" => kw = v.trim().to_string(),
                        "Categories" => cats = v.trim().to_string(),
                        "Icon" => icon = v.trim().to_string(),
                        "NoDisplay" | "Hidden" if v.trim() == "true" => hidden = true,
                        "Terminal" => term = v.trim() == "true",
                        "Type" => is_app = v.trim() == "Application",
                        _ => {}
                    }
                }
                if is_app && !hidden && !name.is_empty() && !exec.is_empty() {
                    apps.push(App { id, name, exec, terminal: term, keywords: kw, categories: cats, icon });
                }
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        self.apps = apps;
        // the Settings app publishes what can be searched
        self.settings_index.clear();
        for p in [crate::config::config_dir().join("settings-index.json"), "/opt/samurai/share/settings-index.json".into()] {
            if let Ok(t) = std::fs::read_to_string(&p) {
                if let Ok(serde_json::Value::Array(a)) = serde_json::from_str::<serde_json::Value>(&t) {
                    for e in a {
                        let g = |k: &str| e.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        self.settings_index.push((g("label"), g("page"), g("where")));
                    }
                }
                break;
            }
        }
    }
}

pub fn app_icon(a: &App) -> &'static str {
    let s = format!("{} {} {}", a.id, a.name, a.categories).to_lowercase();
    if s.contains("terminal") || s.contains("alacritty") { "terminal" }
    else if s.contains("filemanager") || s.contains("thunar") || s.contains("files") { "folder" }
    else if s.contains("webbrowser") || s.contains("firefox") || s.contains("chrom") { "globe" }
    else if s.contains("texteditor") || s.contains("mousepad") || s.contains("editor") || s.contains("development") { "edit" }
    else if s.contains("email") || s.contains("mail") || s.contains("thunderbird") { "mail" }
    else if s.contains("audio") || s.contains("music") { "music" }
    else if s.contains("video") || s.contains("mpv") || s.contains("player") { "video" }
    else if s.contains("graphics") || s.contains("image") || s.contains("imv") || s.contains("photo") { "image" }
    else if s.contains("settings") || s.contains("samurai-settings") { "settings" }
    else if s.contains("office") || s.contains("pdf") || s.contains("zathura") { "doc" }
    else if s.contains("calculator") { "calc" }
    else { "apps" }
}

/// A small calculator for "= …": + - * / % ^ and brackets.
pub fn calc(expr: &str) -> Option<f64> {
    let toks: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).map(|c| if c == ',' { '.' } else if c == 'x' || c == '×' { '*' } else if c == '÷' { '/' } else { c }).collect();
    let mut i = 0;
    fn num(t: &[char], i: &mut usize) -> Option<f64> {
        if *i < t.len() && t[*i] == '(' {
            *i += 1;
            let v = sum(t, i)?;
            if *i < t.len() && t[*i] == ')' { *i += 1; }
            return Some(v);
        }
        if *i < t.len() && t[*i] == '-' {
            *i += 1;
            return num(t, i).map(|v| -v);
        }
        let s = *i;
        while *i < t.len() && (t[*i].is_ascii_digit() || t[*i] == '.') { *i += 1; }
        t[s..*i].iter().collect::<String>().parse().ok()
    }
    fn pow(t: &[char], i: &mut usize) -> Option<f64> {
        let b = num(t, i)?;
        if *i < t.len() && t[*i] == '^' { *i += 1; return Some(b.powf(pow(t, i)?)); }
        Some(b)
    }
    fn prod(t: &[char], i: &mut usize) -> Option<f64> {
        let mut v = pow(t, i)?;
        while *i < t.len() && matches!(t[*i], '*' | '/' | '%') {
            let op = t[*i]; *i += 1;
            let r = pow(t, i)?;
            v = match op { '*' => v * r, '/' => v / r, _ => v % r };
        }
        Some(v)
    }
    fn sum(t: &[char], i: &mut usize) -> Option<f64> {
        let mut v = prod(t, i)?;
        while *i < t.len() && matches!(t[*i], '+' | '-') {
            let op = t[*i]; *i += 1;
            let r = prod(t, i)?;
            v = if op == '+' { v + r } else { v - r };
        }
        Some(v)
    }
    let v = sum(&toks, &mut i)?;
    if i != toks.len() || !v.is_finite() { None } else { Some(v) }
}

fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e15 { format!("{}", v.round() as i64) } else { format!("{:.6}", v).trim_end_matches('0').trim_end_matches('.').to_string() }
}

fn matches(hay: &str, q: &str) -> bool {
    let h = hay.to_lowercase();
    q.split_whitespace().all(|w| h.contains(w))
}

impl Wm {
    pub fn overview_toggle(&mut self) {
        if self.ov.open && !self.ov.closing { self.overview_close(None) } else { self.overview_open() }
    }

    pub fn overview_open(&mut self) {
        if self.lock.locked || self.ov.open && !self.ov.closing {
            return;
        }
        let grabbed = self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok().and_then(|r| r.reply().ok()).is_some_and(|r| r.status == GrabStatus::SUCCESS);
        if !grabbed {
            return;
        }
        self.conn.grab_pointer(false, self.root, EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, self.cursors.normal, CURRENT_TIME).ok();
        self.ov.open = true;
        self.ov.closing = false;
        self.ov.start = Some(Instant::now());
        self.ov.query.clear();
        self.ov.sel = 0;
        self.ov.zone = 0;
        self.ov.app_scroll = 0;
        self.ov.mon = self.pointer_monitor();
        self.ov.snap = self.status.get();
        self.status.refresh();
        self.overview_layout();
        // start on the window you were in; with no windows, on the apps
        if let Some(f) = self.focused.and_then(|f| self.client(f).map(|c| c.frame)) {
            self.ov.sel = self.ov.order.iter().position(|w| *w == f).unwrap_or(0);
        }
        if self.ov.order.is_empty() {
            self.ov.zone = 1;
        }
        self.ov.dirty = true;
    }

    /// Close; `focus` is the window to bring forward as it closes.
    pub fn overview_close(&mut self, focus: Option<Window>) {
        if !self.ov.open || self.ov.closing {
            return;
        }
        self.ov.closing = true;
        self.ov.start = Some(Instant::now());
        self.conn.ungrab_keyboard(CURRENT_TIME).ok();
        self.conn.ungrab_pointer(CURRENT_TIME).ok();
        if let Some(w) = focus {
            self.focus(Some(w));
            if let Some(f) = self.client(w).map(|c| c.frame) {
                // the chosen window lands last, above the others
                let r = self.ov.rects.remove(&f);
                if let Some(r) = r { self.ov.rects.insert(f, r); }
            }
        }
        self.ov.dirty = true;
    }

    /// Where the window cards and the apps go, in screen coordinates: (cards, apps). The cards
    /// take the top part when there are windows; the apps fill the rest (all of it when none).
    pub fn ov_regions(&self, n_windows: usize) -> (Rect, Rect) {
        let s = self.scale;
        let m = self.ov.mon;
        let pad = (40.0 * s) as i32;
        let zone_w = m.w - (PANEL_W * s) as i32 - 3 * pad;
        let top = (84.0 * s) as i32;
        let bottom = (64.0 * s) as i32;
        let avail = m.h - top - bottom;
        let cards_h = if n_windows == 0 { 0 } else { (avail as f32 * 0.46) as i32 };
        let cards = Rect::new(m.x + pad, m.y + top, zone_w, cards_h);
        let head = (30.0 * s) as i32;
        let apps_y = top + cards_h + if n_windows == 0 { 0 } else { (34.0 * s) as i32 } + head;
        let apps = Rect::new(m.x + pad, m.y + apps_y, zone_w, (m.h - apps_y - bottom).max((60.0 * s) as i32));
        (cards, apps)
    }

    /// The apps in the order shown: pinned first, then every other app by name.
    pub fn app_order(&self) -> Vec<usize> {
        let mut v = self.pinned();
        for i in 0..self.ov.apps.len() {
            if !v.contains(&i) {
                v.push(i);
            }
        }
        v
    }

    /// Keep the selected app in view.
    fn app_follow(&mut self) {
        let cols = self.ov.app_cols.max(1);
        let rows = self.ov.app_rows.max(1);
        let row = self.ov.sel / cols;
        if row < self.ov.app_scroll {
            self.ov.app_scroll = row;
        } else if row >= self.ov.app_scroll + rows {
            self.ov.app_scroll = row + 1 - rows;
        }
    }

    /// Lay the windows of this monitor out as cards in the space left of the panel.
    pub fn overview_layout(&mut self) {
        let s = self.scale;
        let m = self.ov.mon;
        let mut wins: Vec<(Window, Rect)> = vec![];
        // most recent first, so the window you were in is first
        let mut list: Vec<&crate::wm::Client> = self.clients.iter().filter(|c| !c.tucked && m.overlap(&self.visible_rect(c)) > 0).collect();
        list.sort_by_key(|c| self.mru.iter().position(|w| *w == c.win).unwrap_or(usize::MAX));
        for c in list {
            wins.push((c.frame, self.visible_rect(c)));
        }
        self.ov.rects.clear();
        self.ov.order.clear();
        let n = wins.len();
        if n == 0 {
            return;
        }
        let area = self.ov_regions(n).0;
        let gap = (32.0 * s) as f32;
        let title_h = (34.0 * s) as f32;
        let mut best = (0usize, 0f32);
        for cols in 1..=n {
            let rows = n.div_ceil(cols);
            let cw = (area.w as f32 - gap * (cols as f32 - 1.0)) / cols as f32;
            let ch = (area.h as f32 - gap * (rows as f32 - 1.0)) / rows as f32 - title_h;
            if cw < 40.0 || ch < 30.0 {
                continue;
            }
            let sc = wins.iter().map(|(_, r)| (cw / r.w as f32).min(ch / r.h as f32)).fold(f32::MAX, f32::min).min(0.6);
            if sc > best.1 {
                best = (cols, sc);
            }
        }
        let cols = best.0.max(1);
        let rows = n.div_ceil(cols);
        let cw = (area.w as f32 - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = (area.h as f32 - gap * (rows as f32 - 1.0)) / rows as f32;
        let total_h = rows as f32 * ch + (rows as f32 - 1.0) * gap;
        for (i, (f, r)) in wins.iter().enumerate() {
            let (row, col) = (i / cols, i % cols);
            let in_row = if row == rows - 1 { n - row * cols } else { cols };
            let row_w = in_row as f32 * cw + (in_row as f32 - 1.0) * gap;
            let x0 = area.x as f32 + (area.w as f32 - row_w) / 2.0 + col as f32 * (cw + gap);
            let y0 = area.y as f32 + (area.h as f32 - total_h) / 2.0 + row as f32 * (ch + gap);
            let sc = ((cw) / r.w as f32).min((ch - title_h) / r.h as f32).min(0.6);
            let (w, h) = ((r.w as f32 * sc) as i32, (r.h as f32 * sc) as i32);
            let rect = Rect::new((x0 + (cw - w as f32) / 2.0) as i32, (y0 + (ch - title_h - h as f32) / 2.0) as i32, w, h);
            self.ov.rects.insert(*f, rect);
            self.ov.order.push(*f);
        }
        self.ov.relayout = false;
    }

    fn ov_results(&mut self) {
        let q = self.ov.query.trim().to_string();
        let mut items = vec![];
        if let Some(e) = q.strip_prefix('=') {
            match calc(e) {
                Some(v) => items.push(Item { group: "Calculator", icon: "calc", title: fmt_num(v), sub: "Enter copies it".into(), act: Act::Copy(fmt_num(v)) }),
                None => items.push(Item { group: "Calculator", icon: "calc", title: "…".into(), sub: "Numbers and + − × ÷ ^ ( )".into(), act: Act::Copy(String::new()) }),
            }
        } else if let Some(c) = q.strip_prefix('>') {
            let c = c.trim();
            if !c.is_empty() {
                items.push(Item { group: "Run", icon: "terminal", title: c.to_string(), sub: "Runs in a terminal".into(), act: Act::Run(c.to_string()) });
            }
        } else if !q.is_empty() {
            let ql = q.to_lowercase();
            for c in self.clients.iter() {
                if matches(&format!("{} {}", c.title, c.class), &ql) {
                    items.push(Item { group: "Windows", icon: "apps", title: if c.title.is_empty() { c.class.clone() } else { c.title.clone() }, sub: if c.tucked { "Tucked away".into() } else { "Open now".into() }, act: Act::Window(c.win) });
                }
            }
            for (i, a) in self.ov.apps.iter().enumerate() {
                if matches(&format!("{} {} {}", a.name, a.keywords, a.id), &ql) {
                    items.push(Item { group: "Apps", icon: app_icon(a), title: a.name.clone(), sub: "Opens a new window".into(), act: Act::App(i) });
                }
            }
            for (label, page, wh) in self.ov.settings_index.iter() {
                if matches(&format!("{label} {page}"), &ql) {
                    items.push(Item { group: "Settings", icon: "settings", title: label.clone(), sub: wh.clone(), act: Act::Setting(page.clone()) });
                }
            }
            for (k, label) in [("lock", "Lock"), ("sleep", "Sleep"), ("logout", "Log out"), ("restart", "Restart"), ("poweroff", "Power off")] {
                if matches(label, &ql) {
                    items.push(Item { group: "Power", icon: if k == "lock" { "lock" } else { "power" }, title: label.into(), sub: String::new(), act: Act::Power(k) });
                }
            }
        }
        // keep each group short
        let mut per: HashMap<&str, usize> = HashMap::new();
        items.retain(|i| {
            let n = per.entry(i.group).or_insert(0);
            *n += 1;
            *n <= 5
        });
        self.ov.items = items;
        // only the results list is limited by the results; the cards and apps keep their place
        if !q.is_empty() && self.ov.sel >= self.ov.items.len().max(1) {
            self.ov.sel = 0;
        }
    }

    pub fn launch_app(&mut self, i: usize) {
        let Some(a) = self.ov.apps.get(i).cloned() else { return };
        if a.terminal {
            status::spawn(&format!("{} -e {}", self.cfg.apps.terminal, a.exec));
        } else {
            status::spawn(&a.exec);
        }
    }

    pub fn pinned(&self) -> Vec<usize> {
        let mut v = vec![];
        for p in &self.cfg.apps.pinned {
            let p = p.trim_end_matches(".desktop").to_lowercase();
            if let Some(i) = self.ov.apps.iter().position(|a| a.id.to_lowercase() == p || a.name.to_lowercase() == p || a.id.to_lowercase().ends_with(&format!(".{p}"))) {
                if !v.contains(&i) {
                    v.push(i);
                }
            }
        }
        v.truncate(10);
        v
    }

    fn activate(&mut self, act: Act) {
        match act {
            Act::Window(w) => {
                if self.client(w).is_some_and(|c| c.tucked) {
                    self.untuck(w);
                }
                self.overview_close(Some(w));
            }
            Act::App(i) => {
                self.launch_app(i);
                self.overview_close(None);
            }
            Act::Setting(page) => {
                status::spawn(&format!("{} --page {}", self.cfg.apps.settings, status::quote(&page)));
                self.overview_close(None);
            }
            Act::Run(c) => {
                status::spawn(&format!("{} --hold -e sh -c {}", self.cfg.apps.terminal, status::quote(&c)));
                self.overview_close(None);
            }
            Act::Copy(v) => {
                if !v.is_empty() {
                    status::spawn(&format!("sh -c 'printf %s {} | xclip -selection clipboard'", v));
                }
                self.overview_close(None);
            }
            Act::Tile(t) => {
                if let Some(cmd) = status::tile(t, &self.ov.snap) {
                    status::spawn(&cmd);
                }
                if t == "shot" {
                    self.overview_close(None);
                } else {
                    // show the change at once; the next read confirms it
                    match t {
                        "wifi" => self.ov.snap.wifi_on = !self.ov.snap.wifi_on,
                        "bt" => self.ov.snap.bt_on = !self.ov.snap.bt_on,
                        "quiet" => self.ov.snap.quiet = !self.ov.snap.quiet,
                        "night" => self.ov.snap.night_light = !self.ov.snap.night_light,
                        _ => {}
                    }
                    self.status.refresh();
                    self.ov.dirty = true;
                }
            }
            Act::Power(p) => {
                self.overview_close(None);
                self.power(p);
            }
            Act::Settings => {
                status::spawn(&self.cfg.apps.settings.clone());
                self.overview_close(None);
            }
        }
    }

    pub fn power(&mut self, what: &str) {
        match what {
            "lock" => self.lock_now(),
            "sleep" => {
                self.lock_now();
                status::spawn("systemctl suspend");
            }
            "logout" => self.running = false,
            "restart" => status::spawn("systemctl reboot"),
            "poweroff" => status::spawn("systemctl poweroff"),
            _ => {}
        }
    }

    pub fn overview_key(&mut self, code: u8, state: u16, sym: u32) {
        if self.ov.closing {
            return;
        }
        let searching = !self.ov.query.is_empty();
        if self.ov.pass_for.is_some() {
            // typing goes to the password field until it is sent or given up
            match sym {
                x::KS_ESCAPE => { self.ov.pass_for = None; self.ov.pass.clear(); }
                x::KS_RETURN | x::KS_KP_ENTER => self.send_password(),
                x::KS_BACKSPACE => { self.ov.pass.pop(); }
                x::KS_SUPER_L | x::KS_SUPER_R => { self.super_down = true; self.super_used = false; return; }
                _ => {
                    if state & (x::MOD_CTRL | x::MOD_ALT | x::MOD_SUPER) == 0 {
                        if let Some(ch) = self.keymap.char_for(code, state) {
                            if !ch.is_control() && self.ov.pass.chars().count() < 128 { self.ov.pass.push(ch); }
                        }
                    }
                }
            }
            self.ov.dirty = true;
            return;
        }
        match sym {
            x::KS_SUPER_L | x::KS_SUPER_R => {
                self.super_down = true;
                self.super_used = false;
                return;
            }
            x::KS_ESCAPE => {
                if searching { self.ov.query.clear(); self.ov.sel = 0; } else { self.overview_close(None); return; }
            }
            x::KS_RETURN | x::KS_KP_ENTER => {
                if searching {
                    if let Some(it) = self.ov.items.get(self.ov.sel).cloned() {
                        // Ctrl+Enter on an app always opens a new window
                        if state & x::MOD_CTRL != 0 {
                            if let Act::Window(w) = it.act {
                                let cls = self.client(w).map(|c| c.class.to_lowercase()).unwrap_or_default();
                                if let Some(i) = self.ov.apps.iter().position(|a| a.id.to_lowercase().contains(&cls) || a.name.to_lowercase() == cls) {
                                    self.activate(Act::App(i));
                                    return;
                                }
                            }
                        }
                        self.activate(it.act);
                    }
                } else if self.ov.zone == 1 {
                    if let Some(&i) = self.app_order().get(self.ov.sel) { self.activate(Act::App(i)); }
                } else if let Some(&f) = self.ov.order.get(self.ov.sel) {
                    let w = self.client(f).map(|c| c.win);
                    self.overview_close(w);
                } else {
                    self.overview_close(None);
                }
                return;
            }
            x::KS_BACKSPACE => {
                self.ov.query.pop();
            }
            x::KS_DELETE if !searching && self.ov.zone == 0 => {
                if let Some(w) = self.ov.order.get(self.ov.sel).and_then(|f| self.client(*f)).map(|c| c.win) {
                    self.close(w);
                }
            }
            x::KS_TAB | x::KS_ISO_LEFT_TAB => {
                self.ov.zone = if self.ov.order.is_empty() { 1 } else { (self.ov.zone + 1) % 2 };
                self.ov.sel = 0;
                self.ov.app_scroll = 0;
            }
            x::KS_UP | x::KS_DOWN if !searching && self.ov.zone == 1 => {
                // the apps are a grid: up and down move a whole row
                let cols = self.ov.app_cols.max(1);
                let n = self.app_order().len();
                if sym == x::KS_UP {
                    self.ov.sel = self.ov.sel.saturating_sub(cols);
                } else if self.ov.sel + cols < n {
                    self.ov.sel += cols;
                } else if n > 0 {
                    self.ov.sel = n - 1;
                }
                self.app_follow();
            }
            x::KS_LEFT | x::KS_UP if !searching || sym == x::KS_UP => {
                self.ov.sel = self.ov.sel.saturating_sub(1);
                if self.ov.zone == 1 && !searching { self.app_follow(); }
            }
            x::KS_RIGHT | x::KS_DOWN if !searching || sym == x::KS_DOWN => {
                let n = if searching { self.ov.items.len() } else if self.ov.zone == 1 { self.app_order().len() } else { self.ov.order.len() };
                if self.ov.sel + 1 < n { self.ov.sel += 1; }
                if self.ov.zone == 1 && !searching { self.app_follow(); }
            }
            _ => {
                if state & (x::MOD_CTRL | x::MOD_ALT | x::MOD_SUPER) == 0 {
                    if let Some(ch) = self.keymap.char_for(code, state) {
                        if !ch.is_control() {
                            self.ov.query.push(ch);
                            self.ov.sel = 0;
                        }
                    }
                }
            }
        }
        self.ov_results();
        self.ov.dirty = true;
    }

    pub fn overview_click(&mut self, x: i32, y: i32, button: u8) {
        if self.ov.closing {
            return;
        }
        if button == 4 || button == 5 {
            // the wheel scrolls the apps (the quick panel does not scroll)
            if self.ov.query.is_empty() {
                let max = self.app_order().len().div_ceil(self.ov.app_cols.max(1)).saturating_sub(self.ov.app_rows.max(1));
                self.ov.app_scroll = if button == 4 { self.ov.app_scroll.saturating_sub(1) } else { (self.ov.app_scroll + 1).min(max) };
                self.ov.dirty = true;
            }
            return;
        }
        let hit = self.ov.hot.iter().rev().find(|(r, _)| r.contains(x, y)).map(|h| h.1.clone());
        if button == 3 {
            // right-click an app: pin it first in the list, or unpin it
            if let Some(Hot::App(i)) = hit {
                if let Some(a) = self.ov.apps.get(i) {
                    status::spawn(&format!("samurai pin {}", status::quote(&a.id)));
                    self.say(format!("{} {}", a.name, if self.pinned().contains(&i) { "unpinned" } else { "pinned" }));
                }
            }
            return;
        }
        let card = self.ov.order.iter().rev().find(|f| self.ov.rects.get(f).is_some_and(|r| r.contains(x, y))).copied();
        match (hit, card) {
            (Some(Hot::Item(i)), _) => {
                if let Some(it) = self.ov.items.get(i).cloned() { self.activate(it.act); }
            }
            (Some(Hot::App(i)), _) => self.activate(Act::App(i)),
            (Some(Hot::Tile(t)), _) => self.activate(Act::Tile(t)),
            (Some(Hot::Power(p)), _) => self.activate(Act::Power(p)),
            (Some(Hot::Settings), _) => self.activate(Act::Settings),
            (Some(Hot::Expand(what)), _) => {
                self.ov.expand = if self.ov.expand == Some(what) { None } else { Some(what) };
                self.ov.pass_for = None;
                self.ov.pass.clear();
                self.ov.note = None;
                self.status.detail.store(self.ov.expand.is_some(), std::sync::atomic::Ordering::Relaxed);
                self.status.refresh();
                self.ov.dirty = true;
            }
            (Some(Hot::Net(i)), _) => self.pick_network(i),
            (Some(Hot::Bt(i)), _) => {
                if let Some(d) = self.ov.snap.bt_devs.get(i).cloned() {
                    let verb = if d.connected { "disconnect" } else { "connect" };
                    let done = if d.connected { format!("{} disconnected", d.name) } else { format!("{} connected", d.name) };
                    self.status.job(vec!["bluetoothctl".into(), verb.into(), d.mac.clone()], None, done);
                    self.ov.dirty = true;
                }
            }
            (Some(Hot::PassGo), _) => self.send_password(),
            (Some(Hot::Taskbar), _) => {
                self.overview_close(None);
                self.taskbar_open();
            }
            (Some(Hot::PassField), _) => {}
            (Some(Hot::ListSettings(page)), _) => {
                status::spawn(&format!("{} --page {}", self.cfg.apps.settings, page));
                self.overview_close(None);
            }
            (Some(Hot::Shelf(w)), _) => {
                self.untuck(w);
                self.overview_close(Some(w));
            }
            (Some(Hot::Slider(name, r)), _) => {
                self.ov.drag_slider = Some(name);
                self.slide(name, r, x);
            }
            (Some(Hot::Blank), _) => {}
            (Some(Hot::Card(_)), Some(f)) | (None, Some(f)) => {
                if let Some(w) = self.client(f).map(|c| c.win) {
                    if button == 2 { self.close(w) } else { self.overview_close(Some(w)) }
                }
            }
            _ => self.overview_close(None),
        }
    }

    /// A network was picked in the panel: leave it, join it, or ask for its password.
    fn pick_network(&mut self, i: usize) {
        let Some(n) = self.ov.snap.nets.get(i).cloned() else { return };
        self.ov.pass_for = None;
        self.ov.pass.clear();
        if n.active {
            let id = if n.profile.is_empty() { n.ssid.clone() } else { n.profile.clone() };
            self.status.job(vec!["nmcli".into(), "connection".into(), "down".into(), "id".into(), id], None, format!("Left {}", n.ssid));
        } else if n.secure && !n.known {
            if n.ssid.to_lowercase().contains("uc-secure") || n.ssid.to_lowercase().contains("eduroam") {
                // campus networks need a sign-in and a certificate: Settings sets them up properly
                status::spawn(&format!("{} --page network", self.cfg.apps.settings));
                self.overview_close(None);
                return;
            }
            self.ov.pass_for = Some(n.ssid.clone());
            self.ov.note = None;
        } else {
            self.status.job(status::join_args(&n, None), None, format!("Connected to {}", n.ssid));
        }
        self.ov.dirty = true;
    }

    fn send_password(&mut self) {
        let Some(ssid) = self.ov.pass_for.take() else { return };
        let pw = std::mem::take(&mut self.ov.pass);
        if let Some(n) = self.ov.snap.nets.iter().find(|n| n.ssid == ssid).cloned() {
            self.status.job(status::join_args(&n, Some(&pw)), Some(pw), format!("Connected to {}", n.ssid));
        }
        self.ov.dirty = true;
    }

    fn slide(&mut self, name: &str, r: Rect, x: i32) {
        let pct = (((x - r.x) as f32 / r.w.max(1) as f32) * 100.0).round().clamp(0.0, 100.0) as u8;
        if name == "volume" {
            self.ov.snap.volume = Some(pct);
            status::spawn(&status::set_volume(pct));
        } else {
            self.ov.snap.brightness = Some(pct);
            status::spawn(&status::set_brightness(pct));
        }
        self.ov.dirty = true;
    }

    pub fn overview_hover(&mut self, x: i32, y: i32) {
        if let Some(name) = self.ov.drag_slider {
            if let Some((r, _)) = self.ov.hot.iter().find(|h| matches!(h.1, Hot::Slider(n, _) if n == name)).cloned() {
                self.slide(name, r, x);
            }
            return;
        }
        let h = self.ov.hot.iter().rposition(|(r, _)| r.contains(x, y));
        if h != self.ov.hover {
            self.ov.hover = h;
            self.ov.dirty = true;
        }
    }

    /// Advance the open/close animation; returns true while it moves.
    pub fn overview_tick(&mut self) -> bool {
        if !self.ov.open {
            return false;
        }
        let dur = 0.22 * self.motion().max(0.001);
        let p = self.ov.start.map(|s| s.elapsed().as_secs_f32() / dur).unwrap_or(1.0);
        let p = if self.motion() <= 0.0 { 1.0 } else { p.min(1.0) };
        self.ov.t = if self.ov.closing { 1.0 - p } else { p };
        if self.ov.relayout {
            self.overview_layout();
            self.ov.dirty = true;
        }
        let s = self.status.get();
        if s != self.ov.snap && self.ov.drag_slider.is_none() {
            self.ov.snap = s;
            self.ov.dirty = true;
        }
        if let Some(n) = self.status.take_note() {
            self.ov.note = Some(n);
            self.ov.dirty = true;
        }
        if self.ov.closing && p >= 1.0 {
            self.ov.open = false;
            self.ov.closing = false;
            self.ov.t = 0.0;
            self.comp.drop_layer(&self.conn, "overview");
            return false;
        }
        if self.ov.dirty {
            self.draw_overview();
        }
        self.comp.layer_alpha("overview", draw_ease(self.ov.t));
        self.comp.dirty = true;
        p < 1.0
    }

    // ── drawing ──────────────────────────────────────────────────────────────────────────
    pub fn draw_overview(&mut self) {
        self.ov.dirty = false;
        let th = self.theme.clone();
        let s = self.scale;
        let m = self.ov.mon;
        let Some(mut px) = draw::Px::new(m.w as u32, m.h as u32) else { return };
        let mut hot: Vec<(Rect, Hot)> = vec![];
        let f = &self.fonts;
        let pad = 40.0 * s;
        let panel_w = PANEL_W * s;
        let zone_w = m.w as f32 - panel_w - 3.0 * pad;
        let cx = pad + zone_w / 2.0;
        let searching = !self.ov.query.is_empty();
        let hover = self.ov.hover;
        let is_hover = |hot: &Vec<(Rect, Hot)>| hover == Some(hot.len());
        let at = |r: Rect| Rect::new(r.x + m.x, r.y + m.y, r.w, r.h);

        let (_, apps_abs) = self.ov_regions(self.ov.order.len());
        let apps_r = Rect::new(apps_abs.x - m.x, apps_abs.y - m.y, apps_abs.w, apps_abs.h);
        let (mut cols_drawn, mut rows_drawn, mut scroll_drawn) = (self.ov.app_cols, self.ov.app_rows, self.ov.app_scroll);
        // the filter line: just a quiet hint until you type; then it shows what you typed
        let sw = (520.0 * s).min(zone_w);
        let (sx, sy, sh) = (cx - sw / 2.0, pad - 4.0 * s, 40.0 * s);
        if searching {
            draw::rrect(&mut px, sx, sy, sw, sh, sh / 2.0, th.glass);
            draw::rrect_line(&mut px, sx, sy, sw, sh, sh / 2.0, th.accent, 1.0);
            draw::icon(&mut px, "search", sx + 14.0 * s, sy + (sh - 16.0 * s) / 2.0, 16.0 * s, th.dim, 1.6);
            let ty = sy + (sh - 15.0 * s * 1.25) / 2.0;
            let w = f.draw(&mut px, Face::Ui, 15.0 * s, sx + 40.0 * s, ty, th.text, &self.ov.query);
            draw::rrect(&mut px, sx + 42.0 * s + w, sy + 11.0 * s, 2.0 * s, sh - 22.0 * s, 1.0, th.accent);
            hot.push((at(Rect::new(sx as i32, sy as i32, sw as i32, sh as i32)), Hot::Blank));
        } else {
            f.draw_centered(&mut px, Face::Ui, 12.5 * s, cx, sy + 12.0 * s, th.dim, "Type to find a window, an app or a setting  ·  = sums  ·  > commands");
        }

        if searching {
            // results list, above the dimmed cards
            let lw = sw.max((560.0 * s).min(zone_w));
            let (lx, mut ly) = (cx - lw / 2.0, sy + sh + 10.0 * s);
            let rows = self.ov.items.len();
            let mut groups = 0;
            let mut last = "";
            for it in &self.ov.items {
                if it.group != last { groups += 1; last = it.group; }
            }
            let lh = rows as f32 * 46.0 * s + groups as f32 * 28.0 * s + 16.0 * s + if rows == 0 { 46.0 * s } else { 0.0 };
            draw::rrect(&mut px, lx, ly, lw, lh, 14.0 * s, th.glass);
            draw::rrect_line(&mut px, lx, ly, lw, lh, 14.0 * s, th.line, 1.0);
            ly += 8.0 * s;
            if rows == 0 {
                f.draw(&mut px, Face::Ui, 13.5 * s, lx + 20.0 * s, ly + 12.0 * s, th.dim, "Nothing matches. Start with > to run it as a command.");
            }
            let mut last = "";
            for (i, it) in self.ov.items.iter().enumerate() {
                if it.group != last {
                    last = it.group;
                    f.draw(&mut px, Face::UiBold, 11.0 * s, lx + 18.0 * s, ly + 8.0 * s, th.dim, &it.group.to_uppercase());
                    ly += 28.0 * s;
                }
                let r = Rect::new((lx + 8.0 * s) as i32, ly as i32, (lw - 16.0 * s) as i32, (46.0 * s) as i32);
                if i == self.ov.sel || is_hover(&hot) {
                    draw::rrect(&mut px, r.x as f32, r.y as f32, r.w as f32, r.h as f32, 10.0 * s, if i == self.ov.sel { th.soft } else { th.text.with_alpha(14) });
                }
                let app_ic = match &it.act { Act::App(k) => self.ov.apps.get(*k).map(|a| a.icon.clone()), _ => None };
                let drew = app_ic.is_some_and(|ic| draw::app_icon(&mut px, &ic, r.x as f32 + 12.0 * s, r.y as f32 + 9.0 * s, 28.0 * s));
                if !drew {
                    draw::rrect(&mut px, r.x as f32 + 12.0 * s, r.y as f32 + 9.0 * s, 28.0 * s, 28.0 * s, 8.0 * s, th.raised);
                    draw::icon(&mut px, it.icon, r.x as f32 + 18.0 * s, r.y as f32 + 15.0 * s, 16.0 * s, th.text, 1.6);
                }
                let tx = r.x as f32 + 52.0 * s;
                let maxw = r.w as f32 - 52.0 * s - 90.0 * s;
                f.draw(&mut px, Face::UiBold, 13.5 * s, tx, r.y as f32 + 6.0 * s, th.text, &f.fit(Face::UiBold, 13.5 * s, &it.title, maxw));
                f.draw(&mut px, Face::Ui, 11.5 * s, tx, r.y as f32 + 25.0 * s, th.dim, &f.fit(Face::Ui, 11.5 * s, &it.sub, maxw));
                if i == self.ov.sel {
                    f.draw_right(&mut px, Face::Ui, 11.5 * s, (r.x + r.w) as f32 - 12.0 * s, r.y as f32 + 15.0 * s, th.dim, "Enter");
                }
                hot.push((at(r), Hot::Item(i)));
                ly += 46.0 * s;
            }
        } else {
            // every app, pinned first: icons in a grid, or a compact list
            let order = self.app_order();
            let pins = self.pinned().len();
            let icons = self.cfg.look.apps_view != "list";
            f.draw(&mut px, Face::UiBold, 11.0 * s, apps_r.x as f32 + 4.0 * s, apps_r.y as f32 - 24.0 * s, th.dim, &format!("APPS  ·  {}", order.len()));
            let (cw, chh) = if icons {
                (104.0 * s, 98.0 * s)
            } else {
                let c = (apps_r.w as f32 / (240.0 * s)).floor().max(1.0);
                (apps_r.w as f32 / c, 40.0 * s)
            };
            let cols = ((apps_r.w as f32 / cw).floor() as usize).max(1);
            let rows_vis = ((apps_r.h as f32 / chh).floor() as usize).max(1);
            let total_rows = order.len().div_ceil(cols);
            let max_scroll = total_rows.saturating_sub(rows_vis);
            let scroll = self.ov.app_scroll.min(max_scroll);
            let x0 = apps_r.x as f32 + (apps_r.w as f32 - cols as f32 * cw) / 2.0;
            for (k, &i) in order.iter().enumerate().skip(scroll * cols).take(rows_vis * cols) {
                let a = &self.ov.apps[i];
                let col = k % cols;
                let row = k / cols - scroll;
                let ax = x0 + col as f32 * cw;
                let ay = apps_r.y as f32 + row as f32 * chh;
                let sel = self.ov.zone == 1 && self.ov.sel == k;
                let r = Rect::new(ax as i32, ay as i32, cw as i32, (chh - 4.0 * s) as i32);
                let hv = is_hover(&hot);
                if sel || hv {
                    draw::rrect(&mut px, ax + 4.0 * s, ay, cw - 8.0 * s, chh - 6.0 * s, 12.0 * s, if sel { th.soft } else { th.text.with_alpha(14) });
                }
                if sel {
                    draw::rrect_line(&mut px, ax + 4.0 * s, ay, cw - 8.0 * s, chh - 6.0 * s, 12.0 * s, th.accent, 1.5);
                }
                if icons {
                    let isz = 48.0 * s;
                    let ix = ax + (cw - isz) / 2.0;
                    let iy = ay + 10.0 * s;
                    if !draw::app_icon(&mut px, &a.icon, ix, iy, isz) {
                        draw::rrect(&mut px, ix, iy, isz, isz, 12.0 * s, th.raised);
                        draw::rrect_line(&mut px, ix, iy, isz, isz, 12.0 * s, th.line, 1.0);
                        draw::icon(&mut px, app_icon(a), ix + (isz - 22.0 * s) / 2.0, iy + (isz - 22.0 * s) / 2.0, 22.0 * s, th.text, 1.5);
                    }
                    let name = f.fit(Face::Ui, 12.0 * s, &a.name, cw - 12.0 * s);
                    f.draw_centered(&mut px, Face::Ui, 12.0 * s, ax + cw / 2.0, iy + isz + 8.0 * s, th.text, &name);
                } else {
                    let isz = 24.0 * s;
                    let iy = ay + (chh - 6.0 * s - isz) / 2.0;
                    if !draw::app_icon(&mut px, &a.icon, ax + 12.0 * s, iy, isz) {
                        draw::icon(&mut px, app_icon(a), ax + 14.0 * s, iy + 2.0 * s, 20.0 * s, th.text, 1.5);
                    }
                    let name = f.fit(Face::Ui, 13.0 * s, &a.name, cw - 56.0 * s);
                    f.draw(&mut px, Face::Ui, 13.0 * s, ax + 46.0 * s, ay + 10.0 * s, th.text, &name);
                }
                if k < pins {
                    // pinned: a small accent dot
                    draw::circle(&mut px, ax + cw - 16.0 * s, ay + 10.0 * s, 3.0 * s, th.accent);
                }
                hot.push((at(r), Hot::App(i)));
            }
            if scroll > 0 {
                f.draw_right(&mut px, Face::Ui, 11.0 * s, (apps_r.x + apps_r.w) as f32, apps_r.y as f32 - 24.0 * s, th.dim, "▲ scroll for more");
            }
            if scroll < max_scroll {
                f.draw_centered(&mut px, Face::Ui, 11.0 * s, cx, (apps_r.y + apps_r.h) as f32 - 4.0 * s, th.dim, "▼ more apps: scroll or use the arrow keys");
            }
            cols_drawn = cols;
            rows_drawn = rows_vis;
            scroll_drawn = scroll;
        }

        // card titles and the selection ring (the cards themselves are the live windows)
        for (k, fr) in self.ov.order.iter().enumerate() {
            let Some(r) = self.ov.rects.get(fr) else { continue };
            let Some(c) = self.client(*fr) else { continue };
            let lr = Rect::new(r.x - m.x, r.y - m.y, r.w, r.h);
            let sel = !searching && self.ov.zone == 0 && self.ov.sel == k;
            if sel {
                draw::rrect_line(&mut px, lr.x as f32 - 4.0, lr.y as f32 - 4.0, lr.w as f32 + 8.0, lr.h as f32 + 8.0, 13.0 * s, th.accent, 2.5);
            }
            let title = if c.title.is_empty() { c.class.clone() } else { c.title.clone() };
            let t = f.fit(if sel { Face::UiBold } else { Face::Ui }, 13.0 * s, &title, lr.w as f32 + 40.0 * s);
            f.draw_centered(&mut px, if sel { Face::UiBold } else { Face::Ui }, 13.0 * s, lr.x as f32 + lr.w as f32 / 2.0, (lr.y + lr.h) as f32 + 10.0 * s, if sel { th.text } else { th.dim }, &t);
            hot.push((*r, Hot::Card(*fr)));
        }

        // minimized windows wait in the taskbar (hidden until called up): one quiet button
        let tucked = self.clients.iter().filter(|c| c.tucked).count();
        if tucked > 0 && !searching && self.cfg.taskbar.on {
            let label = format!("{tucked} minimized  ·  Super+T");
            let bw = f.measure(Face::Ui, 12.5 * s, &label) + 40.0 * s;
            let r = Rect::new((cx - bw / 2.0) as i32, (m.h as f32 - 96.0 * s) as i32, bw as i32, (34.0 * s) as i32);
            draw::rrect(&mut px, r.x as f32, r.y as f32, r.w as f32, r.h as f32, 17.0 * s, if is_hover(&hot) { th.raised } else { th.glass });
            draw::rrect_line(&mut px, r.x as f32, r.y as f32, r.w as f32, r.h as f32, 17.0 * s, th.line, 1.0);
            draw::icon(&mut px, "tuck", r.x as f32 + 14.0 * s, r.y as f32 + 10.0 * s, 14.0 * s, th.dim, 1.6);
            f.draw(&mut px, Face::Ui, 12.5 * s, r.x as f32 + 34.0 * s, r.y as f32 + 9.0 * s, th.text, &label);
            hot.push((at(r), Hot::Taskbar));
        }
        let hint = if searching { "Enter opens · Ctrl+Enter opens a new window · arrows move · Esc clears" } else if self.ov.zone == 1 { "Arrows move · Enter opens · right-click pins · Tab: windows · Esc back" } else { "Arrows move · Enter opens · Delete closes · Tab: apps · type to search · Esc back" };
        f.draw_centered(&mut px, Face::Ui, 12.5 * s, cx, m.h as f32 - 44.0 * s, th.dim, hint);

        // the panel
        self.draw_panel(&mut px, &mut hot, m);
        self.ov.hot = hot;
        self.ov.app_cols = cols_drawn;
        self.ov.app_rows = rows_drawn;
        self.ov.app_scroll = scroll_drawn;
        self.comp.set_layer(&self.conn, "overview", &px, m.x, m.y, draw_ease(self.ov.t));
    }

    /// The quick panel, compact: the time, six small toggle tiles, a row each for Wi-Fi and
    /// Bluetooth that opens into a short list, two slim sliders, the battery, and the power row.
    /// Its height follows what it holds.
    fn draw_panel(&self, px: &mut draw::Px, hot: &mut Vec<(Rect, Hot)>, m: Rect) {
        let th = &self.theme;
        let f = &self.fonts;
        let s = self.scale;
        let sn = &self.ov.snap;
        let pw = PANEL_W * s;
        let pad = 16.0 * s;
        let (x, y) = (m.w as f32 - pw - 40.0 * s, 40.0 * s);
        let at = |r: Rect| Rect::new(r.x + m.x, r.y + m.y, r.w, r.h);
        let hov = |hot: &Vec<(Rect, Hot)>| self.ov.hover == Some(hot.len());
        let ix = x + pad;
        let iw = pw - 2.0 * pad;
        let tile_h = 44.0 * s;
        let row_h = 34.0 * s;
        let item_h = 30.0 * s;
        let gap = 6.0 * s;

        // the height first, from the same measures used below
        let list_rows = match self.ov.expand {
            Some("wifi") => sn.nets.len().max(1),
            Some("bt") => sn.bt_devs.len().max(1),
            _ => 0,
        };
        let mut h = pad + 30.0 * s + 18.0 * s + 12.0 * s;           // time, date
        h += 2.0 * tile_h + gap + 10.0 * s;                          // tiles
        h += 2.0 * (row_h + 4.0 * s);                                // Wi-Fi, Bluetooth rows
        if list_rows > 0 {
            h += list_rows as f32 * item_h + item_h + 8.0 * s;       // list + its footer
            if self.ov.pass_for.is_some() { h += 40.0 * s; }
        }
        if self.ov.note.is_some() { h += 22.0 * s; }
        h += 8.0 * s + 2.0 * 30.0 * s;                               // sliders
        if sn.battery.is_some() { h += 26.0 * s; }
        h += 10.0 * s + 36.0 * s + gap + 36.0 * s + pad;             // power row, settings
        let h = h.min(m.h as f32 - 80.0 * s);
        draw::rrect(px, x, y, pw, h, 14.0 * s, th.glass);
        draw::rrect_line(px, x, y, pw, h, 14.0 * s, th.line, 1.0);
        hot.push((at(Rect::new(x as i32, y as i32, pw as i32, h as i32)), Hot::Blank));

        let mut cy = y + pad;
        let now = local_time();
        let w = f.draw(px, Face::UiBold, 26.0 * s, ix, cy, th.text, &format!("{:02}:{:02}", now.0, now.1));
        f.draw(px, Face::Ui, 12.5 * s, ix + w + 10.0 * s, cy + 11.0 * s, th.dim, &now.2);
        cy += 30.0 * s + 18.0 * s;

        // tiles: icon and one word; the state is the fill
        let tiles: [(&str, &str, &str, bool); 6] = [
            ("wifi", if sn.wifi_on { "wifi" } else { "wifi-off" }, "Wi-Fi", sn.wifi_on),
            ("bt", "bt", "Bluetooth", sn.bt_on),
            ("quiet", "quiet", "Quiet", sn.quiet),
            ("night", "sun", "Night light", sn.night_light),
            ("ground", if th.night { "moon" } else { "sun" }, if th.night { "Night" } else { "Paper" }, false),
            ("shot", "camera", "Screenshot", false),
        ];
        let tw = (iw - 2.0 * gap) / 3.0;
        for (i, (key, ic, name, on)) in tiles.iter().enumerate() {
            let tx = ix + (i % 3) as f32 * (tw + gap);
            let ty = cy + (i / 3) as f32 * (tile_h + gap);
            let r = Rect::new(tx as i32, ty as i32, tw as i32, tile_h as i32);
            let (bg, fg) = if *on { (th.accent, th.on_accent) } else if hov(hot) { (th.line, th.text) } else { (th.raised, th.text) };
            draw::rrect(px, tx, ty, tw, tile_h, 10.0 * s, bg);
            if !on { draw::rrect_line(px, tx, ty, tw, tile_h, 10.0 * s, th.line, 1.0); }
            draw::icon(px, ic, tx + (tw - 16.0 * s) / 2.0, ty + 6.0 * s, 16.0 * s, fg, 1.6);
            let label = f.fit(Face::Ui, 10.5 * s, name, tw - 6.0 * s);
            f.draw_centered(px, Face::Ui, 10.5 * s, tx + tw / 2.0, ty + 25.0 * s, fg, &label);
            hot.push((at(r), Hot::Tile(key)));
        }
        cy += 2.0 * tile_h + gap + 10.0 * s;

        // Wi-Fi and Bluetooth rows, each opening its list
        let wifi_sub = if !sn.have_nm { "NetworkManager is not running".to_string() } else if !sn.wifi_on { "Off".into() } else if sn.wired && sn.ssid.is_none() { "Wired".into() } else { sn.ssid.clone().unwrap_or_else(|| "Not connected".into()) };
        let bt_sub = if !sn.have_bt { "Not installed".to_string() } else if !sn.bt_on { "Off".into() } else { sn.bt_devs.iter().find(|d| d.connected).map(|d| d.name.clone()).unwrap_or_else(|| "On".into()) };
        for (key, ic, name, sub) in [("wifi", "wifi", "Wi-Fi", wifi_sub), ("bt", "bt", "Bluetooth", bt_sub)] {
            let open = self.ov.expand == Some(key);
            let r = Rect::new(ix as i32, cy as i32, iw as i32, row_h as i32);
            if open || hov(hot) {
                draw::rrect(px, ix, cy, iw, row_h, 8.0 * s, if open { th.soft } else { th.text.with_alpha(14) });
            }
            draw::icon(px, ic, ix + 8.0 * s, cy + (row_h - 15.0 * s) / 2.0, 15.0 * s, th.dim, 1.6);
            let nw = f.draw(px, Face::UiBold, 12.5 * s, ix + 32.0 * s, cy + 9.0 * s, th.text, name);
            let room = iw - 32.0 * s - nw - 10.0 * s - 26.0 * s;
            f.draw_right(px, Face::Ui, 12.0 * s, ix + iw - 26.0 * s, cy + 9.5 * s, th.dim, &f.fit(Face::Ui, 12.0 * s, &sub, room));
            draw::icon(px, if open { "chev-down" } else { "chev" }, ix + iw - 21.0 * s, cy + (row_h - 14.0 * s) / 2.0, 14.0 * s, th.dim, 1.6);
            hot.push((at(r), Hot::Expand(key)));
            cy += row_h + 4.0 * s;
            if !open { continue; }
            // the list, right under its row
            let lx = ix + 8.0 * s;
            let lw = iw - 8.0 * s;
            if key == "wifi" {
                if sn.nets.is_empty() {
                    f.draw(px, Face::Ui, 12.0 * s, lx + 24.0 * s, cy + 8.0 * s, th.dim, if sn.wifi_on { "Looking for networks…" } else { "Wi-Fi is off" });
                    cy += item_h;
                }
                for (i, n) in sn.nets.iter().enumerate() {
                    let r = Rect::new(lx as i32, cy as i32, lw as i32, item_h as i32);
                    if hov(hot) { draw::rrect(px, lx, cy, lw, item_h, 7.0 * s, th.text.with_alpha(14)); }
                    // signal: three bars
                    for b in 0..3 {
                        let lit = n.signal as u32 >= [1u32, 40, 70][b];
                        let bh = (4.0 + 3.0 * b as f32) * s;
                        draw::rrect(px, lx + 6.0 * s + b as f32 * 4.0 * s, cy + 20.0 * s - bh, 2.6 * s, bh, 1.0, if lit { th.text } else { th.line });
                    }
                    let face = if n.active { Face::UiBold } else { Face::Ui };
                    let right = if n.active { "Connected" } else if n.known { "Saved" } else { "" };
                    let rw = if right.is_empty() { 0.0 } else { f.measure(Face::Ui, 11.0 * s, right) + 8.0 * s };
                    let lock = if n.secure { 16.0 * s } else { 0.0 };
                    let name = f.fit(face, 12.5 * s, &n.ssid, lw - 26.0 * s - rw - lock - 6.0 * s);
                    let nw = f.draw(px, face, 12.5 * s, lx + 24.0 * s, cy + 8.0 * s, if n.active { th.accent } else { th.text }, &name);
                    if n.secure { draw::icon(px, "lock", lx + 28.0 * s + nw, cy + 9.0 * s, 11.0 * s, th.dim, 1.6); }
                    if !right.is_empty() { f.draw_right(px, Face::Ui, 11.0 * s, lx + lw - 4.0 * s, cy + 9.0 * s, th.dim, right); }
                    hot.push((at(r), Hot::Net(i)));
                    cy += item_h;
                    if self.ov.pass_for.as_deref() == Some(n.ssid.as_str()) {
                        // the password field, inline: dots only, never the text
                        let fw = lw - 60.0 * s;
                        let fr = Rect::new(lx as i32, (cy + 2.0 * s) as i32, fw as i32, (32.0 * s) as i32);
                        draw::rrect(px, lx, cy + 2.0 * s, fw, 32.0 * s, 8.0 * s, th.raised);
                        draw::rrect_line(px, lx, cy + 2.0 * s, fw, 32.0 * s, 8.0 * s, th.accent, 1.2);
                        let dots: String = "•".repeat(self.ov.pass.chars().count().min(24));
                        if dots.is_empty() {
                            f.draw(px, Face::Ui, 12.0 * s, lx + 10.0 * s, cy + 10.0 * s, th.dim, "Password");
                        } else {
                            f.draw(px, Face::Ui, 12.5 * s, lx + 10.0 * s, cy + 10.0 * s, th.text, &dots);
                        }
                        hot.push((at(fr), Hot::PassField));
                        let gr = Rect::new((lx + fw + 6.0 * s) as i32, (cy + 2.0 * s) as i32, (54.0 * s) as i32, (32.0 * s) as i32);
                        draw::rrect(px, gr.x as f32, gr.y as f32, gr.w as f32, gr.h as f32, 8.0 * s, th.accent);
                        f.draw_centered(px, Face::UiBold, 12.0 * s, gr.x as f32 + gr.w as f32 / 2.0, cy + 10.0 * s, th.on_accent, "Join");
                        hot.push((at(gr), Hot::PassGo));
                        cy += 40.0 * s;
                    }
                }
            } else {
                if sn.bt_devs.is_empty() {
                    f.draw(px, Face::Ui, 12.0 * s, lx + 24.0 * s, cy + 8.0 * s, th.dim, if sn.bt_on { "No paired devices" } else { "Bluetooth is off" });
                    cy += item_h;
                }
                for (i, d) in sn.bt_devs.iter().enumerate() {
                    let r = Rect::new(lx as i32, cy as i32, lw as i32, item_h as i32);
                    if hov(hot) { draw::rrect(px, lx, cy, lw, item_h, 7.0 * s, th.text.with_alpha(14)); }
                    draw::circle(px, lx + 10.0 * s, cy + item_h / 2.0, 3.5 * s, if d.connected { th.accent } else { th.line });
                    let right = if d.connected { "Connected" } else { "Connect" };
                    let rw = f.measure(Face::Ui, 11.0 * s, right) + 8.0 * s;
                    f.draw(px, if d.connected { Face::UiBold } else { Face::Ui }, 12.5 * s, lx + 24.0 * s, cy + 8.0 * s, th.text, &f.fit(Face::Ui, 12.5 * s, &d.name, lw - 30.0 * s - rw));
                    f.draw_right(px, Face::Ui, 11.0 * s, lx + lw - 4.0 * s, cy + 9.0 * s, th.dim, right);
                    hot.push((at(r), Hot::Bt(i)));
                    cy += item_h;
                }
            }
            let page = if key == "wifi" { "network" } else { "network" };
            let r = Rect::new(lx as i32, cy as i32, lw as i32, item_h as i32);
            f.draw(px, Face::Ui, 12.0 * s, lx + 24.0 * s, cy + 8.0 * s, if hov(hot) { th.text } else { th.accent }, if key == "wifi" { "Network settings…" } else { "Bluetooth settings…" });
            hot.push((at(r), Hot::ListSettings(page)));
            cy += item_h + 8.0 * s;
        }
        if let Some((text, bad)) = &self.ov.note {
            let t = f.fit(Face::Ui, 11.5 * s, text, iw - 8.0 * s);
            f.draw(px, Face::Ui, 11.5 * s, ix + 4.0 * s, cy + 2.0 * s, if *bad { Rgba(220, 90, 70, 255) } else { th.dim }, &t);
            cy += 22.0 * s;
        }

        // sliders, slim
        cy += 8.0 * s;
        for (key, ic, val) in [("volume", if sn.muted { "mute" } else { "sound" }, sn.volume), ("brightness", "bright", sn.brightness)] {
            draw::icon(px, ic, ix, cy + 3.0 * s, 15.0 * s, th.dim, 1.6);
            let (tx, tw2) = (ix + 26.0 * s, iw - 26.0 * s - 34.0 * s);
            let v = val.unwrap_or(0).min(100) as f32 / 100.0;
            draw::rrect(px, tx, cy + 8.0 * s, tw2, 4.0 * s, 2.0 * s, th.line);
            draw::rrect(px, tx, cy + 8.0 * s, tw2 * v, 4.0 * s, 2.0 * s, if val.is_some() { th.accent } else { th.line });
            draw::circle(px, tx + tw2 * v, cy + 10.0 * s, 6.5 * s, th.raised);
            draw::circle(px, tx + tw2 * v, cy + 10.0 * s, 5.5 * s, if val.is_some() { th.accent } else { th.dim });
            f.draw_right(px, Face::Ui, 11.5 * s, ix + iw, cy + 3.0 * s, th.text, &val.map(|v| v.to_string()).unwrap_or_else(|| "—".into()));
            let r = Rect::new((tx - 8.0 * s) as i32, (cy - 4.0 * s) as i32, (tw2 + 16.0 * s) as i32, (28.0 * s) as i32);
            hot.push((at(Rect::new(tx as i32, r.y, tw2 as i32, r.h)), Hot::Slider(key, at(Rect::new(tx as i32, r.y, tw2 as i32, r.h)))));
            cy += 30.0 * s;
        }
        if let Some((pct, charging)) = sn.battery {
            draw::icon(px, "battery", ix, cy + 3.0 * s, 15.0 * s, th.dim, 1.6);
            let w = f.draw(px, Face::UiBold, 12.0 * s, ix + 26.0 * s, cy + 3.0 * s, th.text, &format!("{pct}%"));
            f.draw(px, Face::Ui, 12.0 * s, ix + 32.0 * s + w, cy + 3.0 * s, th.dim, if charging { "charging" } else { "on battery" });
            cy += 26.0 * s;
        }

        // power row and Settings, small
        cy += 10.0 * s;
        let bh = 36.0 * s;
        let bw = (iw - 3.0 * gap) / 4.0;
        for (i, (key, ic)) in [("lock", "lock"), ("sleep", "sleep"), ("restart", "restart"), ("poweroff", "power")].iter().enumerate() {
            let bx = ix + i as f32 * (bw + gap);
            let r = Rect::new(bx as i32, cy as i32, bw as i32, bh as i32);
            draw::rrect(px, bx, cy, bw, bh, 9.0 * s, if hov(hot) { th.line } else { th.raised });
            draw::rrect_line(px, bx, cy, bw, bh, 9.0 * s, th.line, 1.0);
            draw::icon(px, ic, bx + (bw - 16.0 * s) / 2.0, cy + (bh - 16.0 * s) / 2.0, 16.0 * s, if *key == "poweroff" { th.accent } else { th.text }, 1.6);
            hot.push((at(r), Hot::Power(key)));
        }
        cy += bh + gap;
        let r = Rect::new(ix as i32, cy as i32, iw as i32, bh as i32);
        draw::rrect(px, ix, cy, iw, bh, 9.0 * s, if hov(hot) { th.line } else { th.raised });
        draw::rrect_line(px, ix, cy, iw, bh, 9.0 * s, th.line, 1.0);
        let lw = f.measure(Face::UiBold, 12.5 * s, "Settings") + 24.0 * s;
        draw::icon(px, "settings", ix + (iw - lw) / 2.0, cy + (bh - 15.0 * s) / 2.0, 15.0 * s, th.text, 1.6);
        f.draw(px, Face::UiBold, 12.5 * s, ix + (iw - lw) / 2.0 + 24.0 * s, cy + 10.0 * s, th.text, "Settings");
        hot.push((at(r), Hot::Settings));
    }
}

fn draw_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Local hours, minutes and a written date, from the C library (respects TZ).
pub fn local_time() -> (u32, u32, String) {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let date = format!("{}, {} {}", DAYS[tm.tm_wday as usize % 7], tm.tm_mday, MONTHS[tm.tm_mon as usize % 12]);
        (tm.tm_hour as u32, tm.tm_min as u32, date)
    }
}

pub fn short_date() -> String {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        format!("{} {} {}", DAYS[tm.tm_wday as usize % 7], tm.tm_mday, MONTHS[tm.tm_mon as usize % 12])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calculator() {
        assert_eq!(calc("2+3*4"), Some(14.0));
        assert_eq!(calc("(2+3)*4"), Some(20.0));
        assert_eq!(calc("2^10"), Some(1024.0));
        assert_eq!(calc("1,5*2"), Some(3.0));
        assert_eq!(calc("-3+1"), Some(-2.0));
        assert_eq!(calc("1/0"), None);
        assert_eq!(calc("2+"), None);
        assert_eq!(fmt_num(0.1 + 0.2), "0.3");
    }
    #[test]
    fn field_codes_are_dropped() {
        assert_eq!(strip_codes("firefox %u"), "firefox");
        assert_eq!(strip_codes("mpv --player-operation-mode=pseudo-gui -- %U"), "mpv --player-operation-mode=pseudo-gui --");
    }
}
