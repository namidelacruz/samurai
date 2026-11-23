//! The compositor, built into the window manager: every top-level window is redirected off
//! screen and painted here with XRender (rounded corners, shadows, fades, the morph when a
//! window snaps, the overview layout). XRender runs everywhere X runs and the Xorg modesetting
//! driver accelerates it, so there is no separate compositor to install or configure.

use crate::config::{self, Config};
use crate::draw;
use crate::theme::Rgba;
use crate::wm::Rect;
use crate::x::Atoms;
use std::collections::HashMap;
use std::time::Instant;
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::composite::{ConnectionExt as _, Redirect};
use x11rb::protocol::damage::{ConnectionExt as _, ReportLevel};
use x11rb::protocol::render::{self, ConnectionExt as _, CreatePictureAux, PictOp, Pictformat, Repeat, Transform};
use x11rb::protocol::shape::SK;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

#[derive(Clone, Debug)]
pub struct Top {
    pub win: Window,
    pub rect: Rect,
    pub mapped: bool,
    pub or: bool,
    pub input_only: bool,
    pub depth: u8,
    pub visual: Visualid,
    pub pixmap: Pixmap,
    pub pic: render::Picture,
    pub damage: u32,
    pub morph: Option<(Rect, Rect, Instant, f32)>,
    pub fade_in: Option<(Instant, f32)>,
    pub fade_out: Option<(Instant, f32, bool)>,
    pub ghost: bool,
}

/// How the WM wants a frame painted.
#[derive(Clone, Copy, Debug)]
pub struct FrameLook {
    pub inset: i32,
    pub radius: f32,
    pub shadow: bool,
}

pub struct Layer {
    pub pic: render::Picture,
    pub pixmap: Pixmap,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub alpha: f32,
}

pub struct PaintInfo<'a> {
    pub frames: &'a HashMap<Window, FrameLook>,
    pub night: bool,
    pub ground: Rgba,
    pub shadow: Rgba,
    pub scrim: Option<Rgba>,
    pub overview: Option<(&'a HashMap<Window, Rect>, f32)>,
    /// Cards fade back while the search results are up.
    pub ov_dim: f32,
    pub locked: bool,
    pub tuck_to: (i32, i32),
}

pub struct Comp {
    pub overlay: Window,
    root: Window,
    size: (i32, i32),
    back: Pixmap,
    back_pic: render::Picture,
    out_pic: render::Picture,
    root_fmt: Pictformat,
    argb: Pictformat,
    a8: Pictformat,
    visual_fmt: HashMap<Visualid, Pictformat>,
    gc24: Gcontext,
    gc32: Gcontext,
    gc8: Gcontext,
    root_depth: u8,
    pub tops: Vec<Top>,
    walls: [Option<render::Picture>; 2],
    wall_pixmaps: Vec<Pixmap>,
    shadows: HashMap<(i32, i32, i32, bool), (render::Picture, Pixmap, i32)>,
    masks: HashMap<(i32, i32, i32, i32), (render::Picture, Pixmap)>,
    alphas: HashMap<u8, render::Picture>,
    pub layers: Vec<(String, Layer)>,
    pub dirty: bool,
    pub highlight: Option<Window>,
    max_req: usize,
    pub started: Instant,
}

fn fixed(v: f32) -> render::Fixed {
    (v * 65536.0) as i32
}

fn ease(t: f32) -> f32 {
    // ease-out cubic: quick to start, soft to land
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect::new(
        lerp(a.x as f32, b.x as f32, t).round() as i32,
        lerp(a.y as f32, b.y as f32, t).round() as i32,
        lerp(a.w as f32, b.w as f32, t).round().max(1.0) as i32,
        lerp(a.h as f32, b.h as f32, t).round().max(1.0) as i32,
    )
}

impl Comp {
    pub fn new(conn: &RustConnection, screen: usize, root: Window, atoms: &Atoms, owner: Window) -> Result<Self, String> {
        conn.composite_query_version(0, 4).map_err(|e| e.to_string())?.reply().map_err(|_| "the X server has no Composite extension")?;
        conn.damage_query_version(1, 1).map_err(|e| e.to_string())?.reply().map_err(|_| "the X server has no Damage extension")?;
        conn.xfixes_query_version(5, 0).map_err(|e| e.to_string())?.reply().map_err(|_| "the X server has no XFixes extension")?;
        let formats = conn.render_query_pict_formats().map_err(|e| e.to_string())?.reply().map_err(|_| "the X server has no Render extension")?;
        // announce that a compositor runs, so apps may use transparency
        let sel = conn.intern_atom(false, format!("_NET_WM_CM_S{screen}").as_bytes()).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?.atom;
        conn.set_selection_owner(owner, sel, x11rb::CURRENT_TIME).ok();
        let _ = atoms;
        let s = &conn.setup().roots[screen];
        let (w, h) = (s.width_in_pixels as i32, s.height_in_pixels as i32);
        let root_depth = s.root_depth;
        let mut visual_fmt = HashMap::new();
        for sc in &formats.screens {
            for d in &sc.depths {
                for v in &d.visuals {
                    visual_fmt.insert(v.visual, v.format);
                }
            }
        }
        let root_fmt = *visual_fmt.get(&s.root_visual).ok_or("no picture format for the root window")?;
        let find = |depth: u8, alpha: bool| {
            formats.formats.iter().find(|f| f.depth == depth && f.type_ == render::PictType::DIRECT && ((f.direct.alpha_mask != 0) == alpha)).map(|f| f.id)
        };
        let argb = find(32, true).ok_or("no 32-bit picture format")?;
        let a8 = find(8, true).ok_or("no 8-bit alpha picture format")?;

        conn.composite_redirect_subwindows(root, Redirect::MANUAL).map_err(|e| e.to_string())?.check().map_err(|_| "another compositor is already running")?;
        let overlay = conn.composite_get_overlay_window(root).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?.overlay_win;
        // the overlay shows pictures but never takes clicks
        let region = conn.generate_id().map_err(|e| e.to_string())?;
        conn.xfixes_create_region(region, &[]).ok();
        conn.xfixes_set_window_shape_region(overlay, SK::INPUT, 0, 0, region).ok();
        conn.xfixes_destroy_region(region).ok();
        let out_pic = conn.generate_id().map_err(|e| e.to_string())?;
        conn.render_create_picture(out_pic, overlay, root_fmt, &CreatePictureAux::new()).ok();

        let mk_gc = |depth: u8| -> Result<Gcontext, String> {
            let pm = conn.generate_id().map_err(|e| e.to_string())?;
            conn.create_pixmap(depth, pm, root, 1, 1).map_err(|e| e.to_string())?;
            let gc = conn.generate_id().map_err(|e| e.to_string())?;
            conn.create_gc(gc, pm, &CreateGCAux::new()).map_err(|e| e.to_string())?;
            conn.free_pixmap(pm).ok();
            Ok(gc)
        };
        let gc24 = mk_gc(root_depth)?;
        let gc32 = mk_gc(32)?;
        let gc8 = mk_gc(8)?;
        let max_req = conn.maximum_request_bytes();
        let mut c = Comp {
            overlay,
            root,
            size: (w, h),
            back: 0,
            back_pic: 0,
            out_pic,
            root_fmt,
            argb,
            a8,
            visual_fmt,
            gc24,
            gc32,
            gc8,
            root_depth,
            tops: vec![],
            walls: [None, None],
            wall_pixmaps: vec![],
            shadows: HashMap::new(),
            masks: HashMap::new(),
            alphas: HashMap::new(),
            layers: vec![],
            dirty: true,
            highlight: None,
            max_req,
            started: Instant::now(),
        };
        c.make_back(conn);
        // everything already on screen
        if let Some(tree) = conn.query_tree(root).ok().and_then(|r| r.reply().ok()) {
            for w in tree.children {
                if w == overlay {
                    continue;
                }
                c.add(conn, w);
            }
        }
        Ok(c)
    }

    pub fn is_ours(&self, w: Window) -> bool {
        w == self.overlay
    }

    fn make_back(&mut self, conn: &RustConnection) {
        if self.back != 0 {
            conn.render_free_picture(self.back_pic).ok();
            conn.free_pixmap(self.back).ok();
        }
        let (w, h) = self.size;
        self.back = conn.generate_id().unwrap_or(0);
        conn.create_pixmap(self.root_depth, self.back, self.root, w as u16, h as u16).ok();
        self.back_pic = conn.generate_id().unwrap_or(0);
        conn.render_create_picture(self.back_pic, self.back, self.root_fmt, &CreatePictureAux::new()).ok();
    }

    pub fn resize(&mut self, conn: &RustConnection, size: (i32, i32)) {
        self.size = size;
        self.make_back(conn);
        conn.render_free_picture(self.out_pic).ok();
        self.out_pic = conn.generate_id().unwrap_or(0);
        conn.render_create_picture(self.out_pic, self.overlay, self.root_fmt, &CreatePictureAux::new()).ok();
        self.dirty = true;
    }

    // ── window tracking ──────────────────────────────────────────────────────────────────
    fn add(&mut self, conn: &RustConnection, w: Window) {
        if self.tops.iter().any(|t| t.win == w && !t.ghost) || w == self.overlay {
            return;
        }
        let Some(attr) = conn.get_window_attributes(w).ok().and_then(|r| r.reply().ok()) else { return };
        let Some(geo) = conn.get_geometry(w).ok().and_then(|r| r.reply().ok()) else { return };
        let mut t = Top {
            win: w,
            rect: Rect::new(geo.x as i32, geo.y as i32, geo.width as i32 + 2 * geo.border_width as i32, geo.height as i32 + 2 * geo.border_width as i32),
            mapped: false,
            or: attr.override_redirect,
            input_only: attr.class == WindowClass::INPUT_ONLY,
            depth: geo.depth,
            visual: attr.visual,
            pixmap: 0,
            pic: 0,
            damage: 0,
            morph: None,
            fade_in: None,
            fade_out: None,
            ghost: false,
        };
        if !t.input_only {
            t.damage = conn.generate_id().unwrap_or(0);
            conn.damage_create(t.damage, w, ReportLevel::NON_EMPTY).ok();
        }
        if attr.map_state == MapState::VIEWABLE {
            t.mapped = true;
        }
        self.tops.push(t);
        if attr.map_state == MapState::VIEWABLE {
            let i = self.tops.len() - 1;
            self.bind(conn, i);
        }
    }

    /// Give a window's current contents a picture.
    fn bind(&mut self, conn: &RustConnection, i: usize) {
        let t = &mut self.tops[i];
        if t.input_only || !t.mapped {
            return;
        }
        if t.pic != 0 {
            conn.render_free_picture(t.pic).ok();
            conn.free_pixmap(t.pixmap).ok();
            t.pic = 0;
            t.pixmap = 0;
        }
        let Some(fmt) = self.visual_fmt.get(&t.visual).copied() else { return };
        let pm = conn.generate_id().unwrap_or(0);
        if conn.composite_name_window_pixmap(t.win, pm).ok().and_then(|c| c.check().ok()).is_none() {
            return;
        }
        let pic = conn.generate_id().unwrap_or(0);
        conn.render_create_picture(pic, pm, fmt, &CreatePictureAux::new()).ok();
        conn.render_set_picture_filter(pic, b"bilinear", &[]).ok();
        t.pixmap = pm;
        t.pic = pic;
        self.dirty = true;
    }

    fn free(&self, conn: &RustConnection, t: &Top) {
        if t.pic != 0 {
            conn.render_free_picture(t.pic).ok();
            conn.free_pixmap(t.pixmap).ok();
        }
        if t.damage != 0 && !t.ghost {
            conn.damage_destroy(t.damage).ok();
        }
    }

    fn pos(&self, w: Window) -> Option<usize> {
        self.tops.iter().position(|t| t.win == w && !t.ghost)
    }

    fn restack(&mut self, w: Window, above: Window) {
        let Some(i) = self.pos(w) else { return };
        let t = self.tops.remove(i);
        if above == 0 {
            self.tops.insert(0, t);
        } else if let Some(j) = self.pos(above) {
            self.tops.insert(j + 1, t);
        } else {
            self.tops.push(t);
        }
    }

    pub fn event(&mut self, conn: &RustConnection, ev: &Event) {
        match ev {
            Event::CreateNotify(e) if e.parent == self.root => self.add(conn, e.window),
            Event::MapNotify(e) if e.event == self.root => {
                if let Some(i) = self.pos(e.window) {
                    self.tops[i].mapped = true;
                    self.bind(conn, i);
                } else {
                    self.add(conn, e.window);
                }
                self.dirty = true;
            }
            Event::UnmapNotify(e) if e.event == self.root => {
                if let Some(i) = self.pos(e.window) {
                    self.tops[i].mapped = false;
                    // a fade or tuck keeps the last picture until it ends
                    if self.tops[i].fade_out.is_none() && self.tops[i].pic != 0 {
                        conn.render_free_picture(self.tops[i].pic).ok();
                        conn.free_pixmap(self.tops[i].pixmap).ok();
                        self.tops[i].pic = 0;
                        self.tops[i].pixmap = 0;
                    }
                }
                self.dirty = true;
            }
            Event::ConfigureNotify(e) if e.event == self.root => {
                if let Some(i) = self.pos(e.window) {
                    let t = &mut self.tops[i];
                    let resized = t.rect.w != e.width as i32 + 2 * e.border_width as i32 || t.rect.h != e.height as i32 + 2 * e.border_width as i32;
                    t.rect = Rect::new(e.x as i32, e.y as i32, e.width as i32 + 2 * e.border_width as i32, e.height as i32 + 2 * e.border_width as i32);
                    t.or = e.override_redirect;
                    if resized && t.mapped {
                        self.bind(conn, i);
                    }
                    self.restack(e.window, e.above_sibling);
                } else if e.window == self.root {
                    // the root changed size: handled by the RandR path
                }
                self.dirty = true;
            }
            Event::CirculateNotify(e) if e.event == self.root => {
                if let Some(i) = self.pos(e.window) {
                    let t = self.tops.remove(i);
                    if e.place == Place::ON_TOP { self.tops.push(t) } else { self.tops.insert(0, t) }
                }
                self.dirty = true;
            }
            Event::ReparentNotify(e) if e.event == self.root || e.parent == self.root => {
                if e.parent == self.root {
                    self.add(conn, e.window);
                } else if let Some(i) = self.pos(e.window) {
                    let t = self.tops.remove(i);
                    self.free(conn, &t);
                }
                self.dirty = true;
            }
            Event::DestroyNotify(e) if e.event == self.root => {
                if let Some(i) = self.pos(e.window) {
                    if self.tops[i].fade_out.is_some() && self.tops[i].pic != 0 {
                        // keep painting what it last showed while it fades
                        conn.damage_destroy(self.tops[i].damage).ok();
                        self.tops[i].ghost = true;
                    } else {
                        let t = self.tops.remove(i);
                        self.free(conn, &t);
                    }
                }
                self.dirty = true;
            }
            Event::DamageNotify(e) => {
                conn.damage_subtract(e.damage, x11rb::NONE, x11rb::NONE).ok();
                self.dirty = true;
            }
            _ => {}
        }
    }

    // ── animations the WM asks for ───────────────────────────────────────────────────────
    pub fn opening(&mut self, w: Window) {
        if let Some(i) = self.pos(w) {
            self.tops[i].fade_in = Some((Instant::now(), 0.18));
            self.tops[i].fade_out = None;
        }
    }
    pub fn closing(&mut self, _conn: &RustConnection, w: Window) {
        if let Some(i) = self.pos(w) {
            if self.tops[i].pic != 0 {
                self.tops[i].fade_out = Some((Instant::now(), 0.14, false));
            }
        }
    }
    pub fn tucking(&mut self, w: Window, motion: f32) {
        if motion <= 0.0 {
            return;
        }
        if let Some(i) = self.pos(w) {
            if self.tops[i].pic != 0 {
                self.tops[i].fade_out = Some((Instant::now(), 0.22 * motion, true));
            }
        }
    }
    pub fn morph(&mut self, w: Window, from: Rect, to: Rect, motion: f32) {
        if motion <= 0.0 || from == to {
            return;
        }
        if let Some(i) = self.pos(w) {
            self.tops[i].morph = Some((from, to, Instant::now(), 0.2 * motion));
            self.dirty = true;
        }
    }

    pub fn animating(&self) -> bool {
        let now = Instant::now();
        self.tops.iter().any(|t| {
            t.morph.is_some_and(|m| now.duration_since(m.2).as_secs_f32() < m.3)
                || t.fade_in.is_some_and(|f| now.duration_since(f.0).as_secs_f32() < f.1)
                || t.fade_out.is_some()
        })
    }

    // ── uploads ──────────────────────────────────────────────────────────────────────────
    fn put(&self, conn: &RustConnection, depth: u8, drawable: Drawable, w: u32, h: u32, x: i32, y: i32, data: &[u8]) {
        if w == 0 || h == 0 {
            return;
        }
        let gc = match depth { 32 => self.gc32, 8 => self.gc8, _ => self.gc24 };
        let stride = data.len() / h as usize;
        let rows = ((self.max_req.saturating_sub(64)) / stride.max(1)).max(1);
        let mut y0 = 0;
        while y0 < h as usize {
            let n = rows.min(h as usize - y0);
            let chunk = &data[y0 * stride..(y0 + n) * stride];
            conn.put_image(ImageFormat::Z_PIXMAP, drawable, gc, w as u16, n as u16, x as i16, (y + y0 as i32) as i16, 0, depth, chunk).ok();
            y0 += n;
        }
    }

    /// Draw an opaque CPU pixmap straight into a window (title strips into their frames).
    pub fn put_rgb(&self, conn: &RustConnection, win: Window, px: &draw::Px, x: i32, y: i32) {
        let data = draw::to_bgra(px);
        self.put(conn, self.root_depth, win, px.width(), px.height(), x, y, &data);
    }

    fn upload_argb(&self, conn: &RustConnection, px: &draw::Px) -> Option<(render::Picture, Pixmap)> {
        let pm = conn.generate_id().ok()?;
        conn.create_pixmap(32, pm, self.root, px.width() as u16, px.height() as u16).ok()?;
        self.put(conn, 32, pm, px.width(), px.height(), 0, 0, &draw::to_bgra(px));
        let pic = conn.generate_id().ok()?;
        conn.render_create_picture(pic, pm, self.argb, &CreatePictureAux::new()).ok()?;
        conn.render_set_picture_filter(pic, b"bilinear", &[]).ok();
        Some((pic, pm))
    }

    /// Set (or replace) a named overlay layer: the overview, the lock face, the corner pill…
    pub fn set_layer(&mut self, conn: &RustConnection, name: &str, px: &draw::Px, x: i32, y: i32, alpha: f32) {
        let up = self.upload_argb(conn, px);
        if let Some(i) = self.layers.iter().position(|l| l.0 == name) {
            let old = self.layers.remove(i).1;
            conn.render_free_picture(old.pic).ok();
            conn.free_pixmap(old.pixmap).ok();
        }
        if let Some((pic, pixmap)) = up {
            self.layers.push((name.to_string(), Layer { pic, pixmap, x, y, w: px.width(), h: px.height(), alpha }));
        }
        self.layers.sort_by_key(|l| layer_rank(&l.0));
        self.dirty = true;
    }

    pub fn layer_alpha(&mut self, name: &str, alpha: f32) {
        if let Some(l) = self.layers.iter_mut().find(|l| l.0 == name) {
            if (l.1.alpha - alpha).abs() > 0.001 {
                l.1.alpha = alpha;
                self.dirty = true;
            }
        }
    }

    pub fn drop_layer(&mut self, conn: &RustConnection, name: &str) {
        if let Some(i) = self.layers.iter().position(|l| l.0 == name) {
            let old = self.layers.remove(i).1;
            conn.render_free_picture(old.pic).ok();
            conn.free_pixmap(old.pixmap).ok();
            self.dirty = true;
        }
    }

    pub fn has_layer(&self, name: &str) -> bool {
        self.layers.iter().any(|l| l.0 == name)
    }

    pub fn load_wallpapers(&mut self, conn: &RustConnection, cfg: &Config, monitors: &[Rect]) {
        for p in self.wall_pixmaps.drain(..) {
            conn.free_pixmap(p).ok();
        }
        for (i, path, ground, name) in [(0, &cfg.look.wallpaper_night, Rgba::hex(0x101114), "night"), (1, &cfg.look.wallpaper_paper, Rgba::hex(0xECE8DF), "paper")] {
            if let Some(p) = self.walls[i].take() {
                conn.render_free_picture(p).ok();
            }
            let (w, h) = (self.size.0.max(1) as u32, self.size.1.max(1) as u32);
            let Some(mut px) = draw::Px::new(w, h) else { continue };
            px.fill(draw::tiny_skia_color(ground));
            let file = config::wallpaper(path, name);
            for m in monitors {
                if let Some(img) = draw::svg_cover(&file, m.w as u32, m.h as u32) {
                    px.draw_pixmap(m.x, m.y, img.as_ref(), &Default::default(), Default::default(), None);
                }
            }
            if let Some((pic, pm)) = self.upload_argb(conn, &px) {
                self.walls[i] = Some(pic);
                self.wall_pixmaps.push(pm);
            }
        }
        self.dirty = true;
    }

    pub fn forget_shapes(&mut self, conn: &RustConnection) {
        for (_, (p, pm, _)) in self.shadows.drain() {
            conn.render_free_picture(p).ok();
            conn.free_pixmap(pm).ok();
        }
        for (_, (p, pm)) in self.masks.drain() {
            conn.render_free_picture(p).ok();
            conn.free_pixmap(pm).ok();
        }
    }

    fn mask(&mut self, conn: &RustConnection, w: i32, h: i32, inset: i32, r: i32) -> Option<render::Picture> {
        let key = (w, h, inset, r);
        if let Some(m) = self.masks.get(&key) {
            return Some(m.0);
        }
        if self.masks.len() > 64 {
            self.forget_masks(conn);
        }
        let data = draw::mask_a8(w as u32, h as u32, inset as f32, r as f32);
        let pm = conn.generate_id().ok()?;
        conn.create_pixmap(8, pm, self.root, w as u16, h as u16).ok()?;
        self.put(conn, 8, pm, w as u32, h as u32, 0, 0, &data);
        let pic = conn.generate_id().ok()?;
        conn.render_create_picture(pic, pm, self.a8, &CreatePictureAux::new()).ok()?;
        conn.render_set_picture_filter(pic, b"bilinear", &[]).ok();
        self.masks.insert(key, (pic, pm));
        Some(pic)
    }

    fn forget_masks(&mut self, conn: &RustConnection) {
        for (_, (p, pm)) in self.masks.drain() {
            conn.render_free_picture(p).ok();
            conn.free_pixmap(pm).ok();
        }
    }

    fn shadow(&mut self, conn: &RustConnection, w: i32, h: i32, r: i32, color: Rgba, night: bool) -> Option<(render::Picture, i32)> {
        let key = (w, h, r, night);
        if let Some(s) = self.shadows.get(&key) {
            return Some((s.0, s.2));
        }
        if self.shadows.len() > 48 {
            for (_, (p, pm, _)) in self.shadows.drain() {
                conn.render_free_picture(p).ok();
                conn.free_pixmap(pm).ok();
            }
        }
        let spread = crate::theme::SHADOW;
        let px = draw::shadow(w as u32, h as u32, r as f32, spread as u32, spread as f32 * 0.35, color)?;
        let (pic, pm) = self.upload_argb(conn, &px)?;
        self.shadows.insert(key, (pic, pm, spread));
        Some((pic, spread))
    }

    fn alpha(&mut self, conn: &RustConnection, a: f32) -> render::Picture {
        let k = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        if let Some(p) = self.alphas.get(&k) {
            return *p;
        }
        let pm = conn.generate_id().unwrap_or(0);
        conn.create_pixmap(8, pm, self.root, 1, 1).ok();
        let pic = conn.generate_id().unwrap_or(0);
        conn.render_create_picture(pic, pm, self.a8, &CreatePictureAux::new().repeat(Repeat::NORMAL)).ok();
        let c = render::Color { red: 0, green: 0, blue: 0, alpha: (k as u16) << 8 | k as u16 };
        conn.render_fill_rectangles(PictOp::SRC, pic, c, &[Rectangle { x: 0, y: 0, width: 1, height: 1 }]).ok();
        conn.free_pixmap(pm).ok();
        self.alphas.insert(k, pic);
        pic
    }

    fn set_scale(conn: &RustConnection, pic: render::Picture, sx: f32, sy: f32) {
        let t = Transform {
            matrix11: fixed(sx), matrix12: 0, matrix13: 0,
            matrix21: 0, matrix22: fixed(sy), matrix23: 0,
            matrix31: 0, matrix32: 0, matrix33: fixed(1.0),
        };
        conn.render_set_picture_transform(pic, t).ok();
    }

    /// Draw `src` (sw×sh) into `dst` rect, with an optional mask of the same size and an opacity.
    fn blit(&mut self, conn: &RustConnection, src: render::Picture, sw: i32, sh: i32, mask: Option<render::Picture>, dst: Rect, opacity: f32) {
        if dst.w <= 0 || dst.h <= 0 || sw <= 0 || sh <= 0 || opacity <= 0.003 {
            return;
        }
        let (sx, sy) = (sw as f32 / dst.w as f32, sh as f32 / dst.h as f32);
        let scaled = (sx - 1.0).abs() > 0.001 || (sy - 1.0).abs() > 0.001;
        Self::set_scale(conn, src, sx, sy);
        let mut m = mask;
        if opacity < 0.997 {
            m = Some(match mask {
                // multiply the shape by the opacity into a scratch mask of the source size
                Some(shape) => {
                    let pm = conn.generate_id().unwrap_or(0);
                    conn.create_pixmap(8, pm, self.root, sw as u16, sh as u16).ok();
                    let tmp = conn.generate_id().unwrap_or(0);
                    conn.render_create_picture(tmp, pm, self.a8, &CreatePictureAux::new()).ok();
                    conn.render_set_picture_filter(tmp, b"bilinear", &[]).ok();
                    let k = (opacity * 65535.0) as u16;
                    conn.render_fill_rectangles(PictOp::SRC, tmp, render::Color { red: 0, green: 0, blue: 0, alpha: k }, &[Rectangle { x: 0, y: 0, width: sw as u16, height: sh as u16 }]).ok();
                    Self::set_scale(conn, shape, 1.0, 1.0);
                    conn.render_composite(PictOp::IN, shape, x11rb::NONE, tmp, 0, 0, 0, 0, 0, 0, sw as u16, sh as u16).ok();
                    conn.free_pixmap(pm).ok();
                    Self::set_scale(conn, tmp, sx, sy);
                    conn.render_composite(PictOp::OVER, src, tmp, self.back_pic, 0, 0, 0, 0, dst.x as i16, dst.y as i16, dst.w as u16, dst.h as u16).ok();
                    conn.render_free_picture(tmp).ok();
                    if scaled { Self::set_scale(conn, src, 1.0, 1.0); }
                    return;
                }
                None => self.alpha(conn, opacity),
            });
            if mask.is_none() {
                conn.render_composite(PictOp::OVER, src, m.unwrap_or(0), self.back_pic, 0, 0, 0, 0, dst.x as i16, dst.y as i16, dst.w as u16, dst.h as u16).ok();
                if scaled { Self::set_scale(conn, src, 1.0, 1.0); }
                return;
            }
        }
        if let Some(mk) = m {
            Self::set_scale(conn, mk, sx, sy);
        }
        conn.render_composite(PictOp::OVER, src, m.unwrap_or(0), self.back_pic, 0, 0, 0, 0, dst.x as i16, dst.y as i16, dst.w as u16, dst.h as u16).ok();
        if scaled {
            Self::set_scale(conn, src, 1.0, 1.0);
            if let Some(mk) = m {
                Self::set_scale(conn, mk, 1.0, 1.0);
            }
        }
    }

    fn fill(&self, conn: &RustConnection, r: Rect, c: Rgba) {
        let a = c.3 as u16;
        // premultiplied, 16-bit channels
        let col = render::Color { red: (c.0 as u16 * a / 255) * 257, green: (c.1 as u16 * a / 255) * 257, blue: (c.2 as u16 * a / 255) * 257, alpha: a * 257 };
        conn.render_fill_rectangles(PictOp::OVER, self.back_pic, col, &[Rectangle { x: r.x as i16, y: r.y as i16, width: r.w as u16, height: r.h as u16 }]).ok();
    }

    // ── one frame ────────────────────────────────────────────────────────────────────────
    pub fn paint(&mut self, conn: &RustConnection, info: &PaintInfo) {
        let now = Instant::now();
        let (sw, sh) = self.size;
        let full = Rect::new(0, 0, sw, sh);
        // the ground and the ink
        if let Some(w) = self.walls[if info.night { 0 } else { 1 }] {
            conn.render_composite(PictOp::SRC, w, x11rb::NONE, self.back_pic, 0, 0, 0, 0, 0, 0, sw as u16, sh as u16).ok();
        } else {
            let g = info.ground;
            let col = render::Color { red: g.0 as u16 * 257, green: g.1 as u16 * 257, blue: g.2 as u16 * 257, alpha: 0xffff };
            conn.render_fill_rectangles(PictOp::SRC, self.back_pic, col, &[Rectangle { x: 0, y: 0, width: sw as u16, height: sh as u16 }]).ok();
        }
        if info.locked {
            // nothing of the session shows behind the lock
            self.paint_layers(conn, |n| n == "lock");
            self.present(conn);
            return;
        }
        if let Some(s) = info.scrim {
            self.fill(conn, full, s);
        }
        let ov = info.overview;
        let mut done = vec![];
        for i in 0..self.tops.len() {
            let t = self.tops[i].clone();
            if t.input_only || t.pic == 0 || (!t.mapped && t.fade_out.is_none()) {
                continue;
            }
            if t.rect.x >= sw || t.rect.y >= sh || t.rect.x + t.rect.w <= 0 || t.rect.y + t.rect.h <= 0 {
                if t.fade_out.is_none() { continue; }
            }
            let look = info.frames.get(&t.win).copied();
            // the visible part of the window and where it is drawn this frame
            let inset = look.map(|l| l.inset).unwrap_or(0);
            let vis = t.rect.inset(inset);
            let mut target = vis;
            let mut opacity = 1.0f32;
            if let Some((from, to, start, dur)) = t.morph {
                let p = now.duration_since(start).as_secs_f32() / dur.max(0.001);
                if p >= 1.0 {
                    self.tops[i].morph = None;
                } else if to == vis {
                    target = lerp_rect(from, to, ease(p));
                } else {
                    self.tops[i].morph = None;
                }
            }
            if let Some((start, dur)) = t.fade_in {
                let p = now.duration_since(start).as_secs_f32() / dur.max(0.001);
                if p >= 1.0 {
                    self.tops[i].fade_in = None;
                } else {
                    let e = ease(p);
                    opacity *= e;
                    let s = lerp(0.94, 1.0, e);
                    target = scale_about(target, s);
                }
            }
            if let Some((start, dur, tuck)) = t.fade_out {
                let p = now.duration_since(start).as_secs_f32() / dur.max(0.001);
                if p >= 1.0 {
                    done.push(t.win);
                    continue;
                }
                let e = ease(p);
                opacity *= 1.0 - e;
                if tuck {
                    let to = Rect::new(info.tuck_to.0 - target.w / 8, info.tuck_to.1 - target.h / 8, target.w / 4, target.h / 4);
                    target = lerp_rect(target, to, e);
                } else {
                    target = scale_about(target, lerp(1.0, 0.96, e));
                }
            }
            if let Some((rects, p)) = ov {
                if let Some(r) = rects.get(&t.win) {
                    target = lerp_rect(target, *r, ease(p));
                    opacity *= info.ov_dim;
                } else if look.is_some() {
                    // windows not in the overview (tucked) stay hidden
                    continue;
                } else {
                    opacity *= 1.0 - ease(p);
                }
            }
            // the window, scaled from its visible part to `target`
            let k_x = target.w as f32 / vis.w.max(1) as f32;
            let k_y = target.h as f32 / vis.h.max(1) as f32;
            let dst = Rect::new(
                target.x - (inset as f32 * k_x).round() as i32,
                target.y - (inset as f32 * k_y).round() as i32,
                (t.rect.w as f32 * k_x).round() as i32,
                (t.rect.h as f32 * k_y).round() as i32,
            );
            if let Some(l) = look {
                if l.shadow {
                    let r = l.radius.round() as i32;
                    if let Some((sp, spread)) = self.shadow(conn, vis.w, vis.h, r, info.shadow, info.night) {
                        let sd = Rect::new(target.x - (spread as f32 * k_x) as i32, target.y - (spread as f32 * k_y) as i32,
                                           ((vis.w + 2 * spread) as f32 * k_x) as i32, ((vis.h + 2 * spread) as f32 * k_y) as i32);
                        self.blit(conn, sp, vis.w + 2 * spread, vis.h + 2 * spread, None, sd, opacity);
                    }
                }
                let r = l.radius.round() as i32;
                let mask = if r > 0 || inset > 0 { self.mask(conn, t.rect.w, t.rect.h, inset, r) } else { None };
                self.blit(conn, t.pic, t.rect.w, t.rect.h, mask, dst, opacity);
                if self.highlight.is_some_and(|h| info.frames.contains_key(&h)) && Some(t.win) == self.highlight {
                    // Alt+Tab marks the window it would pick
                    let ring = Rect::new(target.x - 3, target.y - 3, target.w + 6, target.h + 6);
                    if let Some(mut px) = draw::Px::new(ring.w as u32, ring.h as u32) {
                        draw::rrect_line(&mut px, 0.0, 0.0, ring.w as f32, ring.h as f32, l.radius + 3.0, Rgba(147, 171, 235, 255), 3.0);
                        if let Some((p, pm)) = self.upload_argb(conn, &px) {
                            conn.render_composite(PictOp::OVER, p, x11rb::NONE, self.back_pic, 0, 0, 0, 0, ring.x as i16, ring.y as i16, ring.w as u16, ring.h as u16).ok();
                            conn.render_free_picture(p).ok();
                            conn.free_pixmap(pm).ok();
                        }
                    }
                }
            } else {
                self.blit(conn, t.pic, t.rect.w, t.rect.h, None, dst, opacity);
            }
        }
        for w in done {
            if let Some(i) = self.tops.iter().position(|t| t.win == w && (t.ghost || t.fade_out.is_some())) {
                let ghost = self.tops[i].ghost;
                if ghost {
                    let t = self.tops.remove(i);
                    if t.pic != 0 {
                        conn.render_free_picture(t.pic).ok();
                        conn.free_pixmap(t.pixmap).ok();
                    }
                } else {
                    self.tops[i].fade_out = None;
                    if !self.tops[i].mapped && self.tops[i].pic != 0 {
                        conn.render_free_picture(self.tops[i].pic).ok();
                        conn.free_pixmap(self.tops[i].pixmap).ok();
                        self.tops[i].pic = 0;
                        self.tops[i].pixmap = 0;
                    }
                }
            }
        }
        self.paint_layers(conn, |n| n != "lock");
        self.present(conn);
    }

    fn paint_layers(&mut self, conn: &RustConnection, keep: impl Fn(&str) -> bool) {
        let list: Vec<(render::Picture, i32, i32, u32, u32, f32)> = self.layers.iter().filter(|l| keep(&l.0)).map(|l| (l.1.pic, l.1.x, l.1.y, l.1.w, l.1.h, l.1.alpha)).collect();
        for (pic, x, y, w, h, a) in list {
            if a <= 0.003 {
                continue;
            }
            let m = if a < 0.997 { self.alpha(conn, a) } else { 0 };
            conn.render_composite(PictOp::OVER, pic, m, self.back_pic, 0, 0, 0, 0, x as i16, y as i16, w as u16, h as u16).ok();
        }
    }

    fn present(&mut self, conn: &RustConnection) {
        let (w, h) = self.size;
        conn.render_composite(PictOp::SRC, self.back_pic, x11rb::NONE, self.out_pic, 0, 0, 0, 0, 0, 0, w as u16, h as u16).ok();
        conn.flush().ok();
        self.dirty = false;
    }

    /// Hand a fullscreen window the screen directly (no compositing: no tearing, no lag), or
    /// take it back.
    pub fn bypass(&mut self, conn: &RustConnection, on: bool) {
        if on {
            conn.composite_unredirect_subwindows(self.root, Redirect::MANUAL).ok();
            conn.unmap_window(self.overlay).ok();
        } else {
            conn.composite_redirect_subwindows(self.root, Redirect::MANUAL).ok();
            conn.map_window(self.overlay).ok();
            for i in 0..self.tops.len() {
                self.bind(conn, i);
            }
        }
        self.dirty = true;
    }
}

fn scale_about(r: Rect, s: f32) -> Rect {
    let w = (r.w as f32 * s).round() as i32;
    let h = (r.h as f32 * s).round() as i32;
    Rect::new(r.cx() - w / 2, r.cy() - h / 2, w, h)
}

fn layer_rank(name: &str) -> u8 {
    match name {
        "preview" => 0,
        "pill" | "mark" => 1,
        "notice" => 2,
        "overview" => 3,
        "lock" => 9,
        _ => 5,
    }
}
