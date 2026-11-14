//! Settings -> niri config (KDL, niri 25.11 or newer; written against 26.04).
//!
//! Samurai keeps its own config folder and never touches ~/.config/niri:
//!   ~/.config/samurai/niri/config.kdl      the root: includes the two files below
//!   ~/.config/samurai/niri/generated.kdl   written here, from settings; never edited by hand
//!   ~/.config/samurai/niri/user.kdl        the user's own KDL, included last; never written again
//! The Samurai session starts niri with NIRI_CONFIG pointing at the root (samurai-session).
//!
//! Every value written here comes from the schema-checked settings and is either a number, a
//! boolean, one of a fixed set of words, or a quoted string, so a setting can change a value
//! but never add config. A new file replaces the old one only after `niri validate` accepts it,
//! and niri itself keeps the last good config if a reload fails.
use crate::binds::{self, kstr};
use crate::store;
use serde_json::Value;
use std::fmt::Write as _;

pub const ROOT_MARK: &str = "// samurai:root";
/// minimized windows are parked on this named workspace; the frame hides it and keeps it last
pub const SHELF: &str = "min";

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
    let hex = |s: &str| s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit());
    match store::get_str("look.accent_name").as_str() {
        "485C" => "#DA291C".into(),
        "032C" => "#EF3340".into(),
        "own" => Some(store::get_str("look.accent_own")).filter(|s| hex(s)).unwrap_or("#C8102E".into()),
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

fn num(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// a regular expression for a `match` line: quoted, and refused if niri could not compile it
fn regex_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 200 && !s.contains('\n')
}

/// one window-rule effect as a KDL line; None if the effect or its value is not acceptable
fn window_effect(name: &str, value: &str, allowed: &[Value]) -> Option<String> {
    if !allowed.iter().any(|a| a == name) {
        return None;
    }
    let flag = || truthy(value).unwrap_or(true);
    Some(match name {
        "float" => format!("open-floating {}", flag()),
        "maximize" => format!("open-maximized {}", flag()),
        "fullscreen" => format!("open-fullscreen {}", flag()),
        "no_focus" => format!("open-focused {}", !flag()),
        "workspace" => format!("open-on-workspace {}", kstr(Some(value).filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)))?)),
        "output" => format!("open-on-output {}", kstr(Some(value).filter(|v| !v.is_empty())?)),
        "opacity" => format!("opacity {}", num(value).filter(|n| (0.0..=1.0).contains(n))?),
        "rounding" => format!("geometry-corner-radius {}\n    clip-to-geometry true", num(value).filter(|n| (0.0..=64.0).contains(n))?),
        "block_capture" => "block-out-from \"screen-capture\"".into(),
        "width" => format!("default-column-width {{ proportion {:?}; }}", num(value).filter(|n| (0.05..=1.0).contains(n))?),
        _ => return None,
    })
}

fn layer_effect(name: &str, value: &str, allowed: &[Value]) -> Option<String> {
    if !allowed.iter().any(|a| a == name) {
        return None;
    }
    let flag = || truthy(value).unwrap_or(true);
    Some(match name {
        "opacity" => format!("opacity {}", num(value).filter(|n| (0.0..=1.0).contains(n))?),
        "blur" => format!("background-effect {{ blur {}; }}", flag()),
        "xray" => format!("background-effect {{ xray {}; }}", flag()),
        "shadow" => format!("shadow {{ {}; }}", if flag() { "on" } else { "off" }),
        "block_capture" => "block-out-from \"screen-capture\"".into(),
        "rounding" => format!("geometry-corner-radius {}", num(value).filter(|n| (0.0..=64.0).contains(n))?),
        _ => return None,
    })
}

fn transform(s: &str) -> Option<&'static str> {
    Some(match s.to_ascii_lowercase().as_str() {
        "" | "0" | "normal" => "normal",
        "1" | "90" => "90",
        "2" | "180" => "180",
        "3" | "270" => "270",
        "4" | "flipped" => "flipped",
        "5" | "flipped-90" => "flipped-90",
        "6" | "flipped-180" => "flipped-180",
        "7" | "flipped-270" => "flipped-270",
        _ => return None,
    })
}

pub fn generate() -> String {
    let mut o = String::new();
    let b = |k: &str| store::get_bool(k);
    let f = |k: &str| store::get_f64(k);
    let s = |k: &str| store::get_str(k);
    let n = |k: &str| store::get(k).as_i64().unwrap_or(0);
    let on = |o: &mut String, yes: bool, word: &str| {
        if yes {
            let _ = writeln!(o, "        {word}");
        }
    };
    o.push_str("// Written by Samurai from ~/.config/samurai/settings.json. Do not edit: change the\n// setting (Mod+Comma) or put your own KDL in user.kdl next to this file, which loads last.\n\n");

    // ── input ───────────────────────────────────────────────────────────────────────────────
    let mm = binds::main_mod();
    o.push_str("input {\n    keyboard {\n        xkb {\n");
    for (k, word) in [("input.kb_layout", "layout"), ("input.kb_variant", "variant"), ("input.kb_options", "options")] {
        if !s(k).is_empty() {
            let _ = writeln!(o, "            {word} {}", kstr(&s(k)));
        }
    }
    let _ = writeln!(o, "        }}\n        repeat-delay {}\n        repeat-rate {}", n("input.repeat_delay").clamp(100, 2000), n("input.repeat_rate").clamp(1, 100));
    on(&mut o, b("input.numlock"), "numlock");
    o.push_str("    }\n    touchpad {\n");
    on(&mut o, b("input.touchpad.tap"), "tap");
    let _ = writeln!(o, "        drag {}", b("input.touchpad.tap_drag"));
    on(&mut o, b("input.touchpad.drag_lock"), "drag-lock");
    on(&mut o, b("input.touchpad.natural"), "natural-scroll");
    on(&mut o, b("input.touchpad.dwt"), "dwt");
    on(&mut o, b("input.touchpad.clickfinger"), "click-method \"clickfinger\"");
    on(&mut o, b("input.touchpad.middle_emu"), "middle-emulation");
    let _ = writeln!(o, "        scroll-factor {}\n        accel-speed {}\n        accel-profile {}", f("input.touchpad.scroll_factor").clamp(0.1, 5.0), f("input.sensitivity").clamp(-1.0, 1.0), kstr(if s("input.accel_profile") == "flat" { "flat" } else { "adaptive" }));
    o.push_str("    }\n    mouse {\n");
    on(&mut o, b("input.natural_scroll"), "natural-scroll");
    on(&mut o, b("input.left_handed"), "left-handed");
    let _ = writeln!(o, "        scroll-factor {}\n        accel-speed {}\n        accel-profile {}", f("input.scroll_factor").clamp(0.1, 5.0), f("input.sensitivity").clamp(-1.0, 1.0), kstr(if s("input.accel_profile") == "flat" { "flat" } else { "adaptive" }));
    o.push_str("    }\n");
    if b("input.follow_mouse") {
        o.push_str("    focus-follows-mouse max-scroll-amount=\"0%\"\n");
    }
    let _ = writeln!(o, "    mod-key {}\n}}\n", kstr(match mm.as_str() { "ALT" => "Alt", "CTRL" => "Ctrl", _ => "Super" }));

    // ── outputs: only the ones the user gave a row; the rest keep niri's own choice ──────────
    for r in store::get_list("displays.monitors") {
        let out = row(&r, "output");
        if out.is_empty() {
            continue;
        }
        let _ = writeln!(o, "output {} {{", kstr(&out));
        if truthy(&row(&r, "disabled")) == Some(true) {
            o.push_str("    off\n}\n");
            continue;
        }
        let mode = row(&r, "mode");
        // 2560x1600 or 2560x1600@120.000
        let mode_ok = mode.split_once('x').is_some_and(|(w, rest)| {
            let (h, hz) = rest.split_once('@').unwrap_or((rest, "60"));
            w.parse::<u32>().is_ok() && h.parse::<u32>().is_ok() && num(hz).is_some()
        });
        if mode_ok {
            let _ = writeln!(o, "    mode {}", kstr(&mode));
        }
        if let Some(sc) = num(&row(&r, "scale")).filter(|x| (0.25..=10.0).contains(x)) {
            let _ = writeln!(o, "    scale {sc}");
        }
        if let Some(t) = transform(&row(&r, "transform")) {
            let _ = writeln!(o, "    transform {}", kstr(t));
        }
        if let Some((x, y)) = row(&r, "position").split_once(',').and_then(|(x, y)| Some((x.trim().parse::<i32>().ok()?, y.trim().parse::<i32>().ok()?))) {
            let _ = writeln!(o, "    position x={x} y={y}");
        }
        if b("displays.vrr") {
            o.push_str("    variable-refresh-rate\n");
        }
        o.push_str("}\n");
    }

    // ── layout: the window line is niri's border; the focus ring is off (one line, not two) ──
    let base = base_now();
    let (ink, faint) = if base == "white" { ("#000000", "#00000055") } else { ("#ffffff", "#ffffff55") };
    let gaps = n("look.gaps_in").clamp(0, 64);
    let strut = n("look.gaps_out").clamp(0, 96) - gaps;
    let _ = writeln!(o, "\nlayout {{\n    gaps {gaps}\n    struts {{\n        left {strut}\n        right {strut}\n        top {strut}\n        bottom {strut}\n    }}");
    let _ = writeln!(o, "    center-focused-column {}", kstr(match s("look.center_column").as_str() { "always" => "always", "on-overflow" => "on-overflow", _ => "never" }));
    if b("look.single_center") {
        o.push_str("    always-center-single-column\n");
    }
    let _ = writeln!(o, "    default-column-width {{ proportion {:?}; }}", f("look.column_width").clamp(0.2, 1.0));
    o.push_str("    preset-column-widths {\n        proportion 0.33333\n        proportion 0.5\n        proportion 0.66667\n        proportion 1.0\n    }\n    focus-ring {\n        off\n    }\n");
    let border = n("look.border").clamp(0, 8);
    if border == 0 {
        o.push_str("    border {\n        off\n    }\n");
    } else {
        let _ = writeln!(o, "    border {{\n        width {border}\n        active-color {}\n        inactive-color {}\n        urgent-color {}\n    }}", kstr(ink), kstr(faint), kstr(&accent()));
    }
    if b("look.shadow") {
        let r = n("look.shadow_range").clamp(0, 64);
        // hard-edged: no softness, pushed down-right by its size (the brutalist offset block)
        if b("look.shadow_sharp") {
            let _ = writeln!(o, "    shadow {{\n        on\n        softness 0\n        spread 0\n        offset x={r} y={r}\n        color {}\n    }}", kstr(ink));
        } else {
            let _ = writeln!(o, "    shadow {{\n        on\n        softness {r}\n        spread 0\n        offset x=0 y=0\n        color \"#00000080\"\n    }}");
        }
    }
    o.push_str("}\n\n");

    // ── everything else at the top level ─────────────────────────────────────────────────────
    if b("look.no_csd") {
        o.push_str("prefer-no-csd\n");
    }
    o.push_str("hotkey-overlay {\n    skip-at-startup\n}\n");
    o.push_str("cursor {\n");
    if !s("input.cursor.theme").is_empty() {
        let _ = writeln!(o, "    xcursor-theme {}", kstr(&s("input.cursor.theme")));
    }
    let _ = writeln!(o, "    xcursor-size {}", n("input.cursor.size").clamp(12, 96));
    if b("input.cursor.hide_typing") {
        o.push_str("    hide-when-typing\n");
    }
    if n("input.cursor.timeout") > 0 {
        let _ = writeln!(o, "    hide-after-inactive-ms {}", n("input.cursor.timeout").clamp(1, 600) * 1000);
    }
    o.push_str("}\n");
    if !b("input.middle_paste") {
        o.push_str("clipboard {\n    disable-primary\n}\n");
    }
    match s("motion.mode").as_str() {
        "cut" => o.push_str("animations {\n    off\n}\n"),
        m => {
            let slow = f("motion.speed").clamp(0.2, 5.0) * if m == "short" { 0.5 } else { 1.0 };
            let _ = writeln!(o, "animations {{\n    slowdown {slow}\n}}");
        }
    }
    if b("look.blur") {
        let _ = writeln!(o, "blur {{\n    passes {}\n    offset {}\n    noise {}\n}}", n("look.blur_passes").clamp(1, 6), n("look.blur_size").clamp(1, 20), f("look.blur_noise").clamp(0.0, 1.0));
    }
    o.push_str("environment {\n    QT_QPA_PLATFORM \"wayland;xcb\"\n    XDG_CURRENT_DESKTOP \"niri\"\n");
    for r in store::get_list("env") {
        let name = row(&r, "name");
        let reserved = ["QT_QPA_PLATFORM", "XDG_CURRENT_DESKTOP"].contains(&name.as_str());
        if !reserved && !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !name.starts_with(|c: char| c.is_ascii_digit()) {
            let _ = writeln!(o, "    {name} {}", kstr(&row(&r, "value")));
        }
    }
    o.push_str("}\n\n");

    // ── start: the frame first, then the user's commands ─────────────────────────────────────
    o.push_str("spawn-at-startup \"samurai\" \"session-start\"\n");
    for r in store::get_list("autostart") {
        let c = row(&r, "command");
        if !c.is_empty() {
            let _ = writeln!(o, "spawn-sh-at-startup {}", kstr(&c));
        }
    }
    let _ = writeln!(o, "\n// minimized windows are parked here; the rail shows them\nworkspace {}\n", kstr(SHELF));

    // ── keybinds ────────────────────────────────────────────────────────────────────────────
    o.push_str("binds {\n");
    for bd in binds::resolve() {
        let note = |o: &mut String, why: &str| {
            let _ = writeln!(o, "    // not applied ({}): {}", bd.keys.replace('\n', " "), why.replace('\n', " "));
        };
        match (&bd.held, binds::kdl_for(&bd.action, &bd.arg), binds::kdl_keys(&bd.keys)) {
            (Some(why), _, _) => note(&mut o, why),
            (None, Err(e), _) => note(&mut o, &e),
            (None, Ok(_), None) => note(&mut o, "niri has no such key or button"),
            (None, Ok(act), Some(keys)) => {
                let _ = writeln!(o, "    {keys}{} {{ {act}; }}", binds::kdl_props(&bd.flags, &bd.name));
            }
        }
    }
    let mut used: Vec<String> = binds::resolve().iter().filter(|x| x.held.is_none()).map(|x| x.keys.clone()).collect();
    for r in store::get_list("binds.mouse") {
        let (btn, action, arg) = (row(&r, "button"), row(&r, "action"), row(&r, "arg"));
        let Some(k) = binds::canon(&btn, &mm) else { continue };
        if used.contains(&k) {
            continue; // niri refuses a config that binds the same keys twice
        }
        if let (Some(keys), Ok(act)) = (binds::kdl_keys(&k), binds::kdl_for(&action, &arg)) {
            let _ = writeln!(o, "    {keys} {{ {act}; }}");
            used.push(k);
        }
    }
    o.push_str("}\n\n");

    // ── rules ───────────────────────────────────────────────────────────────────────────────
    let (round, act_op, in_op) = (n("look.rounding").clamp(0, 32), f("look.active_opacity").clamp(0.3, 1.0), f("look.inactive_opacity").clamp(0.3, 1.0));
    if round > 0 || act_op < 1.0 || b("look.blur") {
        o.push_str("window-rule {\n");
        if round > 0 {
            let _ = writeln!(o, "    geometry-corner-radius {round}\n    clip-to-geometry true");
        }
        if act_op < 1.0 {
            let _ = writeln!(o, "    opacity {act_op}");
        }
        if b("look.blur") {
            let _ = writeln!(o, "    background-effect {{\n        blur true\n        xray {}\n    }}", b("look.blur_xray"));
        }
        o.push_str("}\n");
    }
    if in_op < 1.0 {
        let _ = writeln!(o, "window-rule {{\n    match is-focused=false\n    opacity {in_op}\n}}");
    }
    let wfx = store::item("rules.window").map(|i| i["effects"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
    for r in store::get_list("rules.window").iter() {
        let (class, title) = (row(r, "class"), row(r, "title"));
        let Some(fx) = window_effect(&row(r, "effect"), &row(r, "value"), &wfx) else { continue };
        let mut m = String::new();
        if regex_ok(&class) {
            let _ = write!(m, " app-id={}", kstr(&class));
        }
        if regex_ok(&title) {
            let _ = write!(m, " title={}", kstr(&title));
        }
        if !m.is_empty() {
            let _ = writeln!(o, "window-rule {{\n    match{m}\n    {fx}\n}}");
        }
    }
    for r in store::get_list("perapp").iter() {
        let class = row(r, "class");
        if !regex_ok(&class) {
            continue;
        }
        let mut fx = String::new();
        if let Some(v) = num(&row(r, "opacity")).filter(|v| (0.0..=1.0).contains(v)) {
            let _ = writeln!(fx, "    opacity {v}");
        }
        if let Some(v) = num(&row(r, "rounding")).filter(|v| (0.0..=64.0).contains(v)) {
            let _ = writeln!(fx, "    geometry-corner-radius {v}\n    clip-to-geometry true");
        }
        match num(&row(r, "border")).filter(|v| (0.0..=16.0).contains(v)) {
            Some(v) if v == 0.0 => fx.push_str("    border {\n        off\n    }\n"),
            Some(v) => {
                let _ = writeln!(fx, "    border {{\n        on\n        width {v}\n    }}");
            }
            None => {}
        }
        if let Some(v) = truthy(&row(r, "blur")) {
            let _ = writeln!(fx, "    background-effect {{\n        blur {v}\n    }}");
        }
        if let Some(v) = truthy(&row(r, "shadow")) {
            let _ = writeln!(fx, "    shadow {{\n        {}\n    }}", if v { "on" } else { "off" });
        }
        if !fx.is_empty() {
            let _ = write!(o, "window-rule {{\n    match app-id={}\n{fx}}}\n", kstr(&class));
        }
    }
    let lfx = store::item("rules.layer").map(|i| i["effects"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
    for r in store::get_list("rules.layer").iter() {
        let ns = row(r, "namespace");
        let Some(fx) = layer_effect(&row(r, "effect"), &row(r, "value"), &lfx) else { continue };
        if regex_ok(&ns) {
            let _ = writeln!(o, "layer-rule {{\n    match namespace={}\n    {fx}\n}}", kstr(&ns));
        }
    }
    o
}

pub fn root_file() -> String {
    format!("{ROOT_MARK}\n// Samurai's niri config. Settings live in Mod+Comma; your own KDL goes in user.kdl.\ninclude \"generated.kdl\"\ninclude \"user.kdl\"\n")
}

fn niri(args: &[&str]) -> Option<(bool, String)> {
    let o = std::process::Command::new("niri").args(args).output().ok()?;
    Some((o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))))
}

/// What niri itself says about a config file. None: niri is not installed, nothing was checked.
fn validate(path: &std::path::Path) -> Option<Result<(), String>> {
    let (ok, text) = niri(&["validate", "-c", &path.to_string_lossy()])?;
    Some(if ok { Ok(()) } else { Err(text.lines().filter(|l| !l.trim().is_empty()).take(12).collect::<Vec<_>>().join("\n")) })
}

/// Write the generated file, but only once niri accepts it. Returns what it did.
pub fn apply() -> Result<Vec<String>, String> {
    let dir = store::niri_dir();
    let mut done = vec![];
    let root = dir.join("config.kdl");
    if !std::fs::read_to_string(&root).is_ok_and(|t| t.starts_with(ROOT_MARK)) {
        store::atomic_write(&root, &root_file()).map_err(|e| format!("{}: {e}", root.display()))?;
        done.push(format!("wrote {}", root.display()));
    }
    let user = dir.join("user.kdl");
    if !user.exists() {
        let _ = store::atomic_write(&user, "// Your own niri config (KDL). Included last. Samurai never writes here again.\n// A bind for keys Samurai already binds must be turned off in Settings > Keybinds first.\n");
    }
    let gen = dir.join("generated.kdl");
    let text = generate();
    // try it next to the real one first: a file niri refuses never replaces a working one
    let trial_dir = dir.join(".trial");
    let _ = std::fs::remove_dir_all(&trial_dir);
    store::atomic_write(&trial_dir.join("generated.kdl"), &text).map_err(|e| e.to_string())?;
    store::atomic_write(&trial_dir.join("config.kdl"), "include \"generated.kdl\"\n").map_err(|e| e.to_string())?;
    let verdict = validate(&trial_dir.join("config.kdl"));
    let _ = std::fs::remove_dir_all(&trial_dir);
    match verdict {
        Some(Err(e)) => return Err(format!("niri refused the new config, so the old one was kept:\n{e}")),
        Some(Ok(())) => done.push("niri accepted the config".into()),
        None => done.push("niri is not installed here: the config was written unchecked".into()),
    }
    store::atomic_write(&gen, &text).map_err(|e| format!("{}: {e}", gen.display()))?;
    done.push(format!("wrote {}", gen.display()));
    let _ = store::set_state("base", Value::String(base_now()));
    // niri watches its config and reloads by itself; with user.kdl in play, say so if the whole is refused
    if let Some(Err(e)) = validate(&root) {
        done.push(format!("warning: with your user.kdl the whole config is refused; niri keeps the last good one:\n{e}"));
    }
    Ok(done)
}

/// Balanced braces and quotes in what would be written, then niri's own verdict when it is installed.
pub fn check() -> Result<String, String> {
    let text = generate();
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (ln, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for c in line.chars() {
            if in_str {
                if esc { esc = false } else if c == '\\' { esc = true } else if c == '"' { in_str = false }
                continue;
            }
            match c {
                '"' => in_str = true,
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        if in_str {
            return Err(format!("line {}: a string does not end", ln + 1));
        }
        if depth < 0 {
            return Err(format!("line {}: a brace closes too early", ln + 1));
        }
    }
    if depth != 0 {
        return Err("braces do not balance".into());
    }
    let lines = text.lines().count();
    let root = store::niri_dir().join("config.kdl");
    match if root.exists() { validate(&root) } else { None } {
        Some(Ok(())) => Ok(format!("{lines} lines, accepted by niri validate")),
        Some(Err(e)) => Err(format!("niri refuses the config:\n{e}")),
        None => Ok(format!("{lines} lines, braces and quotes balance (niri validate was not run)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rule_effects_are_whitelisted_and_typed() {
        let allowed = vec![json!("opacity"), json!("workspace"), json!("float")];
        assert_eq!(window_effect("opacity", "0.9", &allowed).unwrap(), "opacity 0.9");
        assert!(window_effect("opacity", "1 } spawn-at-startup \"x\"", &allowed).is_none());
        assert!(window_effect("opacity", "7", &allowed).is_none());
        assert!(window_effect("workspace", "web\" } binds {", &allowed).is_none());
        assert_eq!(window_effect("workspace", "web", &allowed).unwrap(), "open-on-workspace \"web\"");
        assert!(window_effect("fullscreen", "on", &allowed).is_none(), "not in this list");
        assert_eq!(window_effect("float", "off", &allowed).unwrap(), "open-floating false");
    }

    #[test]
    fn transforms_map_from_numbers_and_names() {
        assert_eq!(transform("1"), Some("90"));
        assert_eq!(transform("flipped-180"), Some("flipped-180"));
        assert_eq!(transform("sideways"), None);
    }
}
