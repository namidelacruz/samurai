//! 一期一会 — this machine's ink, made once.
//!
//! Suminagashi (ink floated on water) done in maths: drops of ink and clear water fall on the same
//! few spots, each new drop pushing the earlier rings outward; then the surface is combed by a
//! slow wave, three tines and a swirl. Every choice comes from 32 bytes of the kernel's random
//! source. The drawing is saved for both grounds (night and paper) with its mark (a fingerprint
//! of the drawing); the 32 bytes are then overwritten and never written anywhere, so nobody,
//! including Samurai, can draw the same ink again. Making a new one throws the current one away.

use crate::store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::f64::consts::PI;
use std::path::PathBuf;

pub const W: f64 = 1440.0;
pub const H: f64 = 900.0;

pub fn dir() -> PathBuf {
    store::config_dir().join("seal")
}

/// xoshiro256**
struct Rng([u64; 4]);
impl Rng {
    fn new(seed: &[u8; 32]) -> Rng {
        let mut s = [0u64; 4];
        for (i, c) in seed.chunks(8).enumerate() {
            s[i] = u64::from_le_bytes(c.try_into().unwrap());
        }
        if s == [0; 4] {
            s[0] = 1;
        }
        Rng(s)
    }
    fn next(&mut self) -> u64 {
        let r = self.0[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.0[1] << 17;
        self.0[2] ^= self.0[0];
        self.0[3] ^= self.0[1];
        self.0[1] ^= self.0[2];
        self.0[0] ^= self.0[3];
        self.0[2] ^= t;
        self.0[3] = self.0[3].rotate_left(45);
        r
    }
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// What colour a ring is: the ground (clear water), one of five inks, or the accent.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Water,
    Ink(usize),
    Accent,
}

pub struct Poly {
    pub pts: Vec<(f64, f64)>,
    pub role: Role,
}

pub struct Drawing {
    pub polys: Vec<Poly>,
}

fn circle(cx: f64, cy: f64, r: f64, n: usize) -> Vec<(f64, f64)> {
    (0..n).map(|i| { let a = 2.0 * PI * i as f64 / n as f64; (cx + r * a.cos(), cy + r * a.sin()) }).collect()
}

fn drop_ink(polys: &mut [Poly], cx: f64, cy: f64, r: f64) {
    let r2 = r * r;
    for p in polys.iter_mut() {
        for pt in p.pts.iter_mut() {
            let (dx, dy) = (pt.0 - cx, pt.1 - cy);
            let d2 = (dx * dx + dy * dy).max(1e-6);
            let k = (1.0 + r2 / d2).sqrt();
            *pt = (cx + dx * k, cy + dy * k);
        }
    }
}

fn tine(polys: &mut [Poly], bx: f64, by: f64, angle: f64, z: f64, lam: f64) {
    let (mx, my) = (angle.cos(), angle.sin());
    let (nx, ny) = (-my, mx);
    for p in polys.iter_mut() {
        for pt in p.pts.iter_mut() {
            let d = ((pt.0 - bx) * nx + (pt.1 - by) * ny).abs();
            let s = z * lam / (d + lam);
            *pt = (pt.0 + mx * s, pt.1 + my * s);
        }
    }
}

fn wave(polys: &mut [Poly], angle: f64, amp: f64, wl: f64, phase: f64) {
    let (mx, my) = (angle.cos(), angle.sin());
    let (nx, ny) = (-my, mx);
    for p in polys.iter_mut() {
        for pt in p.pts.iter_mut() {
            let t = (pt.0 * nx + pt.1 * ny) / wl * 2.0 * PI + phase;
            let s = amp * t.sin();
            *pt = (pt.0 + mx * s, pt.1 + my * s);
        }
    }
}

fn swirl(polys: &mut [Poly], cx: f64, cy: f64, a: f64, lam: f64) {
    for p in polys.iter_mut() {
        for pt in p.pts.iter_mut() {
            let (dx, dy) = (pt.0 - cx, pt.1 - cy);
            let th = a * (-(dx * dx + dy * dy).sqrt() / lam).exp();
            let (c, s) = (th.cos(), th.sin());
            *pt = (cx + dx * c - dy * s, cy + dx * s + dy * c);
        }
    }
}

/// Keep edges short as the ink stretches, and the point count bounded.
fn refine(polys: &mut [Poly], maxlen: f64, cap: usize) {
    for p in polys.iter_mut() {
        let n = p.pts.len();
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let (a, b) = (p.pts[i], p.pts[(i + 1) % n]);
            out.push(a);
            let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let k = (l / maxlen) as usize;
            for j in 1..=k {
                let t = j as f64 / (k + 1) as f64;
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        if out.len() > cap {
            let step = out.len() as f64 / cap as f64;
            out = (0..cap).map(|i| out[(i as f64 * step) as usize]).collect();
        }
        p.pts = out;
    }
}

/// Draw the ink. The seed is overwritten before this returns.
pub fn draw(seed: &mut [u8; 32]) -> Drawing {
    let mut rng = Rng::new(seed);
    seed.iter_mut().for_each(|b| *b = 0);
    let mut polys: Vec<Poly> = vec![];
    let centres: Vec<(f64, f64)> = (0..4).map(|_| (W * (0.18 + 0.64 * rng.f()), H * (0.2 + 0.6 * rng.f()))).collect();
    let mut k = 0usize;
    for round in 0..40 {
        for &(cx, cy) in &centres {
            if round > 30 && rng.f() < 0.5 {
                continue;
            }
            let (jx, jy) = ((rng.f() - 0.5) * 6.0, (rng.f() - 0.5) * 6.0);
            let r = 34.0 + rng.f() * 14.0;
            drop_ink(&mut polys, cx + jx, cy + jy, r);
            k += 1;
            // every other drop is clear water: that is what makes the thin rings
            let role = if k % 2 == 0 { Role::Water } else if (k / 2) % 11 == 10 { Role::Accent } else { Role::Ink((k / 2) % 5) };
            polys.push(Poly { pts: circle(cx + jx, cy + jy, r, 64), role });
        }
        if round % 4 == 3 {
            refine(&mut polys, 10.0, 1800);
        }
    }
    refine(&mut polys, 7.0, 2600);
    let (amp, ph) = (46.0 + 24.0 * rng.f(), rng.f() * 6.3);
    wave(&mut polys, 0.0, amp, 700.0, ph);
    refine(&mut polys, 7.0, 2600);
    for t in 0..3 {
        let x = W * (0.2 + 0.3 * t as f64) + (rng.f() - 0.5) * 80.0;
        let a = (PI / 2.0) * if t % 2 == 0 { 1.0 } else { -1.0 } + 0.12;
        tine(&mut polys, x, 0.0, a, 150.0, 60.0);
        refine(&mut polys, 7.0, 2600);
    }
    let a = 1.6 + rng.f();
    swirl(&mut polys, centres[0].0, centres[0].1, a, 260.0);
    refine(&mut polys, 7.0, 2600);
    let ph = rng.f() * 6.3;
    wave(&mut polys, PI / 2.0, 18.0, 220.0, ph);
    refine(&mut polys, 6.0, 2200);
    // forget the generator's state too
    unsafe { std::ptr::write_volatile(&mut rng.0, [0; 4]) };
    polys.retain(|p| {
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(x, y) in &p.pts {
            x0 = x0.min(x); x1 = x1.max(x); y0 = y0.min(y); y1 = y1.max(y);
        }
        !(x1 < -50.0 || x0 > W + 50.0 || y1 < -50.0 || y0 > H + 50.0)
    });
    Drawing { polys }
}

pub const NIGHT: (&str, [&str; 5]) = ("#101114", ["#1D2129", "#282E3A", "#0A0B0D", "#323A4B", "#191C22"]);
pub const PAPER: (&str, [&str; 5]) = ("#ECE8DF", ["#DCD5C6", "#C9BFAB", "#FBF9F4", "#B9AE98", "#E4DED1"]);

/// The accent as ink: a little deeper on night so it sits in the charcoal.
pub fn ink_accent(accent: &str, night: bool) -> &'static str {
    match (accent, night) {
        ("moss", true) => "#3E5A31",
        ("moss", false) => "#4A6B3A",
        ("rust", true) => "#7E3D23",
        ("rust", false) => "#9A4A2A",
        ("plum", true) => "#5A3561",
        ("plum", false) => "#6B3F73",
        ("slate", true) => "#3A4552",
        ("slate", false) => "#44505E",
        (_, true) => "#34497F",
        (_, false) => "#2F4B8F",
    }
}

pub fn svg(d: &Drawing, night: bool, accent: &str) -> String {
    let (ground, inks) = if night { NIGHT } else { PAPER };
    let acc = ink_accent(accent, night);
    let mut s = String::with_capacity(700_000);
    s.push_str(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" preserveAspectRatio=\"xMidYMid slice\">\n<rect width=\"{W}\" height=\"{H}\" fill=\"{ground}\"/>\n"));
    for p in &d.polys {
        let fill = match p.role { Role::Water => ground, Role::Ink(i) => inks[i % 5], Role::Accent => acc };
        s.push_str("<path d=\"M");
        for (i, (x, y)) in p.pts.iter().enumerate() {
            if i > 0 {
                s.push('L');
            }
            s.push_str(&format!("{x:.1} {y:.1}"));
        }
        s.push_str(&format!("Z\" fill=\"{fill}\" data-role=\"{}\"/>\n", match p.role { Role::Accent => "accent", _ => "ink" }));
    }
    s.push_str("</svg>\n");
    s
}

pub fn mark_of(svg_night: &str) -> String {
    let h = Sha256::digest(svg_night.as_bytes());
    let hex: String = h[..6].iter().map(|b| format!("{b:02X}")).collect();
    hex.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(" · ")
}

pub fn exists() -> bool {
    dir().join("seal.json").exists()
}

pub fn show() -> Value {
    std::fs::read_to_string(dir().join("seal.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({ "forged": false }))
}

pub fn mark() -> String {
    show()["mark"].as_str().unwrap_or("").to_string()
}

/// Make the ink. Refuses when one exists unless `erase` (Settings asks first).
pub fn forge(erase: bool) -> Result<Value, String> {
    if exists() && !erase {
        return Err("this machine already has its ink. Making a new one throws it away for good: samurai seal forge --erase-current".into());
    }
    let mut seed = [0u8; 32];
    {
        use std::io::Read;
        std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut seed)).map_err(|e| format!("no random source: {e}"))?;
    }
    let d = draw(&mut seed);
    debug_assert!(seed.iter().all(|b| *b == 0), "the seed is gone once the drawing exists");
    let accent = store::get_str("look.accent");
    let night = svg(&d, true, &accent);
    let paper = svg(&d, false, &accent);
    let mark = mark_of(&night);
    let made = std::process::Command::new("date").arg("+%Y-%m-%d").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let info = json!({ "forged": true, "mark": mark, "made": made, "accent": accent, "seed": "destroyed" });
    let dir = dir();
    store::atomic_write(&dir.join("wall-night.svg"), &night).map_err(|e| e.to_string())?;
    store::atomic_write(&dir.join("wall-paper.svg"), &paper).map_err(|e| e.to_string())?;
    store::atomic_write(&dir.join("seal.json"), &(info.to_string() + "\n")).map_err(|e| e.to_string())?;
    Ok(info)
}

/// The accent changed: recolour the accent rings of the existing ink (the drawing stays).
pub fn recolor(accent: &str) -> Result<bool, String> {
    let info = show();
    if info["accent"].as_str() == Some(accent) || !exists() {
        return Ok(false);
    }
    for night in [true, false] {
        let p = dir().join(if night { "wall-night.svg" } else { "wall-paper.svg" });
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let new = ink_accent(accent, night);
        let out: String = text
            .lines()
            .map(|l| {
                if l.contains("data-role=\"accent\"") {
                    if let Some(i) = l.find("fill=\"#") {
                        return format!("{}fill=\"{new}\"{}", &l[..i], &l[i + 14..]);
                    }
                }
                l.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        store::atomic_write(&p, &(out + "\n")).map_err(|e| e.to_string())?;
    }
    let mut i = info.as_object().cloned().unwrap_or_default();
    i.insert("accent".into(), json!(accent));
    store::atomic_write(&dir().join("seal.json"), &(Value::Object(i).to_string() + "\n")).map_err(|e| e.to_string())?;
    Ok(true)
}

/// The login screen's ink: the same on every machine (a fixed, public seed), so it shows nothing
/// of yours before anyone has signed in.
pub fn decoy() -> Result<(), String> {
    let dir = dir();
    let mut seed = *b"samurai-greeter-decoy-0000000000";
    let d = draw(&mut seed);
    store::atomic_write(&dir.join("decoy-night.svg"), &svg(&d, true, "indigo")).map_err(|e| e.to_string())?;
    store::atomic_write(&dir.join("decoy-paper.svg"), &svg(&d, false, "indigo")).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn the_seed_is_wiped_and_two_seeds_never_match() {
        let mut a = [7u8; 32];
        let da = draw(&mut a);
        assert!(a.iter().all(|b| *b == 0), "seed overwritten after drawing");
        let mut b = [7u8; 32];
        b[31] = 8; // one bit of difference
        let db = draw(&mut b);
        let (sa, sb) = (svg(&da, true, "indigo"), svg(&db, true, "indigo"));
        assert_ne!(sa, sb);
        assert_ne!(mark_of(&sa), mark_of(&sb));
        assert!(da.polys.len() > 20, "the ink has rings: {}", da.polys.len());
    }

    #[test]
    fn both_grounds_share_one_drawing() {
        let mut s = [3u8; 32];
        let d = draw(&mut s);
        let (n, p) = (svg(&d, true, "indigo"), svg(&d, false, "indigo"));
        assert!(n.contains("fill=\"#101114\"") && p.contains("fill=\"#ECE8DF\""));
        let geo = |t: &str| t.lines().filter_map(|l| l.split("\" fill").next().map(|s| s.to_string())).collect::<Vec<_>>();
        assert_eq!(geo(&n)[2..], geo(&p)[2..], "same rings on both grounds");
        let m = mark_of(&n);
        assert_eq!(m.matches(" · ").count(), 2, "{m}");
    }
}
