//! X11 plumbing: atoms, keyboard tables, key-binding parsing, small property helpers.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS, WM_DELETE_WINDOW, WM_TAKE_FOCUS, WM_STATE, WM_CHANGE_STATE, WM_NAME, WM_CLASS,
        WM_TRANSIENT_FOR, WM_HINTS, WM_NORMAL_HINTS, UTF8_STRING,
        _NET_SUPPORTED, _NET_SUPPORTING_WM_CHECK, _NET_WM_NAME, _NET_CLIENT_LIST, _NET_CLIENT_LIST_STACKING,
        _NET_ACTIVE_WINDOW, _NET_CLOSE_WINDOW, _NET_WM_STATE, _NET_WM_STATE_FULLSCREEN, _NET_WM_STATE_HIDDEN,
        _NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ, _NET_WM_STATE_DEMANDS_ATTENTION,
        _NET_WM_STATE_FOCUSED, _NET_WM_STATE_ABOVE,
        _NET_WM_WINDOW_TYPE, _NET_WM_WINDOW_TYPE_NORMAL, _NET_WM_WINDOW_TYPE_DIALOG, _NET_WM_WINDOW_TYPE_UTILITY,
        _NET_WM_WINDOW_TYPE_SPLASH, _NET_WM_WINDOW_TYPE_DOCK, _NET_WM_WINDOW_TYPE_DESKTOP, _NET_WM_WINDOW_TYPE_TOOLBAR,
        _NET_WM_WINDOW_TYPE_MENU, _NET_WM_WINDOW_TYPE_NOTIFICATION, _NET_WM_WINDOW_TYPE_TOOLTIP,
        _NET_WM_WINDOW_TYPE_POPUP_MENU, _NET_WM_WINDOW_TYPE_DROPDOWN_MENU, _NET_WM_WINDOW_TYPE_COMBO, _NET_WM_WINDOW_TYPE_DND,
        _NET_WM_PID, _NET_WM_USER_TIME, _NET_FRAME_EXTENTS, _NET_REQUEST_FRAME_EXTENTS, _NET_WORKAREA,
        _NET_WM_STRUT, _NET_WM_STRUT_PARTIAL, _NET_MOVERESIZE_WINDOW, _NET_WM_MOVERESIZE,
        _NET_NUMBER_OF_DESKTOPS, _NET_CURRENT_DESKTOP, _NET_DESKTOP_NAMES, _NET_WM_DESKTOP, _NET_DESKTOP_GEOMETRY,
        _NET_DESKTOP_VIEWPORT, _NET_WM_ALLOWED_ACTIONS, _NET_WM_ACTION_CLOSE, _NET_WM_ACTION_MOVE,
        _NET_WM_ACTION_RESIZE, _NET_WM_ACTION_FULLSCREEN, _NET_WM_ACTION_MINIMIZE, _NET_WM_ACTION_MAXIMIZE_HORZ,
        _NET_WM_ACTION_MAXIMIZE_VERT, _NET_WM_WINDOW_OPACITY, _NET_WM_CM_S0, _NET_WM_BYPASS_COMPOSITOR,
        _MOTIF_WM_HINTS, _GTK_FRAME_EXTENTS, _XROOTPMAP_ID, ESETROOT_PMAP_ID, MANAGER,
    }
}

pub const MOD_SHIFT: u16 = 1;
pub const MOD_LOCK: u16 = 2;
pub const MOD_CTRL: u16 = 4;
pub const MOD_ALT: u16 = 8;
pub const MOD_NUM: u16 = 16;
pub const MOD_SUPER: u16 = 64;
/// Lock and NumLock must not change what a binding means.
pub const IGNORED: [u16; 4] = [0, MOD_LOCK, MOD_NUM, MOD_LOCK | MOD_NUM];

pub fn keysym_named(name: &str) -> Option<u32> {
    let n = name.trim();
    let lower = n.to_ascii_lowercase();
    if n.len() == 1 {
        let c = n.chars().next()?;
        if c.is_ascii_alphabetic() {
            return Some(c.to_ascii_lowercase() as u32);
        }
        if c.is_ascii_graphic() {
            return Some(c as u32);
        }
    }
    if let Some(f) = lower.strip_prefix('f') {
        if let Ok(k) = f.parse::<u32>() {
            if (1..=24).contains(&k) {
                return Some(0xffbe + k - 1);
            }
        }
    }
    Some(match lower.as_str() {
        "return" | "enter" => 0xff0d,
        "tab" => 0xff09,
        "escape" | "esc" => 0xff1b,
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "space" => 0x20,
        "comma" => 0x2c,
        "period" => 0x2e,
        "slash" => 0x2f,
        "minus" => 0x2d,
        "equal" => 0x3d,
        "grave" => 0x60,
        "semicolon" => 0x3b,
        "left" => 0xff51,
        "up" => 0xff52,
        "right" => 0xff53,
        "down" => 0xff54,
        "home" => 0xff50,
        "end" => 0xff57,
        "page_up" | "pageup" => 0xff55,
        "page_down" | "pagedown" => 0xff56,
        "print" => 0xff61,
        "super" | "super_l" => 0xffeb,
        "super_r" => 0xffec,
        "xf86audiolowervolume" => 0x1008ff11,
        "xf86audiomute" => 0x1008ff12,
        "xf86audioraisevolume" => 0x1008ff13,
        "xf86audioplay" => 0x1008ff14,
        "xf86audioprev" => 0x1008ff16,
        "xf86audionext" => 0x1008ff17,
        "xf86audiomicmute" => 0x1008ffb2,
        "xf86monbrightnessup" => 0x1008ff02,
        "xf86monbrightnessdown" => 0x1008ff03,
        _ => return None,
    })
}

pub const KS_SUPER_L: u32 = 0xffeb;
pub const KS_SUPER_R: u32 = 0xffec;
pub const KS_ESCAPE: u32 = 0xff1b;
pub const KS_RETURN: u32 = 0xff0d;
pub const KS_KP_ENTER: u32 = 0xff8d;
pub const KS_BACKSPACE: u32 = 0xff08;
pub const KS_DELETE: u32 = 0xffff;
pub const KS_TAB: u32 = 0xff09;
pub const KS_ISO_LEFT_TAB: u32 = 0xfe20;
pub const KS_LEFT: u32 = 0xff51;
pub const KS_UP: u32 = 0xff52;
pub const KS_RIGHT: u32 = 0xff53;
pub const KS_DOWN: u32 = 0xff54;
pub const KS_ALT_L: u32 = 0xffe9;
pub const KS_ALT_R: u32 = 0xffea;

/// Parse "Super+Shift+Left" into (modifiers, keysym). A bare "Super" is the Super key itself.
pub fn parse_combo(s: &str) -> Option<(u16, u32)> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    let (last, mods) = parts.split_last()?;
    let mut m = 0u16;
    for p in mods {
        m |= match p.to_ascii_lowercase().as_str() {
            "super" | "mod4" | "win" => MOD_SUPER,
            "shift" => MOD_SHIFT,
            "ctrl" | "control" => MOD_CTRL,
            "alt" | "mod1" => MOD_ALT,
            _ => return None,
        };
    }
    Some((m, keysym_named(last)?))
}

pub struct Keymap {
    pub min: u8,
    pub per: usize,
    pub syms: Vec<u32>,
}

impl Keymap {
    pub fn load(conn: &RustConnection) -> Option<Self> {
        let setup = conn.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let r = conn.get_keyboard_mapping(min, max - min + 1).ok()?.reply().ok()?;
        Some(Keymap { min, per: r.keysyms_per_keycode as usize, syms: r.keysyms })
    }
    pub fn sym(&self, code: u8, level: usize) -> u32 {
        if code < self.min || self.per == 0 {
            return 0;
        }
        let i = (code - self.min) as usize * self.per;
        let s = self.syms.get(i + level.min(self.per - 1)).copied().unwrap_or(0);
        if s == 0 && level > 0 { self.syms.get(i).copied().unwrap_or(0) } else { s }
    }
    pub fn codes_for(&self, sym: u32) -> Vec<u8> {
        let mut v = vec![];
        for (i, chunk) in self.syms.chunks(self.per.max(1)).enumerate() {
            if chunk.iter().take(2).any(|&s| s == sym || (sym < 0x7b && s == sym)) {
                v.push(self.min + i as u8);
            }
        }
        v
    }
    /// The character a key press types, for the overview search and the lock field.
    pub fn char_for(&self, code: u8, state: u16) -> Option<char> {
        let shift = state & MOD_SHIFT != 0;
        let mut s = self.sym(code, if shift { 1 } else { 0 });
        if state & MOD_LOCK != 0 && (0x61..=0x7a).contains(&s) {
            s -= 0x20;
        }
        keysym_char(s)
    }
}

pub fn keysym_char(s: u32) -> Option<char> {
    match s {
        0x20..=0x7e | 0xa0..=0xff => char::from_u32(s),
        0x1000100..=0x110ffff => char::from_u32(s - 0x1000000),
        _ => None,
    }
}

pub fn get_string(conn: &RustConnection, atoms: &Atoms, win: Window, prop: Atom) -> Option<String> {
    let r = conn.get_property(false, win, prop, AtomEnum::ANY, 0, 1024).ok()?.reply().ok()?;
    if r.value.is_empty() {
        return None;
    }
    let _ = atoms;
    Some(String::from_utf8_lossy(&r.value).trim_end_matches('\0').to_string())
}

pub fn get_u32s(conn: &RustConnection, win: Window, prop: impl Into<Atom>, ty: impl Into<Atom>) -> Vec<u32> {
    conn.get_property(false, win, prop, ty, 0, 1024)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().map(|v| v.collect()))
        .unwrap_or_default()
}

pub fn set_u32s(conn: &RustConnection, win: Window, prop: Atom, ty: impl Into<Atom>, vals: &[u32]) {
    use x11rb::wrapper::ConnectionExt as _;
    let _ = conn.change_property32(PropMode::REPLACE, win, prop, ty, vals);
}

pub fn title(conn: &RustConnection, atoms: &Atoms, win: Window) -> String {
    let utf = conn
        .get_property(false, win, atoms._NET_WM_NAME, atoms.UTF8_STRING, 0, 1024)
        .ok()
        .and_then(|c| c.reply().ok())
        .filter(|r| !r.value.is_empty())
        .map(|r| String::from_utf8_lossy(&r.value).to_string());
    utf.or_else(|| {
        conn.get_property(false, win, AtomEnum::WM_NAME, AtomEnum::ANY, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.value.iter().map(|&b| b as char).collect())
    })
    .unwrap_or_default()
}

pub fn class(conn: &RustConnection, win: Window) -> (String, String) {
    let v = conn
        .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.value)
        .unwrap_or_default();
    let mut it = v.split(|&b| b == 0).map(|s| String::from_utf8_lossy(s).to_string());
    let inst = it.next().unwrap_or_default();
    let cls = it.next().unwrap_or_default();
    (inst, cls)
}
