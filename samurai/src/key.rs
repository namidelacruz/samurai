//! A plain USB stick as a login key.
//!
//! Enrolling writes a random 64-byte secret to a file on the stick and keeps only its SHA-256 on
//! the computer (~/.config/samurai/usbkey.json, readable by you alone), with the stick's
//! filesystem UUID and serial number. Checking finds that stick, reads the secret, compares the
//! hash, and then replaces the secret with a new one: a copy of the stick made earlier stops
//! working the next time the real one is used (one-time pad). The last two hashes are accepted,
//! so a stick pulled out half-way through a rotation still works once more.
//!
//! Honestly: a plain stick can be copied, and a copy made and used before you next use yours will
//! work once. A FIDO2 security key cannot be copied at all; use one if you can.
//!
//! The checker is what PAM runs (pam_exec): `samurai key check` exits 0 for "this user's stick is
//! here and right", anything else otherwise, within a few seconds, and never asks anything.

use crate::store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

const KEY_DIR: &str = ".samurai-key";

extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn random(n: usize) -> Result<Vec<u8>, String> {
    let mut v = vec![0u8; n];
    use std::io::Read;
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut v)).map_err(|e| e.to_string())?;
    Ok(v)
}

fn hash(secret: &[u8]) -> String {
    hex(&Sha256::digest(secret))
}

/// Compare without leaking how much matched.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn record_path(home: &Path) -> PathBuf {
    // always in the home of the user being checked (PAM runs this as root for another user)
    home.join(".config/samurai/usbkey.json")
}

fn machine() -> String {
    std::fs::read_to_string("/etc/machine-id").map(|s| s.trim().chars().take(12).collect()).unwrap_or_else(|_| "machine".into())
}

/// A USB stick's filesystem: (device, fs UUID, serial, where it is mounted now).
#[derive(Clone, Debug)]
pub struct Stick {
    pub dev: String,
    pub uuid: String,
    pub serial: String,
    pub mount: Option<PathBuf>,
    pub label: String,
}

/// USB storage partitions with a filesystem. Tests: SAMURAI_KEY_STICKS points to a folder whose
/// subfolders stand in for mounted sticks ("<uuid>__<serial>").
pub fn sticks() -> Vec<Stick> {
    if let Some(d) = std::env::var_os("SAMURAI_KEY_STICKS") {
        return std::fs::read_dir(d).map(|rd| rd.flatten().filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            let (u, s) = n.split_once("__")?;
            Some(Stick { dev: format!("/dev/test-{u}"), uuid: u.into(), serial: s.into(), mount: Some(e.path()), label: n.clone() })
        }).collect()).unwrap_or_default();
    }
    let Ok(o) = Command::new("lsblk").args(["-J", "-p", "-o", "PATH,UUID,SERIAL,TRAN,MOUNTPOINT,LABEL,FSTYPE,TYPE"]).output() else { return vec![] };
    let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
    let mut out = vec![];
    fn walk(n: &Value, serial: &str, usb: bool, out: &mut Vec<Stick>) {
        let s = n["serial"].as_str().filter(|s| !s.is_empty()).unwrap_or(serial).to_string();
        let usb = usb || n["tran"].as_str() == Some("usb");
        if usb && n["fstype"].as_str().is_some_and(|f| !f.is_empty()) && n["uuid"].as_str().is_some_and(|u| !u.is_empty()) {
            out.push(Stick {
                dev: n["path"].as_str().unwrap_or("").into(),
                uuid: n["uuid"].as_str().unwrap_or("").into(),
                serial: s.clone(),
                mount: n["mountpoint"].as_str().filter(|m| !m.is_empty()).map(PathBuf::from),
                label: n["label"].as_str().unwrap_or("").into(),
            });
        }
        if let Some(ch) = n["children"].as_array() {
            for c in ch {
                walk(c, &s, usb, out);
            }
        }
    }
    for d in v["blockdevices"].as_array().cloned().unwrap_or_default() {
        walk(&d, "", false, &mut out);
    }
    out
}

/// Somewhere the stick can be read and written: where it is mounted already, or (as root, at the
/// login screen) a private mount of our own that is undone afterwards.
struct Opened {
    dir: PathBuf,
    ours: bool,
}

fn open(s: &Stick) -> Result<Opened, String> {
    if let Some(m) = &s.mount {
        return Ok(Opened { dir: m.clone(), ours: false });
    }
    let dir = std::env::temp_dir().join(format!("samurai-key-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ok = Command::new("mount").args(["-o", "nosuid,nodev,noexec", &s.dev, &dir.to_string_lossy()]).status().is_ok_and(|st| st.success());
    if !ok {
        let _ = std::fs::remove_dir(&dir);
        return Err("the stick could not be opened (not mounted, and mounting needs root)".into());
    }
    Ok(Opened { dir, ours: true })
}

fn close(o: Opened) {
    let _ = Command::new("sync").status();
    if o.ours {
        let _ = Command::new("umount").arg(&o.dir).status();
        let _ = std::fs::remove_dir(&o.dir);
    }
}

fn secret_file(dir: &Path, user: &str) -> PathBuf {
    dir.join(KEY_DIR).join(format!("{user}-{}", machine()))
}

fn write_secret(dir: &Path, user: &str, secret: &[u8]) -> Result<(), String> {
    let f = secret_file(dir, user);
    std::fs::create_dir_all(f.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("new");
    std::fs::write(&tmp, hex(secret)).map_err(|e| e.to_string())?;
    std::fs::File::open(&tmp).and_then(|h| h.sync_all()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())
}

fn save_record(home: &Path, rec: &Value) -> Result<(), String> {
    let p = record_path(home);
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    store::atomic_write(&p, &(serde_json::to_string_pretty(rec).unwrap() + "\n")).map_err(|e| e.to_string())?;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    // written as root (the login screen's check): it stays yours, so the lock screen can read it
    if let Ok(m) = std::fs::metadata(home) {
        if unsafe { libc_geteuid() } == 0 {
            let _ = std::os::unix::fs::chown(&p, Some(m.uid()), Some(m.gid()));
        }
    }
    Ok(())
}

/// Make this stick the user's key. Needs the stick mounted (as it is after plugging it in).
pub fn enroll(user: &str, home: &Path, uuid: &str) -> Result<String, String> {
    let s = sticks().into_iter().find(|s| s.uuid == uuid).ok_or("that stick is not plugged in")?;
    let o = open(&s)?;
    let secret = random(64)?;
    let r = write_secret(&o.dir, user, &secret);
    close(o);
    r?;
    save_record(home, &json!({ "uuid": s.uuid, "serial": s.serial, "hashes": [hash(&secret)], "label": s.label }))?;
    Ok(format!("the stick {} is now your key", if s.label.is_empty() { &s.uuid } else { &s.label }))
}

/// Is this user's stick here and right? On success the secret is replaced (one-time pad).
pub fn check(user: &str, home: &Path, rotate: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(record_path(home)).map_err(|_| "no key enrolled")?;
    let mut rec: Value = serde_json::from_str(&text).map_err(|_| "the key record is damaged")?;
    let uuid = rec["uuid"].as_str().unwrap_or("").to_string();
    let serial = rec["serial"].as_str().unwrap_or("").to_string();
    let s = sticks().into_iter().find(|s| s.uuid == uuid && (serial.is_empty() || s.serial == serial)).ok_or("the key stick is not plugged in")?;
    let o = open(&s)?;
    let got = std::fs::read_to_string(secret_file(&o.dir, user)).unwrap_or_default();
    let bytes: Vec<u8> = (0..got.trim().len() / 2).filter_map(|i| u8::from_str_radix(got.trim().get(2 * i..2 * i + 2)?, 16).ok()).collect();
    let h = hash(&bytes);
    let known: Vec<String> = rec["hashes"].as_array().cloned().unwrap_or_default().iter().filter_map(|v| v.as_str().map(String::from)).collect();
    if bytes.len() != 64 || !known.iter().any(|k| same(k, &h)) {
        close(o);
        return Err("this stick does not hold the right key".into());
    }
    if rotate {
        // the new hash is remembered before the stick is written, and the old one is kept, so a
        // stick pulled out half-way still works once more
        let next = random(64)?;
        rec["hashes"] = json!([hash(&next), h]);
        save_record(home, &rec)?;
        let w = write_secret(&o.dir, user, &next);
        close(o);
        w?;
    } else {
        close(o);
    }
    Ok(())
}

pub fn forget(home: &Path) -> Result<String, String> {
    match std::fs::remove_file(record_path(home)) {
        Ok(()) => Ok("this computer no longer accepts the stick".into()),
        Err(_) => Ok("no stick was enrolled".into()),
    }
}

/// The user and home PAM is asking about (pam_exec sets PAM_USER), or the one running this.
pub fn who() -> (String, PathBuf) {
    let user = std::env::var("PAM_USER").ok().filter(|u| !u.is_empty()).or_else(|| std::env::var("USER").ok()).unwrap_or_default();
    let home = Command::new("getent").args(["passwd", &user]).output().ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).split(':').nth(5).map(|h| PathBuf::from(h.trim())))
        .filter(|h| h.is_dir())
        .unwrap_or_else(store::home);
    (user, home)
}

pub fn list() -> String {
    sticks().iter().map(|s| json!({ "uuid": s.uuid, "serial": s.serial, "label": s.label, "dev": s.dev, "mounted": s.mount.is_some() }).to_string()).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_stick_is_a_one_time_key() {
        let base = std::env::temp_dir().join(format!("samurai-key-test-{}", std::process::id()));
        let sticks_dir = base.join("sticks");
        let stick = sticks_dir.join("AAAA-1111__SER123");
        let other = sticks_dir.join("BBBB-2222__SER999");
        std::fs::create_dir_all(&stick).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::env::set_var("SAMURAI_KEY_STICKS", &sticks_dir);
        let home = base.join("home");
        assert!(check("nami", &home, true).is_err(), "nothing enrolled: no");
        enroll("nami", &home, "AAAA-1111").unwrap();
        // a copy of the stick as it is now
        let copy = std::fs::read_to_string(secret_file(&stick, "nami")).unwrap();
        assert!(check("nami", &home, true).is_ok(), "the real stick opens");
        assert!(check("nami", &home, true).is_ok(), "and again (the secret moved on)");
        std::fs::write(secret_file(&stick, "nami"), &copy).unwrap();
        assert!(check("nami", &home, true).is_err(), "the old copy no longer opens");
        assert!(check("someone-else", &home, true).is_err(), "another user's name: no");
        std::fs::remove_dir_all(&stick).unwrap();
        assert!(check("nami", &home, true).is_err(), "stick pulled out: no (fast)");
        std::fs::remove_dir_all(&base).ok();
    }
}
