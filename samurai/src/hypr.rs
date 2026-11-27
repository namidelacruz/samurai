//! Samurai on Hyprland: the same look, size and shortcuts, with Hyprland as the window manager.
//!
//! `samurai hyprland write` writes everything the session needs under ~/.config/samurai/hypr/:
//! hyprland.conf (screens, size, colours, keys), the bar (waybar), the app list (fuzzel),
//! the lock screen (hyprlock) and the idle timers (hypridle). `samurai apply` rewrites them when
//! they exist, so a change in Settings reaches the running session too.
//!
//! The X11 session is not touched: this is a second choice on the login screen.

use crate::store;
use crate::x11::{self, have, palette, run, Pal};
use std::path::PathBuf;

const MARK: &str = "Written by Samurai";

pub fn dir() -> PathBuf {
    store::config_dir().join("hypr")
}

/// Is this a Hyprland session right now?
pub fn running() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Hyprland, and the parts the session uses: (name, program, needed for the session to work).
pub const PARTS: &[(&str, &str, bool)] = &[
    ("Hyprland", "Hyprland", true),
    ("the bar", "waybar", true),
    ("the app list", "fuzzel", true),
    ("the lock screen", "hyprlock", true),
    ("idle timers", "hypridle", false),
    ("the wallpaper", "swaybg", false),
    ("screenshots", "grim", false),
    ("picking an area", "slurp", false),
    ("the clipboard", "wl-copy", false),
];

/// What is missing, as words.
pub fn missing() -> Vec<&'static str> {
    PARTS.iter().filter(|p| !have(p.1)).map(|p| p.0).collect()
}

fn rgb(hex: &str) -> String {
    format!("rgb({})", hex.trim_start_matches('#'))
}

fn rgba(hex: &str, a: u8) -> String {
    format!("rgba({}{:02x})", hex.trim_start_matches('#'), a)
}

/// "Super+Shift+Left" → ("SUPER SHIFT", "Left"); None for a key Hyprland cannot bind.
pub fn key(spec: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = spec.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    let (last, mods) = parts.split_last()?;
    let mut m = vec![];
    for p in mods {
        m.push(match p.to_lowercase().as_str() {
            "super" | "mod4" | "win" => "SUPER",
            "shift" => "SHIFT",
            "ctrl" | "control" => "CTRL",
            "alt" | "mod1" => "ALT",
            _ => return None,
        });
    }
    let k = match *last {
        "Super" => return Some((String::new(), "SUPER_L".into())),
        "Enter" => "Return".to_string(),
        "Esc" => "Escape".to_string(),
        k if k.chars().count() == 1 => k.to_uppercase(),
        k => k.to_string(),
    };
    Some((m.join(" "), k))
}

/// A Samurai action as a Hyprland dispatcher and its argument; None where Hyprland has nothing
/// like it (snapping to corners, the hidden taskbar).
pub fn dispatch(action: &str) -> Option<(&'static str, String)> {
    let files = store::get_str("apps.files");
    let browser = store::get_str("apps.browser");
    let launch = |id: &str, fallback: &str| {
        let id = id.trim().trim_end_matches(".desktop");
        if id.is_empty() { fallback.to_string() } else { format!("gtk-launch {id}") }
    };
    let dir = |d: &str| match d { "left" => "l", "right" => "r", "up" => "u", _ => "d" }.to_string();
    let a: Vec<&str> = action.split_whitespace().collect();
    Some(match a.as_slice() {
        ["overview"] => ("exec", format!("pkill -x fuzzel || fuzzel --config {}", dir_s("fuzzel.ini"))),
        ["terminal"] => ("exec", "alacritty".into()),
        ["files"] => ("exec", launch(&files, "xdg-open ~")),
        ["browser"] => ("exec", launch(&browser, "xdg-open https://")),
        ["settings"] => ("exec", "samurai-settings".into()),
        ["close"] => ("killactive", String::new()),
        ["fullscreen"] => ("fullscreen", "0".into()),
        ["fill"] | ["restore"] => ("fullscreen", "1".into()),
        ["float"] => ("togglefloating", String::new()),
        ["center"] => ("centerwindow", String::new()),
        ["focus", d @ ("left" | "right" | "up" | "down")] => ("movefocus", dir(d)),
        ["move", d @ ("left" | "right" | "up" | "down")] => ("movewindow", dir(d)),
        ["monitor", "next"] => ("movewindow", "mon:+1".into()),
        ["monitor", "prev"] => ("movewindow", "mon:-1".into()),
        ["focus", "monitor", "next"] => ("focusmonitor", "+1".into()),
        ["focus", "monitor", "prev"] => ("focusmonitor", "-1".into()),
        ["tuck"] => ("movetoworkspacesilent", "special:tucked".into()),
        ["untuck"] => ("togglespecialworkspace", "tucked".into()),
        ["lock"] => ("exec", format!("pidof hyprlock || hyprlock --config {}", dir_s("hyprlock.conf"))),
        ["ground"] => ("exec", "samurai ground toggle".into()),
        ["screenshot", "area"] => ("exec", "grim -g \"$(slurp)\" - | wl-copy".into()),
        ["screenshot", "screen"] => ("exec", "grim - | wl-copy".into()),
        ["screenshot", "window"] => ("exec", "grim -g \"$(hyprctl activewindow -j | jq -r '\"\\(.at[0]),\\(.at[1]) \\(.size[0])x\\(.size[1])\"')\" - | wl-copy".into()),
        ["volume", "up"] => ("exec", "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+".into()),
        ["volume", "down"] => ("exec", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-".into()),
        ["volume", "mute"] => ("exec", "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle".into()),
        ["brightness", "up"] => ("exec", "brightnessctl set 5%+".into()),
        ["brightness", "down"] => ("exec", "brightnessctl set 5%-".into()),
        ["quit"] => ("exit", String::new()),
        _ => return None,
    })
}

fn dir_s(name: &str) -> String {
    dir().join(name).to_string_lossy().to_string()
}

/// Samurai's own keys (the same list the window manager starts from), then the chosen set,
/// then your own lines.
fn keys() -> Vec<(String, String)> {
    let mut k: std::collections::BTreeMap<String, String> = DEFAULT_KEYS.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    if store::get_str("keys.preset") == "hyprland" {
        for (a, b) in x11::HYPRLAND_KEYS {
            k.insert(a.to_string(), b.to_string());
        }
    }
    for r in store::get_list("keys.custom") {
        let (a, b) = (r["keys"].as_str().unwrap_or("").trim(), r["action"].as_str().unwrap_or("").trim());
        if !a.is_empty() && !b.is_empty() {
            k.insert(a.to_string(), b.to_string());
        }
    }
    k.into_iter().collect()
}

/// A copy of wm/src/config.rs default_keys (the CLI does not link the window manager).
const DEFAULT_KEYS: &[(&str, &str)] = &[
    ("Super", "overview"), ("Super+Tab", "overview"), ("Super+Return", "terminal"), ("Super+E", "files"),
    ("Super+B", "browser"), ("Super+comma", "settings"), ("Super+Q", "close"), ("Super+F", "fullscreen"),
    ("Super+Up", "fill"), ("Super+Down", "restore"), ("Super+Left", "move left"), ("Super+Right", "move right"),
    ("Super+Alt+Left", "focus left"), ("Super+Alt+Right", "focus right"), ("Super+Alt+Up", "focus up"),
    ("Super+Alt+Down", "focus down"), ("Super+V", "float"), ("Super+Ctrl+Right", "monitor next"),
    ("Super+Ctrl+Left", "monitor prev"), ("Super+Ctrl+Down", "focus monitor next"), ("Super+Ctrl+Up", "focus monitor prev"),
    ("Super+C", "center"), ("Super+M", "tuck"), ("Super+Shift+M", "untuck"), ("Super+L", "lock"),
    ("Super+Shift+B", "ground"), ("Super+Shift+S", "screenshot area"), ("Print", "screenshot screen"),
    ("Alt+Print", "screenshot window"), ("XF86AudioRaiseVolume", "volume up"), ("XF86AudioLowerVolume", "volume down"),
    ("XF86AudioMute", "volume mute"), ("XF86MonBrightnessUp", "brightness up"), ("XF86MonBrightnessDown", "brightness down"),
    ("Super+Shift+Escape", "quit"),
];

pub fn hyprland_conf(p: &Pal) -> String {
    let scale = store::get_f64("look.scale");
    let corners = if store::get_str("look.corners") == "square" { 0 } else { 14 };
    let gap = store::get_f64("windows.gap").max(0.0) as i64;
    let motion = store::get_str("look.motion") != "off";
    let cursor = store::get_str("mouse.cursor").parse::<f64>().unwrap_or(24.0);
    let mut c = format!("# {MARK} (samurai hyprland write). Changes here are replaced; put your own lines in\n# ~/.config/hypr/samurai-local.conf (it is read last and never changed).\n\n");
    // one size for every screen; Hyprland picks the nearest size it can draw sharply
    c += &format!("monitor = , preferred, auto, {scale:.2}\n\n");
    c += "# apps draw at the screen's size themselves (Wayland); older X11 apps are enlarged by Hyprland\n";
    c += "env = XDG_CURRENT_DESKTOP,Samurai:Hyprland\nenv = XDG_SESSION_DESKTOP,samurai-hyprland\nenv = XDG_SESSION_TYPE,wayland\n";
    c += "env = QT_QPA_PLATFORM,wayland;xcb\nenv = QT_QPA_PLATFORMTHEME,qt6ct\nenv = QT_ENABLE_HIGHDPI_SCALING,1\nenv = QT_SCALE_FACTOR_ROUNDING_POLICY,PassThrough\n";
    c += "env = GDK_BACKEND,wayland,x11\nenv = SDL_VIDEODRIVER,wayland,x11\nenv = ELECTRON_OZONE_PLATFORM_HINT,auto\nenv = MOZ_ENABLE_WAYLAND,1\n";
    c += &format!("env = XCURSOR_THEME,Adwaita\nenv = XCURSOR_SIZE,{}\nenv = HYPRCURSOR_SIZE,{}\n\n", cursor as i64, cursor as i64);
    c += "xwayland {\n    force_zero_scaling = false\n}\n\n";
    c += &format!("general {{\n    gaps_in = {}\n    gaps_out = {}\n    border_size = 1\n    col.active_border = {}\n    col.inactive_border = {}\n    layout = dwindle\n    resize_on_border = true\n}}\n\n",
        (gap / 2).max(2), gap.max(4), rgb(p.accent), rgb(p.line));
    c += &format!("decoration {{\n    rounding = {corners}\n    active_opacity = {:.2}\n    inactive_opacity = {:.2}\n    shadow {{\n        enabled = true\n        range = 28\n        render_power = 3\n        color = {}\n    }}\n    blur {{\n        enabled = false\n    }}\n}}\n\n",
        store::get_f64("windows.opacity_active").clamp(0.3, 1.0), store::get_f64("windows.opacity_inactive").clamp(0.3, 1.0), rgba("#000000", if p.night { 0x88 } else { 0x30 }));
    c += &format!("animations {{\n    enabled = {}\n    bezier = ease, 0.2, 0.8, 0.2, 1\n    animation = windows, 1, 3, ease, popin 92%\n    animation = fade, 1, 3, ease\n    animation = workspaces, 1, 3, ease, slide\n}}\n\n", if motion { "true" } else { "false" });
    c += "dwindle {\n    preserve_split = true\n}\n\n";
    c += &format!("misc {{\n    disable_hyprland_logo = true\n    disable_splash_rendering = true\n    background_color = {}\n    focus_on_activate = true\n}}\n\n", rgb(p.ground));
    let layout = store::get_str("kbd.layout");
    c += &format!("input {{\n    kb_layout = {}\n    follow_mouse = 1\n    touchpad {{\n        natural_scroll = {}\n        tap-to-click = true\n    }}\n}}\n\n",
        if layout.trim().is_empty() { "us".to_string() } else { layout.trim().to_string() }, store::get_bool("touch.natural"));
    // what runs with the session
    c += &format!("exec-once = samurai hyprland start\nexec-once = waybar -c {} -s {}\n", dir_s("waybar.jsonc"), dir_s("waybar.css"));
    c += &format!("exec-once = hypridle -c {}\n\n", dir_s("hypridle.conf"));
    c += "# keys: Samurai's own (Settings › Keyboard), then workspaces (hidden from the lists)\n";
    let mut seen = std::collections::HashSet::new();
    for (k, a) in keys() {
        let (Some((mods, key)), Some((d, arg))) = (key(&k), dispatch(&a)) else { continue };
        if !seen.insert((mods.clone(), key.clone())) {
            continue;
        }
        if key == "SUPER_L" {
            // Super alone, on release, so Super+key still works
            c += &format!("bindr = SUPER, SUPER_L, {d}, {arg}\n");
        } else if a.starts_with("volume") || a.starts_with("brightness") {
            c += &format!("bindel = {mods}, {key}, {d}, {arg}\n");
        } else {
            c += &format!("bind = {mods}, {key}, {d}, {arg}\n");
        }
    }
    for n in 1..=9 {
        c += &format!("bind = SUPER, {n}, workspace, {n}\nbind = SUPER SHIFT, {n}, movetoworkspace, {n}\n");
    }
    c += "bindm = SUPER, mouse:272, movewindow\nbindm = SUPER, mouse:273, resizewindow\n\n";
    c += "source = ~/.config/hypr/samurai-local.conf\n";
    c
}

/// The bar: the Sakura pill, the same as the X11 session's top bar.
pub fn waybar(p: &Pal) -> (String, String) {
    let conf = format!(r#"// {MARK}. Changes here are replaced.
{{
  "layer": "top",
  "position": "top",
  "margin-top": 10,
  "margin-left": 14,
  "margin-right": 14,
  "height": 40,
  "spacing": 6,
  "modules-left": ["custom/mark", "hyprland/window"],
  "modules-right": ["network", "pulseaudio", "battery", "clock"],
  "custom/mark": {{ "format": "侍", "tooltip": false, "on-click": "{fuzzel}" }},
  "hyprland/window": {{ "max-length": 48, "separate-outputs": true }},
  "network": {{ "format-wifi": "{{essid}}", "format-ethernet": "Wired", "format-disconnected": "Offline", "tooltip": false, "on-click": "samurai-settings --page network" }},
  "pulseaudio": {{ "format": "Sound {{volume}}%", "format-muted": "Muted", "on-click": "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle" }},
  "battery": {{ "format": "{{capacity}}%", "format-charging": "{{capacity}}% ⚡", "states": {{ "low": 15 }} }},
  "clock": {{ "format": "<span alpha='70%'>{{:%Y.%m.%d %a}}</span>  <b>{{:%H:%M}}</b>", "tooltip": false }}
}}
"#, fuzzel = format!("pkill -x fuzzel || fuzzel --config {}", dir_s("fuzzel.ini")));
    let ui = store::get_str("look.ui_font");
    let mono = store::get_str("look.mono_font");
    let css = format!(r#"/* {MARK}: the Sakura look. */
* {{ font-family: "{ui}", "Noto Sans CJK JP", sans-serif; font-size: 13px; min-height: 0; border: none; }}
window#waybar {{ background: transparent; color: {text}; }}
.modules-left, .modules-right {{ background: alpha({surf}, 0.88); border: 1px solid {line}; border-radius: 999px; padding: 3px 6px; }}
#custom-mark {{ background: {acc}; color: {on}; border-radius: 999px; min-width: 30px; font-weight: 600; margin-right: 6px; padding: 0 2px; }}
#window {{ background: alpha({acc}, 0.16); color: {acc}; border-radius: 999px; padding: 0 14px; font-weight: 600; }}
window#waybar.empty #window {{ background: transparent; }}
#network, #pulseaudio, #battery {{ color: {dim}; padding: 0 10px; }}
#battery.low {{ color: {acc}; }}
#clock {{ background: {chrome}; border-radius: 999px; padding: 0 14px; font-family: "{mono}", monospace; font-size: 12px; }}
"#, text = p.text, surf = p.surf, line = p.line, acc = p.accent, on = p.on_accent, dim = p.dim,
        chrome = if p.night { "#1F2228" } else { "#F2EFE9" });
    (conf, css)
}

/// The app list (Super): fuzzel in the Sakura colours.
pub fn fuzzel(p: &Pal) -> String {
    let h = |s: &str| s.trim_start_matches('#').to_string();
    let ui = store::get_str("look.ui_font");
    format!("# {MARK}\n[main]\nfont={ui}:size=12\nprompt=\"検索  \"\nplaceholder=Type to find an app\nlines=12\nwidth=44\nhorizontal-pad=24\nvertical-pad=18\ninner-pad=10\nletter-spacing=0\nlayer=overlay\nterminal=alacritty -e\n\n[colors]\nbackground={}f2\ntext={}ff\nprompt={}ff\nplaceholder={}ff\ninput={}ff\nmatch={}ff\nselection={}ff\nselection-text={}ff\nselection-match={}ff\nborder={}ff\n\n[border]\nwidth=2\nradius=16\n",
        h(p.surf), h(p.text), h(p.accent), h(p.dim), h(p.text), h(p.accent), h(p.accent), h(p.on_accent), h(p.on_accent), h(p.accent))
}

/// The lock screen: the same face as the login screen.
pub fn hyprlock(p: &Pal) -> String {
    let h = |s: &str| s.trim_start_matches('#').to_string();
    let ui = store::get_str("look.ui_font");
    let wall = crate::seal::dir().join(if p.night { "wall-night.svg" } else { "wall-paper.svg" });
    let mut c = format!("# {MARK}\ngeneral {{\n    hide_cursor = true\n    grace = 0\n}}\n\n");
    c += &format!("background {{\n    monitor =\n    color = rgb({})\n", h(p.ground));
    if wall.exists() {
        c += &format!("    path = {}\n", wall.display());
    }
    c += "    blur_passes = 0\n}\n\n";
    c += &format!("label {{\n    monitor =\n    text = $TIME\n    color = rgb({})\n    font_size = 72\n    font_family = {ui} SemiBold\n    position = 64, 96\n    halign = left\n    valign = bottom\n}}\n\n", h(p.text));
    c += &format!("label {{\n    monitor =\n    text = cmd[update:60000] date +'%Y.%m.%d %A'\n    color = rgb({})\n    font_size = 16\n    font_family = {ui}\n    position = 66, 64\n    halign = left\n    valign = bottom\n}}\n\n", h(p.dim));
    c += &format!("label {{\n    monitor =\n    text = $USER\n    color = rgb({})\n    font_size = 22\n    font_family = {ui} SemiBold\n    position = 0, 70\n    halign = center\n    valign = center\n}}\n\n", h(p.text));
    c += &format!("input-field {{\n    monitor =\n    size = 360, 52\n    rounding = -1\n    outline_thickness = 2\n    outer_color = rgb({acc})\n    inner_color = rgb({surf})\n    font_color = rgb({text})\n    check_color = rgb({acc})\n    fail_color = rgb(E2857A)\n    placeholder_text = パスワード Password\n    fail_text = That was not the password\n    dots_rounding = -1\n    fade_on_empty = false\n    position = 0, 0\n    halign = center\n    valign = center\n}}\n",
        acc = h(p.accent), surf = h(p.surf), text = h(p.text));
    c
}

pub fn hypridle() -> String {
    let lock = store::get_f64("idle.lock") as i64;
    let off = store::get_f64("idle.screen_off") as i64;
    let sleep = store::get_f64("idle.suspend") as i64;
    let lock_cmd = format!("pidof hyprlock || hyprlock --config {}", dir_s("hyprlock.conf"));
    let mut c = format!("# {MARK} (Settings › Power and lock)\ngeneral {{\n    lock_cmd = {lock_cmd}\n    before_sleep_cmd = {}\n    after_sleep_cmd = hyprctl dispatch dpms on\n}}\n",
        if store::get_bool("lock.on_sleep") { "loginctl lock-session" } else { "true" });
    // the times are in minutes in Settings; 0 means never
    for (min, on, back) in [(lock, "loginctl lock-session".to_string(), None), (off, "hyprctl dispatch dpms off".into(), Some("hyprctl dispatch dpms on")), (sleep, "systemctl suspend".into(), None)] {
        if min > 0 {
            c += &format!("\nlistener {{\n    timeout = {}\n    on-timeout = {on}\n", min * 60);
            if let Some(b) = back {
                c += &format!("    on-resume = {b}\n");
            }
            c += "}\n";
        }
    }
    c
}

/// Write every file. Returns what it did, for the log.
pub fn write() -> Result<Vec<String>, String> {
    let p = palette(x11::ground() == "night");
    let d = dir();
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let (bar, css) = waybar(&p);
    let mut log = vec![];
    for (name, text) in [("hyprland.conf", hyprland_conf(&p)), ("waybar.jsonc", bar), ("waybar.css", css), ("fuzzel.ini", fuzzel(&p)), ("hyprlock.conf", hyprlock(&p)), ("hypridle.conf", hypridle())] {
        store::atomic_write(&d.join(name), &text).map_err(|e| format!("{name}: {e}"))?;
        log.push(format!("wrote {}", d.join(name).display()));
    }
    let local = store::home().join(".config/hypr/samurai-local.conf");
    if !local.exists() {
        if let Some(parent) = local.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = store::atomic_write(&local, "# your own Hyprland lines (Samurai never changes this file)\n");
    }
    if running() {
        run(&["hyprctl", "reload"]);
        // the bar reads its style once: start it again
        if x11::running("waybar") {
            run(&["pkill", "-SIGUSR2", "-x", "waybar"]);
        }
    }
    Ok(log)
}

/// Inside the session, once: the helpers (notices, password prompts, wallpaper, GTK theme for
/// older apps) and the first-login setup.
pub fn start() -> Result<Vec<String>, String> {
    let mut log = write()?;
    log.extend(x11::apply()?);
    let p = palette(x11::ground() == "night");
    if !x11::running("dunst") {
        x11::daemon(&["dunst"]);
    }
    if !x11::running("xsettingsd") && std::env::var_os("DISPLAY").is_some() {
        x11::daemon(&["xsettingsd", "-c", &store::home().join(".config/xsettingsd/xsettingsd.conf").to_string_lossy()]);
    }
    if have("swaybg") && !x11::running("swaybg") {
        let wall = crate::seal::dir().join(if p.night { "wall-night.svg" } else { "wall-paper.svg" });
        if wall.exists() {
            x11::daemon(&["swaybg", "-m", "fill", "-c", p.ground, "-i", &wall.to_string_lossy()]);
        } else {
            x11::daemon(&["swaybg", "-c", p.ground]);
        }
    }
    for agent in ["/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1", "/usr/lib/xfce-polkit/xfce-polkit", "/usr/bin/lxqt-policykit-agent"] {
        if std::path::Path::new(agent).exists() {
            x11::daemon(&[agent]);
            break;
        }
    }
    if !store::config_dir().join("first-setup.done").exists() && have("samurai-settings") {
        x11::daemon(&["samurai-settings", "--first-setup"]);
    }
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_become_hyprland_keys() {
        assert_eq!(key("Super+Shift+Left"), Some(("SUPER SHIFT".into(), "Left".into())));
        assert_eq!(key("Super+q"), Some(("SUPER".into(), "Q".into())));
        assert_eq!(key("Super"), Some((String::new(), "SUPER_L".into())));
        assert_eq!(key("Print"), Some((String::new(), "Print".into())));
        assert_eq!(key("Hyper+X"), None);
    }

    #[test]
    fn every_default_key_has_a_meaning_or_is_known_missing() {
        for (k, a) in DEFAULT_KEYS {
            assert!(key(k).is_some(), "{k}");
            assert!(dispatch(a).is_some(), "{a} has no Hyprland dispatcher");
        }
    }

    #[test]
    fn the_config_has_no_repeated_key() {
        let p = palette(true);
        let c = hyprland_conf(&p);
        let mut seen = std::collections::HashSet::new();
        for l in c.lines().filter(|l| l.starts_with("bind")) {
            let k: Vec<&str> = l.splitn(2, '=').nth(1).unwrap().split(',').take(2).map(|s| s.trim()).collect();
            assert!(seen.insert(k.join("+")), "repeated: {l}");
        }
        assert!(c.contains("monitor = , preferred, auto,"));
        assert!(c.contains("bindr = SUPER, SUPER_L, exec"));
    }
}
