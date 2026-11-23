//! samurai: settings, config generation and the small jobs behind the Samurai desktop (X11).
//! The window manager itself is samurai-wm; Wi-Fi campus setup and repair is `shadow wifi`.

mod doctor;
mod misc;
mod seal;
mod store;
mod x11;

use serde_json::Value;

const HELP: &str = "samurai — the settings behind the Samurai desktop

  samurai get KEY                 a setting's value (JSON)
  samurai set KEY VALUE           change one (VALUE is JSON: true, 12, \"text\"); checked first
  samurai reset KEY|PAGE|all      back to the default
  samurai dump | schema           every value / the list of settings
  samurai apply                   write every program's config from the settings
  samurai session-start           apply, arrange the screens, start the session's helpers
  samurai theme night|paper       only what follows the ground (the window manager calls this)
  samurai ground toggle           switch night and paper
  samurai night toggle            night light on or off
  samurai seal show|decoy         this machine's ink
  samurai seal forge [--erase-current]
  samurai shot area|screen|window take a screenshot (saved and copied)
  samurai xserver                 which X server runs: X.Org or XLibre
  samurai status                  battery, network, sound, update (JSON)
  samurai update check|install
  samurai doctor [--quick]        check what commonly goes wrong, and put it right
  samurai pin APP                 pin an app first in the overview (again: unpin)
  samurai greeter apply           set up the login screen (asks for your password)
  samurai version";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let r: Result<String, String> = match a.as_slice() {
        ["get", key] => store::item(key).map(|_| store::get(key).to_string()).ok_or(format!("{key}: no such setting")),
        ["set", key, raw] => {
            let v: Value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
            store::set(key, v).and_then(|_| x11::apply().map(|_| "ok".to_string()))
        }
        ["set", key, raw, "--no-apply"] => {
            let v: Value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
            store::set(key, v).map(|_| "ok".to_string())
        }
        ["reset", what] => store::reset(what).and_then(|n| x11::apply().map(|_| format!("{n} back to default"))),
        ["dump"] => Ok(Value::Object(store::all()).to_string()),
        ["schema"] => Ok(store::SCHEMA.to_string()),
        ["apply"] => x11::apply().map(|v| v.join("\n")),
        ["session-start"] => x11::session_start().map(|v| v.join("\n")),
        ["theme", g @ ("night" | "paper")] => Ok(x11::theme(g).join("\n")),
        ["ground", "toggle"] => x11::toggle("ground"),
        ["night", "toggle"] => x11::toggle("night"),
        ["print", "wm"] => Ok(x11::wm_toml()),
        ["seal", "show"] => Ok(seal::show().to_string()),
        ["seal", "decoy"] => seal::decoy().map(|_| "decoy drawn".to_string()),
        ["seal", "forge"] => seal::forge(false).map(|v| v.to_string()),
        ["seal", "forge", "--erase-current"] => seal::forge(true).and_then(|v| x11::apply().map(|_| v.to_string())),
        ["shot", mode] => x11::shot(mode),
        ["xserver"] => Ok(x11::xserver().to_string()),
        ["status"] => Ok(misc::status().to_string()),
        ["update", rest @ ..] => misc::update(rest),
        ["greeter", "apply"] => {
            let g = x11::ground();
            let st = std::process::Command::new("pkexec")
                .args(["/opt/samurai/bin/samurai-greeter-setup", g, &std::env::var("USER").unwrap_or_default()])
                .status()
                .map_err(|e| format!("pkexec: {e}"));
            match st {
                Ok(s) if s.success() => Ok("login screen set; it shows from the next boot".into()),
                Ok(s) => Err(format!("the login screen was not changed (exit {})", s.code().unwrap_or(-1))),
                Err(e) => Err(e),
            }
        }
        ["greeter", "remove"] => {
            match std::process::Command::new("pkexec").args(["/opt/samurai/bin/samurai-greeter-setup", "--remove"]).status() {
                Ok(s) if s.success() => Ok("the previous login screen is back from the next boot".into()),
                Ok(_) => Err("nothing was changed".into()),
                Err(e) => Err(format!("pkexec: {e}")),
            }
        }
        ["doctor", rest @ ..] => {
            let quick = rest.contains(&"--quick");
            let v = doctor::run(quick);
            Ok(doctor::report(&v, rest.contains(&"--json")))
        }
        ["pin", app] => {
            // pin an app first in the overview's list, or unpin it (the overview's right-click)
            let id = app.trim_end_matches(".desktop").to_string();
            let mut rows = store::get_list("apps.pinned");
            let before = rows.len();
            rows.retain(|r| r["app"].as_str().map(|a| a.trim_end_matches(".desktop")) != Some(id.as_str()));
            let pinned = rows.len() == before;
            if pinned {
                rows.push(serde_json::json!({ "app": id }));
            }
            store::set("apps.pinned", serde_json::Value::Array(rows))
                .and_then(|_| x11::apply().map(|_| ()))
                .map(|_| format!("{} {id}", if pinned { "pinned" } else { "unpinned" }))
        }
        ["version"] => Ok(format!("samurai {} ({})", env!("CARGO_PKG_VERSION"), option_env!("SAMURAI_COMMIT").unwrap_or("dev"))),
        ["help"] | ["-h"] | ["--help"] | [] => Ok(HELP.into()),
        _ => Err(format!("unknown command: {}\n\n{HELP}", args.join(" "))),
    };
    match r {
        Ok(s) => {
            if !s.is_empty() {
                println!("{s}");
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
