//! Writes Hyprland's config (Lua, Hyprland 0.55+; 0.57 accepts nothing else) from the settings.
//!
//!   ~/.config/hypr/hyprland.lua           the root: loads the generated file, then user.lua.
//!                                         Written only when missing or when it is Samurai's own.
//!   ~/.config/hypr/samurai/generated.lua  everything below; rewritten by `samurai apply`
//!   ~/.config/hypr/user.lua               yours; loaded last, so it always wins; never touched
//!
//! Every string that reaches the Lua file goes through `lua_str`, and every option name comes from
//! the schema, so a setting can change a value but never add code.
use crate::binds::{self, lua_str};
use crate::store;
use serde_json::Value;
use std::collections::BTreeMap;

pub const ROOT_MARK: &str = "-- samurai:root";

enum Node {
    Leaf(String),
    Table(BTreeMap<String, Node>),
}

fn key(k: &str) -> String {
    let ident = !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !k.starts_with(|c: char| c.is_ascii_digit());
    if ident { k.to_string() } else { format!("[{}]", lua_str(k)) }
}

fn insert(root: &mut BTreeMap<String, Node>, path: &str, lit: String) {
    let parts: Vec<&str> = path.split([':', '.']).collect();
    let mut cur = root;
    for p in &parts[..parts.len() - 1] {
        let e = cur.entry(p.to_string()).or_insert_with(|| Node::Table(BTreeMap::new()));
        cur = match e {
            Node::Table(t) => t,
            Node::Leaf(_) => return,
        };
    }
    cur.insert(parts[parts.len() - 1].to_string(), Node::Leaf(lit));
}

fn emit(t: &BTreeMap<String, Node>, depth: usize, out: &mut String) {
    let pad = "    ".repeat(depth);
    for (k, n) in t {
        match n {
            Node::Leaf(v) => out.push_str(&format!("{pad}{} = {v},\n", key(k))),
            Node::Table(c) => {
                out.push_str(&format!("{pad}{} = {{\n", key(k)));
                emit(c, depth + 1, out);
                out.push_str(&format!("{pad}}},\n"));
            }
        }
    }
}

fn lit(v: &Value, as_int: bool) -> String {
    match v {
        Value::Bool(b) if as_int => (*b as i32).to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if as_int => s.parse::<i64>().map(|n| n.to_string()).unwrap_or_else(|_| lua_str(s)),
        Value::String(s) => lua_str(s),
        _ => "nil".into(),
    }
}

/// white ground between the given hours ("07:00" … "19:00")
pub fn base_now() -> String {
    match store::get_str("look.base").as_str() {
        "white" => "white".into(),
        "auto" => {
            let mins = |s: &str| s.split_once(':').and_then(|(h, m)| Some(h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()?));
            let now = local_minutes();
            match (mins(&store::get_str("look.auto_light_from")), mins(&store::get_str("look.auto_light_to"))) {
                (Some(a), Some(b)) if a < b && now >= a && now < b => "white".into(),
                _ => "black".into(),
            }
        }
        _ => "black".into(),
    }
}

fn local_minutes() -> u32 {
    let out = std::process::Command::new("date").arg("+%H %M").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut it = out.split_whitespace().filter_map(|x| x.parse::<u32>().ok());
    it.next().unwrap_or(0) * 60 + it.next().unwrap_or(0)
}

pub fn accent() -> String {
    match store::get_str("look.accent_name").as_str() {
        "485C" => "#DA291C".into(),
        "032C" => "#EF3340".into(),
        "own" => store::get_str("look.accent_own"),
        _ => "#C8102E".into(),
    }
}

fn row(r: &Value, k: &str) -> String {
    r[k].as_str().unwrap_or("").trim().to_string()
}

fn truthy(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "on" | "yes" | "true" | "1" => Some(true),
        "off" | "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

/// one rule effect as `name = value`; None if the effect or its value is not acceptable
fn effect(name: &str, value: &str, allowed: &[Value]) -> Option<String> {
    if !allowed.iter().any(|a| a == name) {
        return None;
    }
    let num = |v: &str| v.parse::<f64>().ok().filter(|n| n.is_finite());
    Some(match name {
        "opacity" | "rounding" | "border_size" | "order" => format!("{name} = {}", num(value)?),
        "workspace" | "size" | "move" | "monitor" | "suppress_event" | "ignore_alpha" => format!("{name} = {}", lua_str(value)),
        _ => format!("{name} = {}", truthy(value).unwrap_or(true)),
    })
}

pub fn generate() -> String {
    let all = store::all();
    let mut o = String::new();
    o.push_str("-- Written by Samurai from ~/.config/samurai/settings.json. Do not edit: change the\n-- setting (Mod+Comma) or put your own lines in ~/.config/hypr/user.lua, which loads last.\n\n");

    // ── hl.config: every schema item that names a Hyprland option ────────────────────────────
    let mut root: BTreeMap<String, Node> = BTreeMap::new();
    for (i, _) in store::items() {
        if let Some(h) = i["hl"].as_str() {
            let v = &all[i["key"].as_str().unwrap()];
            insert(&mut root, h, lit(v, i["as_int"] == true));
        }
    }
    let base = base_now();
    let (ink, faint) = if base == "white" { ("rgba(000000ff)", "rgba(00000055)") } else { ("rgba(ffffffff)", "rgba(ffffff55)") };
    insert(&mut root, "general:col.active_border", lua_str(ink));
    insert(&mut root, "general:col.inactive_border", lua_str(faint));
    insert(&mut root, "misc:disable_hyprland_logo", "true".into());
    insert(&mut root, "misc:force_default_wallpaper", "0".into());
    insert(&mut root, "misc:vrr", if store::get_bool("displays.vrr") { "1" } else { "0" }.into());
    insert(&mut root, "render:direct_scanout", if store::get_bool("gpu.direct_scanout") { "1" } else { "0" }.into());
    let motion = store::get_str("motion.mode");
    insert(&mut root, "animations:enabled", (motion != "cut").to_string());
    o.push_str("hl.config({\n");
    emit(&root, 1, &mut o);
    o.push_str("})\n\n");

    // ── motion ────────────────────────────────────────────────────────────────────────────────
    if motion != "cut" {
        let pts: Vec<f64> = store::get_str("motion.curve").split_whitespace().filter_map(|x| x.parse().ok()).collect();
        let p = if pts.len() == 4 && pts.iter().all(|x| x.is_finite() && (-1.0..=2.0).contains(x)) { pts } else { vec![0.2, 0.0, 0.0, 1.0] };
        let sp = store::get_f64("motion.speed");
        o.push_str(&format!("hl.curve(\"samurai\", {{ type = \"bezier\", points = {{ {{{}, {}}}, {{{}, {}}} }} }})\n", p[0], p[1], p[2], p[3]));
        o.push_str(&format!("hl.animation({{ leaf = \"global\", enabled = true, speed = {sp}, bezier = \"samurai\" }})\n"));
        o.push_str(&format!("hl.animation({{ leaf = \"windows\", enabled = true, speed = {sp}, bezier = \"samurai\", style = {} }})\n", lua_str(&store::get_str("motion.window_style"))));
        o.push_str(&format!("hl.animation({{ leaf = \"workspaces\", enabled = true, speed = {sp}, bezier = \"samurai\", style = {} }})\n", lua_str(&store::get_str("motion.workspace_style"))));
        o.push_str(&format!("hl.animation({{ leaf = \"layers\", enabled = {}, speed = {sp}, bezier = \"samurai\" }})\n", store::get_bool("motion.layers")));
        if motion == "short" {
            o.push_str("hl.animation({ leaf = \"fade\", enabled = false })\nhl.animation({ leaf = \"border\", enabled = false })\n");
        }
        o.push('\n');
    }

    // ── monitors ──────────────────────────────────────────────────────────────────────────────
    o.push_str("hl.monitor({ output = \"\", mode = \"preferred\", position = \"auto\", scale = \"auto\" })\n");
    for r in store::get_list("displays.monitors") {
        let out = row(&r, "output");
        if out.is_empty() {
            continue;
        }
        if truthy(&row(&r, "disabled")) == Some(true) {
            o.push_str(&format!("hl.monitor({{ output = {}, disabled = true }})\n", lua_str(&out)));
            continue;
        }
        let or = |k: &str, d: &str| { let v = row(&r, k); if v.is_empty() { d.to_string() } else { v } };
        let mut f = vec![format!("output = {}", lua_str(&out)), format!("mode = {}", lua_str(&or("mode", "preferred"))), format!("position = {}", lua_str(&or("position", "auto")))];
        let sc = or("scale", "auto");
        f.push(format!("scale = {}", sc.parse::<f64>().map(|n| n.to_string()).unwrap_or_else(|_| lua_str("auto"))));
        if let Ok(t) = row(&r, "transform").parse::<u8>() {
            if t <= 7 {
                f.push(format!("transform = {t}"));
            }
        }
        let mirror = row(&r, "mirror");
        if !mirror.is_empty() {
            f.push(format!("mirror = {}", lua_str(&mirror)));
        }
        o.push_str(&format!("hl.monitor({{ {} }})\n", f.join(", ")));
    }
    o.push('\n');

    // ── environment ───────────────────────────────────────────────────────────────────────────
    let theme = store::get_str("input.cursor.theme");
    let size = store::get("input.cursor.size").to_string();
    o.push_str(&format!("hl.env(\"XCURSOR_SIZE\", {})\nhl.env(\"HYPRCURSOR_SIZE\", {})\n", lua_str(&size), lua_str(&size)));
    if !theme.is_empty() {
        o.push_str(&format!("hl.env(\"XCURSOR_THEME\", {})\nhl.env(\"HYPRCURSOR_THEME\", {})\n", lua_str(&theme), lua_str(&theme)));
    }
    o.push_str("hl.env(\"XDG_CURRENT_DESKTOP\", \"Hyprland\")\nhl.env(\"QT_QPA_PLATFORM\", \"wayland;xcb\")\n");
    for r in store::get_list("env") {
        let name = row(&r, "name");
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !name.starts_with(|c: char| c.is_ascii_digit()) {
            o.push_str(&format!("hl.env({}, {})\n", lua_str(&name), lua_str(&row(&r, "value"))));
        }
    }
    o.push('\n');

    // ── gestures ──────────────────────────────────────────────────────────────────────────────
    if store::get_bool("input.swipe.enabled") {
        o.push_str(&format!("hl.gesture({{ fingers = {}, direction = {}, action = \"workspace\" }})\n\n", store::get_str("input.swipe.fingers").parse::<u8>().unwrap_or(3), lua_str(&store::get_str("input.swipe.direction"))));
    }

    // ── autostart: the frame first, then the user's commands ──────────────────────────────────
    o.push_str("hl.on(\"hyprland.start\", function()\n    hl.exec_cmd(\"samurai session-start\")\n");
    for r in store::get_list("autostart") {
        let c = row(&r, "command");
        if !c.is_empty() {
            o.push_str(&format!("    hl.exec_cmd({})\n", lua_str(&c)));
        }
    }
    o.push_str("end)\n\n");

    // ── keybinds ──────────────────────────────────────────────────────────────────────────────
    for b in binds::resolve() {
        match (&b.held, binds::lua_for(&b.action, &b.arg)) {
            (None, Ok(dsp)) => o.push_str(&format!("hl.bind({}, {dsp}{})\n", lua_str(&b.keys), binds::lua_opts(&b.flags, &b.name))),
            (Some(why), _) => o.push_str(&format!("-- not applied ({}): {}\n", b.keys.replace('\n', " "), why.replace('\n', " "))),
            (None, Err(e)) => o.push_str(&format!("-- not applied ({}): {}\n", b.id, e.replace('\n', " "))),
        }
    }
    let mm = binds::main_mod();
    o.push_str(&format!("hl.bind(\"{mm} + mouse:272\", hl.dsp.window.drag(), {{ mouse = true }})\nhl.bind(\"{mm} + mouse:273\", hl.dsp.window.resize(), {{ mouse = true }})\n"));
    for r in store::get_list("binds.mouse") {
        let (btn, action, arg) = (row(&r, "button"), row(&r, "action"), row(&r, "arg"));
        if let (Some(k), Ok(dsp)) = (binds::canon(&btn, &mm), binds::lua_for(&action, &arg)) {
            o.push_str(&format!("hl.bind({}, {dsp})\n", lua_str(&k)));
        }
    }
    o.push('\n');

    // ── rules ─────────────────────────────────────────────────────────────────────────────────
    // minimized windows live on a hidden workspace; the frame's own windows never animate
    o.push_str("hl.workspace_rule({ workspace = \"special:min\", gaps_in = 0, gaps_out = 0 })\n");
    o.push_str("hl.layer_rule({ name = \"samurai-frame\", match = { namespace = \"^samurai-\" }, no_anim = true })\n");
    let wfx = store::item("rules.window").map(|i| i["effects"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
    for (n, r) in store::get_list("rules.window").iter().enumerate() {
        let (class, title) = (row(r, "class"), row(r, "title"));
        let Some(fx) = effect(&row(r, "effect"), &row(r, "value"), &wfx) else { continue };
        if class.is_empty() && title.is_empty() {
            continue;
        }
        let mut m = vec![];
        if !class.is_empty() {
            m.push(format!("class = {}", lua_str(&class)));
        }
        if !title.is_empty() {
            m.push(format!("title = {}", lua_str(&title)));
        }
        o.push_str(&format!("hl.window_rule({{ name = \"samurai-user-{n}\", match = {{ {} }}, {fx} }})\n", m.join(", ")));
    }
    let lfx = store::item("rules.layer").map(|i| i["effects"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
    for (n, r) in store::get_list("rules.layer").iter().enumerate() {
        let ns = row(r, "namespace");
        let Some(fx) = effect(&row(r, "effect"), &row(r, "value"), &lfx) else { continue };
        if !ns.is_empty() {
            o.push_str(&format!("hl.layer_rule({{ name = \"samurai-layer-{n}\", match = {{ namespace = {} }}, {fx} }})\n", lua_str(&ns)));
        }
    }
    for (n, r) in store::get_list("perapp").iter().enumerate() {
        let class = row(r, "class");
        if class.is_empty() {
            continue;
        }
        let mut fx = vec![];
        for (col, name) in [("opacity", "opacity"), ("rounding", "rounding"), ("border", "border_size")] {
            if let Ok(v) = row(r, col).parse::<f64>() {
                if v.is_finite() {
                    fx.push(format!("{name} = {v}"));
                }
            }
        }
        for (col, name) in [("blur", "no_blur"), ("shadow", "no_shadow"), ("dim", "no_dim"), ("anim", "no_anim")] {
            if let Some(on) = truthy(&row(r, col)) {
                fx.push(format!("{name} = {}", !on));
            }
        }
        if truthy(&row(r, "opaque")) == Some(true) {
            fx.push("opaque = true".into());
        }
        if !fx.is_empty() {
            o.push_str(&format!("hl.window_rule({{ name = \"samurai-app-{n}\", match = {{ class = {} }}, {} }})\n", lua_str(&class), fx.join(", ")));
        }
    }
    o
}

pub fn root_file() -> String {
    format!("{ROOT_MARK}\n-- Samurai's Hyprland root. Settings live in Mod+Comma; your own Lua goes in user.lua.\nrequire(\"samurai.generated\")\npcall(require, \"user\")\n")
}

/// Write the generated file (and the root, when it is missing or Samurai's own). Returns what it did.
pub fn apply() -> Result<Vec<String>, String> {
    let dir = store::hypr_dir();
    let mut done = vec![];
    let gen = dir.join("samurai/generated.lua");
    store::atomic_write(&gen, &generate()).map_err(|e| format!("{}: {e}", gen.display()))?;
    done.push(format!("wrote {}", gen.display()));
    let root = dir.join("hyprland.lua");
    match std::fs::read_to_string(&root) {
        Ok(t) if t.starts_with(ROOT_MARK) => {}
        Ok(_) => {
            // someone else's config: keep it, and say how to load Samurai from it
            done.push(format!("{} is your own file and was left alone: add  require(\"samurai.generated\")  to it", root.display()));
        }
        Err(_) => {
            store::atomic_write(&root, &root_file()).map_err(|e| format!("{}: {e}", root.display()))?;
            done.push(format!("wrote {}", root.display()));
        }
    }
    let user = dir.join("user.lua");
    if !user.exists() {
        let _ = store::atomic_write(&user, "-- Your own Hyprland Lua. Loaded last, so it wins. Samurai never writes here.\n");
    }
    let _ = store::set_state("base", Value::String(base_now()));
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        let ok = std::process::Command::new("hyprctl").arg("reload").output().map(|o| o.status.success()).unwrap_or(false);
        done.push(if ok { "Hyprland reloaded".into() } else { "Hyprland did not reload (hyprctl reload failed)".into() });
    }
    Ok(done)
}

/// Balanced brackets and quotes in what was generated, and `luac -p` when a Lua is installed.
pub fn check() -> Result<String, String> {
    let text = generate();
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (n, line) in text.lines().enumerate() {
        for c in line.chars() {
            if in_str {
                if esc { esc = false } else if c == '\\' { esc = true } else if c == '"' { in_str = false }
                continue;
            }
            match c {
                '"' => in_str = true,
                '{' | '(' => depth += 1,
                '}' | ')' => depth -= 1,
                _ => {}
            }
            if line.trim_start().starts_with("--") {
                break;
            }
        }
        if in_str {
            return Err(format!("line {}: a string does not end", n + 1));
        }
        if depth < 0 {
            return Err(format!("line {}: a bracket closes too early", n + 1));
        }
    }
    if depth != 0 {
        return Err("brackets do not balance".into());
    }
    for luac in ["luac", "luac5.4", "luac5.1", "luajit"] {
        let tmp = std::env::temp_dir().join(format!("samurai-check-{}.lua", std::process::id()));
        if std::fs::write(&tmp, &text).is_err() {
            break;
        }
        let args: &[&str] = if luac == "luajit" { &["-bl"] } else { &["-p"] };
        let r = std::process::Command::new(luac).args(args).arg(&tmp).output();
        let _ = std::fs::remove_file(&tmp);
        if let Ok(out) = r {
            return if out.status.success() { Ok(format!("{} lines, syntax checked with {luac}", text.lines().count())) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) };
        }
    }
    Ok(format!("{} lines, brackets and strings balance (no Lua installed for a full syntax check)", text.lines().count()))
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn nested_tables_and_odd_keys() {
        let mut r = BTreeMap::new();
        insert(&mut r, "input:touchpad:tap-to-click", "true".into());
        insert(&mut r, "general:col.active_border", "\"x\"".into());
        let mut s = String::new();
        emit(&r, 0, &mut s);
        assert!(s.contains("[\"tap-to-click\"] = true"), "{s}");
        assert!(s.contains("col = {") && s.contains("active_border = \"x\""), "{s}");
    }

    #[test]
    fn rule_effects_are_whitelisted_and_typed() {
        let allowed = vec![Value::from("opacity"), Value::from("float"), Value::from("workspace")];
        assert_eq!(effect("opacity", "0.9", &allowed).unwrap(), "opacity = 0.9");
        assert_eq!(effect("float", "", &allowed).unwrap(), "float = true");
        assert_eq!(effect("workspace", "2\" }) os.exit() --", &allowed).unwrap(), "workspace = \"2\\\" }) os.exit() --\"");
        assert!(effect("opacity", "x", &allowed).is_none());
        assert!(effect("exec", "rm", &allowed).is_none());
    }
}
