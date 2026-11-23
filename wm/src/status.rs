//! What the quick panel shows (battery, Wi-Fi, sound, brightness…) and the commands its tiles
//! and sliders run. Read in a background thread so a slow tool never stalls the screen.

use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
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
}

fn out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).to_string())
}

fn read() -> Snapshot {
    let mut s = Snapshot::default();
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
        let (s2, k2) = (snap.clone(), kick.clone());
        std::thread::spawn(move || {
            let mut n = 0u32;
            loop {
                let now = *k2.lock().unwrap();
                if now || n % 8 == 0 {
                    *k2.lock().unwrap() = false;
                    let v = read();
                    *s2.lock().unwrap() = v;
                }
                n += 1;
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        Status { snap, kick }
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
