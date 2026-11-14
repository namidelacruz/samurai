//! 一期一会 — the one-time seal.
//!
//! Once per machine, 32 bytes from the kernel's random source drive a drawing: 460 chords strung
//! between two orbits, every endpoint nudged by its own random amount, three red cuts, one red
//! disc, and four cut positions on the frame. The drawing is saved for both grounds (black and
//! white) together with its mark (a fingerprint of the drawing). The seed is then overwritten and
//! is never written anywhere: with it gone, neither this program nor its owner can draw the same
//! picture again. Forging a new seal erases the current one for good.
use crate::store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub const W: f64 = 1440.0;
pub const H: f64 = 900.0;
const CHORDS: usize = 460;

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
        let s = &mut self.0;
        let r = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        r
    }
    /// 0 ≤ x < 1
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn wipe(&mut self) {
        for w in self.0.iter_mut() {
            // volatile, so the compiler may not drop the write
            unsafe { std::ptr::write_volatile(w, 0) };
        }
    }
}

pub struct Drawing {
    pub field: String,
    pub cuts: String,
    pub disc: (f64, f64, f64),
    pub frame_cuts: [f64; 4],
}

/// The drawing for a seed. (The same routine, with a fixed public seed, draws the incognito decoy.)
pub fn draw(seed: &mut [u8; 32]) -> Drawing {
    let mut r = Rng::new(seed);
    let t = std::f64::consts::TAU;
    let (cx, cy) = (W * (0.35 + 0.4 * r.f()), H * (0.3 + 0.4 * r.f()));
    let (a, b, c, d) = (1.0 + (r.f() * 5.0).floor(), 1.0 + (r.f() * 5.0).floor(), 1.0 + (r.f() * 7.0).floor(), 1.0 + (r.f() * 7.0).floor());
    let (p1, p2) = (r.f() * t, r.f() * t);
    let (r1, r2) = (260.0 + r.f() * 320.0, 200.0 + r.f() * 380.0);
    let mut field = String::with_capacity(CHORDS * 28);
    for i in 0..CHORDS {
        let u = i as f64 / CHORDS as f64 * t;
        // each endpoint gets its own nudge: 1840 further random numbers no one can guess back
        let j = |r: &mut Rng| (r.f() - 0.5) * 9.0;
        let (x1, y1) = (cx + r1 * (a * u + p1).cos() + j(&mut r), cy + r1 * 0.62 * (b * u).sin() + j(&mut r));
        let (x2, y2) = (cx + r2 * 1.3 * (c * u + p2).cos() + j(&mut r), cy + r2 * (d * u + p1).sin() + j(&mut r));
        field.push_str(&format!("M{x1:.1} {y1:.1}L{x2:.1} {y2:.1}"));
    }
    let mut cuts = String::new();
    for _ in 0..3 {
        cuts.push_str(&format!("M0 {:.1}L{W} {:.1}", H * r.f(), H * r.f()));
    }
    let disc = ((W * (0.55 + 0.35 * r.f())).round(), (H * (0.2 + 0.5 * r.f())).round(), (70.0 + r.f() * 110.0).round());
    let frame_cuts = [r.f(), r.f(), r.f(), r.f()];
    r.wipe();
    for b in seed.iter_mut() {
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    Drawing { field, cuts, disc, frame_cuts }
}

pub fn svg(d: &Drawing, ground: &str, red: &str, with_disc: bool) -> String {
    let (bg, ink) = if ground == "white" { ("#FFFFFF", "#000000") } else { ("#000000", "#FFFFFF") };
    let disc = if with_disc { format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{red}\"/>", d.disc.0, d.disc.1, d.disc.2) } else { String::new() };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" preserveAspectRatio=\"xMidYMid slice\"><rect width=\"{W}\" height=\"{H}\" fill=\"{bg}\"/>{disc}<path d=\"{}\" fill=\"none\" stroke=\"{ink}\" stroke-width=\"0.55\" opacity=\"0.5\"/><path d=\"{}\" fill=\"none\" stroke=\"{red}\" stroke-width=\"1.5\"/></svg>\n",
        d.field, d.cuts
    )
}

fn mark_of(svg_black: &str) -> String {
    let h = Sha256::digest(svg_black.as_bytes());
    let hex: String = h[..8].iter().map(|b| format!("{b:02X}")).collect();
    hex.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("·")
}

pub fn exists() -> bool {
    dir().join("seal.json").exists()
}

pub fn show() -> Value {
    std::fs::read_to_string(dir().join("seal.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({ "forged": false }))
}

/// Forge the seal. Refuses when one exists unless `erase` (the hub asks first).
pub fn forge(erase: bool) -> Result<Value, String> {
    if exists() && !erase {
        return Err("this machine already has its seal. Forging a new one erases it for good: samurai seal forge --erase-current".into());
    }
    let mut seed = [0u8; 32];
    {
        use std::io::Read;
        std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut seed)).map_err(|e| format!("no random source: {e}"))?;
    }
    let d = draw(&mut seed);
    debug_assert!(seed.iter().all(|b| *b == 0), "the seed is gone once the drawing exists");
    let red = crate::hypr::accent();
    let black = svg(&d, "black", &red, true);
    let white = svg(&d, "white", &red, true);
    let mark = mark_of(&black);
    let made = std::process::Command::new("date").arg("+%Y-%m-%d").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let info = json!({ "forged": true, "mark": mark, "made": made, "frame_cuts": d.frame_cuts, "seed": "destroyed" });
    let dir = dir();
    store::atomic_write(&dir.join("wall-black.svg"), &black).map_err(|e| e.to_string())?;
    store::atomic_write(&dir.join("wall-white.svg"), &white).map_err(|e| e.to_string())?;
    store::atomic_write(&dir.join("seal.json"), &(info.to_string() + "\n")).map_err(|e| e.to_string())?;
    Ok(info)
}

/// The incognito decoy: the same on every machine (a fixed, public seed), so it identifies nobody.
pub fn decoy() -> Result<(), String> {
    let dir = dir();
    for ground in ["black", "white"] {
        let mut seed = *b"samurai-incognito-decoy-00000000";
        let d = draw(&mut seed);
        store::atomic_write(&dir.join(format!("decoy-{ground}.svg")), &svg(&d, ground, "#C8102E", false)).map_err(|e| e.to_string())?;
    }
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
        assert_ne!(da.field, db.field);
        assert_ne!(mark_of(&svg(&da, "black", "#C8102E", true)), mark_of(&svg(&db, "black", "#C8102E", true)));
        assert_eq!(da.field.matches('M').count(), CHORDS);
    }

    #[test]
    fn both_grounds_share_one_drawing() {
        let mut s = [3u8; 32];
        let d = draw(&mut s);
        let (b, w) = (svg(&d, "black", "#C8102E", true), svg(&d, "white", "#C8102E", true));
        assert!(b.contains("fill=\"#000000\"") && w.contains("fill=\"#FFFFFF\""));
        assert!(b.contains(&d.field) && w.contains(&d.field));
        let m = mark_of(&b);
        assert_eq!(m.chars().filter(|c| *c == '·').count(), 3, "{m}");
    }
}
