//! Keybinds: the built-in catalogue, the user's own binds, and the conflict check between them.
//!
//! Rule: a custom bind that lands on the keys of a built-in one is never applied silently. It is
//! reported as a conflict and left out of the generated config until the user decides: mark the
//! row "replace" (the built-in loses those keys), move the built-in, or pick other keys.
use crate::store;
use serde_json::{json, Value};

pub struct Builtin {
    pub id: &'static str,
    pub keys: &'static str,
    pub action: &'static str,
    pub arg: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub flags: &'static str,
}

const fn b(id: &'static str, keys: &'static str, action: &'static str, arg: &'static str, name: &'static str, group: &'static str, flags: &'static str) -> Builtin {
    Builtin { id, keys, action, arg, name, group, flags }
}

/// MOD is replaced by the main modifier (binds.mod).
pub const BUILTIN: &[Builtin] = &[
    b("frame.start", "MOD + SPACE", "frame", "start", "Start", "frame", ""),
    b("frame.quick", "MOD + A", "frame", "quick", "Quick settings", "frame", ""),
    b("frame.widgets", "MOD + W", "frame", "widgets", "Widgets", "frame", ""),
    b("frame.settings", "MOD + COMMA", "frame", "settings", "Settings", "frame", ""),
    b("frame.keys", "MOD + SLASH", "frame", "keys", "Keybind legend", "frame", ""),
    b("frame.base", "MOD + SHIFT + B", "frame", "base", "Switch black / white", "frame", ""),
    b("frame.incognito", "MOD + SHIFT + I", "frame", "incognito", "Incognito on / off", "frame", ""),
    b("frame.lock", "MOD + ALT + L", "frame", "lock", "Lock", "frame", ""),
    b("frame.power", "MOD + SHIFT + E", "frame", "power", "Power menu", "frame", ""),
    b("app.terminal", "MOD + RETURN", "spawn", "$terminal", "Terminal", "apps", ""),
    b("app.files", "MOD + E", "spawn", "$files", "Files", "apps", ""),
    b("app.browser", "MOD + B", "spawn", "$browser", "Browser", "apps", ""),
    b("win.close", "MOD + Q", "window", "close", "Close window", "windows", ""),
    b("win.float", "MOD + V", "window", "float", "Float / tile", "windows", ""),
    b("win.fullscreen", "MOD + F", "window", "fullscreen", "Fullscreen", "windows", ""),
    b("win.maximize", "MOD + UP", "window", "maximize", "Maximize / restore", "windows", ""),
    b("win.minimize", "MOD + M", "window", "minimize", "Minimize to the rail", "windows", ""),
    b("win.restore", "MOD + SHIFT + M", "window", "restore", "Bring the last minimized window back", "windows", ""),
    b("win.next", "MOD + TAB", "window", "next", "Next window", "windows", ""),
    b("win.prev", "MOD + SHIFT + TAB", "window", "prev", "Previous window", "windows", ""),
    b("win.overview", "MOD + O", "niri", "toggle-overview", "Overview of every workspace", "windows", ""),
    b("win.width", "MOD + R", "niri", "switch-preset-column-width", "Next window width", "windows", ""),
    b("win.shrink", "MOD + MINUS", "niri", "set-column-width \"-10%\"", "Narrower", "windows", "repeat"),
    b("win.grow", "MOD + EQUAL", "niri", "set-column-width \"+10%\"", "Wider", "windows", "repeat"),
    b("win.stack_left", "MOD + BRACKETLEFT", "niri", "consume-or-expel-window-left", "Stack with / split from the left column", "windows", ""),
    b("win.stack_right", "MOD + BRACKETRIGHT", "niri", "consume-or-expel-window-right", "Stack with / split from the right column", "windows", ""),
    b("focus.left", "MOD + H", "focus", "left", "Focus left", "windows", ""),
    b("focus.right", "MOD + L", "focus", "right", "Focus right", "windows", ""),
    b("focus.up", "MOD + K", "focus", "up", "Focus up", "windows", ""),
    b("focus.down", "MOD + J", "focus", "down", "Focus down", "windows", ""),
    b("move.left", "MOD + SHIFT + H", "move", "left", "Move window left", "windows", ""),
    b("move.right", "MOD + SHIFT + L", "move", "right", "Move window right", "windows", ""),
    b("move.up", "MOD + SHIFT + K", "move", "up", "Move window up", "windows", ""),
    b("move.down", "MOD + SHIFT + J", "move", "down", "Move window down", "windows", ""),
    b("ws.next", "MOD + CTRL + RIGHT", "workspace", "e+1", "Next workspace", "workspaces", ""),
    b("ws.prev", "MOD + CTRL + LEFT", "workspace", "e-1", "Previous workspace", "workspaces", ""),
    b("ws.move_next", "MOD + CTRL + SHIFT + RIGHT", "window_to", "e+1", "Window to the next workspace", "workspaces", ""),
    b("ws.move_prev", "MOD + CTRL + SHIFT + LEFT", "window_to", "e-1", "Window to the previous workspace", "workspaces", ""),
    b("shot.area", "MOD + SHIFT + S", "spawn", "samurai shot area", "Screenshot an area", "capture", ""),
    b("shot.screen", "PRINT", "spawn", "samurai shot screen", "Screenshot the screen", "capture", ""),
    b("shot.record", "MOD + SHIFT + R", "spawn", "samurai record toggle", "Start / stop recording", "capture", ""),
    b("media.up", "XF86AudioRaiseVolume", "spawn", "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+", "Volume up", "media", "locked,repeat"),
    b("media.down", "XF86AudioLowerVolume", "spawn", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-", "Volume down", "media", "locked,repeat"),
    b("media.mute", "XF86AudioMute", "spawn", "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle", "Mute", "media", "locked"),
    b("media.mic", "XF86AudioMicMute", "spawn", "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle", "Mute the microphone", "media", "locked"),
    b("media.bright_up", "XF86MonBrightnessUp", "spawn", "brightnessctl -e4 -n2 set 5%+", "Brighter", "media", "locked,repeat"),
    b("media.bright_down", "XF86MonBrightnessDown", "spawn", "brightnessctl -e4 -n2 set 5%-", "Dimmer", "media", "locked,repeat"),
    b("media.play", "XF86AudioPlay", "spawn", "playerctl play-pause", "Play / pause", "media", "locked"),
    b("media.next", "XF86AudioNext", "spawn", "playerctl next", "Next track", "media", "locked"),
    b("media.prev", "XF86AudioPrev", "spawn", "playerctl previous", "Previous track", "media", "locked"),
];

const MODS: [&str; 4] = ["SUPER", "CTRL", "ALT", "SHIFT"];

/// "mod+shift+t", "Super + Shift + T", "MOD + SHIFT + T" -> "SUPER + SHIFT + T" (modifiers in a
/// fixed order, so two spellings of the same chord compare equal). None: not a usable chord.
pub fn canon(keys: &str, main_mod: &str) -> Option<String> {
    let mut mods: Vec<&str> = vec![];
    let mut key: Option<String> = None;
    for part in keys.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        let up = part.to_ascii_uppercase();
        let m = match up.as_str() {
            "MOD" | "MAINMOD" => Some(main_mod),
            "SUPER" | "WIN" | "META" | "LOGO" => Some("SUPER"),
            "CTRL" | "CONTROL" => Some("CTRL"),
            "ALT" => Some("ALT"),
            "SHIFT" => Some("SHIFT"),
            _ => None,
        };
        match m {
            Some(m) => {
                if !mods.contains(&m) {
                    mods.push(m);
                }
            }
            None => {
                if key.is_some() || !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
                    return None;
                }
                // XF86… and mouse:272 keep their case; plain keys are upper-cased
                key = Some(if part.starts_with("XF86") || part.contains(':') || part.starts_with("mouse_") { part.to_string() } else { up });
            }
        }
    }
    let key = key?;
    let mut out: Vec<String> = MODS.iter().filter(|m| mods.contains(m)).map(|m| m.to_string()).collect();
    out.push(key);
    Some(out.join(" + "))
}

pub fn main_mod() -> String {
    let m = store::get_str("binds.mod");
    if m.is_empty() { "SUPER".into() } else { m }
}

/// One bind as it will be (or will not be) written.
pub struct Bind {
    pub id: String,
    pub keys: String,
    pub action: String,
    pub arg: String,
    pub name: String,
    pub group: String,
    pub flags: String,
    pub custom: bool,
    /// why this bind is not applied (a conflict), if it is not
    pub held: Option<String>,
}

fn field(row: &Value, k: &str) -> String {
    row[k].as_str().unwrap_or("").trim().to_string()
}

/// Everything, resolved: built-ins (with the user's moves and switch-offs), workspace numbers,
/// then the user's own binds with conflicts marked.
pub fn resolve() -> Vec<Bind> {
    let mm = main_mod();
    let disabled: Vec<String> = store::get_list("binds.disabled").iter().map(|r| field(r, "id")).collect();
    let moved: Vec<(String, String)> = store::get_list("binds.moved").iter().map(|r| (field(r, "id"), field(r, "keys"))).collect();
    let custom = store::get_list("binds.custom");
    // a custom row marked "replace" takes its keys from whichever built-in holds them
    let replacing: Vec<String> = custom.iter().filter(|r| field(r, "flags").split(',').any(|f| f.trim() == "replace")).filter_map(|r| canon(&field(r, "keys"), &mm)).collect();

    let mut out: Vec<Bind> = vec![];
    let mut builtin = |id: String, keys: &str, action: &str, arg: String, name: String, group: &str, flags: &str| {
        if disabled.contains(&id) {
            return;
        }
        let keys = moved.iter().find(|(i, _)| *i == id).map(|(_, k)| k.as_str()).unwrap_or(keys);
        let Some(k) = canon(keys, &mm) else { return };
        if replacing.contains(&k) {
            return;
        }
        // two built-ins can land on the same keys (a moved bind, or Alt as the main modifier
        // folding MOD + ALT + L onto MOD + L): the earlier one keeps them, the later is held
        let held = out.iter().find(|o| o.keys == k && o.held.is_none()).map(|o| format!("{k} already does \"{}\" (built-in {}); move one of them", o.name, o.id));
        out.push(Bind { id, keys: k, action: action.into(), arg, name, group: group.into(), flags: flags.into(), custom: false, held });
    };
    for x in BUILTIN {
        builtin(x.id.into(), x.keys, x.action, x.arg.into(), x.name.into(), x.group, x.flags);
    }
    for n in 1..=10u32 {
        let key = n % 10;
        builtin(format!("ws.{n}"), &format!("MOD + {key}"), "workspace", n.to_string(), format!("Workspace {n}"), "workspaces", "");
        builtin(format!("ws.move_{n}"), &format!("MOD + SHIFT + {key}"), "window_to", n.to_string(), format!("Window to workspace {n}"), "workspaces", "");
    }
    for (n, r) in custom.iter().enumerate() {
        let (keys, action) = (field(r, "keys"), field(r, "action"));
        let mut bind = Bind { id: format!("custom.{n}"), keys: keys.clone(), action, arg: field(r, "arg"), name: field(r, "name"), group: "custom".into(), flags: field(r, "flags"), custom: true, held: None };
        match canon(&keys, &mm) {
            None => bind.held = Some(format!("\"{keys}\" is not a key combination")),
            Some(k) => {
                if let Some(o) = out.iter().find(|o| o.keys == k && o.held.is_none()) {
                    bind.held = Some(if o.custom { format!("the same keys as your bind \"{}\"", o.name) } else { format!("{k} already does \"{}\" (built-in {})", o.name, o.id) });
                }
                if let Err(e) = kdl_for(&bind.action, &bind.arg).and_then(|a| kdl_keys(&bind.keys).map(|_| a).ok_or("niri has no such key or button".to_string())) {
                    bind.held = Some(e);
                }
                bind.keys = k;
            }
        }
        out.push(bind);
    }
    out
}

/// What would happen if `keys` were bound now: the binds already on those keys.
pub fn check(keys: &str) -> Value {
    let mm = main_mod();
    let Some(k) = canon(keys, &mm) else { return json!({ "ok": false, "error": "not a key combination" }) };
    let hits: Vec<Value> = resolve().iter().filter(|b| b.keys == k && b.held.is_none()).map(|b| json!({ "id": b.id, "name": b.name, "custom": b.custom })).collect();
    json!({ "ok": hits.is_empty(), "keys": k, "conflicts": hits })
}

pub fn list_json() -> Value {
    Value::Array(resolve().iter().map(|b| json!({ "id": b.id, "keys": b.keys, "action": b.action, "arg": b.arg, "name": b.name, "group": b.group, "flags": b.flags, "custom": b.custom, "held": b.held })).collect())
}

pub fn kstr(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            c if c.is_control() => {}
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// "SUPER + SHIFT + T" -> "Super+Shift+T"; "mouse:272" -> "MouseLeft". None: niri has no such trigger.
pub fn kdl_keys(keys: &str) -> Option<String> {
    let mut out: Vec<String> = vec![];
    for part in keys.split('+').map(str::trim) {
        out.push(match part {
            "SUPER" => "Super".into(),
            "CTRL" => "Ctrl".into(),
            "ALT" => "Alt".into(),
            "SHIFT" => "Shift".into(),
            "mouse:272" => "MouseLeft".into(),
            "mouse:273" => "MouseRight".into(),
            "mouse:274" => "MouseMiddle".into(),
            "mouse:275" => "MouseBack".into(),
            "mouse:276" => "MouseForward".into(),
            k if k.contains(':') => return None,
            k => {
                // niri reads key names without regard to case; a few of ours are spelled differently
                let up = k.to_ascii_uppercase();
                match up.as_str() {
                    "MOUSELEFT" => "MouseLeft".into(),
                    "MOUSERIGHT" => "MouseRight".into(),
                    "MOUSEMIDDLE" => "MouseMiddle".into(),
                    "MOUSEBACK" => "MouseBack".into(),
                    "MOUSEFORWARD" => "MouseForward".into(),
                    "WHEELSCROLLUP" => "WheelScrollUp".into(),
                    "WHEELSCROLLDOWN" => "WheelScrollDown".into(),
                    "ENTER" => "Return".into(),
                    "ESC" => "Escape".into(),
                    "PGUP" => "Page_Up".into(),
                    "PGDN" | "PGDOWN" => "Page_Down".into(),
                    "DEL" => "Delete".into(),
                    _ => k.to_string(),
                }
            }
        });
    }
    (!out.is_empty()).then(|| out.join("+"))
}

/// The niri action for a bind, as it is written inside `{ … }` (niri 25.11+).
pub fn kdl_for(action: &str, arg: &str) -> Result<String, String> {
    let ws = |arg: &str| -> Result<String, String> {
        if let Ok(n) = arg.parse::<u32>() {
            return if (1..=99).contains(&n) { Ok(n.to_string()) } else { Err(format!("workspace {n} is out of range")) };
        }
        let ok = !arg.is_empty() && arg.len() <= 40 && arg.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c));
        ok.then(|| kstr(arg)).ok_or(format!("\"{arg}\" is not a workspace"))
    };
    Ok(match action {
        "spawn" => {
            if arg.is_empty() {
                return Err("spawn needs a command".into());
            }
            match arg {
                "$terminal" => "spawn \"samurai\" \"open\" \"terminal\"".into(),
                "$files" => "spawn \"samurai\" \"open\" \"files\"".into(),
                "$browser" => "spawn \"samurai\" \"open\" \"browser\"".into(),
                a => format!("spawn-sh {}", kstr(a)),
            }
        }
        "frame" => {
            let ok = ["start", "quick", "widgets", "settings", "keys", "base", "incognito", "lock", "power"].contains(&arg)
                || arg.strip_prefix("settings:").is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase()));
            if !ok {
                return Err(format!("\"{arg}\" is not a frame panel"));
            }
            format!("spawn \"samurai\" \"shell\" {}", kstr(arg))
        }
        "window" => match arg {
            "close" => "close-window".into(),
            "float" => "toggle-window-floating".into(),
            "fullscreen" => "fullscreen-window".into(),
            "maximize" => "maximize-column".into(),
            "center" => "center-column".into(),
            "next" => "focus-column-right-or-first".into(),
            "prev" => "focus-column-left-or-last".into(),
            "minimize" => "spawn \"samurai\" \"win\" \"minimize\"".into(),
            "restore" => "spawn \"samurai\" \"win\" \"restore\"".into(),
            a => return Err(format!("\"{a}\" is not a window action")),
        },
        "focus" => match arg {
            "left" => "focus-column-left".into(),
            "right" => "focus-column-right".into(),
            "up" => "focus-window-or-workspace-up".into(),
            "down" => "focus-window-or-workspace-down".into(),
            a => return Err(format!("\"{a}\" is not a direction")),
        },
        "move" => match arg {
            "left" => "move-column-left".into(),
            "right" => "move-column-right".into(),
            "up" => "move-window-up-or-to-workspace-up".into(),
            "down" => "move-window-down-or-to-workspace-down".into(),
            a => return Err(format!("\"{a}\" is not a direction")),
        },
        "workspace" => match arg {
            "e+1" | "next" | "down" => "focus-workspace-down".into(),
            "e-1" | "prev" | "up" => "focus-workspace-up".into(),
            a => format!("focus-workspace {}", ws(a)?),
        },
        "window_to" => match arg {
            "e+1" | "next" | "down" => "move-column-to-workspace-down".into(),
            "e-1" | "prev" | "up" => "move-column-to-workspace-up".into(),
            a => format!("move-column-to-workspace {}", ws(a)?),
        },
        // one niri action written out by hand, e.g.  set-column-width "-10%"
        "niri" => {
            let a = arg.trim();
            let safe = a.starts_with(|c: char| c.is_ascii_lowercase())
                && a.len() <= 120
                && a.chars().all(|c| c.is_ascii_alphanumeric() || " -_\"%+.=#".contains(c))
                && a.matches('"').count() % 2 == 0
                && !a.starts_with("spawn");
            if !safe {
                return Err("a niri action is one action name with its arguments (use the spawn action to run a command)".into());
            }
            a.to_string()
        }
        a => return Err(format!("\"{a}\" is not an action")),
    })
}

/// bind properties from flags ("locked,repeat") and the name shown in niri's own hotkey overlay
pub fn kdl_props(flags: &str, name: &str) -> String {
    let has = |f: &str| flags.split(',').any(|x| x.trim() == f);
    let mut o = String::new();
    if !has("repeat") {
        o.push_str(" repeat=false");
    }
    if has("locked") {
        o.push_str(" allow-when-locked=true");
    }
    if !name.is_empty() {
        o.push_str(&format!(" hotkey-overlay-title={}", kstr(name)));
    }
    o
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn chords_compare_equal_however_they_are_typed() {
        assert_eq!(canon("mod+shift+t", "SUPER").unwrap(), "SUPER + SHIFT + T");
        assert_eq!(canon("Shift + Super + t", "SUPER").unwrap(), "SUPER + SHIFT + T");
        assert_eq!(canon("MOD + RETURN", "ALT").unwrap(), "ALT + RETURN");
        assert_eq!(canon("XF86AudioMute", "SUPER").unwrap(), "XF86AudioMute");
        assert_eq!(canon("MOD + mouse:276", "SUPER").unwrap(), "SUPER + mouse:276");
        assert!(canon("MOD + A + B", "SUPER").is_none(), "two keys");
        assert!(canon("MOD", "SUPER").is_none(), "no key");
        assert!(canon("MOD + \"); os.exit()", "SUPER").is_none());
    }

    #[test]
    fn every_builtin_has_a_niri_action_and_unique_keys() {
        let mut seen = std::collections::HashSet::new();
        for x in BUILTIN {
            assert!(kdl_for(x.action, x.arg).is_ok(), "{}", x.id);
            assert!(kdl_keys(&canon(x.keys, "SUPER").unwrap()).is_some(), "{}", x.id);
            assert!(seen.insert(canon(x.keys, "SUPER").unwrap()), "{} reuses keys", x.id);
        }
    }

    #[test]
    fn actions_cannot_smuggle_config() {
        assert_eq!(kdl_for("niri", "set-column-width \"-10%\"").unwrap(), "set-column-width \"-10%\"");
        for bad in ["close-window; } spawn \"x\"", "close-window }", "spawn \"x\"", "Quit", "a\nb", "set-column-width \"x"] {
            assert!(kdl_for("niri", bad).is_err(), "{bad}");
        }
        assert_eq!(kdl_for("spawn", "say \"hi\"").unwrap(), "spawn-sh \"say \\\"hi\\\"\"");
        assert!(kdl_for("workspace", "2\" } spawn \"x").is_err());
        assert!(kdl_for("frame", "settings:x\" \"y").is_err());
        assert!(kdl_for("focus", "sideways").is_err());
        assert_eq!(kdl_keys("SUPER + SHIFT + mouse:275").unwrap(), "Super+Shift+MouseBack");
        assert!(kdl_keys("SUPER + mouse:999").is_none());
    }
}
