#!/usr/bin/env python3
"""Set every niri-backed setting to each of its extremes and let niri judge the result.

Uses whatever `niri` is on PATH (the real one, or tests/bin/niri with NIRI_VALIDATOR). A value
niri refuses must come back as an error AND be rolled back; anything else is a failure.
Needs SAMURAI_CONFIG to point at a throwaway folder.
"""
import json, os, subprocess, sys
S = sys.argv[1]
assert os.environ.get("SAMURAI_CONFIG"), "set SAMURAI_CONFIG to a throwaway folder"
schema = json.loads(subprocess.run([S, "schema"], capture_output=True, text=True, check=True).stdout)
tried = refused = broken = 0
for page in schema["pages"]:
    for sec in page["sections"]:
        for it in sec["items"]:
            if it.get("use") != "niri" or not it.get("key") or it["type"] == "list":
                continue
            t = it["type"]
            vals = {"toggle": [not it["default"]], "int": [it.get("min"), it.get("max")], "float": [it.get("min"), it.get("max")],
                    "enum": [o for o in it.get("options", []) if o != it["default"]],
                    "text": ['x"} spawn "evil', "abc"], "color": ["#123456"]}.get(t, [])
            for v in vals:
                tried += 1
                r = subprocess.run([S, "set", it["key"], json.dumps(v)], capture_output=True, text=True)
                now = json.loads(subprocess.run([S, "get", it["key"]], capture_output=True, text=True).stdout)
                if r.returncode != 0:
                    refused += 1
                    if now == v:
                        broken += 1; print("  NOT ROLLED BACK", it["key"], json.dumps(v))
                    else:
                        print("  refused and rolled back:", it["key"], json.dumps(v))
            subprocess.run([S, "reset", it["key"]], capture_output=True)
lists = {
    "displays.monitors": [{"output": "eDP-1", "mode": "2560x1600@120", "position": "0,0", "scale": "1.5", "transform": "0", "disabled": "no"},
                          {"output": "DP-1", "mode": "", "position": "auto", "scale": "", "transform": "90", "disabled": "yes"}],
    "binds.custom": [{"keys": "SUPER + SHIFT + T", "action": "spawn", "arg": 'kitty --class "x y"', "name": 'Scratch "t"', "flags": ""},
                     {"keys": "SUPER + F9", "action": "workspace", "arg": "3", "name": "", "flags": ""},
                     {"keys": "SUPER + F8", "action": "niri", "arg": 'set-column-width "-10%"', "name": "narrower", "flags": "repeat"}],
    "binds.moved": [{"id": "frame.widgets", "keys": "SUPER + SHIFT + W"}], "binds.disabled": [{"id": "win.close"}],
    "binds.mouse": [{"button": "mouse:276", "action": "workspace", "arg": "prev"}],
    "rules.window": [{"class": "firefox", "title": "", "effect": "opacity", "value": "0.9"}, {"class": "mpv", "title": ".*", "effect": "float", "value": "on"},
                     {"class": 'x"} spawn "e', "title": "", "effect": "width", "value": "0.5"}],
    "rules.layer": [{"namespace": "notifications", "effect": "shadow", "value": "off"}],
    "autostart": [{"command": "nm-applet --indicator"}], "env": [{"name": "MOZ_ENABLE_WAYLAND", "value": "1"}, {"name": "BAD NAME", "value": "x"}],
}
for k, v in lists.items():
    tried += 1
    r = subprocess.run([S, "set", k, json.dumps(v)], capture_output=True, text=True)
    if r.returncode != 0:
        broken += 1; print("  list refused:", k, (r.stderr or r.stdout).strip().splitlines()[:8])
gen = open(os.path.join(os.environ["SAMURAI_CONFIG"], "niri", "generated.kdl")).read()
if '\nspawn "e' in gen or "BAD NAME" in gen:
    broken += 1; print("  hostile text reached the config")
for k in lists:
    subprocess.run([S, "reset", k], capture_output=True)
print(f"   {tried} values, {refused} refused and rolled back, {broken} problems")
sys.exit(1 if broken else 0)
