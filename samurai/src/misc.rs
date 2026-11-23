//! The smaller commands: status (for Settings) and updates.
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
        "ground": crate::x11::ground(),
        "accent": store::get_str("look.accent"),
        "update": st.get("update").cloned().unwrap_or(json!({ "available": false })),
        "seal": crate::seal::show(),
        "installed": installed_commit(),
    })
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
    let branch = "main".to_string();
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


pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
