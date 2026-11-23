//! A small socket so the Settings app, scripts and key daemons can talk to the WM:
//!   samurai-wm msg overview | lock | lock --wait | reload | ground | close | snap left | state …
//! One line in, one line (or a few) out.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

pub fn socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("SAMURAI_WM_SOCKET") {
        return PathBuf::from(p);
    }
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/tmp/samurai-{}", unsafe { libc::getuid() }));
    let display = std::env::var("DISPLAY").unwrap_or_default().replace([':', '.', '/'], "_");
    PathBuf::from(dir).join(format!("samurai-wm{display}.sock"))
}

pub struct Server {
    listener: UnixListener,
}

pub struct Request {
    pub line: String,
    pub stream: UnixStream,
}

impl Server {
    pub fn bind() -> Option<Self> {
        let p = socket_path();
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).ok();
        }
        // a socket left by a WM that died is removed; a live one is refused
        if UnixStream::connect(&p).is_ok() {
            eprintln!("samurai-wm: another Samurai is listening on {}", p.display());
            return None;
        }
        let _ = std::fs::remove_file(&p);
        let listener = UnixListener::bind(&p).ok()?;
        listener.set_nonblocking(true).ok()?;
        Some(Server { listener })
    }

    pub fn fd(&self) -> i32 {
        use std::os::unix::io::AsRawFd;
        self.listener.as_raw_fd()
    }

    pub fn accept(&self) -> Vec<Request> {
        let mut out = vec![];
        while let Ok((s, _)) = self.listener.accept() {
            s.set_nonblocking(false).ok();
            s.set_read_timeout(Some(Duration::from_millis(300))).ok();
            let mut r = BufReader::new(match s.try_clone() { Ok(c) => c, Err(_) => continue });
            let mut line = String::new();
            if r.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                out.push(Request { line: line.trim().to_string(), stream: s });
            }
        }
        out
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(socket_path());
    }
}

/// The client side: `samurai-wm msg …`.
pub fn send(words: &[String]) -> i32 {
    let line = words.join(" ");
    let mut s = match UnixStream::connect(socket_path()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("samurai-wm is not running here ({}): {e}", socket_path().display());
            return 1;
        }
    };
    if s.write_all(format!("{line}\n").as_bytes()).is_err() {
        return 1;
    }
    let wait = line == "lock --wait";
    let mut buf = String::new();
    let mut r = BufReader::new(&mut s);
    loop {
        buf.clear();
        match r.read_line(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let l = buf.trim_end();
                if wait && l == "locked" {
                    // xss-lock: the screen is locked, let the machine go to sleep now
                    if let Ok(fd) = std::env::var("XSS_SLEEP_LOCK_FD").map(|v| v.parse::<i32>().unwrap_or(-1)) {
                        if fd >= 0 {
                            unsafe { libc::close(fd) };
                        }
                    }
                    continue;
                }
                if wait && l == "unlocked" {
                    return 0;
                }
                if let Some(e) = l.strip_prefix("error: ") {
                    eprintln!("{e}");
                    return 2;
                }
                println!("{l}");
                if !wait {
                    let mut rest = String::new();
                    let _ = r.read_to_string(&mut rest);
                    print!("{rest}");
                    break;
                }
            }
        }
    }
    0
}
