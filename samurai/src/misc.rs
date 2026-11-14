//! The smaller commands: status for the frame, incognito, minimize / restore, opening default
//! apps, saved looks, the store, updates, capture, GPU readiness.
use crate::store;
use serde_json::{json, Value};
use std::process::{Command, Stdio};

pub fn out(cmd: &[&str]) -> (bool, String) {
    match Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stderr(Stdio::null()).output() {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Err(_) => (false, String::new()),
    }
}
pub fn have(prog: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
}
pub fn spawn(cmd: &[&str]) -> bool {
    Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok()
}
fn sh(line: &str) -> bool {
    spawn(&["sh", "-c", line])
}
fn expand(p: &str) -> std::path::PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => store::home().join(rest),
        None => p.into(),
    }
}
fn stamp() -> String {
    out(&["date", "+%Y-%m-%d_%H-%M-%S"]).1
}

// ── status: what the bar shows ───────────────────────────────────────────────────────────────
fn battery() -> Value {
    let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") else { return Value::Null };
    let (mut ac, mut bat) = (false, None);
    for e in rd.flatten() {
        let r = |f: &str| std::fs::read_to_string(e.path().join(f)).unwrap_or_default().trim().to_string();
        match r("type").as_str() {
            "Mains" | "USB" => ac |= r("online") == "1",
            "Battery" if bat.is_none() => bat = r("capacity").parse::<i64>().ok().map(|c| (c, r("status"))),
            _ => {}
        }
    }
    match bat {
        Some((p, s)) => json!({ "percent": p, "charging": s == "Charging" || s == "Full", "ac": ac }),
        None => Value::Null,
    }
}

fn network() -> Value {
    let mut wifi = Value::Null;
    for l in out(&["nmcli", "-t", "-f", "ACTIVE,SSID,SIGNAL,SECURITY", "dev", "wifi"]).1.lines() {
        let f: Vec<&str> = l.splitn(4, ':').collect();
        if f.len() == 4 && f[0] == "yes" {
            wifi = json!({ "ssid": f[1], "signal": f[2].parse::<i64>().unwrap_or(0), "security": f[3] });
        }
    }
    let wired = out(&["nmcli", "-t", "-f", "TYPE,STATE", "dev"]).1.lines().any(|l| l.starts_with("ethernet:connected"));
    json!({ "wifi_on": out(&["nmcli", "-t", "radio", "wifi"]).1 == "enabled", "wifi": wifi, "wired": wired })
}

pub fn status() -> Value {
    let vol = out(&["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]).1;
    let bri = out(&["brightnessctl", "-m"]).1;
    let st = store::state();
    json!({
        "battery": battery(),
        "network": network(),
        "volume": vol.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()).map(|v| (v * 100.0).round() as i64),
        "muted": vol.contains("MUTED"),
        "brightness": bri.split(',').nth(3).and_then(|p| p.trim_end_matches('%').parse::<i64>().ok()),
        "user": std::env::var("USER").unwrap_or_default(),
        "host": std::fs::read_to_string("/etc/hostname").unwrap_or_default().trim(),
        "base": crate::niri::base_now(),
        "accent": crate::niri::accent(),
        "incognito": st.get("incognito").and_then(|v| v.as_bool()).unwrap_or(false),
        "update": st.get("update").cloned().unwrap_or(json!({ "available": false })),
        "seal": crate::seal::show(),
        "recording": st.get("recording").and_then(|v| v.as_u64()).is_some_and(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists()),
    })
}

// ── incognito ────────────────────────────────────────────────────────────────────────────────
pub fn incognito(arg: &str) -> Result<bool, String> {
    let now = store::state().get("incognito").and_then(|v| v.as_bool()).unwrap_or(false);
    let want = match arg {
        "on" => true,
        "off" => false,
        "toggle" => !now,
        "status" => return Ok(now),
        a => return Err(format!("incognito {a}: on, off, toggle or status")),
    };
    if want {
        crate::seal::decoy()?;
    }
    store::set_state("incognito", json!(want))?;
    Ok(want)
}

// ── minimize: niri has none, so the window is parked on a named workspace ("the shelf") ──────
fn niri_json(what: &str) -> Value {
    serde_json::from_str(&out(&["niri", "msg", "-j", what]).1).unwrap_or(Value::Null)
}
fn action(args: &[&str]) -> bool {
    let mut c = vec!["niri", "msg", "action"];
    c.extend_from_slice(args);
    out(&c).0
}
fn shelf_of(workspaces: &Value) -> Option<Value> {
    workspaces.as_array()?.iter().find(|w| w["name"] == crate::niri::SHELF).cloned()
}

/// Keep the shelf below every workspace that has windows, so Mod+1…9 keep meaning what they say.
pub fn tidy() -> Result<String, String> {
    let ws = niri_json("workspaces");
    let Some(shelf) = shelf_of(&ws) else { return Ok("no shelf".into()) };
    let same: Vec<&Value> = ws.as_array().map(|a| a.iter().filter(|w| w["output"] == shelf["output"]).collect()).unwrap_or_default();
    // the last workspace on an output is niri's always-empty one; the shelf belongs just above it
    let last_used = same.iter().filter(|w| w["name"] != crate::niri::SHELF && !w["active_window_id"].is_null()).filter_map(|w| w["idx"].as_u64()).max().unwrap_or(0);
    let at = shelf["idx"].as_u64().unwrap_or(0);
    if at > last_used {
        return Ok("in place".into());
    }
    action(&["move-workspace-to-index", "--reference", crate::niri::SHELF, &last_used.to_string()]).then(|| "moved".to_string()).ok_or("niri did not move the shelf".into())
}

pub fn win(arg: &str, id: Option<&str>) -> Result<String, String> {
    let ws = niri_json("workspaces");
    let shelf = shelf_of(&ws);
    let shelf_id = shelf.as_ref().map(|s| s["id"].clone()).unwrap_or(Value::Null);
    let valid = |a: &str| !a.is_empty() && a.len() <= 20 && a.chars().all(|c| c.is_ascii_digit());
    match arg {
        "minimize" => {
            if shelf.is_none() {
                return Err("the shelf workspace is missing (samurai apply writes it)".into());
            }
            let wid = match id {
                Some(a) if valid(a) => a.to_string(),
                Some(a) => return Err(format!("{a} is not a window id")),
                None => niri_json("focused-window")["id"].as_u64().ok_or("no focused window")?.to_string(),
            };
            if !action(&["move-window-to-workspace", "--window-id", &wid, "--focus", "false", crate::niri::SHELF]) {
                return Err("niri refused to move the window".into());
            }
            let _ = tidy();
            Ok(format!("minimized {wid}"))
        }
        "restore" => {
            let windows = niri_json("windows");
            let mut parked: Vec<&Value> = windows.as_array().map(|a| a.iter().filter(|w| !shelf_id.is_null() && w["workspace_id"] == shelf_id).collect()).unwrap_or_default();
            parked.sort_by_key(|w| (w["focus_timestamp"]["secs"].as_u64().unwrap_or(0), w["focus_timestamp"]["nanos"].as_u64().unwrap_or(0)));
            let wid = match id {
                Some(a) if valid(a) => a.to_string(),
                Some(a) => return Err(format!("{a} is not a window id")),
                None => parked.last().and_then(|w| w["id"].as_u64()).ok_or("nothing is minimized")?.to_string(),
            };
            // back to the workspace the user is looking at
            let here = ws.as_array().and_then(|a| a.iter().find(|w| w["is_focused"] == true)).cloned().unwrap_or(Value::Null);
            let same_output = shelf.as_ref().is_some_and(|s| s["output"] == here["output"]);
            let moved = if same_output || here.is_null() {
                let idx = here["idx"].as_u64().unwrap_or(1).to_string();
                action(&["move-window-to-workspace", "--window-id", &wid, &idx])
            } else {
                action(&["move-window-to-monitor", "--id", &wid, here["output"].as_str().unwrap_or("")])
            };
            if !moved {
                return Err("niri refused to move the window".into());
            }
            action(&["focus-window", "--id", &wid]);
            let _ = tidy();
            Ok(format!("restored {wid}"))
        }
        "tidy" | "prune" => tidy(),
        a => Err(format!("win {a}: minimize [ID], restore [ID] or tidy")),
    }
}

// ── default apps, terminal, the frame's IPC ──────────────────────────────────────────────────
pub fn terminal() -> Option<String> {
    std::env::var("TERMINAL").ok().filter(|t| have(t)).or_else(|| ["kitty", "foot", "ghostty", "alacritty", "wezterm", "konsole", "gnome-terminal"].iter().find(|t| have(t)).map(|t| t.to_string()))
}

pub fn open(what: &str) -> Result<(), String> {
    let ok = match what {
        "terminal" => spawn(&[&terminal().ok_or("no terminal is installed (kitty, foot, ghostty, alacritty…)")?]),
        "files" => spawn(&["xdg-open", &store::home().to_string_lossy()]),
        "browser" => {
            let d = out(&["xdg-settings", "get", "default-web-browser"]).1;
            if d.is_empty() { spawn(&["xdg-open", "about:blank"]) } else { spawn(&["gtk-launch", d.trim_end_matches(".desktop")]) || spawn(&["xdg-open", "about:blank"]) }
        }
        w => return Err(format!("open {w}: terminal, files or browser")),
    };
    ok.then_some(()).ok_or(format!("could not start the {what}"))
}

/// run a command in a terminal window and keep it open when it ends
pub fn term(cmd: &[&str]) -> Result<(), String> {
    let t = terminal().ok_or("no terminal is installed")?;
    let quoted: Vec<String> = cmd.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect();
    let line = format!("{}; printf '\\n[done: press Enter]'; read _", quoted.join(" "));
    let flag = if t == "gnome-terminal" { "--" } else { "-e" };
    spawn(&[&t, flag, "sh", "-c", &line]).then_some(()).ok_or("could not start the terminal".into())
}

pub fn shell(verb: &str, arg: Option<&str>) -> Result<(), String> {
    let qs = if have("qs") { "qs" } else { "quickshell" };
    match verb {
        "restart" => {
            let _ = out(&["pkill", "-f", "(qs|quickshell) -c samurai"]);
            spawn(&[qs, "-c", "samurai"]).then_some(()).ok_or("quickshell is not installed".into())
        }
        v if v.chars().all(|c| c.is_ascii_alphanumeric() || c == ':' || c == '_') => {
            let (f, a) = v.split_once(':').map(|(f, a)| (f, Some(a))).unwrap_or((v, arg));
            let mut c = vec![qs, "-c", "samurai", "ipc", "call", "frame", f];
            if let Some(a) = a {
                c.push(a);
            }
            out(&c).0.then_some(()).ok_or("the frame is not running (samurai shell restart)".into())
        }
        v => Err(format!("shell {v}: not a frame command")),
    }
}

/// what niri runs at login: the frame, the seal on first login, the generated config in step
pub fn session_start() -> Result<(), String> {
    if !crate::seal::exists() {
        let _ = crate::seal::forge(false);
    }
    let _ = store::set_state("incognito", json!(false));
    let _ = tidy();
    let qs = if have("qs") { "qs" } else { "quickshell" };
    spawn(&[qs, "-c", "samurai"]).then_some(()).ok_or("quickshell is not installed".into())
}

// ── saved looks ──────────────────────────────────────────────────────────────────────────────
const LOOK_PREFIXES: [&str; 4] = ["look.", "frame.", "motion.", "widgets."];

fn look_file(name: &str) -> Result<std::path::PathBuf, String> {
    let ok = !name.is_empty() && name.len() <= 48 && name.chars().all(|c| c.is_alphanumeric() || " -_".contains(c));
    ok.then(|| store::config_dir().join("looks").join(format!("{name}.json"))).ok_or("a look's name: letters, digits, space, - and _".into())
}

pub fn look(args: &[&str]) -> Result<String, String> {
    match args {
        ["list"] => {
            let mut v: Vec<String> = std::fs::read_dir(store::config_dir().join("looks")).map(|rd| rd.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".json").map(String::from)).collect()).unwrap_or_default();
            v.sort();
            Ok(json!(v).to_string())
        }
        ["save", name] => {
            let u: serde_json::Map<String, Value> = store::user().into_iter().filter(|(k, _)| LOOK_PREFIXES.iter().any(|p| k.starts_with(p)) && k != "look.saved").collect();
            store::atomic_write(&look_file(name)?, &(serde_json::to_string_pretty(&u).unwrap() + "\n")).map_err(|e| e.to_string())?;
            Ok(format!("saved look \"{name}\" ({} settings)", u.len()))
        }
        ["apply", name] => {
            let t = std::fs::read_to_string(look_file(name)?).map_err(|_| format!("no look named \"{name}\""))?;
            let look: serde_json::Map<String, Value> = serde_json::from_str(&t).map_err(|e| e.to_string())?;
            let mut u = store::user();
            u.retain(|k, _| !LOOK_PREFIXES.iter().any(|p| k.starts_with(p)));
            let mut n = 0;
            for (k, v) in look {
                // a look from elsewhere is data: only known look settings with valid values get in
                if LOOK_PREFIXES.iter().any(|p| k.starts_with(p)) && store::item(&k).is_some_and(|i| store::check(&i, &v).is_ok()) {
                    u.insert(k, v);
                    n += 1;
                }
            }
            store::save(&u)?;
            crate::niri::apply()?;
            Ok(format!("applied look \"{name}\" ({n} settings)"))
        }
        ["export", name] => std::fs::read_to_string(look_file(name)?).map_err(|_| format!("no look named \"{name}\"")),
        ["delete", name] => std::fs::remove_file(look_file(name)?).map(|_| format!("deleted look \"{name}\"")).map_err(|e| e.to_string()),
        _ => Err("look list | save NAME | apply NAME | export NAME | delete NAME".into()),
    }
}

// ── store: a git catalogue of looks and plugins ──────────────────────────────────────────────
fn catalogue_dir() -> std::path::PathBuf {
    store::state_dir().join("catalogue")
}
fn sha256_file(p: &std::path::Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    std::fs::read(p).ok().map(|b| Sha256::digest(&b).iter().map(|x| format!("{x:02x}")).collect())
}
fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn store_cmd(args: &[&str]) -> Result<String, String> {
    match args {
        ["refresh"] => {
            let url = store::get_str("store.catalogue");
            if !(url.starts_with("https://") || url.starts_with("/")) || url.contains(' ') {
                return Err("set a catalogue first: an https:// git URL (Settings › Store)".into());
            }
            let dir = catalogue_dir();
            let _ = std::fs::remove_dir_all(&dir);
            let ok = Command::new("git").args(["-c", "protocol.file.allow=always", "clone", "-q", "--depth", "1", "--", &url, &dir.to_string_lossy()]).env("GIT_TERMINAL_PROMPT", "0").status().map(|s| s.success()).unwrap_or(false);
            ok.then(|| "catalogue refreshed".to_string()).ok_or("could not fetch the catalogue".into())
        }
        ["list"] => {
            let t = std::fs::read_to_string(catalogue_dir().join("index.json")).map_err(|_| "no catalogue yet: refresh first")?;
            let v: Value = serde_json::from_str(&t).map_err(|e| format!("the catalogue's index.json: {e}"))?;
            let installed = |kind: &str, id: &str| match kind {
                "look" => store::config_dir().join("looks").join(format!("{id}.json")).exists(),
                _ => store::config_dir().join("plugins").join(id).is_dir(),
            };
            let items: Vec<Value> = v["items"].as_array().cloned().unwrap_or_default().into_iter().filter(|i| i["id"].as_str().is_some_and(safe_id)).map(|mut i| {
                let (k, id) = (i["kind"].as_str().unwrap_or("").to_string(), i["id"].as_str().unwrap().to_string());
                i["installed"] = json!(installed(&k, &id));
                i
            }).collect();
            Ok(json!(items).to_string())
        }
        ["install", id] if safe_id(id) => {
            let t = std::fs::read_to_string(catalogue_dir().join("index.json")).map_err(|_| "no catalogue yet: refresh first")?;
            let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
            let it = v["items"].as_array().and_then(|a| a.iter().find(|i| i["id"] == *id)).ok_or(format!("{id} is not in the catalogue"))?;
            let verify = store::get_bool("store.verify");
            let files = it["files"].as_object().cloned().unwrap_or_default();
            if verify && files.is_empty() {
                return Err(format!("{id} lists no checksums, and \"only install entries with a checksum\" is on"));
            }
            let kind = it["kind"].as_str().unwrap_or("");
            let (src, dst) = match kind {
                "look" => (catalogue_dir().join("looks"), store::config_dir().join("looks")),
                "plugin" => (catalogue_dir().join("plugins").join(id), store::config_dir().join("plugins").join(id)),
                k => return Err(format!("{id}: unknown kind {k}")),
            };
            for (rel, want) in &files {
                if rel.contains("..") || rel.starts_with('/') {
                    return Err(format!("{id}: bad path {rel}"));
                }
                if verify && sha256_file(&src.join(rel)).as_deref() != want.as_str() {
                    return Err(format!("{id}: {rel} does not match its checksum; nothing was installed"));
                }
            }
            for rel in files.keys() {
                let to = dst.join(rel);
                if let Some(d) = to.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                std::fs::copy(src.join(rel), &to).map_err(|e| format!("{rel}: {e}"))?;
            }
            Ok(format!("installed {kind} {id}"))
        }
        ["remove", id] if safe_id(id) => {
            let p = store::config_dir().join("plugins").join(id);
            let l = store::config_dir().join("looks").join(format!("{id}.json"));
            if p.is_dir() {
                std::fs::remove_dir_all(p).map_err(|e| e.to_string())?;
            } else if l.exists() {
                std::fs::remove_file(l).map_err(|e| e.to_string())?;
            } else {
                return Err(format!("{id} is not installed"));
            }
            Ok(format!("removed {id}"))
        }
        _ => Err("store refresh | list | install ID | remove ID".into()),
    }
}

/// installed plugins: id, name, entry file
pub fn plugins() -> Value {
    let dir = store::config_dir().join("plugins");
    let enabled: Vec<String> = store::get_list("plugins.enabled").iter().filter_map(|r| r["id"].as_str().map(String::from)).collect();
    let mut v = vec![];
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let meta: Value = std::fs::read_to_string(e.path().join("plugin.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
        let entry = meta["entry"].as_str().unwrap_or("Main.qml");
        if safe_id(&id) && !entry.contains("..") && e.path().join(entry).is_file() {
            v.push(json!({ "id": id, "name": meta["name"].as_str().unwrap_or(&id), "about": meta["about"], "slot": meta["slot"].as_str().unwrap_or("desktop"), "path": e.path().join(entry), "enabled": enabled.contains(&id) }));
        }
    }
    json!(v)
}

// ── updates ──────────────────────────────────────────────────────────────────────────────────
pub const DEFAULT_REPO: &str = match option_env!("SAMURAI_REPO") {
    Some(r) => r,
    None => "https://github.com/namidelacruz/samurai.git",
};

fn installed_commit() -> String {
    std::fs::read_to_string("/opt/samurai/share/COMMIT").unwrap_or_default().trim().to_string()
}

pub fn update(args: &[&str]) -> Result<String, String> {
    let repo = { let r = store::get_str("updates.repo"); if r.is_empty() { DEFAULT_REPO.to_string() } else { r } };
    let branch = { let b = store::get_str("updates.branch"); if b.is_empty() { "main".to_string() } else { b } };
    if !(repo.starts_with("https://") || repo.starts_with('/')) || repo.contains('@') || repo.contains(' ') || !branch.chars().all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c)) {
        return Err("the update repository must be an https:// URL without credentials".into());
    }
    let git = |a: &[&str]| Command::new("git").args(["-c", "credential.helper=", "-c", "protocol.file.allow=always"]).args(a).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stderr(Stdio::null()).output();
    match args {
        ["check"] => {
            let o = git(&["ls-remote", "--", &repo, &format!("refs/heads/{branch}")]).map_err(|e| e.to_string())?;
            let head = String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap_or("").to_string();
            if !o.status.success() || head.len() < 40 {
                return Err("could not reach the update repository".into());
            }
            let have = installed_commit();
            let available = !have.is_empty() && have != head;
            store::set_state("update", json!({ "available": available, "commit": &head[..12], "checked": stamp() }))?;
            Ok(if available { format!("update available: {}", &head[..12]) } else if have.is_empty() { "not installed from a package: nothing to compare".into() } else { "up to date".into() })
        }
        // fetch a clean copy and hand over to its installer (one status bar; detail in the log)
        ["install"] => {
            let dir = store::state_dir().join("update-src");
            let _ = std::fs::remove_dir_all(&dir);
            let o = git(&["clone", "-q", "--depth", "1", "--branch", &branch, "--", &repo, &dir.to_string_lossy()]).map_err(|e| e.to_string())?;
            if !o.status.success() {
                return Err("could not download the update".into());
            }
            let st = Command::new("sh").arg(dir.join("install.sh")).arg("--update").current_dir(&dir).status().map_err(|e| e.to_string())?;
            if !st.success() {
                return Err("the update did not install; the previous version is still in place (see ~/.local/state/samurai/install.log)".into());
            }
            store::set_state("update", json!({ "available": false, "checked": stamp() }))?;
            Ok("updated".into())
        }
        _ => Err("update check | install".into()),
    }
}

// ── capture ──────────────────────────────────────────────────────────────────────────────────
pub fn shot(mode: &str) -> Result<String, String> {
    if !have("grim") {
        return Err("screenshots need grim (and slurp for areas)".into());
    }
    let dir = expand(&store::get_str("shot.dir"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("shot_{}.png", stamp()));
    let f = file.to_string_lossy().to_string();
    let ok = match mode {
        "screen" => out(&["grim", &f]).0,
        "area" => {
            let (ok, geo) = out(&["slurp"]);
            ok && !geo.is_empty() && out(&["grim", "-g", &geo, &f]).0
        }
        m => return Err(format!("shot {m}: area or screen")),
    };
    if !ok {
        return Err("no screenshot taken".into());
    }
    if store::get_bool("shot.copy") && have("wl-copy") {
        sh(&format!("wl-copy --type image/png < '{}'", f.replace('\'', "'\\''")));
    }
    Ok(f)
}

pub fn record(arg: &str) -> Result<String, String> {
    let running = store::state().get("recording").and_then(|v| v.as_u64()).filter(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists());
    match (arg, running) {
        ("toggle" | "stop", Some(pid)) => {
            let _ = out(&["kill", "-INT", &pid.to_string()]);
            store::set_state("recording", Value::Null)?;
            Ok("recording saved".into())
        }
        ("stop", None) => Ok("not recording".into()),
        ("toggle" | "start", None) => {
            let dir = expand(&store::get_str("record.dir"));
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let file = dir.join(format!("rec_{}.mp4", stamp())).to_string_lossy().to_string();
            let (fps, codec) = (store::get_str("record.fps"), store::get_str("record.codec"));
            let gpu = store::get_str("record.encoder") == "gpu";
            let mut c: Vec<String> = if have("gpu-screen-recorder") && gpu {
                let mut c: Vec<String> = ["gpu-screen-recorder", "-w", "screen", "-f", &fps, "-k", &codec, "-q", &store::get_str("record.quality"), "-fm", if store::get_str("record.rate_mode") == "variable" { "vfr" } else { "cfr" }, "-cursor", if store::get_bool("record.cursor") { "yes" } else { "no" }].iter().map(|s| s.to_string()).collect();
                if store::get_bool("record.audio") {
                    c.extend(["-a".into(), "default_output".into()]);
                }
                if store::get_bool("record.mic") {
                    c.extend(["-a".into(), "default_input".into()]);
                }
                c.push("-o".into());
                c
            } else if have("wf-recorder") {
                let enc = match (codec.as_str(), gpu) { ("hevc", true) => "hevc_vaapi", ("av1", true) => "av1_vaapi", (_, true) => "h264_vaapi", ("hevc", false) => "libx265", ("av1", false) => "libsvtav1", _ => "libx264" };
                let mut c: Vec<String> = ["wf-recorder", "-r", &fps, "-c", enc].iter().map(|s| s.to_string()).collect();
                if store::get_bool("record.audio") {
                    c.push("--audio".into());
                }
                c.push("-f".into());
                c
            } else {
                return Err("recording needs gpu-screen-recorder or wf-recorder".into());
            };
            c.push(file.clone());
            let child = Command::new(&c[0]).args(&c[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
            store::set_state("recording", json!(child.id()))?;
            Ok(format!("recording to {file}"))
        }
        ("start", Some(_)) => Ok("already recording".into()),
        (a, _) => Err(format!("record {a}: toggle, start or stop")),
    }
}

// ── GPU passthrough readiness (read-only) ────────────────────────────────────────────────────
pub fn gpu_check() -> Value {
    let groups = std::fs::read_dir("/sys/kernel/iommu_groups").map(|rd| rd.count()).unwrap_or(0);
    let cmdline = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();
    let mut gpus = vec![];
    for e in std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with("card") && !n.contains('-') {
            let r = |f: &str| std::fs::read_to_string(e.path().join("device").join(f)).unwrap_or_default().trim().to_string();
            let vendor = match r("vendor").as_str() { "0x8086" => "Intel", "0x10de" => "NVIDIA", "0x1002" => "AMD", v => v }.to_string();
            let driver = std::fs::read_link(e.path().join("device/driver")).ok().and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string())).unwrap_or_default();
            gpus.push(json!({ "card": n, "vendor": vendor, "driver": driver, "boot": r("boot_vga") == "1" }));
        }
    }
    let ready = groups > 0 && gpus.len() >= 2;
    json!({
        "iommu_on": groups > 0, "iommu_groups": groups, "kernel_flag": cmdline.contains("iommu="), "gpus": gpus, "ready": ready,
        "advice": if groups == 0 { "IOMMU is off: turn on VT-d / AMD-Vi in the firmware setup, then add intel_iommu=on or amd_iommu=on to the kernel command line." } else if gpus.len() < 2 { "Passthrough needs a second GPU to keep the desktop on." } else { "Ready: the discrete GPU can be reserved for virtual machines." },
    })
}
