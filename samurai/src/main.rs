//! samurai — the backend of the Samurai desktop (Hyprland + Quickshell).
//!
//! The frame and the settings hub are QML (shell/); this program is what they call to read and
//! change anything: the settings store, the Hyprland config, keybinds, the one-time seal,
//! incognito, and the small desktop commands behind keybinds.
mod binds;
mod hypr;
mod misc;
mod seal;
mod store;

use serde_json::{json, Value};

const USAGE: &str = "samurai — the white-frame desktop

  settings   get KEY | set KEY VALUE | reset KEY|PAGE|all | dump | schema
  hyprland   apply | check | print
  keybinds   binds list | binds check \"MOD + SHIFT + T\"
  seal       seal show | seal forge [--erase-current]
  incognito  incognito on|off|toggle|status
  frame      shell start|quick|widgets|settings|keys|base|incognito|lock|power|restart
  windows    win minimize | win restore [ADDRESS] | win prune
  apps       open terminal|files|browser | term CMD…
  looks      look list | save NAME | apply NAME | export NAME | delete NAME
  store      store refresh | list | install ID | remove ID      plugins
  capture    shot area|screen | record toggle|start|stop
  updates    update check | update install
  other      status | gpu check | session-start | version
";

/// VALUE on the command line: JSON when it parses as JSON, else plain text
fn parse_value(key: &str, raw: &str) -> Value {
    let item = store::item(key);
    let ty = item.as_ref().and_then(|i| i["type"].as_str()).unwrap_or("");
    match ty {
        "text" | "color" | "enum" => serde_json::from_str::<Value>(raw).ok().filter(|v| v.is_string()).unwrap_or(Value::String(raw.to_string())),
        _ => serde_json::from_str(raw).unwrap_or(Value::String(raw.to_string())),
    }
}

fn run(a: &[&str]) -> Result<String, String> {
    match a {
        ["get", key] => store::item(key).map(|_| store::get(key).to_string()).ok_or(format!("{key}: no such setting")),
        ["set", key, raw] => {
            store::set(key, parse_value(key, raw))?;
            // a setting Hyprland reads is applied at once; the frame watches settings.json itself
            let it = store::item(key).unwrap();
            if it["hl"].is_string() || it["use"] == "hypr" || key.starts_with("look.base") || key.starts_with("look.auto") {
                hypr::apply()?;
            }
            Ok("saved".into())
        }
        ["reset", what] => {
            let n = store::reset(what)?;
            hypr::apply()?;
            Ok(format!("{n} setting(s) back to default"))
        }
        ["dump"] => Ok(Value::Object(store::all()).to_string()),
        ["schema"] => Ok(store::SCHEMA.to_string()),
        ["apply"] => hypr::apply().map(|v| v.join("\n")),
        ["check"] => hypr::check(),
        ["print"] => Ok(hypr::generate()),
        ["binds", "list"] => Ok(binds::list_json().to_string()),
        ["binds", "check", keys] => Ok(binds::check(keys).to_string()),
        ["seal", "show"] => Ok(seal::show().to_string()),
        ["seal", "decoy"] => seal::decoy().map(|_| "decoy drawn".to_string()),
        ["seal", "forge"] => seal::forge(false).map(|v| v.to_string()),
        ["seal", "forge", "--erase-current"] => seal::forge(true).map(|v| v.to_string()),
        ["incognito", arg] => misc::incognito(arg).map(|on| if on { "on".into() } else { "off".into() }),
        ["shell", verb] => misc::shell(verb, None).map(|_| String::new()),
        ["shell", verb, arg] => misc::shell(verb, Some(arg)).map(|_| String::new()),
        ["win", verb] => misc::win(verb, None),
        ["win", verb, addr] => misc::win(verb, Some(addr)),
        ["open", what] => misc::open(what).map(|_| String::new()),
        ["term", cmd @ ..] if !cmd.is_empty() => misc::term(cmd).map(|_| String::new()),
        ["look", rest @ ..] => misc::look(rest),
        ["store", rest @ ..] => misc::store_cmd(rest),
        ["plugins"] => Ok(misc::plugins().to_string()),
        ["shot", mode] => misc::shot(mode),
        ["record", arg] => misc::record(arg),
        ["update", rest @ ..] => misc::update(rest),
        ["status"] => Ok(misc::status().to_string()),
        ["gpu", "check"] => Ok(misc::gpu_check().to_string()),
        ["session-start"] => {
            hypr::apply()?;
            misc::session_start().map(|_| String::new())
        }
        ["greeter", "apply"] => {
            if !std::path::Path::new("/opt/samurai/bin/samurai-greeter-setup").exists() {
                return Err("the login screen is set up by the installer: ./install.sh --greeter".into());
            }
            let base = store::get_str("greeter.base");
            misc::out(&["pkexec", "/opt/samurai/bin/samurai-greeter-setup", &base]).0.then(|| "login screen updated".to_string()).ok_or("the login screen was not changed".into())
        }
        // listed in the hub, not built yet: say so instead of pretending
        ["dictate", ..] => Err("NOT BUILT YET: voice typing (the settings are saved; the engine comes in a later release)".into()),
        ["gpu", "passthrough", ..] => Err("NOT BUILT YET: writing the passthrough setup (samurai gpu check shows readiness)".into()),
        ["version"] => Ok(format!("samurai {} ({})", env!("CARGO_PKG_VERSION"), option_env!("SAMURAI_COMMIT").unwrap_or("dev"))),
        [] | ["help"] | ["--help"] | ["-h"] => Ok(USAGE.into()),
        _ => Err(format!("unknown command: {}\n\n{USAGE}", a.join(" "))),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&a) {
        Ok(s) => {
            if !s.is_empty() {
                println!("{s}");
            }
        }
        Err(e) => {
            // the hub reads errors as JSON when asked to
            if std::env::var_os("SAMURAI_JSON").is_some() {
                println!("{}", json!({ "error": e }));
            } else {
                eprintln!("samurai: {e}");
            }
            std::process::exit(if e.starts_with("NOT BUILT YET") { 3 } else { 1 });
        }
    }
}
