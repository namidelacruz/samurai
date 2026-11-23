//! samurai-wm: the Samurai window manager for X11.
//!   samurai-wm              run (from samurai-session, as the last thing it starts)
//!   samurai-wm msg …        talk to the running one (see actions.rs)
//!   samurai-wm --check      read ~/.config/samurai/wm.toml and say whether it is right

mod actions;
mod assist;
mod comp;
mod config;
mod draw;
mod ipc;
mod lock;
mod overview;
mod status;
mod theme;
mod wm;
mod x;

use comp::{FrameLook, PaintInfo};
use std::collections::HashMap;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};
use theme::Rgba;
use wm::{Rect, Wm};
use x11rb::connection::Connection;
use x11rb::protocol::dpms::{ConnectionExt as _, DPMSMode};
use x11rb::protocol::screensaver::ConnectionExt as _;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("msg") => std::process::exit(ipc::send(&args[1..])),
        Some("--version") | Some("-V") => {
            println!("samurai-wm {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--check") => {
            let (c, e) = config::load();
            match e {
                Some(e) => {
                    eprintln!("{e}");
                    std::process::exit(1)
                }
                None => {
                    for k in c.keys.keys() {
                        if x::parse_combo(k).is_none() {
                            eprintln!("{}: cannot read the key \"{k}\"", config::path().display());
                            std::process::exit(1);
                        }
                    }
                    println!("{} is fine", config::path().display());
                    return;
                }
            }
        }
        Some("--keys") => {
            // every shortcut in effect, for Settings: "keys<TAB>action"
            let (c, _) = config::load();
            let mut rows: Vec<(String, String)> = c.keys.into_iter().collect();
            rows.sort_by_key(|(k, a)| (actions::ACTIONS.iter().position(|x| a.starts_with(x.0)).unwrap_or(99), k.clone()));
            for (k, a) in rows {
                let label = actions::ACTIONS.iter().find(|x| x.0 == a).map(|x| x.1.to_string()).unwrap_or(a);
                println!("{}\t{}", k.replace('+', " + "), label);
            }
            return;
        }
        Some("-h") | Some("--help") => {
            println!("samurai-wm            run the window manager\nsamurai-wm msg ACTION  ask the running one to do something (samurai-wm msg actions lists them)\nsamurai-wm --check     check the settings file");
            return;
        }
        _ => {}
    }
    let mut wm = match Wm::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("samurai-wm: {e}");
            std::process::exit(1);
        }
    };
    unsafe {
        // Children are waited for by status::spawn / Status::job (one thread each), so there are no
        // zombies. SIGCHLD must NOT be ignored: that makes every wait fail (so no tool output
        // could be read: Wi-Fi, sound, brightness all looked off) and it is inherited by every
        // app started from here, which breaks apps that wait for their own children.
        libc::signal(libc::SIGCHLD, libc::SIG_DFL);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    for cmd in wm.cfg.autostart.clone() {
        status::spawn(&cmd);
    }
    run(&mut wm);
}

fn run(wm: &mut Wm) {
    let xfd = wm.conn.stream().as_raw_fd();
    let mut last_paint = Instant::now() - Duration::from_secs(1);
    let mut last_slow = Instant::now() - Duration::from_secs(10);
    let mut sampled: HashMap<u32, Instant> = HashMap::new();
    let mut pill_key = String::new();
    let mut bypassing = false;
    while wm.running {
        loop {
            match wm.conn.poll_for_event() {
                Ok(Some(ev)) => wm.handle(ev),
                Ok(None) => break,
                Err(e) => {
                    eprintln!("samurai-wm: lost the X server: {e}");
                    return;
                }
            }
        }
        wm.serve();
        wm.lock_tick();
        let ov_moving = wm.overview_tick();

        // a new window's own top colour, once it has drawn (about a quarter second in)
        let now = Instant::now();
        for c in wm.clients.clone() {
            if c.strip && c.strip_color.is_none() {
                let t = *sampled.entry(c.win).or_insert(now);
                if now.duration_since(t) > Duration::from_millis(350) && now.duration_since(t) < Duration::from_secs(3) {
                    wm.sample_strip_color(c.win);
                }
            }
        }
        sampled.retain(|w, _| wm.clients.iter().any(|c| c.win == *w));

        if now.duration_since(last_slow) > Duration::from_millis(1000) {
            last_slow = now;
            slow_tick(wm);
        }
        let key = pill_state(wm);
        if key != pill_key {
            pill_key = key;
            draw_corners(wm);
        }
        // a fullscreen app on its own gets the screen directly
        let want_bypass = wm.cfg.windows.unredirect_fullscreen && !wm.ov.open && !wm.lock.locked && wm.monitors.len() == 1
            && wm.focused.and_then(|f| wm.client(f)).is_some_and(|c| c.fullscreen) && !wm.comp.animating();
        if want_bypass != bypassing {
            bypassing = want_bypass;
            wm.comp.bypass(&wm.conn, bypassing);
        }
        let animating = wm.comp.animating() || ov_moving;
        if !bypassing && (wm.comp.dirty || animating) && now.duration_since(last_paint) >= Duration::from_millis(14) {
            paint(wm);
            last_paint = now;
        }
        let _ = wm.conn.flush();
        let timeout = if animating || wm.comp.dirty { 8 } else { 500 };
        let mut fds = vec![libc::pollfd { fd: xfd, events: libc::POLLIN, revents: 0 }];
        if let Some(s) = &wm.ipc {
            fds.push(libc::pollfd { fd: s.fd(), events: libc::POLLIN, revents: 0 });
        }
        unsafe {
            libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout);
        }
    }
    // leaving: put every window back on the root so nothing is lost; tucked ones are shown again
    // so that the next window manager (or this one, restarted) finds them
    for c in wm.clients.clone() {
        let root = wm.root;
        let _ = x11rb::protocol::xproto::ConnectionExt::reparent_window(&wm.conn, c.win, root, c.rect.x as i16, c.rect.y as i16);
        if c.tucked {
            let _ = x11rb::protocol::xproto::ConnectionExt::map_window(&wm.conn, c.win);
        }
    }
    let _ = wm.conn.flush();
    if wm.restart {
        use std::os::unix::process::CommandExt;
        // the path it was started by: after an update that is the new program, while
        // current_exe() would still name the old, moved-away one
        let argv0 = std::env::args().next().unwrap_or_default();
        let exe: std::path::PathBuf = if argv0.starts_with('/') { argv0.into() } else { std::env::current_exe().unwrap_or_else(|_| "samurai-wm".into()) };
        let args: Vec<String> = std::env::args().skip(1).collect();
        eprintln!("samurai-wm: restarting");
        let e = std::process::Command::new(exe).args(args).exec();
        eprintln!("samurai-wm: could not restart: {e}");
        std::process::exit(1);
    }
}

fn hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn slow_tick(wm: &mut Wm) {
    // settings changed on disk
    if config::mtime() != wm.cfg_mtime {
        wm.reload();
    }
    // paper by day, night after dark
    if wm.cfg.look.follow_sun {
        let (from, to) = (hhmm(&wm.cfg.look.sun_from).unwrap_or(450), hhmm(&wm.cfg.look.sun_to).unwrap_or(1080));
        let (h, m, _) = overview::local_time();
        let n = h * 60 + m;
        let day = if from <= to { n >= from && n < to } else { n >= from || n < to };
        if day == wm.theme.night {
            let g = if day { "paper" } else { "night" };
            wm.cfg.look.ground = g.to_string();
            wm.set_ground(g);
            status::spawn(&format!("samurai theme {g}"));
        }
    }
    // idle: lock, screens off, sleep
    if let Some(info) = wm.conn.screensaver_query_info(wm.root).ok().and_then(|c| c.reply().ok()) {
        let idle_min = info.ms_since_user_input / 60_000;
        let i = wm.cfg.idle.clone();
        if i.lock > 0 && idle_min >= i.lock && !wm.lock.locked {
            wm.lock_now();
        }
        if i.screen_off > 0 && idle_min >= i.screen_off && info.ms_since_user_input % 60_000 < 1_100 {
            let _ = wm.conn.dpms_force_level(DPMSMode::OFF);
        }
        if i.suspend > 0 && idle_min >= i.suspend && info.ms_since_user_input % 60_000 < 1_100 {
            if !wm.lock.locked {
                wm.lock_now();
            }
            status::spawn("systemctl suspend");
        }
    }
    if let Some((_, t)) = &wm.notice {
        if t.elapsed() > Duration::from_secs(5) {
            wm.notice = None;
            wm.comp.drop_layer(&wm.conn, "notice");
        }
    }
}

/// What the corner pill and the mark show; redrawn only when this changes.
fn pill_state(wm: &Wm) -> String {
    let (h, m, _) = overview::local_time();
    let s = wm.status.get();
    let covered = corner_covered(wm);
    format!("{h}:{m}|{:?}|{:?}|{}|{:?}|{}|{}|{}|{:?}|{:?}|{}", s.battery, s.ssid, s.wifi_on, s.volume, s.muted, wm.theme.night, wm.ov.open, covered, wm.notice.as_ref().map(|n| n.0.clone()), wm.scale)
}

fn corner_covered(wm: &Wm) -> (bool, bool) {
    let m = wm.monitors[0];
    let s = wm.scale;
    let pill = Rect::new(m.x + m.w - (280.0 * s) as i32, m.y, (280.0 * s) as i32, (60.0 * s) as i32);
    let mark = Rect::new(m.x, m.y + m.h - (56.0 * s) as i32, (260.0 * s) as i32, (56.0 * s) as i32);
    let any = |r: &Rect| wm.clients.iter().any(|c| !c.tucked && wm.visible_rect(c).overlap(r) > 0);
    (any(&pill), any(&mark))
}

/// The two quiet things on an empty desktop: the time pill (top right) and the machine's mark
/// (bottom left). Each hides while a window covers its corner.
fn draw_corners(wm: &mut Wm) {
    let th = wm.theme.clone();
    let s = wm.scale;
    let m = wm.monitors[0];
    let (pill_hidden, mark_hidden) = corner_covered(wm);
    if wm.cfg.look.show_clock && !pill_hidden && !wm.ov.open {
        let snap = wm.status.get();
        let (h, mi, _) = overview::local_time();
        let date = overview::short_date();
        let time = format!("{h:02}:{mi:02}");
        let fs = 12.5 * s;
        let dw = wm.fonts.measure(draw::Face::Ui, fs, &date);
        let tw = wm.fonts.measure(draw::Face::UiBold, fs, &time);
        let icons = 3.0 * 15.0 * s + 2.0 * 8.0 * s;
        let w = (14.0 * s + dw + 12.0 * s + tw + 12.0 * s + icons + 14.0 * s).ceil();
        let hgt = (32.0 * s).ceil();
        if let Some(mut px) = draw::Px::new(w as u32 + 2, hgt as u32 + 2) {
            draw::rrect(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.glass);
            draw::rrect_line(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.line, 1.0);
            let ty = 1.0 + (hgt - fs * 1.25) / 2.0;
            let mut x = 1.0 + 14.0 * s;
            wm.fonts.draw(&mut px, draw::Face::Ui, fs, x, ty, th.dim, &date);
            x += dw + 12.0 * s;
            wm.fonts.draw(&mut px, draw::Face::UiBold, fs, x, ty, th.text, &time);
            x += tw + 12.0 * s;
            let iy = 1.0 + (hgt - 15.0 * s) / 2.0;
            let wifi = if snap.wifi_on && snap.ssid.is_some() { "wifi" } else { "wifi-off" };
            for ic in [wifi, if snap.muted { "mute" } else { "sound" }, "battery"] {
                if ic == "battery" && snap.battery.is_none() {
                    continue;
                }
                draw::icon(&mut px, ic, x, iy, 15.0 * s, th.dim, 1.6);
                x += 15.0 * s + 8.0 * s;
            }
            let (x0, y0) = (m.x + m.w - w as i32 - (20.0 * s) as i32, m.y + (16.0 * s) as i32);
            wm.comp.set_layer(&wm.conn, "pill", &px, x0, y0, 1.0);
        }
    } else {
        wm.comp.drop_layer(&wm.conn, "pill");
    }
    let mark = wm.cfg.look.mark.clone();
    if wm.cfg.look.show_mark && !mark.is_empty() && !mark_hidden && !wm.ov.open {
        let fs = 11.5 * s;
        let mw = wm.fonts.measure(draw::Face::Ui, fs, &mark);
        let w = (12.0 * s + 16.0 * s + 10.0 * s + mw + 12.0 * s).ceil();
        let hgt = (28.0 * s).ceil();
        if let Some(mut px) = draw::Px::new(w as u32 + 2, hgt as u32 + 2) {
            draw::rrect(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.glass);
            draw::rrect_line(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.line, 1.0);
            wm.fonts.draw(&mut px, draw::Face::Jp, 13.0 * s, 1.0 + 12.0 * s, 1.0 + (hgt - 13.0 * s * 1.3) / 2.0, th.accent, "墨");
            wm.fonts.draw(&mut px, draw::Face::Ui, fs, 1.0 + 12.0 * s + 16.0 * s + 10.0 * s, 1.0 + (hgt - fs * 1.25) / 2.0, th.dim, &mark);
            wm.comp.set_layer(&wm.conn, "mark", &px, m.x + (20.0 * s) as i32, m.y + m.h - hgt as i32 - (16.0 * s) as i32, 1.0);
        }
    } else {
        wm.comp.drop_layer(&wm.conn, "mark");
    }
    // a message from the WM itself (a broken settings file, a screenshot saved…)
    if let Some((text, _)) = wm.notice.clone() {
        let fs = 13.0 * s;
        let text = wm.fonts.fit(draw::Face::Ui, fs, &text, m.w as f32 * 0.6);
        let tw = wm.fonts.measure(draw::Face::Ui, fs, &text);
        let (w, hgt) = ((tw + 36.0 * s).ceil(), (40.0 * s).ceil());
        if let Some(mut px) = draw::Px::new(w as u32 + 2, hgt as u32 + 2) {
            draw::rrect(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.glass);
            draw::rrect_line(&mut px, 1.0, 1.0, w, hgt, hgt / 2.0, th.line, 1.0);
            wm.fonts.draw(&mut px, draw::Face::Ui, fs, 1.0 + 18.0 * s, 1.0 + (hgt - fs * 1.25) / 2.0, th.text, &text);
            wm.comp.set_layer(&wm.conn, "notice", &px, m.cx() - w as i32 / 2, m.y + m.h - hgt as i32 - (40.0 * s) as i32, 1.0);
        }
    }
}

fn paint(wm: &mut Wm) {
    let mut frames = HashMap::new();
    for c in &wm.clients {
        let decorated = c.strip;
        frames.insert(c.frame, FrameLook { inset: wm.border(c), radius: wm.radius(c), shadow: !c.fullscreen && (decorated || c.kind != wm::Kind::Splash) });
    }
    // the snap preview follows the pointer while a window is dragged to an edge
    match wm.snap_preview {
        Some(r) => {
            let key = format!("{r:?}");
            if wm.comp.has_layer("preview") && wm.comp.layers.iter().any(|l| l.0 == "preview" && format!("{:?}", Rect::new(l.1.x, l.1.y, l.1.w as i32, l.1.h as i32)) == key) {
            } else if let Some(mut px) = draw::Px::new(r.w as u32, r.h as u32) {
                let rad = wm.cfg.look.corners * wm.scale + 2.0;
                draw::rrect(&mut px, 0.0, 0.0, r.w as f32, r.h as f32, rad, wm.theme.soft);
                draw::rrect_line(&mut px, 0.0, 0.0, r.w as f32, r.h as f32, rad, wm.theme.accent, 2.0);
                wm.comp.set_layer(&wm.conn, "preview", &px, r.x, r.y, 1.0);
            }
        }
        None => wm.comp.drop_layer(&wm.conn, "preview"),
    }
    let t = wm.ov.t;
    let scrim = if wm.ov.open {
        let s = wm.theme.scrim;
        Some(Rgba(s.0, s.1, s.2, (s.3 as f32 * overview_ease(t)) as u8))
    } else {
        None
    };
    let rects = wm.ov.rects.clone();
    let m = wm.monitors[0];
    let info = PaintInfo {
        frames: &frames,
        night: wm.theme.night,
        ground: wm.theme.ground,
        shadow: wm.theme.shadow,
        scrim,
        overview: if wm.ov.open { Some((&rects, t)) } else { None },
        ov_dim: if wm.ov.open && !wm.ov.query.is_empty() { 0.25 } else { 1.0 },
        locked: wm.lock.locked,
        tuck_to: (m.cx(), m.y + m.h),
    };
    wm.comp.paint(&wm.conn, &info);
}

fn overview_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
