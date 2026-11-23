//! `samurai apply`: turn the settings into what each program reads, so every app looks and
//! behaves the same way: the window manager, the terminal, notifications, GTK and Qt apps,
//! the keyboard, mouse and screens, night light, and which app opens what.
//!
//! A file is only written when it is absent or was written by Samurai (its first line says so).
//! A file you wrote yourself is left alone, and `apply` says which ones it kept.

use crate::{seal, store};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MARK: &str = "samurai: written by Samurai";

fn home() -> PathBuf {
    store::home()
}
fn xdg_config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
}
fn have(prog: &str) -> bool {
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| Path::new(d).join(prog).is_file())
}
fn run(cmd: &[&str]) -> bool {
    if cmd.is_empty() || !have(cmd[0]) {
        return false;
    }
    Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}
fn output(cmd: &[&str]) -> Option<String> {
    if !have(cmd[0]) {
        return None;
    }
    let o = Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}
/// Start a program detached from us (daemons the session keeps).
fn daemon(cmd: &[&str]) {
    if cmd.is_empty() || !have(cmd[0]) {
        return;
    }
    let _ = Command::new("setsid").arg("-f").args(cmd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
}
fn running(name: &str) -> bool {
    output(&["pgrep", "-x", name]).is_some_and(|o| !o.trim().is_empty())
}

/// Write `text` to `path` unless the user owns that file. Returns a line for the log.
fn put(path: &Path, text: &str, log: &mut Vec<String>) -> bool {
    if let Ok(old) = std::fs::read_to_string(path) {
        if !old.lines().take(3).any(|l| l.contains(MARK)) {
            log.push(format!("kept your own {}", path.display()));
            return false;
        }
        if old == text {
            return true;
        }
    }
    match store::atomic_write(path, text) {
        Ok(()) => {
            log.push(format!("wrote {}", path.display()));
            true
        }
        Err(e) => {
            log.push(format!("could not write {}: {e}", path.display()));
            false
        }
    }
}

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn now_minutes() -> u32 {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        (tm.tm_hour * 60 + tm.tm_min) as u32
    }
}

/// Night or paper, right now.
pub fn ground() -> &'static str {
    match store::get_str("look.ground").as_str() {
        "night" => "night",
        "paper" => "paper",
        _ => {
            let (from, to) = (hhmm(&store::get_str("look.sun_from")).unwrap_or(450), hhmm(&store::get_str("look.sun_to")).unwrap_or(1080));
            let n = now_minutes();
            let day = if from <= to { n >= from && n < to } else { n >= from || n < to };
            if day { "paper" } else { "night" }
        }
    }
}

// ── colours shared by every program ──────────────────────────────────────────────────────────
pub struct Pal {
    pub night: bool,
    pub ground: &'static str,
    pub surf: &'static str,
    pub raised: &'static str,
    pub line: &'static str,
    pub text: &'static str,
    pub dim: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
    pub term_bg: &'static str,
    pub term_fg: &'static str,
    pub term_sel: &'static str,
    pub ansi: [&'static str; 16],
}

pub fn accent_hex(accent: &str, night: bool) -> &'static str {
    match (accent, night) {
        ("moss", false) => "#4A6B3A",
        ("moss", true) => "#9CC28A",
        ("rust", false) => "#9A4A2A",
        ("rust", true) => "#E39A78",
        ("plum", false) => "#6B3F73",
        ("plum", true) => "#C9A0D6",
        ("slate", false) => "#44505E",
        ("slate", true) => "#A9B6C6",
        (_, false) => "#2F4B8F",
        (_, true) => "#93ABEB",
    }
}

pub fn palette(night: bool) -> Pal {
    let accent = accent_hex(&store::get_str("look.accent"), night);
    if night {
        Pal {
            night, ground: "#101114", surf: "#1A1C21", raised: "#252930", line: "#30343C", text: "#ECEBE6", dim: "#A3A098",
            accent, on_accent: "#0E1424", term_bg: "#0E0F12", term_fg: "#DEDCD5", term_sel: "#2A3350",
            ansi: ["#16181C", "#E2857A", "#8FC08A", "#D8B46A", "#93ABEB", "#C7A0DA", "#82C6C3", "#CFCDC6",
                   "#3A3E47", "#F0A096", "#A9D6A4", "#E8C988", "#B3C5F2", "#DBBCE8", "#A3DAD7", "#F4F2EC"],
        }
    } else {
        Pal {
            night, ground: "#ECE8DF", surf: "#FAF8F4", raised: "#FFFFFF", line: "#DCD6CA", text: "#1B1B1D", dim: "#5F5C56",
            accent, on_accent: "#FFFFFF", term_bg: "#F7F4EE", term_fg: "#1E1E20", term_sel: "#D9E0F0",
            ansi: ["#1E1E20", "#A8392C", "#3C7A3A", "#86600E", "#2F4B8F", "#7D4A93", "#2C6E6B", "#D8D3C8",
                   "#5C5852", "#C24A3B", "#4E9249", "#A57914", "#4361AB", "#9562AD", "#3A8884", "#F7F4EE"],
        }
    }
}

// ── the window manager ───────────────────────────────────────────────────────────────────────
fn desktop_exec(id: &str) -> String {
    let id = id.trim().trim_end_matches(".desktop");
    if id.is_empty() {
        return String::new();
    }
    format!("gtk-launch {}", id)
}

pub fn wm_toml() -> String {
    let g = store::get_str;
    let b = |k: &str| if store::get_bool(k) { "true" } else { "false" };
    let mut s = format!("# {MARK} from Settings. Change things in Settings; edits here are replaced.\n\n");
    s += "[look]\n";
    s += &format!("ground = {}\n", q(match g("look.ground").as_str() { "sun" => ground(), x => if x == "paper" { "paper" } else { "night" } }));
    s += &format!("follow_sun = {}\n", g("look.ground") == "sun");
    s += &format!("sun_from = {}\nsun_to = {}\n", q(&g("look.sun_from")), q(&g("look.sun_to")));
    s += &format!("accent = {}\n", q(&g("look.accent")));
    s += &format!("corners = {}\n", if g("look.corners") == "square" { "0.0" } else { "12.0" });
    s += &format!("motion = {}\n", q(&g("look.motion")));
    s += &format!("scale = {:.2}\n", store::get_f64("look.scale"));
    s += &format!("mark = {}\n", q(&seal::mark()));
    s += &format!("show_clock = {}\nshow_mark = {}\n", b("look.show_clock"), b("look.show_mark"));
    s += &format!("ui_font = {}\nmono_font = {}\n", q(&g("look.ui_font")), q(&g("look.mono_font")));
    s += &format!("apps_view = {}\n", q(if g("look.apps_view") == "list" { "list" } else { "icons" }));
    s += "\n[windows]\n";
    s += &format!("gap = {}\n", store::get_f64("windows.gap") as i64);
    s += &format!("strip_follows_app = {}\n", b("windows.strip_follows_app"));
    s += &format!("focus = {}\n", q(&g("windows.focus")));
    s += &format!("snap = {}\nshake_to_tuck = {}\n", b("windows.snap"), b("windows.shake"));
    s += &format!("unredirect_fullscreen = {}\nremember_places = {}\n", b("windows.unredirect"), b("windows.remember"));
    s += &format!("snap_assist = {}\ntiling = {}\n", b("windows.snap_assist"), b("windows.tiling"));
    s += &format!("opacity_active = {:.2}\nopacity_inactive = {:.2}\n", store::get_f64("windows.opacity_active"), store::get_f64("windows.opacity_inactive"));
    let solid: Vec<String> = store::get_list("windows.opaque_apps").iter().filter_map(|r| r["app"].as_str().map(|a| q(a.trim()))).collect();
    if !solid.is_empty() {
        s += &format!("opaque_apps = [{}]\n", solid.join(", "));
    }
    s += "\n[apps]\n";
    s += "terminal = \"alacritty\"\n";
    let files = desktop_exec(&g("apps.files"));
    let browser = desktop_exec(&g("apps.browser"));
    if !files.is_empty() { s += &format!("files = {}\n", q(&files)); }
    if !browser.is_empty() { s += &format!("browser = {}\n", q(&browser)); }
    s += "settings = \"samurai-settings\"\n";
    let pinned: Vec<String> = store::get_list("apps.pinned").iter().filter_map(|r| r["app"].as_str().map(|a| q(a.trim()))).collect();
    if !pinned.is_empty() {
        s += &format!("pinned = [{}]\n", pinned.join(", "));
    }
    s += "\n[idle]\n";
    s += &format!("lock = {}\nscreen_off = {}\nsuspend = {}\n", store::get_f64("idle.lock") as i64, store::get_f64("idle.screen_off") as i64, store::get_f64("idle.suspend") as i64);
    let auto: Vec<String> = store::get_list("autostart").iter().filter_map(|r| r["command"].as_str().map(|c| q(c.trim()))).filter(|c| c.len() > 2).collect();
    if !auto.is_empty() {
        s += &format!("\nautostart = [{}]\n", auto.join(", "));
    }
    s += "\n[keys]\n";
    // the chosen set first, then your own lines on top (one line per key: TOML allows no repeats)
    let mut keys: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    if store::get_str("keys.preset") == "hyprland" {
        for (k, a) in HYPRLAND_KEYS {
            keys.insert(k.to_string(), a.to_string());
        }
    }
    for r in store::get_list("keys.custom") {
        let (k, a) = (r["keys"].as_str().unwrap_or("").trim(), r["action"].as_str().unwrap_or("").trim());
        if !k.is_empty() && !a.is_empty() {
            keys.insert(k.to_string(), a.to_string());
        }
    }
    for (k, a) in keys {
        s += &format!("{} = {}\n", q(&k), q(&a));
    }
    s
}

/// Hyprland's stock shortcuts, as far as Samurai has the same thing (it has no workspaces, so
/// no workspace keys). Laid over Samurai's own; any key not listed keeps Samurai's meaning.
pub const HYPRLAND_KEYS: &[(&str, &str)] = &[
    ("Super+Q", "terminal"),
    ("Super+C", "close"),
    ("Super+M", "quit"),
    ("Super+E", "files"),
    ("Super+V", "float"),
    ("Super+R", "overview"),
    ("Super+F", "fullscreen"),
    ("Super+H", "tuck"),
    ("Super+Shift+F", "fill"),
    ("Super+Left", "focus left"),
    ("Super+Right", "focus right"),
    ("Super+Up", "focus up"),
    ("Super+Down", "focus down"),
    ("Super+Shift+Left", "move left"),
    ("Super+Shift+Right", "move right"),
    ("Super+Shift+Up", "move up"),
    ("Super+Shift+Down", "move down"),
];

// ── the terminal ─────────────────────────────────────────────────────────────────────────────
fn alacritty(p: &Pal, log: &mut Vec<String>) {
    let dir = xdg_config().join("alacritty");
    let mut c = format!("# {MARK} (Settings › Look › Terminal).\n# Put your own lines in local.toml in this folder: they are kept.\n\n");
    c += "[general]\nimport = [\"~/.config/samurai/alacritty/colors.toml\", \"~/.config/alacritty/local.toml\"]\nlive_config_reload = true\n\n";
    let pad = store::get_f64("term.padding") as i64;
    c += &format!("[window]\npadding = {{ x = {}, y = {} }}\ndynamic_padding = true\n\n", pad + 2, pad);
    let mono = store::get_str("look.mono_font");
    c += &format!("[font]\nsize = {:.1}\nnormal = {{ family = {}, style = \"Regular\" }}\nbold = {{ family = {}, style = \"Bold\" }}\n\n", store::get_f64("term.size"), q(&mono), q(&mono));
    c += "[cursor]\nstyle = { shape = \"Beam\", blinking = \"On\" }\n\n[scrolling]\nhistory = 20000\n\n[selection]\nsave_to_clipboard = true\n";
    put(&dir.join("alacritty.toml"), &c, log);
    if !dir.join("local.toml").exists() {
        let _ = store::atomic_write(&dir.join("local.toml"), "# your own alacritty settings (Samurai never changes this file)\n");
    }
    let night = if store::get_bool("term.follow") { p.night } else { true };
    let t = palette(night);
    let mut k = format!("# {MARK}: the Sumi colours, {}.\n", if t.night { "night" } else { "paper" });
    k += &format!("[colors.primary]\nbackground = {}\nforeground = {}\n\n", q(t.term_bg), q(t.term_fg));
    k += &format!("[colors.cursor]\ntext = {}\ncursor = {}\n\n", q(t.term_bg), q(t.ansi[4]));
    k += &format!("[colors.selection]\nbackground = {}\ntext = \"CellForeground\"\n\n", q(t.term_sel));
    let names = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
    k += "[colors.normal]\n";
    for (i, n) in names.iter().enumerate() {
        k += &format!("{n} = {}\n", q(t.ansi[i]));
    }
    k += "\n[colors.bright]\n";
    for (i, n) in names.iter().enumerate() {
        k += &format!("{n} = {}\n", q(t.ansi[8 + i]));
    }
    put(&store::config_dir().join("alacritty/colors.toml"), &k, log);
}

pub const PROMPT: &str = include_str!("../../share/prompt.sh");
pub const LOGO: &str = include_str!("../../share/samurai-logo.txt");

/// "#RRGGBB" → "38;2;R;G;B", the colour form fastfetch takes.
fn ansi_rgb(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128);
    format!("38;2;{};{};{}", v(0), v(2), v(4))
}

/// The system summary a new terminal shows (fastfetch, with Samurai's helmet in the accent).
/// Written only while the setting is on; the prompt script shows it once per terminal window.
fn fetch(log: &mut Vec<String>) {
    let dir = store::config_dir().join("shell");
    let conf = dir.join("fetch.jsonc");
    if !store::get_bool("term.fetch") {
        for f in [&conf] {
            if std::fs::read_to_string(f).map(|t| t.contains(MARK)).unwrap_or(false) {
                let _ = std::fs::remove_file(f);
                log.push(format!("removed {}", f.display()));
            }
        }
        return;
    }
    let night = ground() == "night";
    let p = palette(night);
    let accent = ansi_rgb(p.accent);
    let dim = ansi_rgb(p.dim);
    // the helmet in $1 (accent), the kanji in $2 (dim); the mark line is not part of the logo
    let art: String = LOGO.lines().enumerate().map(|(i, l)| if i + 1 == LOGO.lines().count() { format!("$2{l}\n") } else { format!("$1{l}\n") }).collect();
    // fastfetch reads JSONC, so the mark sits in a comment at the top
    let c = format!(
        "// {MARK} (Settings › Look › Terminal). Turn it off there.\n{{\n  \"logo\": {{ \"type\": \"data\", \"source\": {}, \"color\": {{ \"1\": \"{accent}\", \"2\": \"{dim}\" }}, \"padding\": {{ \"left\": 1, \"right\": 4, \"top\": 1 }} }},\n  \"display\": {{ \"separator\": \"  \", \"color\": {{ \"keys\": \"{accent}\", \"title\": \"{accent}\" }}, \"key\": {{ \"width\": 10 }} }},\n  \"modules\": [\n    \"title\",\n    {{ \"type\": \"os\", \"key\": \"system\" }},\n    {{ \"type\": \"kernel\", \"key\": \"kernel\" }},\n    {{ \"type\": \"wm\", \"key\": \"desktop\" }},\n    {{ \"type\": \"command\", \"key\": \"x server\", \"text\": \"samurai-xserver status 2>/dev/null | sed -n 's/^X server: //p'\" }},\n    {{ \"type\": \"uptime\", \"key\": \"up\" }},\n    {{ \"type\": \"packages\", \"key\": \"packages\" }},\n    {{ \"type\": \"memory\", \"key\": \"memory\" }},\n    {{ \"type\": \"wifi\", \"key\": \"wi-fi\" }},\n    \"break\",\n    \"colors\"\n  ]\n}}\n",
        serde_json::Value::String(art)
    );
    put(&conf, &c, log);
}

fn prompt(log: &mut Vec<String>) {
    let file = store::config_dir().join("shell/prompt.sh");
    let on = store::get_bool("term.prompt");
    if on {
        put(&file, PROMPT, log);
    }
    let line = "[ -f \"${XDG_CONFIG_HOME:-$HOME/.config}/samurai/shell/prompt.sh\" ] && . \"${XDG_CONFIG_HOME:-$HOME/.config}/samurai/shell/prompt.sh\"  # samurai prompt";
    for rc in [".bashrc", ".zshrc"] {
        let p = home().join(rc);
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let has = text.contains("# samurai prompt");
        if on && !has && (p.exists() || rc == ".bashrc") {
            let _ = store::atomic_write(&p, &format!("{}{}{line}\n", text, if text.is_empty() || text.ends_with('\n') { "" } else { "\n" }));
            log.push(format!("added the prompt to ~/{rc}"));
        } else if !on && has {
            let kept: Vec<&str> = text.lines().filter(|l| !l.contains("# samurai prompt")).collect();
            let _ = store::atomic_write(&p, &(kept.join("\n") + "\n"));
            log.push(format!("took the prompt out of ~/{rc}"));
        }
    }
}

// ── notifications ────────────────────────────────────────────────────────────────────────────
fn dunst(p: &Pal, log: &mut Vec<String>) {
    let secs = store::get_f64("notices.seconds") as i64;
    let corner = store::get_str("notices.corner");
    let s = store::get_f64("look.scale");
    let (origin, off) = match corner.as_str() {
        "top-center" => ("top-center", (0, 60)),
        "bottom-right" => ("bottom-right", (20, 60)),
        _ => ("top-right", (20, 60)),
    };
    let ui = store::get_str("look.ui_font");
    let bg = format!("{}F0", p.surf);
    let mut c = format!("# {MARK} (Settings › Notifications).\n# Your own changes go in ~/.config/dunst/dunstrc.d/*.conf: they are kept.\n\n[global]\n");
    c += &format!("font = \"{ui} {}\"\nwidth = {}\nheight = (0, 300)\norigin = {origin}\noffset = ({}, {})\n", (10.0 * s).round(), (360.0 * s) as i64, off.0, off.1);
    c += &format!("corner_radius = {}\nframe_width = 1\nframe_color = \"{}\"\nseparator_height = 1\nseparator_color = frame\n", if store::get_str("look.corners") == "square" { 0 } else { 12 }, p.line);
    c += &format!("padding = {}\nhorizontal_padding = {}\ngap_size = 8\nicon_position = left\nmax_icon_size = {}\n", (14.0 * s) as i64, (16.0 * s) as i64, (36.0 * s) as i64);
    c += &format!("background = \"{bg}\"\nforeground = \"{}\"\nhighlight = \"{}\"\n", p.text, p.accent);
    c += "markup = full\nformat = \"<b>%s</b>\\n%b\"\nalignment = left\nshow_age_threshold = 60\nellipsize = end\nfollow = mouse\n";
    c += &format!("sticky_history = {}\nhistory_length = 40\nmouse_left_click = do_action, close_current\nmouse_right_click = close_current\n", if store::get_bool("notices.history") { "yes" } else { "no" });
    c += &format!("\n[urgency_low]\nbackground = \"{bg}\"\nforeground = \"{}\"\ntimeout = {secs}\n", p.dim);
    c += &format!("\n[urgency_normal]\nbackground = \"{bg}\"\nforeground = \"{}\"\ntimeout = {secs}\n", p.text);
    c += &format!("\n[urgency_critical]\nbackground = \"{bg}\"\nforeground = \"{}\"\nframe_color = \"{}\"\ntimeout = 0\n", p.text, p.accent);
    if put(&xdg_config().join("dunst/dunstrc"), &c, log) && running("dunst") {
        if !run(&["dunstctl", "reload"]) {
            run(&["pkill", "-x", "dunst"]);
            daemon(&["dunst"]);
        }
    }
}

// ── GTK, Qt, fonts, cursor ───────────────────────────────────────────────────────────────────
fn toolkits(p: &Pal, log: &mut Vec<String>) {
    let s = store::get_f64("look.scale");
    let cursor = store::get_str("mouse.cursor").parse::<f64>().unwrap_or(24.0);
    let cursor_px = (cursor * s).round() as i64;
    let ui = store::get_str("look.ui_font");
    let mono = store::get_str("look.mono_font");
    let dark = p.night;
    let gtk_theme = if store::get_bool("apps.gtk") { if dark { "adw-gtk3-dark" } else { "adw-gtk3" } } else { "Adwaita" };
    let icons = if dark { "Papirus-Dark" } else { "Papirus" };
    let dpi = (96.0 * s).round() as i64;
    for v in ["gtk-3.0", "gtk-4.0"] {
        let mut c = format!("# {MARK} (Settings › Apps).\n[Settings]\n");
        if v == "gtk-3.0" {
            c += &format!("gtk-theme-name={gtk_theme}\n");
        }
        c += &format!("gtk-application-prefer-dark-theme={}\ngtk-icon-theme-name={icons}\ngtk-font-name={ui} 10\ngtk-cursor-theme-name=Adwaita\ngtk-cursor-theme-size={cursor_px}\ngtk-decoration-layout=:\n", dark as u8);
        put(&xdg_config().join(v).join("settings.ini"), &c, log);
    }
    // xsettingsd tells running GTK apps at once, without a restart
    let mut x = format!("# {MARK}\nNet/ThemeName \"{gtk_theme}\"\nNet/IconThemeName \"{icons}\"\nGtk/FontName \"{ui} 10\"\nGtk/MonospaceFontName \"{mono} 10\"\n");
    x += &format!("Gtk/CursorThemeName \"Adwaita\"\nGtk/CursorThemeSize {cursor_px}\nXft/DPI {}\nXft/Antialias 1\nXft/Hinting 1\nXft/HintStyle \"hintslight\"\nXft/RGBA \"rgb\"\nNet/EnableEventSounds 0\nGtk/DecorationLayout \":\"\n", dpi * 1024);
    if put(&xdg_config().join("xsettingsd/xsettingsd.conf"), &x, log) && running("xsettingsd") {
        run(&["pkill", "-HUP", "-x", "xsettingsd"]);
    }
    // GTK4 / libadwaita and Firefox read these through the desktop portal
    if have("gsettings") && store::get_bool("apps.gtk") {
        let i = "org.gnome.desktop.interface";
        run(&["gsettings", "set", i, "color-scheme", if dark { "prefer-dark" } else { "default" }]);
        run(&["gsettings", "set", i, "gtk-theme", gtk_theme]);
        run(&["gsettings", "set", i, "icon-theme", icons]);
        run(&["gsettings", "set", i, "cursor-theme", "Adwaita"]);
        run(&["gsettings", "set", i, "cursor-size", &cursor_px.to_string()]);
        if store::get_bool("apps.fonts") {
            run(&["gsettings", "set", i, "font-name", &format!("{ui} 10")]);
            run(&["gsettings", "set", i, "monospace-font-name", &format!("{mono} 10")]);
        }
    }
    // Xft (older apps) and the cursor
    let xr = format!("! {MARK}\nXft.dpi: {dpi}\nXft.antialias: 1\nXft.hinting: 1\nXft.hintstyle: hintslight\nXft.rgba: rgb\nXcursor.theme: Adwaita\nXcursor.size: {cursor_px}\n");
    let xr_path = store::config_dir().join("Xresources");
    put(&xr_path, &xr, log);
    if std::env::var_os("DISPLAY").is_some() {
        run(&["xrdb", "-merge", &xr_path.to_string_lossy()]);
    }
    // Qt: qt6ct and qt5ct, with a palette made from ours
    if store::get_bool("apps.qt") {
        let scheme = qt_scheme(p);
        let scheme_path = store::config_dir().join(format!("qt/samurai-{}.conf", if dark { "night" } else { "paper" }));
        put(&scheme_path, &scheme, log);
        for v in ["qt6ct", "qt5ct"] {
            let font = if v == "qt6ct" { format!("\"{ui},10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1\"") } else { format!("\"{ui},10,-1,5,50,0,0,0,0,0\"") };
            let mfont = if v == "qt6ct" { format!("\"{mono},10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1\"") } else { format!("\"{mono},10,-1,5,50,0,0,0,0,0\"") };
            let c = format!("# {MARK} (Settings › Apps)\n[Appearance]\ncolor_scheme_path={}\ncustom_palette=true\nicon_theme={icons}\nstandard_dialogs=gtk3\nstyle=Fusion\n\n[Fonts]\nfixed={mfont}\ngeneral={font}\n", scheme_path.display());
            put(&xdg_config().join(v).join(format!("{v}.conf")), &c, log);
        }
    }
}

fn qt_scheme(p: &Pal) -> String {
    let a = |h: &str| format!("#ff{}", h.trim_start_matches('#').to_lowercase());
    // WindowText Button Light Midlight Dark Mid Text BrightText ButtonText Base Window Shadow
    // Highlight HighlightedText Link LinkVisited AlternateBase NoRole ToolTipBase ToolTipText PlaceholderText
    let roles = [p.text, p.raised, p.raised, p.surf, p.line, p.line, p.text, "#ffffff", p.text, p.surf, p.ground, "#000000",
                 p.accent, p.on_accent, p.accent, p.accent, p.raised, p.ground, p.raised, p.text, p.dim];
    let active: Vec<String> = roles.iter().map(|r| a(r)).collect();
    let disabled: Vec<String> = roles.iter().enumerate().map(|(i, r)| if [0, 6, 8].contains(&i) { a(p.dim) } else { a(r) }).collect();
    format!("# {MARK}\n[ColorScheme]\nactive_colors={}\ndisabled_colors={}\ninactive_colors={}\n", active.join(", "), disabled.join(", "), active.join(", "))
}

// ── keyboard, mouse, touchpad ────────────────────────────────────────────────────────────────
fn input(log: &mut Vec<String>) {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let mut kb = vec!["setxkbmap".to_string(), "-layout".into(), store::get_str("kbd.layout")];
    let variant = store::get_str("kbd.variant");
    if !variant.is_empty() {
        kb.push("-variant".into());
        kb.push(variant);
    }
    kb.push("-option".into());
    kb.push(String::new());
    let opts = store::get_str("kbd.options");
    if !opts.is_empty() {
        kb.push("-option".into());
        kb.push(opts);
    }
    if !run(&kb.iter().map(|s| s.as_str()).collect::<Vec<_>>()) {
        log.push("keyboard layout not set (setxkbmap missing or the layout name is wrong)".into());
    }
    run(&["xset", "r", "rate", &(store::get_f64("kbd.delay") as i64).to_string(), &(store::get_f64("kbd.rate") as i64).to_string()]);
    let Some(ids) = output(&["xinput", "list", "--id-only"]) else { return };
    for id in ids.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
        let Some(props) = output(&["xinput", "list-props", id]) else { continue };
        if !props.contains("libinput Accel Speed") {
            continue;
        }
        let touch = props.contains("libinput Tapping Enabled");
        let set = |name: &str, val: &str| {
            if props.contains(name) {
                run(&["xinput", "set-prop", id, name, val]);
            }
        };
        set("libinput Accel Speed", &format!("{:.2}", store::get_f64("mouse.speed")));
        set("libinput Left Handed Enabled", if store::get_bool("mouse.left_handed") { "1" } else { "0" });
        if touch {
            set("libinput Tapping Enabled", if store::get_bool("touch.tap") { "1" } else { "0" });
            set("libinput Disable While Typing Enabled", if store::get_bool("touch.dwt") { "1" } else { "0" });
            set("libinput Natural Scrolling Enabled", if store::get_bool("touch.natural") { "1" } else { "0" });
        } else {
            set("libinput Natural Scrolling Enabled", if store::get_bool("mouse.natural") { "1" } else { "0" });
        }
    }
}

// ── screens ──────────────────────────────────────────────────────────────────────────────────
pub fn displays(log: &mut Vec<String>) {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let rows = store::get_list("displays.monitors");
    let Some(q) = output(&["xrandr", "--query"]) else { return };
    let connected: Vec<String> = q.lines().filter(|l| l.contains(" connected")).filter_map(|l| l.split_whitespace().next().map(|s| s.to_string())).collect();
    let mut args: Vec<String> = vec!["xrandr".into()];
    let mut prev: Option<String> = None;
    for out in &connected {
        args.push("--output".into());
        args.push(out.clone());
        if let Some(r) = rows.iter().find(|r| r["output"].as_str() == Some(out.as_str())) {
            let mode = r["mode"].as_str().unwrap_or("").trim();
            if mode == "off" {
                args.push("--off".into());
                continue;
            }
            if let Some((m, rate)) = mode.split_once('@') {
                args.extend(["--mode".into(), m.to_string(), "--rate".into(), rate.to_string()]);
            } else if !mode.is_empty() {
                args.extend(["--mode".into(), mode.to_string()]);
            } else {
                args.push("--auto".into());
            }
            let pos = r["position"].as_str().unwrap_or("").trim();
            let words: Vec<&str> = pos.split_whitespace().collect();
            match words.as_slice() {
                [rel, other] if ["right-of", "left-of", "above", "below", "same-as"].contains(rel) => args.extend([format!("--{rel}"), other.to_string()]),
                [xy] if xy.contains('x') => args.extend(["--pos".into(), xy.to_string()]),
                _ => if let Some(p) = &prev { args.extend(["--right-of".into(), p.clone()]) },
            }
            let rot = r["rotate"].as_str().unwrap_or("normal").trim();
            if ["normal", "left", "right", "inverted"].contains(&rot) {
                args.extend(["--rotate".into(), rot.to_string()]);
            }
            if r["primary"].as_str().is_some_and(|p| p.trim() == "yes") {
                args.push("--primary".into());
            }
        } else {
            args.push("--auto".into());
            if let Some(p) = &prev {
                args.extend([if store::get_bool("displays.mirror_new") { "--same-as".into() } else { "--right-of".into() }, p.clone()]);
            }
        }
        prev = Some(out.clone());
    }
    if connected.len() > 1 || !rows.is_empty() {
        if !run(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>()) {
            log.push(format!("screens: xrandr refused \"{}\"", args[1..].join(" ")));
        }
    }
}

// ── night light ──────────────────────────────────────────────────────────────────────────────
fn night_light(log: &mut Vec<String>) {
    let c = format!(
        "; {MARK} (Settings › Displays › Night light)\n[general]\ntemp-day=6500\ntemp-night={}\ndawn-time={}\ndusk-time={}\nfade=1\nadjustment-method=randr\n",
        store::get_f64("night.temp") as i64,
        store::get_str("night.to"),
        store::get_str("night.from")
    );
    let changed = put(&xdg_config().join("gammastep/config.ini"), &c, log);
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let on = store::get_bool("night.enabled");
    let is = running("gammastep");
    if on && (!is || changed) {
        run(&["pkill", "-x", "gammastep"]);
        daemon(&["gammastep"]);
    } else if !on && is {
        run(&["pkill", "-x", "gammastep"]);
        run(&["gammastep", "-x"]);
    }
}

// ── which app opens what ─────────────────────────────────────────────────────────────────────
fn desktop_exists(id: &str) -> bool {
    let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let mut all: Vec<PathBuf> = dirs.split(':').map(PathBuf::from).collect();
    all.insert(0, home().join(".local/share"));
    all.iter().any(|d| d.join("applications").join(id).is_file())
}

fn mime_defaults(log: &mut Vec<String>) {
    if !have("xdg-mime") {
        return;
    }
    let map: [(&str, &[&str]); 8] = [
        ("apps.browser", &["x-scheme-handler/http", "x-scheme-handler/https", "text/html", "application/xhtml+xml"]),
        ("apps.files", &["inode/directory"]),
        ("apps.editor", &["text/plain", "text/markdown", "text/x-log", "application/x-shellscript", "text/x-python", "application/json", "application/toml"]),
        ("apps.images", &["image/png", "image/jpeg", "image/gif", "image/webp", "image/bmp", "image/tiff"]),
        ("apps.pdf", &["application/pdf"]),
        ("apps.video", &["video/mp4", "video/x-matroska", "video/webm", "video/quicktime", "video/x-msvideo", "video/mpeg"]),
        ("apps.music", &["audio/mpeg", "audio/flac", "audio/ogg", "audio/x-wav", "audio/mp4", "audio/x-opus+ogg"]),
        ("apps.mail", &["x-scheme-handler/mailto"]),
    ];
    let mut missing = vec![];
    for (key, types) in map {
        let mut id = store::get_str(key).trim().to_string();
        if id.is_empty() {
            continue;
        }
        if !id.ends_with(".desktop") {
            id.push_str(".desktop");
        }
        if !desktop_exists(&id) {
            missing.push(id);
            continue;
        }
        let mut cmd = vec!["xdg-mime", "default", id.as_str()];
        cmd.extend(types.iter());
        run(&cmd);
        if key == "apps.browser" {
            run(&["xdg-settings", "set", "default-web-browser", &id]);
        }
    }
    if !missing.is_empty() {
        log.push(format!("not installed, so not made the default: {}", missing.join(", ")));
    }
}

// ── all of it ────────────────────────────────────────────────────────────────────────────────
/// Only what follows night and paper (the window manager calls this when the sun sets or rises).
pub fn theme(ground: &str) -> Vec<String> {
    let mut log = vec![];
    let p = palette(ground == "night");
    alacritty(&p, &mut log);
    dunst(&p, &mut log);
    toolkits(&p, &mut log);
    log
}

pub fn apply() -> Result<Vec<String>, String> {
    let mut log = vec![];
    if !seal::exists() {
        seal::forge(false).map(|i| log.push(format!("made this machine's ink: {}", i["mark"].as_str().unwrap_or(""))))?;
    }
    if seal::recolor(&store::get_str("look.accent")).unwrap_or(false) {
        log.push("recoloured the ink for the new accent".into());
    }
    if !seal::dir().join("decoy-night.svg").exists() {
        let _ = seal::decoy();
    }
    let wm = store::config_dir().join("wm.toml");
    store::atomic_write(&wm, &wm_toml()).map_err(|e| format!("{}: {e}", wm.display()))?;
    log.push(format!("wrote {}", wm.display()));
    log.extend(theme(ground()));
    prompt(&mut log);
    fetch(&mut log);
    input(&mut log);
    night_light(&mut log);
    mime_defaults(&mut log);
    // the running window manager reads its file again
    if std::env::var_os("DISPLAY").is_some() && have("samurai-wm") {
        run(&["samurai-wm", "msg", "reload"]);
    }
    Ok(log)
}

/// At login, before the window manager starts: settings, screens, and the helpers a session
/// needs (live theme for GTK, notifications, locking before sleep, password prompts, night light).
pub fn session_start() -> Result<Vec<String>, String> {
    let mut log = apply()?;
    displays(&mut log);
    // the first login: a short setup (shortcuts, look, Wi-Fi); skipping it counts as done
    if !store::config_dir().join("first-setup.done").exists() && have("samurai-settings") {
        daemon(&["samurai-settings", "--first-setup"]);
        log.push("first setup opened".into());
    }
    if !running("xsettingsd") {
        daemon(&["xsettingsd", "-c", &xdg_config().join("xsettingsd/xsettingsd.conf").to_string_lossy()]);
    }
    if !running("dunst") {
        daemon(&["dunst"]);
    }
    if store::get_bool("lock.on_sleep") && !running("xss-lock") {
        daemon(&["xss-lock", "--transfer-sleep-lock", "--", "samurai-wm", "msg", "lock", "--wait"]);
    }
    // a password prompt for apps that ask for more rights
    for agent in ["/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1", "/usr/lib/xfce-polkit/xfce-polkit", "/usr/bin/lxqt-policykit-agent"] {
        if Path::new(agent).exists() {
            if !running(Path::new(agent).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default().as_str()) {
                daemon(&[agent]);
            }
            break;
        }
    }
    // after a system update, check the campus Wi-Fi once
    if store::get_bool("net.repair_after_update") {
        let stamp = store::state_dir().join("pacman-seen");
        let log_mtime = std::fs::metadata("/var/log/pacman.log").and_then(|m| m.modified()).ok();
        let seen = std::fs::metadata(&stamp).and_then(|m| m.modified()).ok();
        if log_mtime.is_some() && (seen.is_none() || log_mtime > seen) {
            daemon(&["shadow", "wifi", "repair", "--quiet"]);
            let _ = store::atomic_write(&stamp, "");
        }
    }
    Ok(log)
}

/// Which X server runs: X.Org or XLibre, and its version.
pub fn xserver() -> Value {
    let raw = ["Xorg", "X"].iter().find_map(|b| {
        if !have(b) { return None; }
        Command::new(b).arg("-version").stdin(Stdio::null()).output().ok().map(|o| String::from_utf8_lossy(&o.stderr).to_string() + &String::from_utf8_lossy(&o.stdout))
    }).unwrap_or_default();
    let first = raw.lines().find(|l| l.contains("X Server") || l.contains("X server") || l.contains("XLibre")).unwrap_or("").trim().to_string();
    let kind = if raw.contains("XLibre") || raw.to_lowercase().contains("xlibre") { "xlibre" } else if raw.contains("X.Org") { "xorg" } else if raw.is_empty() { "none" } else { "other" };
    let version = first.split_whitespace().find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit())).unwrap_or("").to_string();
    let pkg = output(&["pacman", "-Qq", "xlibre-xserver"]).or_else(|| output(&["pacman", "-Qq", "xlibre-server"])).map(|s| s.trim().to_string());
    serde_json::json!({ "server": kind, "version": version, "line": first, "xlibre_package": pkg })
}

/// Screenshots: an area, the whole screen, or the focused window. Saved and copied.
pub fn shot(mode: &str) -> Result<String, String> {
    if !have("maim") {
        return Err("screenshots need maim (and xclip to copy them)".into());
    }
    let dir = std::env::var("XDG_PICTURES_DIR").map(PathBuf::from).unwrap_or_else(|_| home().join("Pictures")).join("Screenshots");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = output(&["date", "+%Y-%m-%d_%H-%M-%S"]).unwrap_or_default().trim().to_string();
    let file = dir.join(format!("Screenshot_{stamp}.png"));
    let f = file.to_string_lossy().to_string();
    let ok = match mode {
        "area" => run(&["maim", "-s", "-u", "-b", "2", "-c", "0.58,0.67,0.92,0.9", &f]),
        "screen" => run(&["maim", "-u", &f]),
        "window" => {
            let id = output(&["xdotool", "getactivewindow"]).unwrap_or_default();
            if id.trim().is_empty() { run(&["maim", "-u", &f]) } else { run(&["maim", "-u", "-i", id.trim(), &f]) }
        }
        m => return Err(format!("shot {m}: area, screen or window")),
    };
    if !ok || !file.exists() {
        return Err("no screenshot taken".into());
    }
    if have("xclip") {
        let _ = Command::new("sh").arg("-c").arg(format!("xclip -selection clipboard -t image/png -i {}", crate::misc::quote(&f))).status();
    }
    if have("samurai-wm") {
        run(&["samurai-wm", "msg", "say", &format!("Screenshot saved to {} and copied", file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())]);
    }
    Ok(f)
}

/// The quick-panel toggles and keys that change a setting.
pub fn toggle(what: &str) -> Result<String, String> {
    match what {
        "ground" => {
            let now = ground();
            store::set("look.ground", Value::String(if now == "night" { "paper".into() } else { "night".into() }))?;
        }
        "night" => store::set("night.enabled", Value::Bool(!store::get_bool("night.enabled")))?,
        _ => return Err("toggle ground | night".into()),
    }
    apply().map(|_| "ok".into())
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn the_wm_file_is_valid_toml_shape() {
        let s = wm_toml();
        assert!(s.contains("[look]") && s.contains("[keys]") && s.contains("terminal = \"alacritty\""));
        assert!(!s.contains("\"\"\""));
    }
    #[test]
    fn fetch_colours() {
        assert_eq!(ansi_rgb("#93ABEB"), "38;2;147;171;235");
    }
    #[test]
    fn hyprland_set_has_no_repeats() {
        let mut seen = std::collections::HashSet::new();
        for (k, _) in HYPRLAND_KEYS {
            assert!(seen.insert(k.to_lowercase()), "{k} twice");
        }
    }
    #[test]
    fn quotes_are_escaped() {
        assert_eq!(q("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
    #[test]
    fn qt_palette_has_every_role() {
        let s = qt_scheme(&palette(true));
        let line = s.lines().find(|l| l.starts_with("active_colors=")).unwrap();
        assert_eq!(line.matches("#ff").count(), 21);
    }
}
