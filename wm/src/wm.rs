//! The window manager: frames, focus, move/resize, snapping, EWMH. The compositor (comp.rs)
//! paints what this file arranges; the overview, lock and quick panel live in their own files.

use crate::comp::Comp;
use crate::config::{self, Config};
use crate::theme::{self, Rgba, Theme};
use crate::x::{self, Atoms, Keymap};
use crate::{draw, ipc, lock, overview, status};
use std::collections::HashMap;
use std::time::Instant;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::CURRENT_TIME;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
    pub fn cx(&self) -> i32 {
        self.x + self.w / 2
    }
    pub fn cy(&self) -> i32 {
        self.y + self.h / 2
    }
    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2 * d).max(1), (self.h - 2 * d).max(1))
    }
    pub fn overlap(&self, o: &Rect) -> i64 {
        let w = (self.x + self.w).min(o.x + o.w) - self.x.max(o.x);
        let h = (self.y + self.h).min(o.y + o.h) - self.y.max(o.y);
        if w > 0 && h > 0 { w as i64 * h as i64 } else { 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Normal,
    Dialog,
    Utility,
    Splash,
}

#[derive(Clone, Debug)]
pub struct Client {
    pub win: Window,
    pub frame: Window,
    /// The client's own area, in root coordinates.
    pub rect: Rect,
    pub title: String,
    pub class: String,
    pub instance: String,
    pub kind: Kind,
    pub transient: Option<Window>,
    /// Our title strip is drawn (false for apps that draw their own: GTK headerbars, Firefox).
    pub strip: bool,
    pub tucked: bool,
    pub fullscreen: bool,
    pub filled: bool,
    /// Where the window was before it was filled, snapped or made fullscreen.
    pub restore: Option<Rect>,
    pub snapped: Option<String>,
    pub urgent: bool,
    pub delete: bool,
    pub take_focus: bool,
    pub accepts_input: bool,
    pub min: (i32, i32),
    pub max: (i32, i32),
    pub strip_color: Option<Rgba>,
    pub pid: u32,
    pub ignore_unmaps: u32,
    pub hover: Option<u8>,
    /// floated by hand (Super+V, or dragged): left out of the tiling
    pub floating: bool,
    /// the app's own wish (_NET_WM_WINDOW_OPACITY), 0..1
    pub opacity_hint: Option<f32>,
    /// the (hidden) workspace it is on, 1..9
    pub ws: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    Move { win: Window, dx: i32, dy: i32, start: (i32, i32), moved: bool },
    Resize { win: Window, edges: u8, start: (i32, i32), orig: Rect },
}

pub const E_LEFT: u8 = 1;
pub const E_RIGHT: u8 = 2;
pub const E_TOP: u8 = 4;
pub const E_BOTTOM: u8 = 8;

pub struct Cursors {
    pub normal: Cursor,
    pub mov: Cursor,
    pub edges: HashMap<u8, Cursor>,
}

pub struct Wm {
    pub conn: RustConnection,
    pub screen: usize,
    pub root: Window,
    pub atoms: Atoms,
    pub keymap: Keymap,
    pub cfg: Config,
    pub cfg_error: Option<String>,
    pub cfg_mtime: Option<std::time::SystemTime>,
    pub theme: Theme,
    pub fonts: draw::Fonts,
    pub scale: f32,
    pub clients: Vec<Client>,
    /// Focus history, most recent first.
    pub mru: Vec<Window>,
    pub focused: Option<Window>,
    pub monitors: Vec<Rect>,
    pub struts: HashMap<Window, [u32; 4]>,
    pub drag: Option<Drag>,
    pub snap_preview: Option<Rect>,
    pub comp: Comp,
    pub ov: overview::Overview,
    pub assist: crate::assist::Assist,
    pub taskbar: crate::taskbar::Taskbar,
    /// the workspace on screen (1..9; there are no visible workspaces, only keys)
    pub ws: u32,
    /// the order tiled windows are laid out in
    pub tile_order: Vec<Window>,
    pub lock: lock::Lock,
    pub status: status::Status,
    pub ipc: Option<ipc::Server>,
    pub binds: Vec<(u16, u32, String)>,
    pub super_down: bool,
    pub super_used: bool,
    pub alt_tab: Option<usize>,
    pub check: Window,
    pub cursors: Cursors,
    pub running: bool,
    /// leave the loop and start this program again in place (after an update)
    pub restart: bool,
    pub last_click: (Window, Instant),
    pub shake: Vec<(Instant, i32)>,
    pub shaken: Vec<Window>,
    pub places: HashMap<String, Rect>,
    pub notice: Option<(String, Instant)>,
}

impl Wm {
    pub fn new() -> Result<Self, String> {
        let (conn, screen) = x11rb::connect(None).map_err(|e| format!("cannot open the display: {e}"))?;
        let root = conn.setup().roots[screen].root;
        // become the window manager; fails if another one runs
        let mask = EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::STRUCTURE_NOTIFY
            | EventMask::PROPERTY_CHANGE
            | EventMask::BUTTON_PRESS
            | EventMask::POINTER_MOTION;
        conn.change_window_attributes(root, &ChangeWindowAttributesAux::new().event_mask(mask))
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|_| "another window manager is already running".to_string())?;
        let atoms = Atoms::new(&conn).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
        let keymap = Keymap::load(&conn).ok_or("cannot read the keyboard map")?;
        let (cfg, cfg_error) = config::load();
        let theme = Theme::new(cfg.look.ground == "night", &cfg.look.accent);
        let fonts = draw::Fonts::load(&cfg.look.ui_font, &cfg.look.mono_font);
        let scale = cfg.look.scale;
        let cursors = Self::load_cursors(&conn, screen, root);
        conn.change_window_attributes(root, &ChangeWindowAttributesAux::new().cursor(cursors.normal)).ok();
        let check = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_window(0, check, root, -10, -10, 1, 1, 0, WindowClass::INPUT_ONLY, x11rb::COPY_FROM_PARENT, &CreateWindowAux::new())
            .map_err(|e| e.to_string())?;
        let comp = Comp::new(&conn, screen, root, &atoms, check)?;
        let mut wm = Wm {
            conn,
            screen,
            root,
            atoms,
            keymap,
            cfg_mtime: config::mtime(),
            cfg,
            cfg_error,
            theme,
            fonts,
            scale,
            clients: vec![],
            mru: vec![],
            focused: None,
            monitors: vec![],
            struts: HashMap::new(),
            drag: None,
            snap_preview: None,
            comp,
            ov: overview::Overview::default(),
            assist: crate::assist::Assist::default(),
            taskbar: crate::taskbar::Taskbar::default(),
            ws: 1,
            tile_order: vec![],
            lock: lock::Lock::default(),
            status: status::Status::start(),
            ipc: ipc::Server::bind(),
            binds: vec![],
            super_down: false,
            super_used: false,
            alt_tab: None,
            check,
            cursors,
            running: true,
            restart: false,
            last_click: (0, Instant::now()),
            shake: vec![],
            shaken: vec![],
            places: load_places(),
            notice: None,
        };
        wm.setup_ewmh();
        wm.load_monitors();
        wm.grab_keys();
        wm.conn.randr_select_input(root, x11rb::protocol::randr::NotifyMask::SCREEN_CHANGE).ok();
        wm.adopt_existing();
        wm.comp.load_wallpapers(&wm.conn, &wm.cfg, &wm.monitors);
        wm.ov.load_apps();
        if let Some(e) = wm.cfg_error.clone() {
            wm.say(format!("Settings file has a mistake; using defaults. {e}"));
        }
        wm.conn.flush().ok();
        Ok(wm)
    }

    fn load_cursors(conn: &RustConnection, screen: usize, _root: Window) -> Cursors {
        let db = x11rb::resource_manager::new_from_default(conn).unwrap_or_else(|_| x11rb::resource_manager::Database::new_from_data(b""));
        let handle = x11rb::cursor::Handle::new(conn, screen, &db).ok().and_then(|c| c.reply().ok());
        let load = |name: &str| handle.as_ref().and_then(|h| h.load_cursor(conn, name).ok()).unwrap_or(x11rb::NONE);
        let mut edges = HashMap::new();
        for (e, n) in [
            (E_LEFT, "left_side"),
            (E_RIGHT, "right_side"),
            (E_TOP, "top_side"),
            (E_BOTTOM, "bottom_side"),
            (E_TOP | E_LEFT, "top_left_corner"),
            (E_TOP | E_RIGHT, "top_right_corner"),
            (E_BOTTOM | E_LEFT, "bottom_left_corner"),
            (E_BOTTOM | E_RIGHT, "bottom_right_corner"),
        ] {
            edges.insert(e, load(n));
        }
        Cursors { normal: load("left_ptr"), mov: load("fleur"), edges }
    }

    fn setup_ewmh(&mut self) {
        let a = &self.atoms;
        let supported = [
            a._NET_SUPPORTED, a._NET_SUPPORTING_WM_CHECK, a._NET_WM_NAME, a._NET_CLIENT_LIST, a._NET_CLIENT_LIST_STACKING,
            a._NET_ACTIVE_WINDOW, a._NET_CLOSE_WINDOW, a._NET_WM_STATE, a._NET_WM_STATE_FULLSCREEN, a._NET_WM_STATE_HIDDEN,
            a._NET_WM_STATE_MAXIMIZED_VERT, a._NET_WM_STATE_MAXIMIZED_HORZ, a._NET_WM_STATE_DEMANDS_ATTENTION,
            a._NET_WM_STATE_FOCUSED, a._NET_WM_WINDOW_TYPE, a._NET_WM_WINDOW_TYPE_NORMAL, a._NET_WM_WINDOW_TYPE_DIALOG,
            a._NET_WM_WINDOW_TYPE_DOCK, a._NET_WM_WINDOW_TYPE_DESKTOP, a._NET_WM_WINDOW_TYPE_SPLASH, a._NET_WM_WINDOW_TYPE_UTILITY,
            a._NET_FRAME_EXTENTS, a._NET_REQUEST_FRAME_EXTENTS, a._NET_WORKAREA, a._NET_WM_STRUT, a._NET_WM_STRUT_PARTIAL,
            a._NET_MOVERESIZE_WINDOW, a._NET_WM_MOVERESIZE, a._NET_NUMBER_OF_DESKTOPS, a._NET_CURRENT_DESKTOP,
            a._NET_WM_DESKTOP, a._NET_DESKTOP_GEOMETRY, a._NET_DESKTOP_VIEWPORT, a._NET_WM_ALLOWED_ACTIONS,
        ];
        let (root, check) = (self.root, self.check);
        x::set_u32s(&self.conn, root, a._NET_SUPPORTED, AtomEnum::ATOM, &supported);
        x::set_u32s(&self.conn, root, a._NET_SUPPORTING_WM_CHECK, AtomEnum::WINDOW, &[check]);
        x::set_u32s(&self.conn, check, a._NET_SUPPORTING_WM_CHECK, AtomEnum::WINDOW, &[check]);
        self.conn.change_property8(PropMode::REPLACE, check, a._NET_WM_NAME, a.UTF8_STRING, b"Samurai").ok();
        // one desktop, always: apps that ask get a sensible answer
        x::set_u32s(&self.conn, root, a._NET_NUMBER_OF_DESKTOPS, AtomEnum::CARDINAL, &[crate::workspace::COUNT]);
        x::set_u32s(&self.conn, root, a._NET_CURRENT_DESKTOP, AtomEnum::CARDINAL, &[0]);
        x::set_u32s(&self.conn, root, a._NET_DESKTOP_VIEWPORT, AtomEnum::CARDINAL, &[0, 0]);
        self.conn.change_property8(PropMode::REPLACE, root, a._NET_DESKTOP_NAMES, a.UTF8_STRING, b"Samurai\0").ok();
    }

    pub fn load_monitors(&mut self) {
        let mut mons: Vec<Rect> = self
            .conn
            .randr_get_monitors(self.root, true)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.monitors.iter().map(|m| Rect::new(m.x as i32, m.y as i32, m.width as i32, m.height as i32)).collect())
            .unwrap_or_default();
        // tests: a file of "x y w h" lines stands in for the monitors (a virtual X server has one)
        if let Some(f) = std::env::var_os("SAMURAI_TEST_MONITORS") {
            let fake: Vec<Rect> = std::fs::read_to_string(f).unwrap_or_default().lines().filter_map(|l| {
                let v: Vec<i32> = l.split_whitespace().filter_map(|n| n.parse().ok()).collect();
                (v.len() == 4).then(|| Rect::new(v[0], v[1], v[2], v[3]))
            }).collect();
            if !fake.is_empty() {
                mons = fake;
            }
        }
        if mons.is_empty() {
            let s = &self.conn.setup().roots[self.screen];
            mons.push(Rect::new(0, 0, s.width_in_pixels as i32, s.height_in_pixels as i32));
        }
        self.monitors = mons;
        let (w, h) = self.screen_size();
        x::set_u32s(&self.conn, self.root, self.atoms._NET_DESKTOP_GEOMETRY, AtomEnum::CARDINAL, &[w as u32, h as u32]);
        self.publish_workarea();
    }

    /// A screen was plugged in, unplugged or changed: read them again, bring windows that are
    /// now off every screen onto one (keeping their place relative to the screen they were on),
    /// and lay things out again. No restart needed.
    pub fn screens_changed(&mut self) {
        let before = self.monitors.clone();
        self.load_monitors();
        self.comp.resize(&self.conn, self.screen_size());
        self.comp.load_wallpapers(&self.conn, &self.cfg, &self.monitors);
        let lost: Vec<Window> = self.clients.iter().filter(|c| !self.monitors.iter().any(|m| m.overlap(&self.visible_rect(c)) > 0)).map(|c| c.win).collect();
        let home = self.monitors[0];
        for w in lost {
            let Some(c) = self.client(w).cloned() else { continue };
            let vis = self.visible_rect(&c);
            let old = before.iter().copied().find(|m| m.overlap(&vis) > 0).unwrap_or(home);
            let wa = self.workarea(&home);
            let (nw, nh) = (vis.w.min(wa.w), vis.h.min(wa.h));
            let fx = (vis.x - old.x) as f32 / old.w.max(1) as f32;
            let fy = (vis.y - old.y) as f32 / old.h.max(1) as f32;
            let area = Rect::new((wa.x + (fx * wa.w as f32) as i32).clamp(wa.x, wa.x + wa.w - nw), (wa.y + (fy * wa.h as f32) as i32).clamp(wa.y, wa.y + wa.h - nh), nw, nh);
            let to = self.client_rect_for(&c, area);
            if let Some(cm) = self.client_mut(w) {
                cm.snapped = None;
                cm.restore = None;
            }
            self.move_to(w, to);
        }
        self.retile();
        self.ov.relayout = true;
        if self.taskbar.open {
            self.draw_taskbar();
        }
    }

    pub fn screen_size(&self) -> (i32, i32) {
        let w = self.monitors.iter().map(|m| m.x + m.w).max().unwrap_or(1);
        let h = self.monitors.iter().map(|m| m.y + m.h).max().unwrap_or(1);
        (w, h)
    }

    /// The monitor under the pointer.
    pub fn pointer_monitor(&self) -> Rect {
        let p = self.conn.query_pointer(self.root).ok().and_then(|c| c.reply().ok());
        let (x, y) = p.map(|p| (p.root_x as i32, p.root_y as i32)).unwrap_or((0, 0));
        self.monitor_at(x, y)
    }

    pub fn monitor_at(&self, x: i32, y: i32) -> Rect {
        self.monitors.iter().copied().find(|m| m.contains(x, y)).unwrap_or(self.monitors[0])
    }

    pub fn monitor_of(&self, r: &Rect) -> Rect {
        self.monitors.iter().copied().max_by_key(|m| m.overlap(r)).filter(|m| m.overlap(r) > 0).unwrap_or_else(|| self.monitor_at(r.cx(), r.cy()))
    }

    /// The monitor minus space docks reserve (Samurai itself reserves none).
    pub fn workarea(&self, mon: &Rect) -> Rect {
        let (sw, sh) = self.screen_size();
        let (mut l, mut r, mut t, mut b) = (mon.x, mon.x + mon.w, mon.y, mon.y + mon.h);
        for s in self.struts.values() {
            if s[0] > 0 && (s[0] as i32) > l && mon.x == 0 { l = l.max(s[0] as i32); }
            if s[1] > 0 && mon.x + mon.w == sw { r = r.min(sw - s[1] as i32); }
            if s[2] > 0 && mon.y == 0 { t = t.max(s[2] as i32); }
            if s[3] > 0 && mon.y + mon.h == sh { b = b.min(sh - s[3] as i32); }
        }
        Rect::new(l, t, r - l, b - t)
    }

    fn publish_workarea(&self) {
        let wa = self.workarea(&self.monitors[0]);
        x::set_u32s(&self.conn, self.root, self.atoms._NET_WORKAREA, AtomEnum::CARDINAL, &[wa.x as u32, wa.y as u32, wa.w as u32, wa.h as u32]);
    }

    // ── measures that follow the interface scale ──────────────────────────────────────────
    pub fn strip_h(&self, c: &Client) -> i32 {
        if c.fullscreen || !c.strip { 0 } else { (theme::STRIP * self.scale).round() as i32 }
    }
    pub fn border(&self, c: &Client) -> i32 {
        if c.fullscreen { 0 } else { theme::BORDER }
    }
    pub fn frame_rect(&self, c: &Client) -> Rect {
        let (b, s) = (self.border(c), self.strip_h(c));
        Rect::new(c.rect.x - b, c.rect.y - b - s, c.rect.w + 2 * b, c.rect.h + 2 * b + s)
    }
    /// What the user sees of a frame: the strip and the client, without the grab margin.
    pub fn visible_rect(&self, c: &Client) -> Rect {
        let f = self.frame_rect(c);
        f.inset(self.border(c))
    }
    pub fn radius(&self, c: &Client) -> f32 {
        if c.fullscreen { 0.0 } else { self.cfg.look.corners * self.scale }
    }

    pub fn client(&self, w: Window) -> Option<&Client> {
        self.clients.iter().find(|c| c.win == w || c.frame == w)
    }
    pub fn client_mut(&mut self, w: Window) -> Option<&mut Client> {
        self.clients.iter_mut().find(|c| c.win == w || c.frame == w)
    }
    pub fn idx(&self, w: Window) -> Option<usize> {
        self.clients.iter().position(|c| c.win == w || c.frame == w)
    }

    pub fn say(&mut self, text: String) {
        eprintln!("samurai-wm: {text}");
        self.notice = Some((text, Instant::now()));
        self.comp.dirty = true;
    }

    // ── managing windows ──────────────────────────────────────────────────────────────────
    fn adopt_existing(&mut self) {
        let Some(tree) = self.conn.query_tree(self.root).ok().and_then(|c| c.reply().ok()) else { return };
        for &w in &tree.children {
            if w == self.check || w == self.comp.overlay || self.comp.is_ours(w) {
                continue;
            }
            let Some(attr) = self.conn.get_window_attributes(w).ok().and_then(|c| c.reply().ok()) else { continue };
            if attr.override_redirect || attr.map_state != MapState::VIEWABLE {
                continue;
            }
            self.manage(w);
        }
    }

    fn window_type(&self, w: Window) -> Vec<Atom> {
        x::get_u32s(&self.conn, w, self.atoms._NET_WM_WINDOW_TYPE, AtomEnum::ATOM)
    }

    pub fn manage(&mut self, win: Window) {
        if self.client(win).is_some() {
            return;
        }
        let a = self.atoms;
        let types = self.window_type(win);
        // docks and desktop windows are shown as they are, never framed
        if types.contains(&a._NET_WM_WINDOW_TYPE_DOCK) || types.contains(&a._NET_WM_WINDOW_TYPE_DESKTOP) {
            let s = x::get_u32s(&self.conn, win, a._NET_WM_STRUT_PARTIAL, AtomEnum::CARDINAL);
            let s = if s.len() >= 4 { s } else { x::get_u32s(&self.conn, win, a._NET_WM_STRUT, AtomEnum::CARDINAL) };
            if s.len() >= 4 {
                self.struts.insert(win, [s[0], s[1], s[2], s[3]]);
                self.publish_workarea();
            }
            self.conn.change_window_attributes(win, &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY)).ok();
            self.conn.map_window(win).ok();
            if types.contains(&a._NET_WM_WINDOW_TYPE_DESKTOP) {
                self.conn.configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::BELOW)).ok();
            }
            return;
        }
        let Some(geo) = self.conn.get_geometry(win).ok().and_then(|c| c.reply().ok()) else { return };
        let kind = if types.contains(&a._NET_WM_WINDOW_TYPE_DIALOG) {
            Kind::Dialog
        } else if types.contains(&a._NET_WM_WINDOW_TYPE_UTILITY) || types.contains(&a._NET_WM_WINDOW_TYPE_TOOLBAR) {
            Kind::Utility
        } else if types.contains(&a._NET_WM_WINDOW_TYPE_SPLASH) {
            Kind::Splash
        } else {
            Kind::Normal
        };
        let transient = x::get_u32s(&self.conn, win, AtomEnum::WM_TRANSIENT_FOR, AtomEnum::WINDOW).first().copied().filter(|&t| t != 0);
        let kind = if transient.is_some() && kind == Kind::Normal { Kind::Dialog } else { kind };
        let (instance, class) = x::class(&self.conn, win);
        let title = x::title(&self.conn, &a, win);
        let motif = x::get_u32s(&self.conn, win, a._MOTIF_WM_HINTS, a._MOTIF_WM_HINTS);
        // flags bit 1 = decorations field present; decorations 0 = the app draws its own
        let own_decor = motif.len() >= 3 && motif[0] & 2 != 0 && motif[2] == 0;
        let strip = !own_decor && kind != Kind::Splash;
        let protocols = x::get_u32s(&self.conn, win, a.WM_PROTOCOLS, AtomEnum::ATOM);
        let hints = x::get_u32s(&self.conn, win, AtomEnum::WM_HINTS, AtomEnum::WM_HINTS);
        let accepts_input = !(hints.len() >= 2 && hints[0] & 1 != 0 && hints[1] == 0);
        let normal = x::get_u32s(&self.conn, win, AtomEnum::WM_NORMAL_HINTS, AtomEnum::WM_SIZE_HINTS);
        let (mut min, mut max) = ((1, 1), (i32::MAX, i32::MAX));
        let mut user_pos = false;
        if normal.len() >= 18 {
            let f = normal[0];
            user_pos = f & 1 != 0;
            if f & 16 != 0 { min = (normal[5] as i32, normal[6] as i32); }
            if f & 32 != 0 { max = (normal[7] as i32, normal[8] as i32); }
        }
        let pid = x::get_u32s(&self.conn, win, a._NET_WM_PID, AtomEnum::CARDINAL).first().copied().unwrap_or(0);
        let state = x::get_u32s(&self.conn, win, a._NET_WM_STATE, AtomEnum::ATOM);
        let fullscreen = state.contains(&a._NET_WM_STATE_FULLSCREEN);

        let mut c = Client {
            win,
            frame: 0,
            rect: Rect::new(geo.x as i32, geo.y as i32, geo.width.max(1) as i32, geo.height.max(1) as i32),
            title,
            class,
            instance,
            kind,
            transient,
            strip,
            tucked: false,
            fullscreen: false,
            filled: false,
            restore: None,
            snapped: None,
            urgent: false,
            delete: protocols.contains(&a.WM_DELETE_WINDOW),
            take_focus: protocols.contains(&a.WM_TAKE_FOCUS),
            accepts_input,
            min,
            max,
            strip_color: None,
            pid,
            ignore_unmaps: 0,
            hover: None,
            floating: false,
            opacity_hint: read_opacity(&self.conn, win, a._NET_WM_WINDOW_OPACITY),
            ws: self.ws,
        };
        self.place(&mut c, user_pos);

        // the frame: an ordinary window holding the client, with room for the strip and an
        // invisible grab margin all round
        let frame = match self.conn.generate_id() { Ok(f) => f, Err(_) => return };
        c.frame = frame;
        let fr = self.frame_rect(&c);
        let screen = &self.conn.setup().roots[self.screen];
        let fmask = EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE
            | EventMask::POINTER_MOTION
            | EventMask::ENTER_WINDOW
            | EventMask::LEAVE_WINDOW
            | EventMask::EXPOSURE;
        let bg = self.theme.chrome;
        let pixel = ((bg.0 as u32) << 16) | ((bg.1 as u32) << 8) | bg.2 as u32;
        if self
            .conn
            .create_window(
                screen.root_depth, frame, self.root, fr.x as i16, fr.y as i16, fr.w as u16, fr.h as u16, 0,
                WindowClass::INPUT_OUTPUT, screen.root_visual,
                &CreateWindowAux::new().event_mask(fmask).background_pixel(pixel),
            )
            .is_err()
        {
            return;
        }
        self.conn.change_window_attributes(win, &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY)).ok();
        self.conn.change_save_set(SetMode::INSERT, win).ok();
        let (b, s) = (self.border(&c), self.strip_h(&c));
        self.conn.configure_window(win, &ConfigureWindowAux::new().border_width(0)).ok();
        self.conn.reparent_window(win, frame, b as i16, (b + s) as i16).ok();
        self.conn.grab_button(false, frame, EventMask::BUTTON_PRESS, GrabMode::SYNC, GrabMode::ASYNC, x11rb::NONE, x11rb::NONE, ButtonIndex::ANY, ModMask::ANY).ok();
        for m in x::IGNORED {
            for b in [ButtonIndex::M1, ButtonIndex::M3] {
                self.conn.grab_button(false, frame, EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, x11rb::NONE, b, ModMask::from(x::MOD_SUPER | m)).ok();
            }
        }
        self.conn.map_window(win).ok();
        self.conn.map_window(frame).ok();
        self.set_wm_state(win, 1);
        let ext = [b as u32, b as u32, (b + s) as u32, b as u32];
        x::set_u32s(&self.conn, win, a._NET_FRAME_EXTENTS, AtomEnum::CARDINAL, &ext);
        x::set_u32s(&self.conn, win, a._NET_WM_DESKTOP, AtomEnum::CARDINAL, &[self.ws - 1]);
        let allowed = [a._NET_WM_ACTION_CLOSE, a._NET_WM_ACTION_MOVE, a._NET_WM_ACTION_RESIZE, a._NET_WM_ACTION_FULLSCREEN,
                       a._NET_WM_ACTION_MINIMIZE, a._NET_WM_ACTION_MAXIMIZE_HORZ, a._NET_WM_ACTION_MAXIMIZE_VERT];
        x::set_u32s(&self.conn, win, a._NET_WM_ALLOWED_ACTIONS, AtomEnum::ATOM, &allowed);
        self.clients.push(c);
        self.send_configure(win);
        self.comp.opening(frame);
        if fullscreen {
            self.set_fullscreen(win, true);
        }
        let user_time = x::get_u32s(&self.conn, win, a._NET_WM_USER_TIME, AtomEnum::CARDINAL);
        if user_time.first() != Some(&0) {
            self.focus(Some(win));
        }
        self.update_client_list();
        self.draw_strip(win);
        self.retile();
        if self.ov.open {
            self.ov.relayout = true;
        }
    }

    /// Where a new window goes: a remembered place, its own request, beside its parent, or the
    /// middle of the monitor under the pointer, never off screen and never bigger than it.
    fn place(&self, c: &mut Client, user_pos: bool) {
        let mon = self.pointer_monitor();
        let wa = self.workarea(&mon);
        let m = (48.0 * self.scale) as i32;
        let s = (theme::STRIP * self.scale) as i32;
        c.rect.w = c.rect.w.min(wa.w - 2 * m).max(c.min.0.min(wa.w)).max(80);
        c.rect.h = c.rect.h.min(wa.h - 2 * m - s).max(c.min.1.min(wa.h)).max(60);
        if self.cfg.windows.remember_places && c.kind == Kind::Normal {
            let same = self.clients.iter().any(|o| o.class == c.class);
            if let Some(p) = self.places.get(&c.class) {
                if !same && self.monitors.iter().any(|mo| mo.overlap(p) > (p.w as i64 * p.h as i64) / 2) {
                    c.rect = *p;
                    return;
                }
            }
        }
        if user_pos && wa.contains(c.rect.x, c.rect.y) && c.rect.x + c.rect.w <= wa.x + wa.w {
            return;
        }
        if let Some(parent) = c.transient.and_then(|t| self.client(t)) {
            c.rect.x = parent.rect.cx() - c.rect.w / 2;
            c.rect.y = parent.rect.cy() - c.rect.h / 2;
        } else {
            c.rect.x = wa.cx() - c.rect.w / 2;
            c.rect.y = wa.cy() - c.rect.h / 2 + s / 2;
            // cascade off a window already there
            let step = (32.0 * self.scale) as i32;
            for _ in 0..12 {
                if self.clients.iter().any(|o| !o.tucked && (o.rect.x - c.rect.x).abs() < 8 && (o.rect.y - c.rect.y).abs() < 8) {
                    c.rect.x += step;
                    c.rect.y += step;
                } else {
                    break;
                }
            }
        }
        c.rect.x = c.rect.x.clamp(wa.x + theme::BORDER, (wa.x + wa.w - c.rect.w - theme::BORDER).max(wa.x));
        c.rect.y = c.rect.y.clamp(wa.y + s + theme::BORDER, (wa.y + wa.h - c.rect.h - theme::BORDER).max(wa.y + s));
    }

    pub fn unmanage(&mut self, win: Window, destroyed: bool) {
        let Some(i) = self.idx(win) else {
            if self.struts.remove(&win).is_some() {
                self.publish_workarea();
            }
            return;
        };
        let c = self.clients.remove(i);
        if self.cfg.windows.remember_places && c.kind == Kind::Normal && !c.fullscreen && !c.filled && c.snapped.is_none() {
            self.places.insert(c.class.clone(), c.rect);
            save_places(&self.places);
        }
        self.comp.closing(&self.conn, c.frame);
        if !destroyed {
            self.set_wm_state(c.win, 0);
            let root = self.root;
            self.conn.reparent_window(c.win, root, c.rect.x as i16, c.rect.y as i16).ok();
            self.conn.change_save_set(SetMode::DELETE, c.win).ok();
        }
        self.conn.destroy_window(c.frame).ok();
        self.tile_order.retain(|w| *w != c.win);
        self.retile();
        self.mru.retain(|&w| w != c.win);
        if self.focused == Some(c.win) {
            self.focused = None;
            let next = c.transient.filter(|t| self.client(*t).is_some_and(|p| !p.tucked)).or_else(|| {
                self.mru.iter().copied().find(|w| self.client(*w).is_some_and(|o| !o.tucked))
            });
            self.focus(next);
        }
        self.update_client_list();
        if self.ov.open {
            self.ov.relayout = true;
        }
    }

    pub fn set_wm_state(&self, win: Window, state: u32) {
        let a = self.atoms.WM_STATE;
        self.conn.change_property32(PropMode::REPLACE, win, a, a, &[state, 0]).ok();
    }

    pub fn update_client_list(&self) {
        let list: Vec<u32> = self.clients.iter().map(|c| c.win).collect();
        x::set_u32s(&self.conn, self.root, self.atoms._NET_CLIENT_LIST, AtomEnum::WINDOW, &list);
        let mut stack: Vec<u32> = self.mru.iter().rev().copied().collect();
        for c in &self.clients {
            if !stack.contains(&c.win) {
                stack.insert(0, c.win);
            }
        }
        x::set_u32s(&self.conn, self.root, self.atoms._NET_CLIENT_LIST_STACKING, AtomEnum::WINDOW, &stack);
    }

    /// Put frame and client where `rect` says, and tell the client (ICCCM 4.1.5).
    pub fn apply_geometry(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        let fr = self.frame_rect(&c);
        let (b, s) = (self.border(&c), self.strip_h(&c));
        self.conn.configure_window(c.frame, &ConfigureWindowAux::new().x(fr.x).y(fr.y).width(fr.w.max(1) as u32).height(fr.h.max(1) as u32)).ok();
        self.conn.configure_window(c.win, &ConfigureWindowAux::new().x(b).y(b + s).width(c.rect.w.max(1) as u32).height(c.rect.h.max(1) as u32)).ok();
        let ext = [b as u32, b as u32, (b + s) as u32, b as u32];
        x::set_u32s(&self.conn, c.win, self.atoms._NET_FRAME_EXTENTS, AtomEnum::CARDINAL, &ext);
        self.send_configure(win);
        self.draw_strip(win);
        self.comp.dirty = true;
    }

    fn send_configure(&self, win: Window) {
        let Some(c) = self.client(win) else { return };
        let ev = ConfigureNotifyEvent {
            response_type: CONFIGURE_NOTIFY_EVENT,
            sequence: 0,
            event: c.win,
            window: c.win,
            above_sibling: x11rb::NONE,
            x: c.rect.x as i16,
            y: c.rect.y as i16,
            width: c.rect.w as u16,
            height: c.rect.h as u16,
            border_width: 0,
            override_redirect: false,
        };
        self.conn.send_event(false, c.win, EventMask::STRUCTURE_NOTIFY, ev).ok();
    }

    /// Move a window to `to` with the morph animation.
    pub fn move_to(&mut self, win: Window, to: Rect) {
        let Some(c) = self.client(win).cloned() else { return };
        let from = self.visible_rect(&c);
        if let Some(c) = self.client_mut(win) {
            let mut r = to;
            r.w = r.w.clamp(c.min.0.max(1), c.max.0.max(1));
            r.h = r.h.clamp(c.min.1.max(1), c.max.1.max(1));
            c.rect = r;
        }
        self.apply_geometry(win);
        let after = self.client(win).map(|c| self.visible_rect(c)).unwrap_or(from);
        self.comp.morph(c.frame, from, after, self.motion());
    }

    pub fn motion(&self) -> f32 {
        match self.cfg.look.motion.as_str() {
            "off" => 0.0,
            "full" => 1.4,
            _ => 1.0,
        }
    }

    // ── focus and stacking ────────────────────────────────────────────────────────────────
    pub fn focus(&mut self, win: Option<Window>) {
        let old = self.focused;
        let win = win.and_then(|w| self.client(w).map(|c| c.win));
        if let Some(w) = win {
            if let Some(c) = self.client(w).cloned() {
                if c.tucked {
                    self.untuck(w);
                }
                if c.accepts_input {
                    self.conn.set_input_focus(InputFocus::POINTER_ROOT, w, CURRENT_TIME).ok();
                } else {
                    self.conn.set_input_focus(InputFocus::POINTER_ROOT, self.check, CURRENT_TIME).ok();
                }
                if c.take_focus {
                    self.send_protocol(w, self.atoms.WM_TAKE_FOCUS);
                }
                self.raise(w);
                // focused windows take clicks directly; others are grabbed for click-to-focus
                self.conn.ungrab_button(ButtonIndex::ANY, c.frame, ModMask::ANY).ok();
                for m in x::IGNORED {
                    for b in [ButtonIndex::M1, ButtonIndex::M3] {
                        self.conn.grab_button(false, c.frame, EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, x11rb::NONE, b, ModMask::from(x::MOD_SUPER | m)).ok();
                    }
                }
                self.mru.retain(|&x| x != w);
                self.mru.insert(0, w);
                if let Some(c) = self.client_mut(w) {
                    c.urgent = false;
                }
            }
        } else {
            self.conn.set_input_focus(InputFocus::POINTER_ROOT, self.check, CURRENT_TIME).ok();
        }
        self.focused = win;
        x::set_u32s(&self.conn, self.root, self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, &[win.unwrap_or(0)]);
        if let Some(o) = old.filter(|o| Some(*o) != win) {
            if let Some(oc) = self.client(o).cloned() {
                self.conn.grab_button(false, oc.frame, EventMask::BUTTON_PRESS, GrabMode::SYNC, GrabMode::ASYNC, x11rb::NONE, x11rb::NONE, ButtonIndex::ANY, ModMask::ANY).ok();
            }
            self.draw_strip(o);
        }
        if let Some(w) = win {
            self.draw_strip(w);
        }
        self.update_client_list();
        self.comp.dirty = true;
    }

    pub fn raise(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        self.conn.configure_window(c.frame, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE)).ok();
        // its dialogs stay above it
        let kids: Vec<Window> = self.clients.iter().filter(|o| o.transient == Some(c.win)).map(|o| o.frame).collect();
        for k in kids {
            self.conn.configure_window(k, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE)).ok();
        }
    }

    pub fn send_protocol(&self, win: Window, proto: Atom) {
        let ev = ClientMessageEvent::new(32, win, self.atoms.WM_PROTOCOLS, [proto, CURRENT_TIME, 0, 0, 0]);
        self.conn.send_event(false, win, EventMask::NO_EVENT, ev).ok();
    }

    pub fn close(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        if c.delete {
            self.send_protocol(c.win, self.atoms.WM_DELETE_WINDOW);
        } else {
            self.conn.kill_client(c.win).ok();
        }
    }

    // ── tuck away (minimize) ─────────────────────────────────────────────────────────────
    pub fn tuck(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        if c.tucked {
            return;
        }
        self.comp.tucking(c.frame, self.motion());
        if let Some(cm) = self.client_mut(win) {
            cm.tucked = true;
            cm.ignore_unmaps += 1;
        }
        self.conn.unmap_window(c.win).ok();
        self.conn.unmap_window(c.frame).ok();
        self.set_wm_state(c.win, 3);
        self.set_net_state(c.win);
        self.retile();
        if self.focused == Some(c.win) {
            let next = self.mru.iter().copied().find(|&w| w != c.win && self.client(w).is_some_and(|o| !o.tucked));
            self.focus(next);
        }
        self.comp.dirty = true;
    }

    pub fn untuck(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        if !c.tucked {
            return;
        }
        let here = self.ws;
        if let Some(cm) = self.client_mut(win) {
            cm.tucked = false;
            cm.ws = here; // brought back here, whichever workspace it was minimized on
        }
        self.conn.map_window(c.win).ok();
        self.conn.map_window(c.frame).ok();
        self.set_wm_state(c.win, 1);
        self.set_net_state(c.win);
        self.comp.opening(c.frame);
        self.retile();
        if self.taskbar.open {
            self.draw_taskbar();
        }
    }

    pub fn set_net_state(&self, win: Window) {
        let Some(c) = self.client(win) else { return };
        let a = self.atoms;
        let mut st = vec![];
        if c.fullscreen { st.push(a._NET_WM_STATE_FULLSCREEN); }
        if c.tucked { st.push(a._NET_WM_STATE_HIDDEN); }
        if c.filled { st.push(a._NET_WM_STATE_MAXIMIZED_VERT); st.push(a._NET_WM_STATE_MAXIMIZED_HORZ); }
        if c.urgent { st.push(a._NET_WM_STATE_DEMANDS_ATTENTION); }
        if self.focused == Some(c.win) { st.push(a._NET_WM_STATE_FOCUSED); }
        x::set_u32s(&self.conn, c.win, a._NET_WM_STATE, AtomEnum::ATOM, &st);
    }

    // ── fill, snap, fullscreen ───────────────────────────────────────────────────────────
    /// The client rect that makes a frame occupy `area` (area = what the user sees).
    pub fn client_rect_for(&self, c: &Client, area: Rect) -> Rect {
        let s = (theme::STRIP * self.scale).round() as i32 * c.strip as i32;
        Rect::new(area.x, area.y + s, area.w, area.h - s)
    }

    pub fn snap_area(&self, mon: &Rect, which: &str) -> Option<Rect> {
        let wa = self.workarea(mon);
        let g = self.cfg.windows.gap;
        let full = wa.inset(g);
        let (hw, hh) = ((full.w - g) / 2, (full.h - g) / 2);
        Some(match which {
            "fill" => full,
            "left" => Rect::new(full.x, full.y, hw, full.h),
            "right" => Rect::new(full.x + hw + g, full.y, full.w - hw - g, full.h),
            "top-left" => Rect::new(full.x, full.y, hw, hh),
            "top-right" => Rect::new(full.x + hw + g, full.y, full.w - hw - g, hh),
            "bottom-left" => Rect::new(full.x, full.y + hh + g, hw, full.h - hh - g),
            "bottom-right" => Rect::new(full.x + hw + g, full.y + hh + g, full.w - hw - g, full.h - hh - g),
            _ => return None,
        })
    }

    pub fn snap(&mut self, win: Window, which: &str) {
        // a tiled window snapped by hand floats from then on (fill keeps it tiled: it comes back)
        if which != "fill" && self.client(win).is_some_and(|c| self.is_tiled(c)) {
            if let Some(cm) = self.client_mut(win) { cm.floating = true; }
            self.retile();
        }
        let Some(c) = self.client(win).cloned() else { return };
        if c.fullscreen {
            self.set_fullscreen(win, false);
        }
        let mon = self.monitor_of(&self.visible_rect(&c));
        let Some(area) = self.snap_area(&mon, which) else { return };
        let to = self.client_rect_for(&c, area);
        if let Some(cm) = self.client_mut(win) {
            if cm.restore.is_none() {
                cm.restore = Some(cm.rect);
            }
            cm.filled = which == "fill";
            cm.snapped = Some(which.to_string());
        }
        self.move_to(win, to);
        self.set_net_state(win);
        if which == "fill" {
            self.retile();
        }
    }

    pub fn restore(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        if c.fullscreen {
            self.set_fullscreen(win, false);
            return;
        }
        if let Some(r) = c.restore {
            if let Some(cm) = self.client_mut(win) {
                cm.filled = false;
                cm.snapped = None;
                cm.restore = None;
            }
            self.move_to(win, r);
            self.set_net_state(win);
            self.retile();
        } else {
            self.tuck(win);
        }
    }

    pub fn toggle_fill(&mut self, win: Window) {
        if self.client(win).is_some_and(|c| c.filled) { self.restore(win) } else { self.snap(win, "fill") }
    }

    pub fn set_fullscreen(&mut self, win: Window, on: bool) {
        let Some(c) = self.client(win).cloned() else { return };
        if c.fullscreen == on {
            return;
        }
        let from = self.visible_rect(&c);
        if on {
            let mon = self.monitor_of(&from);
            if let Some(cm) = self.client_mut(win) {
                if cm.restore.is_none() || cm.snapped.is_none() {
                    cm.restore = Some(cm.rect);
                }
                cm.fullscreen = true;
                cm.rect = mon;
            }
            self.raise(win);
        } else if let Some(cm) = self.client_mut(win) {
            cm.fullscreen = false;
            let back = if cm.snapped.is_some() { None } else { cm.restore.take() };
            if let Some(r) = back {
                cm.rect = r;
            }
        }
        let snapped = self.client(win).and_then(|c| c.snapped.clone());
        self.apply_geometry(win);
        if !on {
            if let Some(s) = snapped {
                self.snap(win, &s);
            }
        }
        let after = self.client(win).map(|c| self.visible_rect(c)).unwrap_or(from);
        self.comp.morph(c.frame, from, after, self.motion());
        self.set_net_state(win);
        self.retile();
    }

    pub fn center(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        let wa = self.workarea(&self.monitor_of(&self.visible_rect(&c)));
        let s = self.strip_h(&c);
        let to = Rect::new(wa.cx() - c.rect.w / 2, wa.cy() - (c.rect.h - s) / 2, c.rect.w, c.rect.h);
        self.move_to(win, to);
    }

    // ── the title strip ──────────────────────────────────────────────────────────────────
    /// Buttons on the strip, right to left: close, fill, tuck. Returns (x, width) in frame coords.
    /// Each is a word in a pill (the Sakura look: words over icons).
    pub fn strip_buttons(&self, c: &Client) -> [(i32, i32); 3] {
        let fr = self.frame_rect(c);
        let b = self.border(c);
        let sc = self.scale;
        let right = fr.w - b - (6.0 * sc) as i32;
        let mut out = [(0, 0); 3];
        let mut x = right;
        for (i, word) in Self::strip_words(c).iter().enumerate() {
            let bw = (self.fonts.measure(draw::Face::Ui, 12.0 * sc, word) + 22.0 * sc).ceil() as i32;
            x -= bw;
            out[i] = (x, bw);
            x -= (2.0 * sc) as i32;
        }
        out
    }

    fn strip_words(c: &Client) -> [&'static str; 3] {
        ["Close", if c.filled { "Back" } else { "Fill" }, "Tuck"]
    }

    pub fn draw_strip(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        let s = self.strip_h(&c);
        if s == 0 {
            return;
        }
        let fr = self.frame_rect(&c);
        let b = self.border(&c);
        let (w, h) = (fr.w as u32, (b + s) as u32);
        let Some(mut px) = draw::Px::new(w.max(1), h.max(1)) else { return };
        let focused = self.focused == Some(c.win);
        let bg = c.strip_color.filter(|_| self.cfg.windows.strip_follows_app).unwrap_or(self.theme.chrome);
        let dark = bg.luma() < 0.4;
        let (fg, dim) = if dark { (Rgba::hex(0xECEBE6), Rgba::hex(0xA3A098)) } else { (Rgba::hex(0x1B1B1D), Rgba::hex(0x5F5C56)) };
        px.fill(draw::tiny_skia_color(bg));
        let sc = self.scale;
        let fs = 12.5 * sc;
        let ty = b as f32 + (s as f32 - fs * 1.25) / 2.0;
        let buttons = self.strip_buttons(&c);
        let left = b as f32 + 16.0 * sc;
        let max_w = buttons[2].0 as f32 - left - 8.0 * sc;
        let face = if focused { draw::Face::UiBold } else { draw::Face::Ui };
        let title = self.fonts.fit(face, fs, if c.title.is_empty() { &c.class } else { &c.title }, max_w);
        self.fonts.draw(&mut px, face, fs, left, ty, if focused { fg } else { dim }, &title);
        let words = Self::strip_words(&c);
        for (i, (bx, bw)) in buttons.iter().enumerate() {
            let hover = c.hover == Some(i as u8);
            let ph = s as f32 - 10.0 * sc;
            if hover {
                let hb = if i == 0 { Rgba(200, 70, 60, 230) } else { fg.with_alpha(28) };
                draw::rrect(&mut px, *bx as f32, b as f32 + 5.0 * sc, *bw as f32, ph, ph / 2.0, hb);
            }
            let col = if hover && i == 0 { Rgba(255, 255, 255, 255) } else if focused { fg } else { dim };
            let wfs = 12.0 * sc;
            self.fonts.draw_centered(&mut px, draw::Face::Ui, wfs, *bx as f32 + *bw as f32 / 2.0, b as f32 + (s as f32 - wfs * 1.25) / 2.0, col, words[i]);
        }
        self.comp.put_rgb(&self.conn, c.frame, &px, 0, 0);
        self.comp.dirty = true;
    }

    /// Sample the app's own top colour, so the strip continues the app (a terminal's strip is
    /// the terminal's background). Done once, shortly after the window first draws.
    pub fn sample_strip_color(&mut self, win: Window) {
        let Some(c) = self.client(win).cloned() else { return };
        if !self.cfg.windows.strip_follows_app || c.strip_color.is_some() || c.rect.w < 20 || c.rect.h < 20 {
            return;
        }
        let img = self.conn.get_image(ImageFormat::Z_PIXMAP, c.win, 4, 2, 12, 2, !0).ok().and_then(|r| r.reply().ok());
        let Some(img) = img else { return };
        if img.depth < 24 || img.data.len() < 4 * 24 {
            return;
        }
        // average the sampled pixels (BGRX)
        let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
        let n = (img.data.len() / 4) as u32;
        for p in img.data.chunks_exact(4) {
            b += p[0] as u32;
            g += p[1] as u32;
            r += p[2] as u32;
        }
        let col = Rgba((r / n) as u8, (g / n) as u8, (b / n) as u8, 255);
        // an all-black first frame usually means the app has not drawn yet
        if col == Rgba(0, 0, 0, 255) && !c.class.to_lowercase().contains("term") {
            return;
        }
        if let Some(cm) = self.client_mut(win) {
            cm.strip_color = Some(col);
        }
        self.draw_strip(win);
    }

    /// Which part of a frame is at (fx, fy): 0 nothing, 1..=3 a strip button (close, fill,
    /// tuck), 10 the strip, 20+edges a resize edge.
    pub fn hit(&self, c: &Client, fx: i32, fy: i32) -> u8 {
        let fr = self.frame_rect(c);
        let (b, s) = (self.border(c), self.strip_h(c));
        let corner = (14.0 * self.scale) as i32;
        let mut e = 0u8;
        if !c.fullscreen {
            if fx < b || (fx < corner && (fy < b || fy >= fr.h - b)) { e |= E_LEFT; }
            if fx >= fr.w - b || (fx >= fr.w - corner && (fy < b || fy >= fr.h - b)) { e |= E_RIGHT; }
            if fy < b || (fy < corner && (fx < b || fx >= fr.w - b)) { e |= E_TOP; }
            if fy >= fr.h - b || (fy >= fr.h - corner && (fx < b || fx >= fr.w - b)) { e |= E_BOTTOM; }
        }
        if e != 0 {
            return 20 + e;
        }
        if s > 0 && fy < b + s {
            for (i, (bx, bw)) in self.strip_buttons(c).iter().enumerate() {
                if fx >= *bx && fx < bx + bw {
                    return 1 + i as u8;
                }
            }
            return 10;
        }
        0
    }

    // ── keys ─────────────────────────────────────────────────────────────────────────────
    pub fn grab_keys(&mut self) {
        let root = self.root;
        self.conn.ungrab_key(Grab::ANY, root, ModMask::ANY).ok();
        self.binds.clear();
        for (combo, action) in self.cfg.keys.clone() {
            let Some((mods, sym)) = x::parse_combo(&combo) else {
                eprintln!("samurai-wm: cannot read the key \"{combo}\"; skipped");
                continue;
            };
            let syms = if sym == x::KS_SUPER_L { vec![x::KS_SUPER_L, x::KS_SUPER_R] } else { vec![sym] };
            for s in syms {
                for code in self.keymap.codes_for(s) {
                    for m in x::IGNORED {
                        self.conn.grab_key(true, root, ModMask::from(mods | m), code, GrabMode::ASYNC, GrabMode::ASYNC).ok();
                    }
                }
                self.binds.push((mods, s, action.clone()));
            }
        }
    }

    pub fn bound(&self, mods: u16, sym: u32) -> Option<String> {
        let m = mods & (x::MOD_SHIFT | x::MOD_CTRL | x::MOD_ALT | x::MOD_SUPER);
        // a bare Super is the tap, handled on release
        self.binds.iter().find(|(bm, bs, _)| *bm == m && *bs == sym && *bs != x::KS_SUPER_L && *bs != x::KS_SUPER_R).map(|b| b.2.clone())
    }

    // ── events ───────────────────────────────────────────────────────────────────────────
    pub fn handle(&mut self, ev: Event) {
        self.comp.event(&self.conn, &ev);
        if self.lock.locked {
            if self.lock_event(&ev) {
                return;
            }
        }
        match ev {
            Event::MapRequest(e) => {
                if let Some(c) = self.client(e.window).cloned() {
                    if c.tucked { self.untuck(c.win); }
                    self.focus(Some(c.win));
                } else {
                    self.manage(e.window);
                }
            }
            Event::UnmapNotify(e) => {
                // X tells the client, its frame and (on reparenting) the root; only the frame's
                // copy counts. A synthetic one to the root is a tucked window withdrawing (ICCCM 4.1.4).
                let synthetic = e.response_type & 0x80 != 0;
                if let Some(c) = self.clients.iter_mut().find(|c| c.win == e.window) {
                    if e.event == c.frame {
                        if c.ignore_unmaps > 0 {
                            c.ignore_unmaps -= 1;
                        } else {
                            let w = c.win;
                            self.unmanage(w, false);
                        }
                    } else if synthetic && e.event == self.root && c.tucked {
                        let w = c.win;
                        self.unmanage(w, false);
                    }
                } else if self.struts.remove(&e.window).is_some() {
                    self.publish_workarea();
                }
            }
            Event::DestroyNotify(e) => {
                if self.clients.iter().any(|c| c.win == e.window) {
                    self.unmanage(e.window, true);
                } else if self.struts.remove(&e.window).is_some() {
                    self.publish_workarea();
                }
            }
            Event::ConfigureRequest(e) => self.configure_request(e),
            Event::PropertyNotify(e) => self.property(e),
            Event::ClientMessage(e) => self.client_message(e),
            Event::ButtonPress(e) => self.button_press(e),
            Event::ButtonRelease(e) => self.button_release(e),
            Event::MotionNotify(e) => self.motion_notify(e),
            Event::EnterNotify(e) => {
                if self.cfg.windows.focus == "hover" && self.drag.is_none() && !self.ov.open {
                    if let Some(w) = self.client(e.event).map(|c| c.win) {
                        if self.focused != Some(w) {
                            self.focus(Some(w));
                        }
                    }
                }
            }
            Event::LeaveNotify(e) => {
                if let Some(c) = self.client_mut(e.event) {
                    if c.hover.take().is_some() {
                        let w = c.win;
                        self.draw_strip(w);
                    }
                }
            }
            Event::Expose(e) => {
                if e.count == 0 {
                    if let Some(c) = self.client(e.window).filter(|c| c.frame == e.window).map(|c| c.win) {
                        self.draw_strip(c);
                    }
                }
            }
            Event::KeyPress(e) => self.key_press(e),
            Event::KeyRelease(e) => self.key_release(e),
            Event::MappingNotify(_) => {
                if let Some(k) = Keymap::load(&self.conn) {
                    self.keymap = k;
                    self.grab_keys();
                }
            }
            Event::RandrScreenChangeNotify(_) => self.screens_changed(),
            _ => {}
        }
    }

    fn configure_request(&mut self, e: ConfigureRequestEvent) {
        if let Some(c) = self.client(e.window).cloned() {
            if c.fullscreen || c.filled || c.snapped.is_some() {
                self.send_configure(c.win);
                return;
            }
            let m = e.value_mask;
            let mut r = c.rect;
            if m.contains(ConfigWindow::X) { r.x = e.x as i32; }
            if m.contains(ConfigWindow::Y) { r.y = e.y as i32; }
            if m.contains(ConfigWindow::WIDTH) { r.w = e.width as i32; }
            if m.contains(ConfigWindow::HEIGHT) { r.h = e.height as i32; }
            if let Some(cm) = self.client_mut(e.window) {
                cm.rect = r;
            }
            self.apply_geometry(e.window);
            if m.contains(ConfigWindow::STACK_MODE) && e.stack_mode == StackMode::ABOVE && self.focused == Some(c.win) {
                self.raise(c.win);
            }
        } else {
            let mut aux = ConfigureWindowAux::from_configure_request(&e);
            aux.sibling = None;
            aux.stack_mode = None;
            self.conn.configure_window(e.window, &aux).ok();
        }
    }

    fn property(&mut self, e: PropertyNotifyEvent) {
        let a = self.atoms;
        if e.window == self.root {
            return;
        }
        if e.atom == a._NET_WM_WINDOW_OPACITY {
            let v = read_opacity(&self.conn, e.window, a._NET_WM_WINDOW_OPACITY);
            if let Some(cm) = self.client_mut(e.window) {
                cm.opacity_hint = v;
                self.comp.dirty = true;
            }
            return;
        }
        if e.atom == a._NET_WM_STRUT_PARTIAL || e.atom == a._NET_WM_STRUT {
            let s = x::get_u32s(&self.conn, e.window, e.atom, AtomEnum::CARDINAL);
            if s.len() >= 4 {
                self.struts.insert(e.window, [s[0], s[1], s[2], s[3]]);
                self.publish_workarea();
            }
            return;
        }
        if self.client(e.window).is_none() {
            return;
        }
        if e.atom == a._NET_WM_NAME || e.atom == u32::from(AtomEnum::WM_NAME) {
            let t = x::title(&self.conn, &a, e.window);
            if let Some(c) = self.client_mut(e.window) {
                c.title = t;
            }
            self.draw_strip(e.window);
            self.ov.dirty = true;
        } else if e.atom == u32::from(AtomEnum::WM_HINTS) {
            let h = x::get_u32s(&self.conn, e.window, e.atom, AtomEnum::WM_HINTS);
            let urgent = h.first().is_some_and(|f| f & 256 != 0);
            if let Some(c) = self.client_mut(e.window) {
                c.urgent = urgent;
            }
            self.set_net_state(e.window);
        }
    }

    fn client_message(&mut self, e: ClientMessageEvent) {
        let a = self.atoms;
        let d = e.data.as_data32();
        if e.type_ == a._NET_ACTIVE_WINDOW {
            if self.client(e.window).is_some() {
                self.focus(Some(e.window));
            }
        } else if e.type_ == a._NET_CLOSE_WINDOW {
            self.close(e.window);
        } else if e.type_ == a.WM_CHANGE_STATE && d[0] == 3 {
            self.tuck(e.window);
        } else if e.type_ == a._NET_WM_STATE {
            for prop in [d[1], d[2]] {
                if prop == 0 {
                    continue;
                }
                let Some(c) = self.client(e.window).cloned() else { return };
                let cur = if prop == a._NET_WM_STATE_FULLSCREEN {
                    c.fullscreen
                } else if prop == a._NET_WM_STATE_MAXIMIZED_VERT || prop == a._NET_WM_STATE_MAXIMIZED_HORZ {
                    c.filled
                } else if prop == a._NET_WM_STATE_HIDDEN {
                    c.tucked
                } else if prop == a._NET_WM_STATE_DEMANDS_ATTENTION {
                    c.urgent
                } else {
                    continue;
                };
                let want = match d[0] { 0 => false, 1 => true, _ => !cur };
                if want == cur {
                    continue;
                }
                if prop == a._NET_WM_STATE_FULLSCREEN {
                    self.set_fullscreen(e.window, want);
                } else if prop == a._NET_WM_STATE_MAXIMIZED_VERT || prop == a._NET_WM_STATE_MAXIMIZED_HORZ {
                    if want { self.snap(e.window, "fill") } else { self.restore(e.window) }
                } else if prop == a._NET_WM_STATE_HIDDEN {
                    if want { self.tuck(e.window) } else { self.untuck(e.window) }
                } else if let Some(cm) = self.client_mut(e.window) {
                    cm.urgent = want;
                }
            }
            self.set_net_state(e.window);
        } else if e.type_ == a._NET_WM_MOVERESIZE {
            // apps that draw their own title bar ask us to move or resize them
            let (x, y, dir) = (d[0] as i32, d[1] as i32, d[2]);
            let Some(c) = self.client(e.window).cloned() else { return };
            if dir == 11 {
                self.end_drag();
                return;
            }
            let edges = match dir {
                0 => E_TOP | E_LEFT, 1 => E_TOP, 2 => E_TOP | E_RIGHT, 3 => E_RIGHT,
                4 => E_BOTTOM | E_RIGHT, 5 => E_BOTTOM, 6 => E_BOTTOM | E_LEFT, 7 => E_LEFT,
                _ => 0,
            };
            if dir == 8 || dir == 10 {
                self.start_move(c.win, x, y);
            } else if edges != 0 {
                self.start_resize(c.win, edges, x, y);
            }
        } else if e.type_ == a._NET_MOVERESIZE_WINDOW {
            let Some(c) = self.client(e.window).cloned() else { return };
            let flags = d[0];
            let mut r = c.rect;
            if flags & (1 << 8) != 0 { r.x = d[1] as i32; }
            if flags & (1 << 9) != 0 { r.y = d[2] as i32; }
            if flags & (1 << 10) != 0 { r.w = d[3] as i32; }
            if flags & (1 << 11) != 0 { r.h = d[4] as i32; }
            if let Some(cm) = self.client_mut(e.window) {
                cm.rect = r;
            }
            self.apply_geometry(e.window);
        } else if e.type_ == a._NET_REQUEST_FRAME_EXTENTS {
            let b = theme::BORDER as u32;
            let s = (theme::STRIP * self.scale) as u32;
            x::set_u32s(&self.conn, e.window, a._NET_FRAME_EXTENTS, AtomEnum::CARDINAL, &[b, b, b + s, b]);
        }
    }

    // ── pointer ──────────────────────────────────────────────────────────────────────────
    fn button_press(&mut self, e: ButtonPressEvent) {
        let state = u16::from(e.state);
        if self.assist.open {
            self.conn.allow_events(Allow::ASYNC_POINTER, CURRENT_TIME).ok();
            self.assist_click(e.root_x as i32, e.root_y as i32);
            return;
        }
        if self.taskbar.open {
            self.conn.allow_events(Allow::ASYNC_POINTER, CURRENT_TIME).ok();
            self.taskbar_press(e.root_x as i32, e.root_y as i32, e.detail);
            return;
        }
        if self.ov.open {
            self.conn.allow_events(Allow::ASYNC_POINTER, CURRENT_TIME).ok();
            self.overview_click(e.root_x as i32, e.root_y as i32, e.detail);
            return;
        }
        if self.super_down {
            self.super_used = true;
        }
        let Some(c) = self.client(e.event).cloned() else {
            self.conn.allow_events(Allow::REPLAY_POINTER, CURRENT_TIME).ok();
            return;
        };
        // a click on an unfocused window focuses it and still reaches the app
        if self.focused != Some(c.win) {
            self.focus(Some(c.win));
        }
        self.raise(c.win);
        let (rx, ry) = (e.root_x as i32, e.root_y as i32);
        if state & x::MOD_SUPER != 0 && e.child != 0 || (state & x::MOD_SUPER != 0 && e.event == c.frame && self.hit(&c, e.event_x as i32, e.event_y as i32) == 0) {
            self.conn.allow_events(Allow::ASYNC_POINTER, CURRENT_TIME).ok();
            if e.detail == 1 {
                self.start_move(c.win, rx, ry);
            } else if e.detail == 3 {
                let vr = self.visible_rect(&c);
                let mut edges = 0;
                edges |= if rx < vr.cx() { E_LEFT } else { E_RIGHT };
                edges |= if ry < vr.cy() { E_TOP } else { E_BOTTOM };
                self.start_resize(c.win, edges, rx, ry);
            }
            return;
        }
        if e.event == c.frame && e.child == 0 {
            self.conn.allow_events(Allow::ASYNC_POINTER, CURRENT_TIME).ok();
            let h = self.hit(&c, e.event_x as i32, e.event_y as i32);
            match h {
                1..=3 if e.detail == 1 => {} // acted on at release, like any button
                10 if e.detail == 1 => {
                    let now = Instant::now();
                    if self.last_click.0 == c.win && now.duration_since(self.last_click.1).as_millis() < 380 {
                        self.last_click = (0, now);
                        self.toggle_fill(c.win);
                        return;
                    }
                    self.last_click = (c.win, now);
                    self.start_move(c.win, rx, ry);
                }
                10 if e.detail == 2 => self.tuck(c.win),
                h if h >= 20 && e.detail == 1 => self.start_resize(c.win, h - 20, rx, ry),
                _ => {}
            }
            return;
        }
        self.conn.allow_events(Allow::REPLAY_POINTER, CURRENT_TIME).ok();
    }

    fn button_release(&mut self, e: ButtonReleaseEvent) {
        if self.taskbar.open {
            self.taskbar_release(e.root_x as i32, e.root_y as i32);
            return;
        }
        if self.drag.is_some() {
            self.end_drag();
            return;
        }
        if let Some(c) = self.client(e.event).cloned() {
            if e.event == c.frame && e.detail == 1 {
                match self.hit(&c, e.event_x as i32, e.event_y as i32) {
                    1 => self.close(c.win),
                    2 => self.toggle_fill(c.win),
                    3 => self.tuck(c.win),
                    _ => {}
                }
            }
        }
    }

    pub fn start_move(&mut self, win: Window, rx: i32, ry: i32) {
        let Some(c) = self.client(win).cloned() else { return };
        let grabbed = self
            .conn
            .grab_pointer(false, self.root, EventMask::POINTER_MOTION | EventMask::BUTTON_RELEASE, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, self.cursors.mov, CURRENT_TIME)
            .ok()
            .and_then(|r| r.reply().ok())
            .is_some_and(|r| r.status == GrabStatus::SUCCESS);
        if !grabbed {
            return;
        }
        self.float_on_drag(c.win);
        self.drag = Some(Drag::Move { win: c.win, dx: rx - c.rect.x, dy: ry - c.rect.y, start: (rx, ry), moved: false });
        self.shake.clear();
    }

    pub fn start_resize(&mut self, win: Window, edges: u8, rx: i32, ry: i32) {
        let Some(c) = self.client(win).cloned() else { return };
        if c.fullscreen {
            return;
        }
        let cur = self.cursors.edges.get(&edges).copied().unwrap_or(self.cursors.normal);
        let ok = self
            .conn
            .grab_pointer(false, self.root, EventMask::POINTER_MOTION | EventMask::BUTTON_RELEASE, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, cur, CURRENT_TIME)
            .ok()
            .and_then(|r| r.reply().ok())
            .is_some_and(|r| r.status == GrabStatus::SUCCESS);
        if !ok {
            return;
        }
        if let Some(cm) = self.client_mut(win) {
            cm.filled = false;
            cm.snapped = None;
            cm.restore = None;
        }
        self.float_on_drag(c.win);
        self.drag = Some(Drag::Resize { win: c.win, edges, start: (rx, ry), orig: c.rect });
    }

    fn motion_notify(&mut self, e: MotionNotifyEvent) {
        let (rx, ry) = (e.root_x as i32, e.root_y as i32);
        match self.drag {
            Some(Drag::Move { win, dx, dy, start, moved }) => {
                if !moved && (rx - start.0).abs() + (ry - start.1).abs() < 4 {
                    return;
                }
                if !moved {
                    // dragging a filled or snapped window off its place gives back its old size
                    if let Some(c) = self.client(win).cloned() {
                        if c.fullscreen {
                            self.set_fullscreen(win, false);
                        }
                        if let (Some(r), true) = (c.restore, c.snapped.is_some() || c.filled) {
                            let fx = (dx as f32 / c.rect.w.max(1) as f32 * r.w as f32) as i32;
                            if let Some(cm) = self.client_mut(win) {
                                cm.rect = Rect::new(rx - fx, ry - dy, r.w, r.h);
                                cm.filled = false;
                                cm.snapped = None;
                                cm.restore = None;
                            }
                            self.drag = Some(Drag::Move { win, dx: fx, dy, start, moved: true });
                            self.apply_geometry(win);
                            return;
                        }
                    }
                }
                let (ndx, ndy) = match self.drag { Some(Drag::Move { dx, dy, .. }) => (dx, dy), _ => (dx, dy) };
                self.drag = Some(Drag::Move { win, dx: ndx, dy: ndy, start, moved: true });
                if let Some(c) = self.client_mut(win) {
                    c.rect.x = rx - ndx;
                    c.rect.y = ry - ndy;
                }
                if let Some(c) = self.client(win).cloned() {
                    let fr = self.frame_rect(&c);
                    self.conn.configure_window(c.frame, &ConfigureWindowAux::new().x(fr.x).y(fr.y)).ok();
                }
                self.snap_preview = if self.cfg.windows.snap { self.snap_target(rx, ry).and_then(|w| self.snap_area(&self.monitor_at(rx, ry), &w)) } else { None };
                self.track_shake(win, rx);
                self.comp.dirty = true;
            }
            Some(Drag::Resize { win, edges, start, orig }) => {
                let (ddx, ddy) = (rx - start.0, ry - start.1);
                let mut r = orig;
                let Some(c) = self.client(win).cloned() else { return };
                let (minw, minh) = (c.min.0.max(60), c.min.1.max(40));
                if edges & E_LEFT != 0 { let w = (orig.w - ddx).clamp(minw, c.max.0); r.x = orig.x + orig.w - w; r.w = w; }
                if edges & E_RIGHT != 0 { r.w = (orig.w + ddx).clamp(minw, c.max.0); }
                if edges & E_TOP != 0 { let h = (orig.h - ddy).clamp(minh, c.max.1); r.y = orig.y + orig.h - h; r.h = h; }
                if edges & E_BOTTOM != 0 { r.h = (orig.h + ddy).clamp(minh, c.max.1); }
                if let Some(cm) = self.client_mut(win) {
                    cm.rect = r;
                }
                self.apply_geometry(win);
            }
            None => {
                // hover over strip buttons, and the resize cursor on edges
                if let Some(c) = self.client(e.event).cloned() {
                    if e.event == c.frame {
                        let h = self.hit(&c, e.event_x as i32, e.event_y as i32);
                        let hov = if (1..=3).contains(&h) { Some(h - 1) } else { None };
                        if hov != c.hover {
                            if let Some(cm) = self.client_mut(c.win) {
                                cm.hover = hov;
                            }
                            self.draw_strip(c.win);
                        }
                        let cur = if h >= 20 { self.cursors.edges.get(&(h - 20)).copied().unwrap_or(self.cursors.normal) } else { self.cursors.normal };
                        self.conn.change_window_attributes(c.frame, &ChangeWindowAttributesAux::new().cursor(cur)).ok();
                    }
                } else if self.ov.open {
                    self.overview_hover(rx, ry);
                } else if self.assist.open {
                    self.assist_hover(rx, ry);
                } else if self.taskbar.open {
                    self.taskbar_hover(rx, ry);
                }
            }
        }
    }

    /// Where a window dragged to (x, y) would snap: a side, a corner or the top edge.
    fn snap_target(&self, x: i32, y: i32) -> Option<String> {
        let m = self.monitor_at(x, y);
        let edge = 2;
        let corner = (64.0 * self.scale) as i32;
        let (l, r, t, b) = (x <= m.x + edge, x >= m.x + m.w - 1 - edge, y <= m.y + edge, y >= m.y + m.h - 1 - edge);
        let (near_t, near_b) = (y < m.y + corner, y > m.y + m.h - corner);
        let (near_l, near_r) = (x < m.x + corner, x > m.x + m.w - corner);
        Some(match () {
            _ if (l && near_t) || (t && near_l) => "top-left",
            _ if (r && near_t) || (t && near_r) => "top-right",
            _ if (l && near_b) || (b && near_l) => "bottom-left",
            _ if (r && near_b) || (b && near_r) => "bottom-right",
            _ if l => "left",
            _ if r => "right",
            _ if t => "fill",
            _ => return None,
        }.to_string())
    }

    pub fn end_drag(&mut self) {
        let Some(d) = self.drag.take() else { return };
        self.conn.ungrab_pointer(CURRENT_TIME).ok();
        if let Drag::Move { win, moved: true, .. } = d {
            let p = self.conn.query_pointer(self.root).ok().and_then(|c| c.reply().ok());
            if let (Some(p), true) = (p, self.cfg.windows.snap) {
                if let Some(t) = self.snap_target(p.root_x as i32, p.root_y as i32) {
                    if let Some(cm) = self.client_mut(win) {
                        cm.restore = Some(cm.rect);
                    }
                    self.snap(win, &t);
                    self.offer_assist(win, &t);
                }
            }
            self.send_configure(win);
        }
        self.snap_preview = None;
        self.comp.dirty = true;
    }

    /// Shaking a window (quick back-and-forth) tucks every other window on that screen away;
    /// shaking it again brings them back.
    fn track_shake(&mut self, win: Window, x: i32) {
        if !self.cfg.windows.shake_to_tuck {
            return;
        }
        let now = Instant::now();
        self.shake.push((now, x));
        self.shake.retain(|(t, _)| now.duration_since(*t).as_millis() < 700);
        let mut turns = 0;
        let mut dir = 0;
        let mut last = self.shake.first().map(|s| s.1).unwrap_or(x);
        let mut span = 0;
        for &(_, px) in &self.shake {
            let d = (px - last).signum();
            if d != 0 && d != dir {
                if dir != 0 { turns += 1; }
                dir = d;
            }
            span = span.max((px - last).abs());
            last = px;
        }
        let travel = self.shake.iter().map(|s| s.1).max().unwrap_or(0) - self.shake.iter().map(|s| s.1).min().unwrap_or(0);
        if turns >= 4 && travel > (60.0 * self.scale) as i32 {
            self.shake.clear();
            if self.shaken.is_empty() {
                let mon = self.client(win).map(|c| self.monitor_of(&c.rect)).unwrap_or(self.monitors[0]);
                let others: Vec<Window> = self.clients.iter().filter(|c| c.win != win && !c.tucked && mon.overlap(&c.rect) > 0).map(|c| c.win).collect();
                for o in &others {
                    self.tuck(*o);
                }
                self.shaken = others;
            } else {
                for o in std::mem::take(&mut self.shaken) {
                    self.untuck(o);
                }
                self.raise(win);
            }
            let _ = span;
        }
    }

    // ── keyboard ─────────────────────────────────────────────────────────────────────────
    fn key_press(&mut self, e: KeyPressEvent) {
        let state = u16::from(e.state);
        let sym = self.keymap.sym(e.detail, 0);
        if self.assist.open {
            self.assist_key(sym);
            return;
        }
        if self.taskbar.open {
            self.taskbar_key(sym);
            return;
        }
        if self.ov.open {
            self.overview_key(e.detail, state, sym);
            return;
        }
        if sym == x::KS_SUPER_L || sym == x::KS_SUPER_R {
            self.super_down = true;
            self.super_used = false;
            return;
        }
        if self.super_down {
            self.super_used = true;
        }
        // Alt+Tab: walk the focus history while Alt is held
        if sym == x::KS_TAB && state & x::MOD_ALT != 0 {
            let order: Vec<Window> = self.mru.iter().copied().filter(|w| self.client(*w).is_some_and(|c| c.ws == self.ws)).collect();
            if order.is_empty() {
                return;
            }
            let back = state & x::MOD_SHIFT != 0;
            let i = self.alt_tab.unwrap_or(0);
            let n = order.len();
            let next = if back { (i + n - 1) % n } else { (i + 1) % n };
            self.alt_tab = Some(next);
            self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok();
            let w = order[next];
            if self.client(w).is_some_and(|c| c.tucked) {
                self.untuck(w);
            }
            self.raise(w);
            self.comp.highlight = Some(w);
            self.comp.dirty = true;
            return;
        }
        if let Some(action) = self.bound(state, sym) {
            let _ = self.run(&action);
        }
    }

    fn key_release(&mut self, e: KeyReleaseEvent) {
        let sym = self.keymap.sym(e.detail, 0);
        if (sym == x::KS_SUPER_L || sym == x::KS_SUPER_R) && self.super_down {
            self.super_down = false;
            if !self.super_used && self.binds.iter().any(|b| b.1 == x::KS_SUPER_L && b.0 == 0) {
                let act = self.binds.iter().find(|b| b.1 == x::KS_SUPER_L && b.0 == 0).map(|b| b.2.clone()).unwrap_or_default();
                let _ = self.run(&act);
            }
            return;
        }
        if (sym == x::KS_ALT_L || sym == x::KS_ALT_R) && self.alt_tab.is_some() {
            let order: Vec<Window> = self.mru.iter().copied().filter(|w| self.client(*w).is_some()).collect();
            let i = self.alt_tab.take().unwrap_or(0);
            self.conn.ungrab_keyboard(CURRENT_TIME).ok();
            self.comp.highlight = None;
            if let Some(&w) = order.get(i) {
                self.focus(Some(w));
            }
        }
    }

    /// Switch night and paper without reading the settings file again.
    pub fn set_ground(&mut self, ground: &str) {
        self.theme = Theme::new(ground == "night", &self.cfg.look.accent);
        self.comp.forget_shapes(&self.conn);
        let wins: Vec<Window> = self.clients.iter().map(|c| c.win).collect();
        for w in wins {
            self.draw_strip(w);
        }
        self.ov.dirty = true;
        if self.lock.locked {
            self.draw_lock();
        }
        self.comp.dirty = true;
    }

    // ── config reload ────────────────────────────────────────────────────────────────────
    pub fn reload(&mut self) {
        let (cfg, err) = config::load();
        let wall_changed = cfg.look.wallpaper_night != self.cfg.look.wallpaper_night || cfg.look.wallpaper_paper != self.cfg.look.wallpaper_paper;
        let fonts_changed = cfg.look.ui_font != self.cfg.look.ui_font || cfg.look.mono_font != self.cfg.look.mono_font;
        let tiling_was = self.cfg.windows.tiling;
        self.cfg = cfg;
        self.cfg_error = err.clone();
        if self.cfg.windows.tiling && !tiling_was {
            // tiling switched on: windows floated by hand stay floating, the rest tile now
            self.retile();
        }
        self.cfg_mtime = config::mtime();
        self.theme = Theme::new(self.cfg.look.ground == "night", &self.cfg.look.accent);
        if fonts_changed {
            self.fonts = draw::Fonts::load(&self.cfg.look.ui_font, &self.cfg.look.mono_font);
        }
        let scale_changed = (self.scale - self.cfg.look.scale).abs() > 0.001;
        self.scale = self.cfg.look.scale;
        self.grab_keys();
        if wall_changed {
            self.comp.load_wallpapers(&self.conn, &self.cfg, &self.monitors);
        }
        self.comp.forget_shapes(&self.conn);
        let wins: Vec<Window> = self.clients.iter().map(|c| c.win).collect();
        for w in wins {
            if let Some(c) = self.client_mut(w) {
                c.strip_color = None;
            }
            if scale_changed { self.apply_geometry(w); } else { self.draw_strip(w); }
        }
        self.ov.load_apps();
        self.ov.dirty = true;
        if let Some(e) = err {
            self.say(format!("Settings file has a mistake; using defaults. {e}"));
        }
        self.comp.dirty = true;
    }
}

pub fn state_dir() -> std::path::PathBuf {
    std::env::var("XDG_STATE_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| config::home().join(".local/state")).join("samurai")
}

fn load_places() -> HashMap<String, Rect> {
    let p = state_dir().join("places.json");
    let v: serde_json::Value = std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut out = HashMap::new();
    if let Some(o) = v.as_object() {
        for (k, r) in o {
            if let Some(a) = r.as_array() {
                let n: Vec<i32> = a.iter().filter_map(|x| x.as_i64()).map(|x| x as i32).collect();
                if n.len() == 4 {
                    out.insert(k.clone(), Rect::new(n[0], n[1], n[2], n[3]));
                }
            }
        }
    }
    out
}

fn save_places(p: &HashMap<String, Rect>) {
    let o: serde_json::Map<String, serde_json::Value> = p.iter().map(|(k, r)| (k.clone(), serde_json::json!([r.x, r.y, r.w, r.h]))).collect();
    let dir = state_dir();
    std::fs::create_dir_all(&dir).ok();
    std::fs::write(dir.join("places.json"), serde_json::Value::Object(o).to_string()).ok();
}

/// An app's own opacity wish: _NET_WM_WINDOW_OPACITY, 0..0xffffffff.
fn read_opacity(conn: &RustConnection, win: Window, atom: Atom) -> Option<f32> {
    x::get_u32s(conn, win, atom, AtomEnum::CARDINAL).first().map(|v| *v as f32 / u32::MAX as f32)
}

impl Wm {
    /// How solid a window is drawn: the focused one and the others each have a setting; full
    /// screen and "always solid" apps are solid; an app's own wish is multiplied in.
    pub fn window_opacity(&self, c: &Client) -> f32 {
        if c.fullscreen || self.lock.locked {
            return 1.0;
        }
        let solid = self.cfg.windows.opaque_apps.iter().any(|a| a.eq_ignore_ascii_case(&c.class) || a.eq_ignore_ascii_case(&c.instance));
        let base = if solid { 1.0 } else if self.focused == Some(c.win) { self.cfg.windows.opacity_active } else { self.cfg.windows.opacity_inactive };
        (base * c.opacity_hint.unwrap_or(1.0)).clamp(0.2, 1.0)
    }
}

impl Wm {
    /// The screens left to right (then top to bottom), and which one holds `r`.
    fn monitor_index(&self, r: &Rect) -> usize {
        let m = self.monitor_of(r);
        self.monitors_sorted().iter().position(|x| *x == m).unwrap_or(0)
    }

    pub fn monitors_sorted(&self) -> Vec<Rect> {
        let mut v = self.monitors.clone();
        v.sort_by_key(|m| (m.x, m.y));
        v
    }

    /// Move a window to the next (or previous) screen, keeping where it sat on its screen.
    pub fn send_to_monitor(&mut self, w: Window, next: bool) {
        let mons = self.monitors_sorted();
        if mons.len() < 2 {
            self.say("Only one screen is connected".into());
            return;
        }
        let Some(c) = self.client(w).cloned() else { return };
        let vis = self.visible_rect(&c);
        let i = self.monitor_index(&vis);
        let j = if next { (i + 1) % mons.len() } else { (i + mons.len() - 1) % mons.len() };
        let (from, to) = (self.workarea(&mons[i]), self.workarea(&mons[j]));
        // the same place, relative to the screen; never larger than the new screen
        let fx = (vis.x - from.x) as f32 / from.w.max(1) as f32;
        let fy = (vis.y - from.y) as f32 / from.h.max(1) as f32;
        let nw = vis.w.min(to.w);
        let nh = vis.h.min(to.h);
        let area = Rect::new((to.x + (fx * to.w as f32) as i32).min(to.x + to.w - nw), (to.y + (fy * to.h as f32) as i32).min(to.y + to.h - nh), nw, nh);
        if c.fullscreen {
            self.set_fullscreen(w, false);
        }
        let snapped = c.snapped.clone();
        let target = self.client_rect_for(&c, area);
        if let Some(cm) = self.client_mut(w) {
            cm.restore = None;
        }
        self.move_to(w, target);
        if let Some(sn) = snapped {
            self.snap(w, &sn); // a snapped window keeps its half / quarter on the new screen
        }
        self.retile();
        self.focus(Some(w));
    }

    /// Go to the next (or previous) screen: its last-used window gets the focus, and the pointer
    /// moves there.
    pub fn focus_monitor(&mut self, next: bool) {
        let mons = self.monitors_sorted();
        if mons.len() < 2 {
            return;
        }
        let cur = self.pointer_monitor();
        let i = mons.iter().position(|m| *m == cur).unwrap_or(0);
        let m = mons[if next { (i + 1) % mons.len() } else { (i + mons.len() - 1) % mons.len() }];
        let w = self.mru.iter().copied().find(|w| self.client(*w).is_some_and(|c| !c.tucked && c.ws == self.ws && self.monitor_of(&self.visible_rect(c)) == m));
        self.conn.warp_pointer(x11rb::NONE, self.root, 0, 0, 0, 0, m.cx() as i16, m.cy() as i16).ok();
        self.focus(w);
    }
}
