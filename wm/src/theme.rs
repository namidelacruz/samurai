//! Colours and measures. Two grounds (night, paper) and one accent; nothing else is a colour.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const fn hex(v: u32) -> Self {
        Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
    }
    pub const fn with_alpha(self, a: u8) -> Self {
        Rgba(self.0, self.1, self.2, a)
    }
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        let v = u32::from_str_radix(s, 16).ok()?;
        match s.len() {
            6 => Some(Rgba::hex(v)),
            8 => Some(Rgba((v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8)),
            _ => None,
        }
    }
    /// Relative luminance, for picking readable text on a sampled colour.
    pub fn luma(self) -> f32 {
        let f = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(self.0) + 0.7152 * f(self.1) + 0.0722 * f(self.2)
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub night: bool,
    pub ground: Rgba,
    pub surf: Rgba,
    pub chrome: Rgba,
    pub raised: Rgba,
    pub line: Rgba,
    pub text: Rgba,
    pub dim: Rgba,
    pub accent: Rgba,
    pub on_accent: Rgba,
    pub glass: Rgba,
    pub scrim: Rgba,
    pub soft: Rgba,
    pub shadow: Rgba,
}

/// (name, on paper, on night). Night uses a lighter tint so it reads on charcoal.
pub const ACCENTS: &[(&str, u32, u32)] = &[
    ("indigo", 0x2F4B8F, 0x93ABEB),
    ("moss", 0x4A6B3A, 0x9CC28A),
    ("rust", 0x9A4A2A, 0xE39A78),
    ("plum", 0x6B3F73, 0xC9A0D6),
    ("slate", 0x44505E, 0xA9B6C6),
];

impl Theme {
    pub fn new(night: bool, accent: &str) -> Self {
        let (_, paper_acc, night_acc) = ACCENTS.iter().find(|a| a.0 == accent).copied().unwrap_or(ACCENTS[0]);
        if night {
            let accent = Rgba::hex(night_acc);
            Theme {
                night,
                ground: Rgba::hex(0x101114),
                surf: Rgba::hex(0x1A1C21),
                chrome: Rgba::hex(0x1F2228),
                raised: Rgba::hex(0x252930),
                line: Rgba::hex(0x30343C),
                text: Rgba::hex(0xECEBE6),
                dim: Rgba::hex(0xA3A098),
                accent,
                on_accent: Rgba::hex(0x0E1424),
                glass: Rgba(24, 26, 31, 214),
                scrim: Rgba(9, 10, 12, 158),
                soft: accent.with_alpha(41),
                shadow: Rgba(0, 0, 0, 150),
            }
        } else {
            let accent = Rgba::hex(paper_acc);
            Theme {
                night,
                ground: Rgba::hex(0xECE8DF),
                surf: Rgba::hex(0xFAF8F4),
                chrome: Rgba::hex(0xF2EFE9),
                raised: Rgba::hex(0xFFFFFF),
                line: Rgba::hex(0xDCD6CA),
                text: Rgba::hex(0x1B1B1D),
                dim: Rgba::hex(0x5F5C56),
                accent,
                on_accent: Rgba::hex(0xFFFFFF),
                glass: Rgba(250, 248, 244, 222),
                scrim: Rgba(236, 232, 223, 153),
                soft: accent.with_alpha(26),
                shadow: Rgba(60, 48, 28, 70),
            }
        }
    }
}

/// Measures at scale 1. Everything the WM draws is multiplied by the interface scale.
pub const STRIP: f32 = 36.0; // title strip height
pub const BORDER: i32 = 6; // invisible grab margin around every frame, for resizing
pub const SHADOW: i32 = 40; // how far a shadow reaches past the window
