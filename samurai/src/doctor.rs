//! `samurai doctor`: look for what commonly goes wrong and put it right where that is safe.
//! Every check says what it found; every repair is logged (~/.local/state/samurai/doctor.log);
//! nothing of yours is deleted (a damaged file is moved aside, never removed). What needs root
//! is not done here: it is said, with the command that does it.
//!
//!   samurai doctor            check and repair everything (Settings › System › Check and repair)
//!   samurai doctor --quick    the fast checks only (run at every login)
//!   samurai doctor --json     the same, as JSON lines for Settings

use crate::{seal, store, x11};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Ok,
    Fixed,
    Warn,
}

pub struct Finding {
    pub what: &'static str,
    pub state: State,
    pub detail: String,
}

fn ok(what: &'static str, d: impl Into<String>) -> Finding { Finding { what, state: State::Ok, detail: d.into() } }
fn fixed(what: &'static str, d: impl Into<String>) -> Finding { Finding { what, state: State::Fixed, detail: d.into() } }
fn warn(what: &'static str, d: impl Into<String>) -> Finding { Finding { what, state: State::Warn, detail: d.into() } }

fn stamp() -> String {
    x11::output(&["date", "+%Y-%m-%d %H:%M:%S"]).unwrap_or_default().trim().to_string()
}

/// settings.json: valid JSON object, every value accepted by the schema. A damaged file is moved
/// aside and the last good copy put back (or, without one, the defaults are used).
pub fn check_settings() -> Finding {
    let f = store::settings_file();
    let Ok(text) = std::fs::read_to_string(&f) else { return ok("settings", "no changes from the defaults yet") };
    let parsed = serde_json::from_str::<Value>(&text).ok().filter(|v| v.is_object());
    if let Some(v) = parsed {
        let bad: Vec<String> = v.as_object().unwrap().iter().filter(|(k, val)| store::item(k).is_some_and(|i| store::check(&i, val).is_err())).map(|(k, _)| k.clone()).collect();
        if bad.is_empty() {
            let _ = store::atomic_write(&store::good_copy(), &text);
            return ok("settings", "settings.json is fine");
        }
        // a value the schema refuses: drop just those (they already read as defaults)
        let mut u = store::user();
        for k in &bad {
            u.remove(k);
        }
        return match store::save(&u) {
            Ok(()) => fixed("settings", format!("dropped values that were not valid: {}", bad.join(", "))),
            Err(e) => warn("settings", format!("values not valid ({}), and the file could not be written: {e}", bad.join(", "))),
        };
    }
    let aside = f.with_extension(format!("json.damaged-{}", stamp().replace([' ', ':'], "-")));
    if std::fs::rename(&f, &aside).is_err() {
        return warn("settings", "settings.json is damaged and could not be moved aside");
    }
    let good = store::good_copy();
    if let Ok(g) = std::fs::read_to_string(&good) {
        if serde_json::from_str::<Value>(&g).is_ok_and(|v| v.is_object()) && store::atomic_write(&f, &g).is_ok() {
            return fixed("settings", format!("settings.json was damaged: the last good copy is back (the damaged one is {})", aside.display()));
        }
    }
    fixed("settings", format!("settings.json was damaged and had no good copy: defaults are used (the damaged one is {})", aside.display()))
}

/// The folders Samurai writes to, and this machine's ink.
pub fn check_folders() -> Finding {
    let mut made = vec![];
    for d in [store::config_dir(), store::state_dir()] {
        if !d.is_dir() {
            if std::fs::create_dir_all(&d).is_ok() { made.push(d.display().to_string()) } else { return warn("folders", format!("{} could not be made", d.display())) }
        }
    }
    if !seal::dir().join("decoy-night.svg").exists() && seal::decoy().is_ok() {
        made.push("the login screen's drawing".into());
    }
    if made.is_empty() { ok("folders", "all there") } else { fixed("folders", format!("made: {}", made.join(", "))) }
}

/// Where X writes its log when started without root (see samurai-x).
pub fn check_xlog() -> Finding {
    let base = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| store::home().join(".local/share"));
    let d = base.join("xorg");
    if d.is_dir() {
        let probe = d.join(".samurai-write-test");
        if std::fs::write(&probe, b"").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return ok("x log", format!("{} is writable", d.display()));
        }
        return warn("x log", format!("{} is not writable (X would use a folder in /run instead)", d.display()));
    }
    match std::fs::create_dir_all(&d) {
        Ok(()) => fixed("x log", format!("made {}", d.display())),
        Err(_) => warn("x log", format!("{} could not be made (X uses a folder in /run instead)", d.display())),
    }
}

/// App settings (Alacritty, GTK, Qt, dunst…): written again from the settings.
pub fn check_apps() -> Finding {
    match x11::apply() {
        Ok(log) => {
            let wrote: Vec<&String> = log.iter().filter(|l| l.starts_with("wrote ")).collect();
            if wrote.is_empty() { ok("app settings", "every app's settings match") } else { fixed("app settings", format!("written again: {}", wrote.len())) }
        }
        Err(e) => warn("app settings", e),
    }
}

/// The helpers a session needs: started again if they are not running.
pub fn check_daemons() -> Finding {
    if std::env::var_os("DISPLAY").is_none() {
        return ok("helpers", "not in a desktop session: nothing to start");
    }
    let mut started = vec![];
    if x11::have("xsettingsd") && !x11::running("xsettingsd") {
        x11::daemon(&["xsettingsd", "-c", &store::home().join(".config/xsettingsd/xsettingsd.conf").to_string_lossy()]);
        started.push("xsettingsd (app look)");
    }
    if x11::have("dunst") && !x11::running("dunst") {
        x11::daemon(&["dunst"]);
        started.push("dunst (notices)");
    }
    let agent = "/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1";
    if Path::new(agent).exists() && !x11::running("polkit-gnome-au") {
        x11::daemon(&[agent]);
        started.push("the password prompt (polkit)");
    }
    if started.is_empty() { ok("helpers", "running") } else { fixed("helpers", format!("started: {}", started.join(", "))) }
}

/// System services the desktop uses. Turning them on needs root: said, not done.
pub fn check_services() -> Vec<Finding> {
    let mut v = vec![];
    if !x11::have("systemctl") {
        return v;
    }
    let active = |u: &str| x11::run(&["systemctl", "is-active", "--quiet", u]);
    for (unit, what, need) in [("NetworkManager.service", "network", "Wi-Fi in Settings, the overview and the login screen"), ("bluetooth.service", "bluetooth", "Bluetooth")] {
        if !x11::run(&["systemctl", "cat", unit]) {
            continue; // not installed
        }
        v.push(if active(unit) { ok(what, format!("{unit} is running")) } else { warn(what, format!("{unit} is not running; {need} needs it. Start it: sudo systemctl enable --now {unit}")) });
    }
    // sound runs per user: that one can be put right here
    if x11::have("pipewire") {
        let user_active = |u: &str| x11::run(&["systemctl", "--user", "is-active", "--quiet", u]);
        if user_active("pipewire.service") || user_active("pipewire.socket") {
            v.push(ok("sound", "PipeWire is running"));
        } else if x11::run(&["systemctl", "--user", "restart", "pipewire.socket", "pipewire.service", "wireplumber.service"]) {
            v.push(fixed("sound", "PipeWire was not running: started"));
        } else {
            v.push(warn("sound", "PipeWire is not running and could not be started (systemctl --user status pipewire)"));
        }
    }
    v
}

/// The parts installed as root: checked; repairs are named, not run.
pub fn check_system() -> Vec<Finding> {
    let mut v = vec![];
    if Path::new("/opt/samurai").is_dir() {
        v.push(if Path::new("/etc/pam.d/samurai-lock").exists() { ok("lock screen", "its password check is set up") } else { warn("lock screen", "/etc/pam.d/samurai-lock is missing (the lock falls back to the system login check). Put it back: ./install.sh --update") });
    }
    if Path::new("/etc/greetd/config.toml").exists() && std::fs::read_to_string("/etc/greetd/config.toml").unwrap_or_default().contains("samurai") {
        let conf = std::fs::read_to_string("/etc/samurai/greeter.json").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
        v.push(match conf {
            Some(c) if c.is_object() => ok("login screen", "its settings are fine"),
            _ => warn("login screen", "/etc/samurai/greeter.json is missing or damaged (the login screen uses its defaults). Write it again: samurai greeter apply"),
        });
    }
    v
}

/// The campus Wi-Fi: repaired when one of its profiles exists and the network is not fully up.
pub fn check_campus() -> Option<Finding> {
    if !x11::have("nmcli") || !x11::have("shadow") {
        return None;
    }
    let profiles = x11::output(&["nmcli", "-t", "-f", "NAME", "connection", "show"]).unwrap_or_default();
    let campus = profiles.lines().any(|l| { let l = l.to_lowercase(); l.contains("uc-secure") || l.contains("eduroam") });
    if !campus {
        return None;
    }
    let conn = x11::output(&["nmcli", "-t", "networking", "connectivity"]).unwrap_or_default();
    if conn.trim() == "full" {
        return Some(ok("campus wi-fi", "connected"));
    }
    Some(if x11::run(&["shadow", "wifi", "repair", "--quiet"]) { fixed("campus wi-fi", "repaired (shadow wifi log tells what was done)") } else { warn("campus wi-fi", "not connected, and the repair did not help: Settings › Network › Problem log") })
}

/// Your own app entries whose program is gone (never deleted: they are yours).
pub fn check_entries() -> Finding {
    let dir = store::home().join(".local/share/applications");
    let Ok(rd) = std::fs::read_dir(&dir) else { return ok("app entries", "none of your own") };
    let mut broken = vec![];
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "desktop") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let exec = text.lines().find_map(|l| l.strip_prefix("Exec=")).unwrap_or("").split_whitespace().next().unwrap_or("").trim_matches('"').to_string();
        if exec.is_empty() || exec == "env" || exec.contains('=') {
            continue;
        }
        let found = if exec.starts_with('/') { Path::new(&exec).exists() } else { x11::have(&exec) };
        if !found {
            broken.push(p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        }
    }
    if broken.is_empty() { ok("app entries", "every one starts a program that is there") } else { warn("app entries", format!("these start a program that is gone (in {}): {}", dir.display(), broken.join(", "))) }
}

/// Space left on your home and on the system.
pub fn check_disk() -> Vec<Finding> {
    let mut v = vec![];
    for (what, path) in [("disk (home)", store::home()), ("disk (system)", "/".into())] {
        let Some(out) = x11::output(&["df", "-Pk", &path.to_string_lossy()]) else { continue };
        let Some(line) = out.lines().nth(1) else { continue };
        let f: Vec<&str> = line.split_whitespace().collect();
        let (size, avail) = (f.get(1).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0), f.get(3).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0));
        if size <= 0.0 {
            continue;
        }
        let gb = avail / 1024.0 / 1024.0;
        v.push(if gb < 2.0 || (avail / size < 0.03 && gb < 8.0) { warn(what, format!("almost full: {gb:.1} GB left. Updates and apps can fail; free some space.")) } else { ok(what, format!("{gb:.0} GB free")) });
    }
    v
}

pub fn run(quick: bool) -> Vec<Finding> {
    let mut v = vec![check_settings(), check_folders(), check_xlog(), check_daemons()];
    if !quick {
        v.push(check_apps());
        v.extend(check_services());
        v.extend(check_system());
        v.extend(check_campus());
        v.push(check_entries());
        v.extend(check_disk());
    }
    // the log: what was found and done, one run per block
    let mut text = format!("\n== {} samurai doctor{}\n", stamp(), if quick { " --quick" } else { "" });
    for f in &v {
        text += &format!("{:<5} {:<14} {}\n", match f.state { State::Ok => "ok", State::Fixed => "fixed", State::Warn => "look" }, f.what, f.detail);
    }
    let log = store::state_dir().join("doctor.log");
    let _ = std::fs::create_dir_all(store::state_dir());
    let old = std::fs::read_to_string(&log).unwrap_or_default();
    // keep it short: the last ~200 lines
    let keep: Vec<&str> = old.lines().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
    let _ = store::atomic_write(&log, &(keep.join("\n") + &text));
    v
}

pub fn report(v: &[Finding], as_json: bool) -> String {
    if as_json {
        return v.iter().map(|f| json!({ "what": f.what, "state": match f.state { State::Ok => "ok", State::Fixed => "fixed", State::Warn => "look" }, "detail": f.detail }).to_string()).collect::<Vec<_>>().join("\n");
    }
    let fixed = v.iter().filter(|f| f.state == State::Fixed).count();
    let warns = v.iter().filter(|f| f.state == State::Warn).count();
    let mut s = String::new();
    for f in v {
        s += &format!("{} {:<14} {}\n", match f.state { State::Ok => "✓", State::Fixed => "↻", State::Warn => "!" }, f.what, f.detail);
    }
    s += &format!("{} checks · {fixed} put right · {warns} to look at", v.len());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damaged_settings_come_back_from_the_good_copy() {
        let dir = std::env::temp_dir().join(format!("samurai-doctor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SAMURAI_CONFIG", &dir);
        let good = "{\n  \"look.ground\": \"paper\"\n}\n";
        std::fs::write(dir.join("settings.json"), good).unwrap();
        assert_eq!(check_settings().state, State::Ok, "a good file is fine and becomes the good copy");
        std::fs::write(dir.join("settings.json"), "{ broken").unwrap();
        let f = check_settings();
        assert_eq!(f.state, State::Fixed);
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), good, "the good copy is back");
        assert!(std::fs::read_dir(&dir).unwrap().flatten().any(|e| e.file_name().to_string_lossy().contains("damaged")), "the damaged file is kept aside");
        std::fs::write(dir.join("settings.json"), "{\"look.ground\": 5, \"look.accent\": \"rust\"}").unwrap();
        assert_eq!(check_settings().state, State::Fixed, "a value the schema refuses is dropped");
        assert!(!std::fs::read_to_string(dir.join("settings.json")).unwrap().contains("look.ground"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
