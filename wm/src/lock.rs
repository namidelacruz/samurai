//! The lock screen, drawn by the compositor itself: while locked, no window is painted at all,
//! and the keyboard and pointer are held by the WM. The password is checked with PAM in a
//! separate thread. If the WM dies the X session ends with it, so a crash can never leave an
//! unlocked screen behind.

use crate::draw::{self, Face};
use crate::overview::local_time;
use crate::theme::Rgba;
use crate::wm::Wm;
use crate::x;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::CURRENT_TIME;

#[derive(Default)]
pub struct Lock {
    pub locked: bool,
    pub input: String,
    pub message: String,
    pub failed: bool,
    pub busy: Option<Receiver<Result<(), String>>>,
    pub waiters: Vec<UnixStream>,
    pub since: Option<Instant>,
    pub shake: Option<Instant>,
    pub tries: u32,
    pub drawn_minute: u32,
}

/// The PAM service: our own when the installer put it there, else the system's login rules.
fn service() -> &'static str {
    if std::path::Path::new("/etc/pam.d/samurai-lock").exists() { "samurai-lock" } else { "login" }
}

// ── PAM, called directly (no build-time C tooling needed) ───────────────────────────────
mod pam_ffi {
    use std::ffi::{c_char, c_int, c_void};
    #[repr(C)]
    pub struct Message {
        pub style: c_int,
        pub msg: *const c_char,
    }
    #[repr(C)]
    pub struct Response {
        pub resp: *mut c_char,
        pub retcode: c_int,
    }
    pub type ConvFn = extern "C" fn(c_int, *mut *const Message, *mut *mut Response, *mut c_void) -> c_int;
    #[repr(C)]
    pub struct Conv {
        pub conv: ConvFn,
        pub data: *mut c_void,
    }
    #[link(name = "pam")]
    extern "C" {
        pub fn pam_start(service: *const c_char, user: *const c_char, conv: *const Conv, h: *mut *mut c_void) -> c_int;
        pub fn pam_authenticate(h: *mut c_void, flags: c_int) -> c_int;
        pub fn pam_acct_mgmt(h: *mut c_void, flags: c_int) -> c_int;
        pub fn pam_setcred(h: *mut c_void, flags: c_int) -> c_int;
        pub fn pam_end(h: *mut c_void, status: c_int) -> c_int;
    }
    pub const SUCCESS: c_int = 0;
    pub const BUF_ERR: c_int = 5;
    pub const CONV_ERR: c_int = 19;
    pub const PROMPT_ECHO_OFF: c_int = 1;
    pub const PROMPT_ECHO_ON: c_int = 2;
    pub const REFRESH_CRED: c_int = 0x0010;
}

/// PAM asks; we answer every prompt with the password (the user is already given).
extern "C" fn converse(n: std::ffi::c_int, msgs: *mut *const pam_ffi::Message, resp: *mut *mut pam_ffi::Response, data: *mut std::ffi::c_void) -> std::ffi::c_int {
    unsafe {
        if n <= 0 || msgs.is_null() || resp.is_null() {
            return pam_ffi::CONV_ERR;
        }
        let out = libc::calloc(n as usize, std::mem::size_of::<pam_ffi::Response>()) as *mut pam_ffi::Response;
        if out.is_null() {
            return pam_ffi::BUF_ERR;
        }
        let pw = data as *const std::ffi::c_char;
        for i in 0..n as usize {
            let m = *msgs.add(i);
            if m.is_null() {
                continue;
            }
            let style = (*m).style;
            if style == pam_ffi::PROMPT_ECHO_OFF || style == pam_ffi::PROMPT_ECHO_ON {
                (*out.add(i)).resp = libc::strdup(pw);
            }
        }
        *resp = out;
        pam_ffi::SUCCESS
    }
}

pub fn check_password(user: &str, password: &str) -> Result<(), String> {
    use std::ffi::CString;
    let service = CString::new(service()).map_err(|_| "bad service name")?;
    let user = CString::new(user).map_err(|_| "bad user name")?;
    let pw = CString::new(password).map_err(|_| "the password has a NUL in it".to_string())?;
    let conv = pam_ffi::Conv { conv: converse, data: pw.as_ptr() as *mut std::ffi::c_void };
    let mut h: *mut std::ffi::c_void = std::ptr::null_mut();
    unsafe {
        let r = pam_ffi::pam_start(service.as_ptr(), user.as_ptr(), &conv, &mut h);
        if r != pam_ffi::SUCCESS || h.is_null() {
            return Err(format!("PAM could not start ({r})"));
        }
        let mut r = pam_ffi::pam_authenticate(h, 0);
        if r == pam_ffi::SUCCESS {
            r = pam_ffi::pam_acct_mgmt(h, 0);
        }
        if r == pam_ffi::SUCCESS {
            // renews things like Kerberos tickets; a failure here does not keep you locked out
            pam_ffi::pam_setcred(h, pam_ffi::REFRESH_CRED);
        }
        pam_ffi::pam_end(h, r);
        if r == pam_ffi::SUCCESS { Ok(()) } else { Err("wrong password".into()) }
    }
}

impl Wm {
    pub fn lock_now(&mut self) {
        if self.lock.locked {
            return;
        }
        if self.ov.open {
            self.ov.open = false;
            self.ov.closing = false;
            self.comp.drop_layer(&self.conn, "overview");
        }
        self.end_drag();
        // hold every input; try a few times, apps can hold a grab for a moment
        let mut ok = false;
        for _ in 0..50 {
            let k = self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok().and_then(|r| r.reply().ok()).is_some_and(|r| r.status == GrabStatus::SUCCESS);
            let p = self.conn.grab_pointer(false, self.root, EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, self.cursors.normal, CURRENT_TIME).ok().and_then(|r| r.reply().ok()).is_some_and(|r| r.status == GrabStatus::SUCCESS);
            if k && p {
                ok = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !ok {
            // still lock the picture: nothing shows, and we keep trying to grab each tick
            eprintln!("samurai-wm: could not take the keyboard for the lock; retrying");
        }
        self.lock.locked = true;
        self.lock.input.clear();
        self.lock.message.clear();
        self.lock.failed = false;
        self.lock.since = Some(Instant::now());
        self.lock.tries = 0;
        self.draw_lock();
        self.comp.dirty = true;
        for w in self.lock.waiters.iter_mut() {
            use std::io::Write;
            let _ = w.write_all(b"locked\n");
        }
    }

    fn unlock(&mut self) {
        self.lock.locked = false;
        self.lock.input.clear();
        self.conn.ungrab_keyboard(CURRENT_TIME).ok();
        self.conn.ungrab_pointer(CURRENT_TIME).ok();
        self.comp.drop_layer(&self.conn, "lock");
        let f = self.focused;
        self.focus(f);
        for mut w in self.lock.waiters.drain(..) {
            use std::io::Write;
            let _ = w.write_all(b"unlocked\n");
        }
        self.comp.dirty = true;
    }

    /// Lock-screen input. Returns true when the event was used (everything is, while locked).
    pub fn lock_event(&mut self, ev: &Event) -> bool {
        match ev {
            Event::KeyPress(e) => {
                let sym = self.keymap.sym(e.detail, 0);
                let state = u16::from(e.state);
                match sym {
                    x::KS_RETURN | x::KS_KP_ENTER => {
                        // while a check runs, typing goes on; Enter waits for the answer
                        if !self.lock.input.is_empty() && self.lock.busy.is_none() {
                            let (user, pw) = (std::env::var("USER").unwrap_or_default(), std::mem::take(&mut self.lock.input));
                            let (tx, rx) = channel();
                            std::thread::spawn(move || {
                                let r = check_password(&user, &pw);
                                let _ = tx.send(r);
                            });
                            self.lock.busy = Some(rx);
                            self.lock.message = "Checking…".into();
                            self.lock.failed = false;
                        }
                    }
                    x::KS_BACKSPACE => {
                        if state & x::MOD_CTRL != 0 { self.lock.input.clear() } else { self.lock.input.pop(); }
                    }
                    x::KS_ESCAPE => self.lock.input.clear(),
                    _ if state & (x::MOD_SUPER | x::MOD_CTRL | x::MOD_ALT) != 0 => {}
                    _ => {
                        if let Some(c) = self.keymap.char_for(e.detail, state) {
                            if !c.is_control() && self.lock.input.len() < 512 {
                                self.lock.input.push(c);
                                if self.lock.busy.is_none() {
                                    self.lock.failed = false;
                                    self.lock.message.clear();
                                }
                            }
                        }
                    }
                }
                self.draw_lock();
                true
            }
            Event::MapRequest(_) | Event::ConfigureRequest(_) | Event::UnmapNotify(_) | Event::DestroyNotify(_)
            | Event::PropertyNotify(_) | Event::ClientMessage(_) | Event::RandrScreenChangeNotify(_) | Event::Expose(_) => false,
            _ => true,
        }
    }

    pub fn lock_tick(&mut self) {
        if !self.lock.locked {
            return;
        }
        if let Some(rx) = &self.lock.busy {
            if let Ok(r) = rx.try_recv() {
                self.lock.busy = None;
                match r {
                    Ok(()) => {
                        self.unlock();
                        return;
                    }
                    Err(e) => {
                        self.lock.tries += 1;
                        self.lock.failed = true;
                        self.lock.message = if e.starts_with("PAM") { format!("Cannot check the password: {e}") } else { "Wrong password. Try again.".into() };
                        self.lock.shake = Some(Instant::now());
                    }
                }
                self.draw_lock();
            }
        }
        // keep holding the input
        let p = self.conn.grab_keyboard(false, self.root, CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC).ok().and_then(|r| r.reply().ok());
        let _ = p;
        let (h, m, _) = local_time();
        if h * 60 + m != self.lock.drawn_minute || self.lock.shake.is_some_and(|t| t.elapsed().as_millis() < 400) {
            self.draw_lock();
        }
    }

    pub fn draw_lock(&mut self) {
        let (sw, sh) = self.screen_size();
        let Some(mut px) = draw::Px::new(sw as u32, sh as u32) else { return };
        let th = self.theme.clone();
        let s = self.scale;
        // the wallpaper is painted under this layer; dim it
        px.fill(draw::tiny_skia_color(if th.night { Rgba(9, 10, 12, 120) } else { Rgba(236, 232, 223, 110) }));
        let f = &self.fonts;
        let (h, mi, date) = local_time();
        self.lock.drawn_minute = h * 60 + mi;
        // the face goes on the monitor with the pointer; the others only show the clock
        let mon = self.pointer_monitor();
        for m in &self.monitors {
            let main = *m == mon;
            let (cx, cy) = (m.x as f32 + m.w as f32 / 2.0, m.y as f32 + m.h as f32 / 2.0);
            let clock = format!("{h:02}:{mi:02}");
            f.draw(&mut px, Face::UiBold, 76.0 * s, m.x as f32 + 56.0 * s, (m.y + m.h) as f32 - 150.0 * s, th.text, &clock);
            f.draw(&mut px, Face::Ui, 18.0 * s, m.x as f32 + 58.0 * s, (m.y + m.h) as f32 - 58.0 * s, th.text, &date);
            if !main {
                continue;
            }
            let (cw, ch) = (360.0 * s, 300.0 * s);
            let shake = self.lock.shake.map(|t| t.elapsed().as_secs_f32()).filter(|t| *t < 0.4).map(|t| (t * 50.0).sin() * 10.0 * s * (1.0 - t / 0.4)).unwrap_or(0.0);
            let (x0, y0) = (cx - cw / 2.0 + shake, cy - ch / 2.0);
            draw::rrect(&mut px, x0, y0, cw, ch, 16.0 * s, th.glass);
            draw::rrect_line(&mut px, x0, y0, cw, ch, 16.0 * s, th.line, 1.0);
            let user = self.status.get().user;
            let initial = user.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            draw::circle(&mut px, cx + shake, y0 + 70.0 * s, 38.0 * s, th.raised);
            f.draw_centered(&mut px, Face::UiBold, 30.0 * s, cx + shake, y0 + 50.0 * s, th.accent, &initial);
            f.draw_centered(&mut px, Face::UiBold, 18.0 * s, cx + shake, y0 + 120.0 * s, th.text, &user);
            let (fx, fy, fw, fh) = (x0 + 28.0 * s, y0 + 164.0 * s, cw - 56.0 * s, 48.0 * s);
            draw::rrect(&mut px, fx, fy, fw, fh, 12.0 * s, if th.night { Rgba::hex(0x14161A) } else { Rgba::hex(0xFFFFFF) });
            draw::rrect_line(&mut px, fx, fy, fw, fh, 12.0 * s, if self.lock.failed { Rgba(200, 80, 70, 255) } else { th.accent }, 2.0);
            let dots = self.lock.input.chars().count().min(24);
            if dots == 0 {
                f.draw(&mut px, Face::Ui, 15.0 * s, fx + 16.0 * s, fy + 14.0 * s, th.dim, if self.lock.busy.is_some() { "Checking…" } else { "Password" });
            }
            for i in 0..dots {
                draw::circle(&mut px, fx + 22.0 * s + i as f32 * 14.0 * s, fy + fh / 2.0, 4.0 * s, th.text);
            }
            let msg = if self.lock.message.is_empty() { "Locked. Type your password and press Enter.".to_string() } else { self.lock.message.clone() };
            f.draw_centered(&mut px, if self.lock.failed { Face::UiBold } else { Face::Ui }, 13.0 * s, cx + shake, fy + fh + 22.0 * s, if self.lock.failed { th.text } else { th.dim }, &f.fit(Face::Ui, 13.0 * s, &msg, cw - 40.0 * s));
        }
        self.comp.set_layer(&self.conn, "lock", &px, 0, 0, 1.0);
    }
}
