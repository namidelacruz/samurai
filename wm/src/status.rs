//! What the quick panel shows (battery, Wi-Fi, sound, brightness…) and the commands its tiles
//! and sliders run. Read in a background thread so a slow tool never stalls the screen.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One Wi-Fi network in range.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Net {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
    pub active: bool,
    pub known: bool,
    /// the saved NetworkManager profile for it, when known (Samurai's are named "SN <ssid>")
    pub profile: String,
}

/// One paired Bluetooth device.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BtDev {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// networks and devices: read only while their list is open (`Status::detail`)
    pub nets: Vec<Net>,
    pub bt_devs: Vec<BtDev>,
    pub have_nm: bool,
    pub have_bt: bool,
    pub battery: Option<(u8, bool)>,
    pub wifi_on: bool,
    pub ssid: Option<String>,
    pub wired: bool,
    pub bt_on: bool,
    pub volume: Option<u8>,
    pub muted: bool,
    pub brightness: Option<u8>,
    pub quiet: bool,
    pub night_light: bool,
    pub user: String,
    pub host: String,
}

pub struct Status {
    pub snap: Arc<Mutex<Snapshot>>,
    kick: Arc<Mutex<bool>>,
    /// read the network / device lists too (a list is open in the panel)
    pub detail: Arc<AtomicBool>,
    /// the last thing a job said: (text, is it a problem)
    pub note: Arc<Mutex<Option<(String, bool)>>>,
}

/// nmcli -t separates fields with ':' and writes a ':' inside a field as '\:'.
pub fn nm_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut esc = false;
    for ch in line.chars() {
        if esc {
            out.last_mut().unwrap().push(ch);
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else if ch == ':' {
            out.push(String::new());
        } else {
            out.last_mut().unwrap().push(ch);
        }
    }
    out
}

/// Networks in range, strongest first, one line per name (the strongest access point).
pub fn parse_nets(list: &str, known: &str) -> Vec<Net> {
    // saved profiles: (the network's name, the profile's name)
    let known: Vec<(String, String)> = known.lines().map(nm_fields).filter(|f| f.len() >= 2 && f[1].contains("wireless"))
        .map(|f| (f[0].strip_prefix("SN ").unwrap_or(&f[0]).to_string(), f[0].clone())).collect();
    let mut nets: Vec<Net> = vec![];
    for l in list.lines() {
        let f = nm_fields(l);
        if f.len() < 4 || f[1].is_empty() {
            continue; // hidden networks have no name: Settings › Network joins those
        }
        let n = Net {
            ssid: f[1].clone(),
            signal: f[2].parse().unwrap_or(0),
            secure: !f[3].trim().is_empty() && f[3].trim() != "--",
            active: f[0].trim() == "*",
            known: known.iter().any(|k| k.0 == f[1]),
            profile: known.iter().find(|k| k.0 == f[1]).map(|k| k.1.clone()).unwrap_or_default(),
        };
        match nets.iter_mut().find(|o| o.ssid == n.ssid) {
            Some(o) => {
                o.active |= n.active;
                if n.signal > o.signal { o.signal = n.signal; }
            }
            None => nets.push(n),
        }
    }
    nets.sort_by(|a, b| b.active.cmp(&a.active).then(b.known.cmp(&a.known)).then(b.signal.cmp(&a.signal)));
    nets.truncate(12);
    nets
}

/// Paired devices from `bluetoothctl devices Paired` plus which are connected.
pub fn parse_bt(paired: &str, connected: &str) -> Vec<BtDev> {
    let line = |l: &str| -> Option<(String, String)> {
        let mut it = l.splitn(3, ' ');
        (it.next()? == "Device").then_some(())?;
        Some((it.next()?.to_string(), it.next().unwrap_or("").trim().to_string()))
    };
    let conn: Vec<String> = connected.lines().filter_map(line).map(|d| d.0).collect();
    paired.lines().filter_map(line).map(|(mac, name)| BtDev { connected: conn.contains(&mac), name: if name.is_empty() { mac.clone() } else { name }, mac }).take(10).collect()
}

fn out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).to_string())
}

fn read(detail: bool) -> Snapshot {
    let mut s = Snapshot::default();
    s.have_nm = have("nmcli");
    s.have_bt = have("bluetoothctl");
    if detail && s.have_nm {
        let list = out("nmcli", &["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list"]).unwrap_or_default();
        let known = out("nmcli", &["-t", "-f", "NAME,TYPE", "connection", "show"]).unwrap_or_default();
        s.nets = parse_nets(&list, &known);
    }
    if detail && s.have_bt {
        let paired = out("bluetoothctl", &["devices", "Paired"]).unwrap_or_default();
        let connected = out("bluetoothctl", &["devices", "Connected"]).unwrap_or_default();
        s.bt_devs = parse_bt(&paired, &connected);
    }
    // battery from sysfs: no tool needed
    if let Ok(dir) = std::fs::read_dir("/sys/class/power_supply") {
        for e in dir.flatten() {
            let p = e.path();
            let kind = std::fs::read_to_string(p.join("type")).unwrap_or_default();
            if kind.trim() == "Battery" {
                let cap = std::fs::read_to_string(p.join("capacity")).ok().and_then(|v| v.trim().parse::<u8>().ok());
                let st = std::fs::read_to_string(p.join("status")).unwrap_or_default();
                if let Some(c) = cap {
                    s.battery = Some((c, st.trim() == "Charging" || st.trim() == "Full"));
                }
                break;
            }
        }
    }
    if let Some(t) = out("nmcli", &["-t", "-f", "WIFI", "radio"]) {
        s.wifi_on = t.trim() == "enabled";
    }
    if let Some(t) = out("nmcli", &["-t", "-f", "TYPE,STATE,CONNECTION", "device"]) {
        for l in t.lines() {
            let f: Vec<&str> = l.splitn(3, ':').collect();
            if f.len() == 3 && f[1] == "connected" {
                match f[0] {
                    "wifi" => s.ssid = Some(f[2].to_string()),
                    "ethernet" => s.wired = true,
                    _ => {}
                }
            }
        }
    }
    if let Some(t) = out("bluetoothctl", &["show"]) {
        s.bt_on = t.contains("Powered: yes");
    }
    if let Some(t) = out("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
        // "Volume: 0.45 [MUTED]"
        s.muted = t.contains("MUTED");
        s.volume = t.split_whitespace().nth(1).and_then(|v| v.parse::<f32>().ok()).map(|v| (v * 100.0).round().clamp(0.0, 150.0) as u8);
    }
    if let Some(t) = out("brightnessctl", &["-m"]) {
        // "intel_backlight,backlight,400,40%,1000"
        s.brightness = t.split(',').nth(3).and_then(|v| v.trim().trim_end_matches('%').parse().ok());
    }
    if let Some(t) = out("dunstctl", &["is-paused"]) {
        s.quiet = t.trim() == "true";
    }
    s.night_light = out("pgrep", &["-x", "gammastep"]).is_some_and(|t| !t.trim().is_empty());
    s.user = std::env::var("USER").unwrap_or_default();
    s.host = std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_string()).unwrap_or_default();
    s
}

impl Status {
    pub fn start() -> Self {
        let snap = Arc::new(Mutex::new(Snapshot::default()));
        let kick = Arc::new(Mutex::new(true));
        let detail = Arc::new(AtomicBool::new(false));
        let (s2, k2, d2) = (snap.clone(), kick.clone(), detail.clone());
        std::thread::spawn(move || {
            let mut n = 0u32;
            loop {
                let now = *k2.lock().unwrap();
                if now || n % 8 == 0 {
                    *k2.lock().unwrap() = false;
                    let v = read(d2.load(Ordering::Relaxed));
                    *s2.lock().unwrap() = v;
                }
                n += 1;
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        Status { snap, kick, detail, note: Arc::new(Mutex::new(None)) }
    }
    pub fn take_note(&self) -> Option<(String, bool)> {
        self.note.lock().ok().and_then(|mut n| n.take())
    }
    /// Run a command in the background; when it ends, say how it went (in the panel) and read
    /// again. `input` is written to its stdin (a password: never on the command line).
    pub fn job(&self, args: Vec<String>, input: Option<String>, ok: String) {
        let note = self.note.clone();
        let kick = self.kick.clone();
        if let Ok(mut n) = note.lock() {
            *n = Some(("Working…".into(), false));
        }
        std::thread::spawn(move || {
            use std::io::Write;
            let child = Command::new(&args[0]).args(&args[1..]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn();
            let said = match child {
                Ok(mut c) => {
                    if let (Some(text), Some(mut sin)) = (input, c.stdin.take()) {
                        let _ = sin.write_all(text.as_bytes());
                        let _ = sin.write_all(b"\n");
                    }
                    match c.wait_with_output() {
                        Ok(o) if o.status.success() => (ok, false),
                        Ok(o) => {
                            let all = String::from_utf8_lossy(&o.stderr).to_string() + &String::from_utf8_lossy(&o.stdout);
                            (plain_error(&all), true)
                        }
                        Err(e) => (e.to_string(), true),
                    }
                }
                Err(e) => (format!("{} could not run ({e})", args[0]), true),
            };
            if let Ok(mut n) = note.lock() {
                *n = Some(said);
            }
            if let Ok(mut k) = kick.lock() {
                *k = true;
            }
        });
    }
    pub fn get(&self) -> Snapshot {
        self.snap.lock().map(|s| s.clone()).unwrap_or_default()
    }
    /// Read again soon (after a tile or a key changed something).
    pub fn refresh(&self) {
        if let Ok(mut k) = self.kick.lock() {
            *k = true;
        }
    }
}

/// The last useful line of a tool's error, in plain words where the usual ones are known.
pub fn plain_error(all: &str) -> String {
    let low = all.to_lowercase();
    if low.contains("secrets were required") || low.contains("wrong password") || low.contains("invalid key") || low.contains("802-1x") && low.contains("fail") {
        return "Wrong password, or the network refused it.".into();
    }
    if low.contains("no network with ssid") || low.contains("not found") {
        return "That network is out of range now.".into();
    }
    if low.contains("not authorized") || low.contains("permission") {
        return "Not allowed: this needs a password prompt (polkit).".into();
    }
    let last = all.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("It did not work.");
    let last = last.trim_start_matches("Error:").trim();
    let mut t: String = last.chars().take(90).collect();
    if last.chars().count() > 90 { t.push('…'); }
    t
}

/// Run a command in the background, detached from the WM. Output goes to the session log.
pub fn spawn(cmd: &str) {
    let cmd = cmd.to_string();
    std::thread::spawn(move || {
        let _ = Command::new("sh").arg("-c").arg(format!("exec {cmd}")).stdin(std::process::Stdio::null()).spawn().and_then(|mut c| c.wait());
    });
}

pub fn have(tool: &str) -> bool {
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| Path::new(d).join(tool).is_file())
}

pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// What joining a network runs. A password goes through stdin to `shadow`, never on a command line.
pub fn join_args(n: &Net, password: Option<&str>) -> Vec<String> {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    if n.known {
        v(&["nmcli", "connection", "up", "id", if n.profile.is_empty() { &n.ssid } else { &n.profile }])
    } else if password.is_some() {
        v(&["shadow", "wifi", "connect", &n.ssid, "--password-stdin"])
    } else {
        v(&["nmcli", "device", "wifi", "connect", &n.ssid])
    }
}

pub fn tile(name: &str, s: &Snapshot) -> Option<String> {
    Some(match name {
        "wifi" => format!("nmcli radio wifi {}", if s.wifi_on { "off" } else { "on" }),
        "bt" => format!("bluetoothctl power {}", if s.bt_on { "off" } else { "on" }),
        "quiet" => "dunstctl set-paused toggle".into(),
        "night" => if s.night_light { "pkill -x gammastep".into() } else { "gammastep -O 4000".into() },
        "ground" => "samurai ground toggle".into(),
        "shot" => "samurai shot area".into(),
        _ => return None,
    })
}

pub fn set_volume(pct: u8) -> String {
    format!("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ {pct}%")
}

pub fn set_brightness(pct: u8) -> String {
    format!("brightnessctl set {}%", pct.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn networks_from_nmcli() {
        let list = "*:Home\\:5G:80:WPA2\n:Cafe:60:\n:Home\\:5G:40:WPA2\n::30:WPA2\n:UC-Secure:70:WPA2 802.1X\n";
        let known = "Home\\:5G:802-11-wireless\nSN UC-Secure:802-11-wireless\nWired:802-3-ethernet\n";
        let n = parse_nets(list, known);
        assert_eq!(n.len(), 3, "hidden network left out, duplicates merged");
        assert_eq!(n[0].ssid, "Home:5G");
        assert!(n[0].active && n[0].secure && n[0].signal == 80);
        assert!(n[0].known, "an escaped name still matches");
        assert!(n[1].known && n[1].ssid == "UC-Secure" && n[1].profile == "SN UC-Secure", "Samurai's own profiles count");
        assert!(!n[2].secure && !n[2].known);
    }
    #[test]
    fn bluetooth_devices() {
        let d = parse_bt("Device AA:BB Headphones\nDevice CC:DD\nnoise\n", "Device AA:BB Headphones\n");
        assert_eq!(d.len(), 2);
        assert!(d[0].connected && d[0].name == "Headphones");
        assert!(!d[1].connected && d[1].name == "CC:DD");
    }
    #[test]
    fn errors_in_plain_words() {
        assert_eq!(plain_error("Error: Connection activation failed: Secrets were required, but not provided."), "Wrong password, or the network refused it.");
        assert!(plain_error("x\nError: something odd\n").starts_with("something odd"));
        let k = Net { ssid: "A".into(), known: true, profile: "SN A".into(), ..Default::default() };
        assert_eq!(join_args(&k, None), vec!["nmcli", "connection", "up", "id", "SN A"]);
        let u = Net { ssid: "B".into(), secure: true, ..Default::default() };
        assert!(join_args(&u, Some("pw")).iter().all(|a| a != "pw"), "the password is never an argument");
    }
}
