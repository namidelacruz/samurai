#!/usr/bin/env python3
"""The window manager in a virtual X server (Xvfb): windows are managed, snapped, tucked, filled,
closed, survive a restart, and Super opens the overview. Needs Xvfb and python-xlib.
Run: python3 tests/wm_xvfb.py [path/to/samurai-wm]"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
WM = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target/release/samurai-wm")
if not shutil.which("Xvfb"):
    print("skip: Xvfb is not installed"); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="samurai-wmtest-"))
disp = ":%d" % (90 + os.getpid() % 9)
env = dict(os.environ, PATH=str(ROOT / "tests/x11/net") + ":" + os.environ["PATH"], NET_CALLS=str(tmp / "calls"), DISPLAY=disp, XDG_RUNTIME_DIR=str(tmp), SAMURAI_CONFIG=str(tmp / "config"), HOME=str(tmp / "home"), USER="samtest")
(tmp / "home").mkdir()
procs = []
def spawn(*a, **k):
    p = subprocess.Popen(a, env=env, stdout=subprocess.DEVNULL, stderr=k.get("err", subprocess.DEVNULL)); procs.append(p); return p
def msg(*a):
    r = subprocess.run([WM, "msg", *a], env=env, capture_output=True, text=True, timeout=5); return r.stdout.strip()
def state():
    return json.loads(msg("state") or "{}")
def wins():
    return {w["title"]: w for w in state().get("windows", [])}
def keys(*a):
    subprocess.run([sys.executable, str(ROOT / "tests/x11/keys.py"), *a], env=env, timeout=10)
checks = []
def check(name, ok):
    checks.append(ok); print(("  ok   " if ok else "  FAIL ") + name, flush=True)
try:
    spawn("Xvfb", disp, "-screen", "0", "1440x900x24", "-nolisten", "tcp"); time.sleep(1)
    wm = spawn(WM, err=open(tmp / "wm.log", "w"))
    for _ in range(80):
        if msg("ping") == "pong": break
        time.sleep(0.1)
    check("it starts and answers", msg("ping") == "pong")
    for t, c, w, h in [("Files", "1a1c21", "800", "520"), ("Terminal", "0e0f12", "690", "400"), ("Editor", "f7f4ee", "560", "420")]:
        spawn(sys.executable, str(ROOT / "tests/x11/win.py"), t, c, w, h); time.sleep(0.5)
    time.sleep(0.6)
    w = wins()
    check("three windows are managed", set(w) == {"Files", "Terminal", "Editor"})
    check("the newest window has focus", w.get("Editor", {}).get("focused") is True)
    msg("snap", "left"); time.sleep(0.5)
    e = wins()["Editor"]; x, y, rw, rh = e["rect"]
    check("snap left fills the left half", e.get("snapped") == "left" and x < 40 and 600 < rw < 740 and rh > 780)
    msg("tuck"); time.sleep(0.5)
    check("tuck hides it", wins()["Editor"]["tucked"] is True)
    msg("untuck"); time.sleep(0.5)
    check("untuck brings it back", wins()["Editor"]["tucked"] is False)
    keys("tap", "Super_L"); time.sleep(0.6)
    check("a tap on Super opens the overview", state().get("overview") is True)
    keys("tap", "Escape"); time.sleep(0.6)
    check("Escape closes it", state().get("overview") is False)
    msg("tuck"); time.sleep(0.4)
    pid = wm.pid
    msg("restart"); time.sleep(2)
    for _ in range(40):
        if msg("ping") == "pong": break
        time.sleep(0.1)
    check("restart keeps every window, tucked ones included", set(wins()) == {"Files", "Terminal", "Editor"} and wm.poll() is None and wm.pid == pid)
    # the quick panel: open the Wi-Fi list, pick a secured network, type a password, send it
    msg("overview"); time.sleep(1.0)
    keys("click", "1200", "225"); time.sleep(2.5)
    calls = (tmp / "calls").read_text() if (tmp / "calls").exists() else ""
    check("the Wi-Fi list is read only once it is opened", "device wifi list" in calls)
    keys("click", "1200", "381"); time.sleep(0.6)
    keys("type", "wrong"); keys("tap", "Return"); time.sleep(1.5)
    calls = (tmp / "calls").read_text()
    check("joining sends the password on stdin, never as an argument",
          "shadow wifi connect Neighbour:Net --password-stdin (password length 5)" in calls and "wrong" not in calls)
    keys("tap", "Escape"); time.sleep(0.6)
    check("Escape closes the overview after that", state().get("overview") is False)
    msg("close"); time.sleep(0.6)
    check("close asks the focused window to leave", len(wins()) == 2)
finally:
    for p in reversed(procs):
        p.terminate()
    for p in procs:
        try: p.wait(timeout=3)
        except subprocess.TimeoutExpired: p.kill()
    log = (tmp / "wm.log").read_text(errors="replace") if (tmp / "wm.log").exists() else ""
    if "panicked" in log: print(log[-2000:]); checks.append(False)
    shutil.rmtree(tmp, ignore_errors=True)
print(f"{len(checks)} checks, {checks.count(False)} failed")
sys.exit(0 if checks and all(checks) else 1)
