//! CPU drawing for everything the WM paints itself (title strips, the overview, the lock face,
//! the corner pill): tiny-skia shapes, fontdue text, SVG icons. Pixmaps are uploaded to X as
//! ARGB pictures by the compositor.

use crate::theme::Rgba;
use fontdue::{Font, FontSettings};
use resvg::tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};
use std::cell::RefCell;
use std::collections::HashMap;

pub use tiny_skia::Pixmap as Px;

fn paint(c: Rgba) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, c.3);
    p.anti_alias = true;
    p
}

pub fn rrect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = PathBuilder::new();
    if r <= 0.01 {
        pb.push_rect(Rect::from_xywh(x, y, w, h)?);
        return pb.finish();
    }
    let k = 0.5523 * r;
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

pub fn rrect(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Rgba) {
    if let Some(p) = rrect_path(x, y, w, h, r) {
        px.fill_path(&p, &paint(fill), FillRule::Winding, Transform::identity(), None);
    }
}

pub fn rrect_line(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, color: Rgba, width: f32) {
    let i = width / 2.0;
    if let Some(p) = rrect_path(x + i, y + i, w - width, h - width, (r - i).max(0.0)) {
        let s = Stroke { width, ..Stroke::default() };
        px.stroke_path(&p, &paint(color), &s, Transform::identity(), None);
    }
}

pub fn circle(px: &mut Pixmap, cx: f32, cy: f32, r: f32, fill: Rgba) {
    if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
        px.fill_path(&p, &paint(fill), FillRule::Winding, Transform::identity(), None);
    }
}

pub fn line(px: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(p) = pb.finish() {
        let s = Stroke { width, line_cap: tiny_skia::LineCap::Round, ..Stroke::default() };
        px.stroke_path(&p, &paint(color), &s, Transform::identity(), None);
    }
}

// ── text ───────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Face {
    Ui,
    UiBold,
    Mono,
    Jp,
}

pub struct Fonts {
    faces: Vec<(Face, Font)>,
    cache: RefCell<HashMap<(Face, char, u32), (fontdue::Metrics, Vec<u8>)>>,
}

fn fc_match(pattern: &str) -> Option<Vec<u8>> {
    let out = std::process::Command::new("fc-match").args(["-f", "%{file}", pattern]).output().ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    if path.is_empty() {
        return None;
    }
    std::fs::read(path.trim()).ok()
}

fn load(pattern: &str) -> Option<Font> {
    let data = fc_match(pattern)?;
    Font::from_bytes(data, FontSettings { scale: 40.0, ..FontSettings::default() }).ok()
}

impl Fonts {
    pub fn load(ui: &str, mono: &str) -> Self {
        let mut faces = vec![];
        for (face, pat) in [
            (Face::Ui, format!("{ui}:weight=80")),
            (Face::UiBold, format!("{ui}:weight=200")),
            (Face::Mono, format!("{mono}")),
            (Face::Jp, "Noto Sans CJK JP:weight=200".to_string()),
        ] {
            if let Some(f) = load(&pat).or_else(|| load("sans-serif")) {
                faces.push((face, f));
            }
        }
        Fonts { faces, cache: RefCell::new(HashMap::new()) }
    }

    fn font(&self, face: Face) -> Option<&Font> {
        self.faces.iter().find(|f| f.0 == face).or(self.faces.first()).map(|f| &f.1)
    }

    /// The face that has this character: the asked one, else the Japanese face.
    fn face_for(&self, face: Face, c: char) -> Face {
        match self.font(face) {
            Some(f) if f.lookup_glyph_index(c) != 0 || c == ' ' => face,
            _ if self.faces.iter().any(|f| f.0 == Face::Jp && f.1.lookup_glyph_index(c) != 0) => Face::Jp,
            _ => face,
        }
    }

    fn glyph(&self, face: Face, c: char, size: f32) -> Option<(fontdue::Metrics, Vec<u8>)> {
        let key = (face, c, (size * 10.0) as u32);
        if let Some(g) = self.cache.borrow().get(&key) {
            return Some(g.clone());
        }
        let g = self.font(face)?.rasterize(c, size);
        self.cache.borrow_mut().insert(key, g.clone());
        Some(g)
    }

    pub fn measure(&self, face: Face, size: f32, text: &str) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<(Face, char)> = None;
        for c in text.chars() {
            let f = self.face_for(face, c);
            if let (Some((pf, pc)), Some(font)) = (prev, self.font(f)) {
                if pf == f {
                    w += font.horizontal_kern(pc, c, size).unwrap_or(0.0);
                }
            }
            w += self.font(f).map(|font| font.metrics(c, size).advance_width).unwrap_or(size * 0.5);
            prev = Some((f, c));
        }
        w
    }

    /// Cut `text` to fit `max` pixels, ending in an ellipsis.
    pub fn fit(&self, face: Face, size: f32, text: &str, max: f32) -> String {
        if self.measure(face, size, text) <= max {
            return text.to_string();
        }
        let mut s: String = text.to_string();
        while !s.is_empty() && self.measure(face, size, &(s.clone() + "…")) > max {
            s.pop();
        }
        s.trim_end().to_string() + "…"
    }

    pub fn ascent(&self, face: Face, size: f32) -> f32 {
        self.font(face).and_then(|f| f.horizontal_line_metrics(size)).map(|m| m.ascent).unwrap_or(size * 0.8)
    }

    /// Draw text with its top at `y` (the line box starts at y; the baseline is y + ascent).
    /// Returns the width drawn.
    pub fn draw(&self, px: &mut Pixmap, face: Face, size: f32, x: f32, y: f32, color: Rgba, text: &str) -> f32 {
        let base = y + self.ascent(face, size);
        let mut pen = x;
        let mut prev: Option<(Face, char)> = None;
        let (pw, ph) = (px.width() as i32, px.height() as i32);
        let data = px.data_mut();
        for c in text.chars() {
            let f = self.face_for(face, c);
            if let (Some((pf, pc)), Some(font)) = (prev, self.font(f)) {
                if pf == f {
                    pen += font.horizontal_kern(pc, c, size).unwrap_or(0.0);
                }
            }
            prev = Some((f, c));
            let Some((m, cov)) = self.glyph(f, c, size) else { continue };
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (base - m.height as f32 - m.ymin as f32).round() as i32;
            for row in 0..m.height as i32 {
                let yy = gy + row;
                if yy < 0 || yy >= ph {
                    continue;
                }
                for col in 0..m.width as i32 {
                    let xx = gx + col;
                    if xx < 0 || xx >= pw {
                        continue;
                    }
                    let a = cov[(row * m.width as i32 + col) as usize] as u32 * color.3 as u32 / 255;
                    if a == 0 {
                        continue;
                    }
                    let i = ((yy * pw + xx) * 4) as usize;
                    // premultiplied source over premultiplied destination
                    let inv = 255 - a;
                    data[i] = ((color.0 as u32 * a + data[i] as u32 * inv) / 255) as u8;
                    data[i + 1] = ((color.1 as u32 * a + data[i + 1] as u32 * inv) / 255) as u8;
                    data[i + 2] = ((color.2 as u32 * a + data[i + 2] as u32 * inv) / 255) as u8;
                    data[i + 3] = (a + data[i + 3] as u32 * inv / 255) as u8;
                }
            }
            pen += m.advance_width;
        }
        pen - x
    }

    pub fn draw_centered(&self, px: &mut Pixmap, face: Face, size: f32, cx: f32, y: f32, color: Rgba, text: &str) {
        let w = self.measure(face, size, text);
        self.draw(px, face, size, cx - w / 2.0, y, color, text);
    }

    pub fn draw_right(&self, px: &mut Pixmap, face: Face, size: f32, right: f32, y: f32, color: Rgba, text: &str) {
        let w = self.measure(face, size, text);
        self.draw(px, face, size, right - w, y, color, text);
    }
}

// ── icons ──────────────────────────────────────────────────────────────────────────────────

pub fn icon_svg(name: &str) -> &'static str {
    match name {
        "chev" => r#"<path d="M9.5 6.5l5.5 5.5-5.5 5.5"/>"#,
        "chev-down" => r#"<path d="M6.5 9.5l5.5 5.5 5.5-5.5"/>"#,
        "search" => r#"<circle cx="11" cy="11" r="7"/><path d="M16.5 16.5L21 21"/>"#,
        "wifi" => r#"<path d="M2.5 8.8a14 14 0 0 1 19 0"/><path d="M5.6 12.2a9.5 9.5 0 0 1 12.8 0"/><path d="M8.8 15.5a5 5 0 0 1 6.4 0"/><circle cx="12" cy="19" r="0.9"/>"#,
        "wifi-off" => r#"<path d="M2.5 8.8a14 14 0 0 1 19 0" opacity="0.35"/><path d="M5.6 12.2a9.5 9.5 0 0 1 12.8 0" opacity="0.35"/><path d="M4 4l16 16"/>"#,
        "bt" => r#"<path d="M7 7.5l10 9-5 4.5V3l5 4.5-10 9"/>"#,
        "moon" => r#"<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>"#,
        "sun" => r#"<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"/>"#,
        "sound" => r#"<path d="M4 9.5h3.5L12 6v12l-4.5-3.5H4z"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/><path d="M18 7a7 7 0 0 1 0 10"/>"#,
        "mute" => r#"<path d="M4 9.5h3.5L12 6v12l-4.5-3.5H4z"/><path d="M16 9.5l5 5M21 9.5l-5 5"/>"#,
        "bright" => r#"<circle cx="12" cy="12" r="3.2"/><path d="M12 4v1.5M12 18.5V20M4 12h1.5M18.5 12H20M6.3 6.3l1 1M16.7 16.7l1 1M6.3 17.7l1-1M16.7 7.3l1-1"/>"#,
        "quiet" => r#"<circle cx="12" cy="12" r="8.5"/><path d="M8 12h8"/>"#,
        "battery" => r#"<rect x="2.5" y="7.5" width="17" height="9" rx="2"/><path d="M21.5 10.5v3"/>"#,
        "power" => r#"<path d="M12 3.5v8"/><path d="M6.6 7a7.5 7.5 0 1 0 10.8 0"/>"#,
        "restart" => r#"<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>"#,
        "sleep" => r#"<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>"#,
        "lock" => r#"<rect x="5" y="11" width="14" height="9.5" rx="2"/><path d="M8.5 11V8a3.5 3.5 0 0 1 7 0v3"/>"#,
        "logout" => r#"<path d="M14 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4"/><path d="M10 8l-4 4 4 4"/><path d="M6 12h10"/>"#,
        "terminal" => r#"<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M7 9.5l3 2.5-3 2.5"/><path d="M12.5 15h4.5"/>"#,
        "folder" => r#"<path d="M3 7.5a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#,
        "globe" => r#"<circle cx="12" cy="12" r="9"/><ellipse cx="12" cy="12" rx="4" ry="9"/><path d="M3 12h18"/>"#,
        "settings" => r#"<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>"#,
        "edit" => r#"<path d="M4 20h4L19.5 8.5l-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>"#,
        "image" => r#"<rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-9 8"/>"#,
        "music" => r#"<path d="M9 17.5V5.5l11-2v12"/><circle cx="6.5" cy="17.5" r="2.5"/><circle cx="17.5" cy="15.5" r="2.5"/>"#,
        "mail" => r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3.5 7l8.5 6 8.5-6"/>"#,
        "doc" => r#"<path d="M6.5 3h8l4 4v14h-12z"/><path d="M14.5 3v4h4"/><path d="M9 12.5h6M9 16h6"/>"#,
        "video" => r#"<rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10.5l5-3v9l-5-3"/>"#,
        "close" => r#"<path d="M7 7l10 10M17 7L7 17"/>"#,
        "max" => r#"<rect x="6" y="6" width="12" height="12" rx="2"/>"#,
        "unmax" => r#"<rect x="5" y="9" width="10" height="10" rx="2"/><path d="M9 9V7a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2h-2"/>"#,
        "tuck" => r#"<path d="M6 12h12"/>"#,
        "arrow" => r#"<path d="M5 12h14"/><path d="M13 6l6 6-6 6"/>"#,
        "check" => r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#,
        "apps" => r#"<rect x="4" y="4" width="7" height="7" rx="1.5"/><rect x="13" y="4" width="7" height="7" rx="1.5"/><rect x="4" y="13" width="7" height="7" rx="1.5"/><rect x="13" y="13" width="7" height="7" rx="1.5"/>"#,
        "calc" => r#"<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M8.5 7.5h7M8.5 12h.01M12 12h.01M15.5 12h.01M8.5 16h.01M12 16h.01M15.5 16h.01"/>"#,
        "keyboard" => r#"<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6 10h.01M9.5 10h.01M13 10h.01M16.5 10h.01M7 14h10"/>"#,
        "camera" => r#"<path d="M4 8h3l2-2.5h6L17 8h3v11H4z"/><circle cx="12" cy="13" r="3.5"/>"#,
        "ink" => r#"<path d="M12 3c4.2 5 6.3 8.2 6.3 11.2a6.3 6.3 0 0 1-12.6 0C5.7 11.2 7.8 8 12 3z"/>"#,
        "plug" => r#"<path d="M9 2.5v5M15 2.5v5"/><path d="M6.5 7.5h11v3.5a5.5 5.5 0 0 1-11 0z"/><path d="M12 16.5v5"/>"#,
        _ => r#"<rect x="4" y="4" width="16" height="16" rx="4"/>"#,
    }
}

thread_local! {
    static ICONS: RefCell<HashMap<(String, u32, (u8, u8, u8, u8), u32), Pixmap>> = RefCell::new(HashMap::new());
}

/// Draw a stroke icon of `size` px with its top-left at (x, y).
pub fn icon(px: &mut Pixmap, name: &str, x: f32, y: f32, size: f32, color: Rgba, sw: f32) {
    let key = (name.to_string(), size as u32, (color.0, color.1, color.2, color.3), (sw * 10.0) as u32);
    let pm = ICONS.with(|c| {
        if let Some(p) = c.borrow().get(&key) {
            return Some(p.clone());
        }
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="0 0 24 24" fill="none" stroke="#{:02x}{:02x}{:02x}" stroke-opacity="{:.3}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
            color.0, color.1, color.2, color.3 as f32 / 255.0, icon_svg(name), s = size as u32
        );
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
        let mut p = Pixmap::new(size.ceil() as u32, size.ceil() as u32)?;
        resvg::render(&tree, Transform::identity(), &mut p.as_mut());
        c.borrow_mut().insert(key.clone(), p.clone());
        Some(p)
    });
    if let Some(p) = pm {
        px.draw_pixmap(x.round() as i32, y.round() as i32, p.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
    }
}

/// Render an SVG file to cover `w`×`h` (centre-cropped).
pub fn svg_cover(path: &std::path::Path, w: u32, h: u32) -> Option<Pixmap> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let sz = tree.size();
    let s = (w as f32 / sz.width()).max(h as f32 / sz.height());
    let tx = (w as f32 - sz.width() * s) / 2.0;
    let ty = (h as f32 - sz.height() * s) / 2.0;
    let mut p = Pixmap::new(w, h)?;
    resvg::render(&tree, Transform::from_row(s, 0.0, 0.0, s, tx, ty), &mut p.as_mut());
    Some(p)
}

/// A soft shadow: a rounded rect blurred by three box passes, alpha only, as a pixmap of
/// (w + 2·spread) × (h + 2·spread) with the rect at (spread, spread + dy).
pub fn shadow(w: u32, h: u32, r: f32, spread: u32, dy: f32, color: Rgba) -> Option<Pixmap> {
    let (pw, ph) = (w + 2 * spread, h + 2 * spread);
    let mut alpha = vec![0f32; (pw * ph) as usize];
    let mut base = Pixmap::new(pw, ph)?;
    rrect(&mut base, spread as f32, spread as f32 + dy, w as f32, h as f32, r, Rgba(0, 0, 0, 255));
    for (i, px) in base.pixels().iter().enumerate() {
        alpha[i] = px.alpha() as f32;
    }
    let rad = (spread as i32 / 3).max(1);
    for _ in 0..3 {
        box_blur(&mut alpha, pw as i32, ph as i32, rad);
    }
    let mut out = Pixmap::new(pw, ph)?;
    let data = out.data_mut();
    for (i, a) in alpha.iter().enumerate() {
        let a = (a * color.3 as f32 / 255.0).clamp(0.0, 255.0) as u32;
        data[i * 4] = (color.0 as u32 * a / 255) as u8;
        data[i * 4 + 1] = (color.1 as u32 * a / 255) as u8;
        data[i * 4 + 2] = (color.2 as u32 * a / 255) as u8;
        data[i * 4 + 3] = a as u8;
    }
    Some(out)
}

fn box_blur(a: &mut [f32], w: i32, h: i32, r: i32) {
    let mut tmp = vec![0f32; a.len()];
    let n = (2 * r + 1) as f32;
    for y in 0..h {
        let mut acc = 0.0;
        for x in -r..=r {
            acc += a[(y * w + x.clamp(0, w - 1)) as usize];
        }
        for x in 0..w {
            tmp[(y * w + x) as usize] = acc / n;
            acc += a[(y * w + (x + r + 1).clamp(0, w - 1)) as usize] - a[(y * w + (x - r).clamp(0, w - 1)) as usize];
        }
    }
    for x in 0..w {
        let mut acc = 0.0;
        for y in -r..=r {
            acc += tmp[(y.clamp(0, h - 1) * w + x) as usize];
        }
        for y in 0..h {
            a[(y * w + x) as usize] = acc / n;
            acc += tmp[((y + r + 1).clamp(0, h - 1) * w + x) as usize] - tmp[((y - r).clamp(0, h - 1) * w + x) as usize];
        }
    }
}

/// Premultiplied RGBA (tiny-skia) → premultiplied BGRA, the byte order of an X ARGB32 picture.
pub fn to_bgra(px: &Pixmap) -> Vec<u8> {
    let mut v = px.data().to_vec();
    for p in v.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    v
}

/// An alpha-only rounded-rect mask (A8), for cutting window corners.
pub fn mask_a8(w: u32, h: u32, inset: f32, r: f32) -> Vec<u8> {
    let mut p = match Pixmap::new(w.max(1), h.max(1)) {
        Some(p) => p,
        None => return vec![255; (w * h) as usize],
    };
    rrect(&mut p, inset, inset, w as f32 - 2.0 * inset, h as f32 - 2.0 * inset, r, Rgba(0, 0, 0, 255));
    // A8 rows are padded to 4 bytes by the X server's image format
    let stride = ((w + 3) & !3) as usize;
    let mut out = vec![0u8; stride * h as usize];
    for y in 0..h as usize {
        for x in 0..w as usize {
            out[y * stride + x] = p.pixels()[y * w as usize + x].alpha();
        }
    }
    out
}

pub fn tiny_skia_color(c: Rgba) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.0, c.1, c.2, c.3)
}

// ── app icons from the icon theme ──────────────────────────────────────────────────────────

thread_local! {
    static APP_ICONS: std::cell::RefCell<std::collections::HashMap<(String, u32), Option<Pixmap>>> = Default::default();
}

/// Where an app's icon (the Icon= of its .desktop file) lives: an absolute path, or a name looked
/// up in the usual icon themes (Papirus first: Samurai sets it), then hicolor and pixmaps.
pub fn find_app_icon(name: &str) -> Option<std::path::PathBuf> {
    use std::path::{Path, PathBuf};
    if name.is_empty() {
        return None;
    }
    if name.starts_with('/') {
        return Path::new(name).is_file().then(|| PathBuf::from(name));
    }
    let home = crate::config::home();
    let bases = [home.join(".local/share/icons"), home.join(".icons"), PathBuf::from("/usr/local/share/icons"), PathBuf::from("/usr/share/icons")];
    let themes = ["Papirus-Dark", "Papirus", "Adwaita", "breeze", "hicolor"];
    let subs = ["scalable/apps", "48x48/apps", "64x64/apps", "96x96/apps", "128x128/apps", "256x256/apps", "apps/48", "apps/scalable", "512x512/apps", "32x32/apps"];
    for t in themes {
        for b in &bases {
            let dir = b.join(t);
            if !dir.is_dir() {
                continue;
            }
            for sub in subs {
                for ext in ["svg", "png"] {
                    let p = dir.join(sub).join(format!("{name}.{ext}"));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    for ext in ["svg", "png", "xpm"] {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
        if p.is_file() && ext != "xpm" {
            return Some(p);
        }
    }
    None
}

fn load_app_icon(name: &str, size: u32) -> Option<Pixmap> {
    let path = find_app_icon(name)?;
    let data = std::fs::read(&path).ok()?;
    let mut out = Pixmap::new(size, size)?;
    if path.extension().is_some_and(|e| e == "svg") {
        let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
        let sz = tree.size();
        let k = size as f32 / sz.width().max(sz.height()).max(1.0);
        let dx = (size as f32 - sz.width() * k) / 2.0;
        let dy = (size as f32 - sz.height() * k) / 2.0;
        resvg::render(&tree, Transform::from_row(k, 0.0, 0.0, k, dx, dy), &mut out.as_mut());
    } else {
        let img = Pixmap::decode_png(&data).ok()?;
        let k = size as f32 / img.width().max(img.height()).max(1) as f32;
        let paint = tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
        out.draw_pixmap(0, 0, img.as_ref(), &paint, Transform::from_scale(k, k), None);
    }
    Some(out)
}

/// Draw an app's own icon at (x, y), `size` px. Returns false when it has none (the caller then
/// draws a stroke icon instead). Results, found or not, are kept for the session.
pub fn app_icon(px: &mut Pixmap, name: &str, x: f32, y: f32, size: f32) -> bool {
    let key = (name.to_string(), size.round() as u32);
    let pm = APP_ICONS.with(|c| {
        if let Some(p) = c.borrow().get(&key) {
            return p.clone();
        }
        let p = load_app_icon(name, key.1);
        c.borrow_mut().insert(key.clone(), p.clone());
        p
    });
    match pm {
        Some(p) => {
            px.draw_pixmap(x.round() as i32, y.round() as i32, p.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
            true
        }
        None => false,
    }
}
