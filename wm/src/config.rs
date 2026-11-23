//! ~/.config/samurai/wm.toml. Written by the Settings app (through `samurai apply`); every key has
//! a default here, so the WM runs with no file at all. Reloaded when the file changes.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SAMURAI_CONFIG") {
        return PathBuf::from(p);
    }
    let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".config"));
    base.join("samurai")
}

pub fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"))
}

/// Where a wallpaper is: the configured file, else this machine's one-time ink.
pub fn wallpaper(configured: &str, ground: &str) -> PathBuf {
    if configured.trim().is_empty() {
        config_dir().join("seal").join(format!("wall-{ground}.svg"))
    } else {
        expand(configured)
    }
}

pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") { home().join(rest) } else { PathBuf::from(p) }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Look {
    pub ground: String,
    /// Paper by day and night after dark, at these times (HH:MM).
    pub follow_sun: bool,
    pub sun_from: String,
    pub sun_to: String,
    pub accent: String,
    pub corners: f32,
    pub motion: String,
    pub scale: f32,
    pub wallpaper_night: String,
    pub wallpaper_paper: String,
    pub mark: String,
    pub show_clock: bool,
    pub show_mark: bool,
    pub ui_font: String,
    pub mono_font: String,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            ground: "night".into(),
            follow_sun: false,
            sun_from: "07:30".into(),
            sun_to: "18:00".into(),
            accent: "indigo".into(),
            corners: 12.0,
            motion: "gentle".into(),
            scale: 1.0,
            wallpaper_night: String::new(),
            wallpaper_paper: String::new(),
            mark: String::new(),
            show_clock: true,
            show_mark: true,
            ui_font: "Inter".into(),
            mono_font: "JetBrains Mono".into(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Windows {
    pub gap: i32,
    pub strip_follows_app: bool,
    pub focus: String,
    pub snap: bool,
    pub shake_to_tuck: bool,
    pub unredirect_fullscreen: bool,
    pub remember_places: bool,
}

impl Default for Windows {
    fn default() -> Self {
        Windows { gap: 8, strip_follows_app: true, focus: "click".into(), snap: true, shake_to_tuck: true, unredirect_fullscreen: true, remember_places: true }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Apps {
    pub terminal: String,
    pub files: String,
    pub browser: String,
    pub settings: String,
    pub pinned: Vec<String>,
}

impl Default for Apps {
    fn default() -> Self {
        Apps {
            terminal: "alacritty".into(),
            files: "thunar".into(),
            browser: "firefox".into(),
            settings: "samurai-settings".into(),
            pinned: ["Alacritty", "thunar", "firefox", "org.xfce.mousepad", "org.mozilla.Thunderbird", "mpv", "imv", "samurai-settings"]
                .iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Idle {
    pub lock: u32,
    pub screen_off: u32,
    pub suspend: u32,
}

impl Default for Idle {
    fn default() -> Self {
        Idle { lock: 10, screen_off: 15, suspend: 0 }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub look: Look,
    pub windows: Windows,
    pub apps: Apps,
    pub idle: Idle,
    pub keys: BTreeMap<String, String>,
    pub autostart: Vec<String>,
}

pub fn default_keys() -> BTreeMap<String, String> {
    [
        ("Super", "overview"),
        ("Super+Tab", "overview"),
        ("Alt+Tab", "focus next"),
        ("Alt+Shift+Tab", "focus prev"),
        ("Super+Return", "terminal"),
        ("Super+E", "files"),
        ("Super+B", "browser"),
        ("Super+comma", "settings"),
        ("Super+Q", "close"),
        ("Super+F", "fullscreen"),
        ("Super+Up", "fill"),
        ("Super+Down", "restore"),
        ("Super+Left", "snap left"),
        ("Super+Right", "snap right"),
        ("Super+Shift+Left", "snap top-left"),
        ("Super+Shift+Up", "snap top-right"),
        ("Super+Shift+Down", "snap bottom-left"),
        ("Super+Shift+Right", "snap bottom-right"),
        ("Super+C", "center"),
        ("Super+M", "tuck"),
        ("Super+Shift+M", "untuck"),
        ("Super+L", "lock"),
        ("Super+Shift+B", "ground"),
        ("Super+Shift+S", "screenshot area"),
        ("Print", "screenshot screen"),
        ("Alt+Print", "screenshot window"),
        ("XF86AudioRaiseVolume", "volume up"),
        ("XF86AudioLowerVolume", "volume down"),
        ("XF86AudioMute", "volume mute"),
        ("XF86MonBrightnessUp", "brightness up"),
        ("XF86MonBrightnessDown", "brightness down"),
        ("Super+Shift+Escape", "quit"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

impl Default for Config {
    fn default() -> Self {
        Config { look: Look::default(), windows: Windows::default(), apps: Apps::default(), idle: Idle::default(), keys: default_keys(), autostart: vec![] }
    }
}

pub fn path() -> PathBuf {
    config_dir().join("wm.toml")
}

/// Load the config. A broken file never stops the WM: the error is returned beside the defaults.
pub fn load() -> (Config, Option<String>) {
    let p = path();
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => return (Config::default(), None),
    };
    match toml::from_str::<Config>(&text) {
        Ok(mut c) => {
            // keys in the file are added to (and override) the defaults; "none" turns one off
            let mut keys = default_keys();
            for (k, v) in c.keys.iter() {
                keys.insert(k.clone(), v.clone());
            }
            keys.retain(|_, v| v != "none");
            c.keys = keys;
            c.look.scale = c.look.scale.clamp(0.75, 3.0);
            c.look.corners = c.look.corners.clamp(0.0, 24.0);
            (c, None)
        }
        Err(e) => (Config::default(), Some(format!("{}: {e}", p.display()))),
    }
}

pub fn mtime() -> Option<std::time::SystemTime> {
    std::fs::metadata(path()).and_then(|m| m.modified()).ok()
}
