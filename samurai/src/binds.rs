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
        out.push(Bind { id, keys: k, action: action.into(), arg, name, group: group.into(), flags: flags.into(), custom: false, held: None });
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
                if let Err(e) = lua_for(&bind.action, &bind.arg) {
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

pub fn lua_str(s: &str) -> String {
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

fn ws_arg(arg: &str) -> Result<String, String> {
    if let Ok(n) = arg.parse::<u32>() {
        return if (1..=99).contains(&n) { Ok(n.to_string()) } else { Err(format!("workspace {n} is out of range")) };
    }
    let ok = arg.len() <= 40 && arg.chars().all(|c| c.is_ascii_alphanumeric() || "+-:_".contains(c)) && !arg.is_empty();
    ok.then(|| lua_str(arg)).ok_or(format!("\"{arg}\" is not a workspace"))
}

/// The Lua dispatcher expression for an action (Hyprland 0.55+ `hl.dsp.*`).
pub fn lua_for(action: &str, arg: &str) -> Result<String, String> {
    let dir = |a: &str| ["left", "right", "up", "down"].contains(&a).then(|| a.to_string()).ok_or(format!("\"{a}\" is not a direction"));
    Ok(match action {
        "spawn" => {
            if arg.is_empty() {
                return Err("spawn needs a command".into());
            }
            let cmd = match arg {
                "$terminal" => "samurai open terminal",
                "$files" => "samurai open files",
                "$browser" => "samurai open browser",
                a => a,
            };
            format!("hl.dsp.exec_cmd({})", lua_str(cmd))
        }
        "frame" => {
            let ok = ["start", "quick", "widgets", "settings", "keys", "base", "incognito", "lock", "power"].contains(&arg) || arg.starts_with("settings:");
            if !ok {
                return Err(format!("\"{arg}\" is not a frame panel"));
            }
            format!("hl.dsp.exec_cmd({})", lua_str(&format!("samurai shell {arg}")))
        }
        "window" => match arg {
            "close" => "hl.dsp.window.close()".into(),
            "kill" => "hl.dsp.window.kill()".into(),
            "float" => "hl.dsp.window.float({ action = \"toggle\" })".into(),
            "fullscreen" => "hl.dsp.window.fullscreen({ mode = \"fullscreen\" })".into(),
            "maximize" => "hl.dsp.window.fullscreen({ mode = \"maximized\" })".into(),
            "pin" => "hl.dsp.window.pin()".into(),
            "center" => "hl.dsp.window.center()".into(),
            "pseudo" => "hl.dsp.window.pseudo()".into(),
            "next" => "hl.dsp.window.cycle_next({ next = true })".into(),
            "prev" => "hl.dsp.window.cycle_next({ next = false })".into(),
            "minimize" => format!("hl.dsp.exec_cmd({})", lua_str("samurai win minimize")),
            "restore" => format!("hl.dsp.exec_cmd({})", lua_str("samurai win restore")),
            a => return Err(format!("\"{a}\" is not a window action")),
        },
        "focus" => format!("hl.dsp.focus({{ direction = \"{}\" }})", dir(arg)?),
        "move" => format!("hl.dsp.window.move({{ direction = \"{}\" }})", dir(arg)?),
        "workspace" => format!("hl.dsp.focus({{ workspace = {} }})", ws_arg(arg)?),
        "window_to" => format!("hl.dsp.window.move({{ workspace = {} }})", ws_arg(arg)?),
        "window_to_quiet" => format!("hl.dsp.window.move({{ workspace = {}, follow = false }})", ws_arg(arg)?),
        "special" => format!("hl.dsp.workspace.toggle_special({})", lua_str(arg)),
        "layout" => format!("hl.dsp.layout({})", lua_str(arg)),
        "nothing" => "hl.dsp.no_op()".into(),
        // an expert's own dispatcher expression: only a single hl.dsp call, nothing else
        "lua" => {
            let a = arg.trim();
            let safe = a.starts_with("hl.dsp.") && a.ends_with(')') && !a.contains(';') && !a.contains('\n') && !a.contains("os.") && !a.contains("io.") && !a.contains("require") && !a.contains("load");
            if !safe {
                return Err("a Lua action must be one hl.dsp.…(…) call".into());
            }
            a.to_string()
        }
        a => return Err(format!("\"{a}\" is not an action")),
    })
}

/// bind option table from flags ("locked,repeat,release")
pub fn lua_opts(flags: &str, name: &str) -> String {
    let mut o: Vec<String> = vec![];
    for f in flags.split(',').map(str::trim) {
        match f {
            "locked" => o.push("locked = true".into()),
            "repeat" => o.push("repeating = true".into()),
            "release" => o.push("release = true".into()),
            "passthrough" => o.push("non_consuming = true".into()),
            _ => {}
        }
    }
    if !name.is_empty() {
        o.push(format!("description = {}", lua_str(name)));
    }
    if o.is_empty() { String::new() } else { format!(", {{ {} }}", o.join(", ")) }
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
    fn every_builtin_has_valid_lua_and_unique_keys() {
        let mut seen = std::collections::HashSet::new();
        for x in BUILTIN {
            assert!(lua_for(x.action, x.arg).is_ok(), "{}", x.id);
            assert!(seen.insert(canon(x.keys, "SUPER").unwrap()), "{} reuses keys", x.id);
        }
    }

    #[test]
    fn actions_cannot_smuggle_lua() {
        assert!(lua_for("lua", "hl.dsp.window.pin()").is_ok());
        for bad in ["os.execute('x')", "hl.dsp.exit(); os.exit()", "hl.dsp.exec_cmd(io.read())", "require('x')"] {
            assert!(lua_for("lua", bad).is_err(), "{bad}");
        }
        assert_eq!(lua_for("spawn", "say \"hi\"").unwrap(), "hl.dsp.exec_cmd(\"say \\\"hi\\\"\")");
        assert!(lua_for("workspace", "2\"}) os.exit() --").is_err());
        assert!(lua_for("focus", "sideways").is_err());
    }
}
