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
env = dict(os.environ, SAMURAI_TEST_MONITORS=str(tmp / "monitors"), PATH=str(ROOT / "tests/x11/net") + ":" + os.environ["PATH"], NET_CALLS=str(tmp / "calls"), DISPLAY=disp, XDG_RUNTIME_DIR=str(tmp), SAMURAI_CONFIG=str(tmp / "config"), HOME=str(tmp / "home"), USER="samtest")
(tmp / "home").mkdir()
# the floating checks first; tiling gets its own part below
(tmp / "config").mkdir()
(tmp / "config/wm.toml").write_text("[windows]\ntiling = false\n")
# an app of our own, first by name, that leaves a mark when it runs
appdir = tmp / "home/.local/share/applications"; appdir.mkdir(parents=True)
(appdir / "aaa-samurai-test.desktop").write_text(f"[Desktop Entry]\nType=Application\nName=AAA Samurai test\nExec=touch {tmp}/app-ran\nIcon=utilities-terminal\n")
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
    check("snap assist offers the other windows for the right half", state().get("assist") is True)
    import_shot = os.environ.get("ASSIST_SHOT")
    if import_shot and shutil.which("import"):
        subprocess.run(["import", "-window", "root", import_shot], env=env)
    keys("tap", "Return"); time.sleep(0.6)
    w2 = wins(); picked = [t for t, v in w2.items() if v.get("snapped") == "right"]
    check("Enter puts the picked window on the right half", state().get("assist") is False and len(picked) == 1 and w2[picked[0]]["focused"])
    msg("focus", "window", str(w2["Editor"]["id"])); time.sleep(0.3)
    msg("snap", "left"); time.sleep(0.5)
    check("with the other side already taken, no assist", state().get("assist") is False)
    msg("tuck"); time.sleep(0.5)
    check("tuck hides it", wins()["Editor"]["tucked"] is True)
    msg("untuck"); time.sleep(0.5)
    check("untuck brings it back", wins()["Editor"]["tucked"] is False)
    keys("tap", "Super_L"); time.sleep(0.6)
    check("a tap on Super opens the overview", state().get("overview") is True)
    keys("tap", "Escape"); time.sleep(0.6)
    check("Escape closes it", state().get("overview") is False)
    # the apps, under the windows
    msg("overview"); time.sleep(1.0)
    keys("tap", "Tab"); time.sleep(0.3)
    st = state()
    check("Tab moves to the apps; every installed app is there (ours too)", st.get("ov_zone") == "apps" and st.get("ov_apps", 0) >= 1)
    keys("tap", "Right"); time.sleep(0.2); keys("tap", "Down"); time.sleep(0.3)
    st = state(); cols = st.get("ov_cols", 0)
    check("arrow keys move through the grid (Down = a whole row)", cols >= 1 and st.get("ov_sel") == min(1 + cols, st["ov_apps"] - 1))
    keys("type", "aaa sam"); time.sleep(0.4)
    check("typing filters quietly", state().get("query") == "aaa sam")
    keys("tap", "Return"); time.sleep(1.2)
    check("Enter opens the app that was found", (tmp / "app-ran").exists() and state().get("overview") is False)
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
    # tiling (on by default; switched on here, the way Settings does it)
    (tmp / "config/wm.toml").write_text("[windows]\ntiling = true\n")
    msg("reload"); time.sleep(1.2)
    w = wins()
    rects = {t: v["rect"] for t, v in w.items() if not v.get("tucked")}
    def overlap(a, b):
        return not (a[0] + a[2] <= b[0] or b[0] + b[2] <= a[0] or a[1] + a[3] <= b[1] or b[1] + b[3] <= a[1])
    names = list(rects)
    no_overlap = all(not overlap(rects[a], rects[b]) for i, a in enumerate(names) for b in names[i + 1:])
    check("tiling on: the windows share the screen without overlapping", len(rects) >= 2 and no_overlap)
    check("the first one takes a full-height half", any(r[3] > 780 and 600 < r[2] < 740 for r in rects.values()))
    spawn(sys.executable, str(ROOT / "tests/x11/win.py"), "Fourth", "2a2e36", "500", "300"); time.sleep(1.0)
    w = wins(); rects = {t: v["rect"] for t, v in w.items() if not v.get("tucked")}
    check("a new window takes its share at once", "Fourth" in rects and rects["Fourth"][2] > 300 and
          all(not overlap(rects["Fourth"], r) for t, r in rects.items() if t != "Fourth"))
    before = wins()["Fourth"]["rect"]
    msg("float"); time.sleep(0.6)
    after = wins()["Fourth"]["rect"]
    check("Super+V floats it: it comes forward, larger, in the middle", after != before and abs((after[0] + after[2] / 2) - 720) < 60)
    msg("float"); time.sleep(0.6)
    check("Super+V again tiles it", wins()["Fourth"]["rect"] == before)
    left = min(rects.values(), key=lambda r: r[0])
    first = [t for t, r in rects.items() if r == left][0]
    msg("focus", "window", str(w[first]["id"])); time.sleep(0.3)
    msg("move", "right"); time.sleep(0.7)
    check("moving a tiled window swaps it with its neighbour", wins()[first]["rect"] != left)
    # the taskbar for minimized windows: hidden until called up
    msg("tuck"); time.sleep(0.4)
    tucked_title = [t for t, v in wins().items() if v.get("tucked")]
    check("minimizing shows no taskbar by itself", state().get("taskbar") is False and len(tucked_title) == 1)
    keys("tap", "Super_L", "t"); time.sleep(0.6)
    st = state()
    check("Super+T calls it up, with the minimized window in it", st.get("taskbar") is True and st.get("taskbar_items") == 1)
    if os.environ.get("TASKBAR_SHOT") and shutil.which("import"):
        subprocess.run(["import", "-window", "root", os.environ["TASKBAR_SHOT"]], env=env)
    keys("tap", "Return"); time.sleep(0.7)
    st = state()
    check("Enter brings it back and the taskbar goes away", st.get("taskbar") is False and st.get("taskbar_items") == 0 and not wins()[tucked_title[0]]["tucked"])
    msg("taskbar"); time.sleep(0.4)
    check("the command calls it up too (empty: it says so)", state().get("taskbar") is True)
    keys("tap", "Escape"); time.sleep(0.4)
    check("Escape hides it", state().get("taskbar") is False)
    # hidden workspaces: keys only, and not in any list
    before = {t for t, v in wins().items() if not v.get("tucked")}
    keys("tap", "Super_L", "Shift_L", "3"); time.sleep(0.5)
    sent = [t for t, v in wins().items() if v.get("ws") == 3]
    check("Super+Shift+3 sends the focused window away (you stay)", state().get("ws") == 1 and len(sent) == 1)
    keys("tap", "Super_L", "3"); time.sleep(0.5)
    st = state()
    check("Super+3 goes there: only its window is on screen", st.get("ws") == 3 and [t for t, v in wins().items() if v["focused"]] == sent)
    keys("tap", "Super_L", "1"); time.sleep(0.5)
    check("Super+1 comes back", state().get("ws") == 1)
    listed = subprocess.run([WM, "--keys"], env=env, capture_output=True, text=True).stdout
    check("the workspace keys are in no list", "orkspace" not in listed and "send" not in listed and "Super + 1" not in listed)
    # bring the sent window home for the checks that follow
    keys("tap", "Super_L", "3"); time.sleep(0.4); msg("send", "1"); time.sleep(0.3); keys("tap", "Super_L", "1"); time.sleep(0.5)
    # two screens appear (as when one is plugged in), then one goes again
    (tmp / "monitors").write_text("0 0 720 900\n720 0 720 900\n")
    msg("screens", "changed"); time.sleep(0.9)
    check("a second screen is picked up without a restart", len(state().get("monitors", [])) == 2)
    w = wins(); foc = [t for t, v in w.items() if v["focused"]][0]
    x0 = w[foc]["rect"][0]
    msg("monitor", "next"); time.sleep(0.8)
    x1 = wins()[foc]["rect"][0]
    check("Super+Ctrl+Right sends the window to the other screen", (x0 < 720) != (x1 < 720))
    msg("focus", "monitor", "next"); time.sleep(0.4)
    check("the windows are tiled on each screen on their own", all(v["rect"][0] + v["rect"][2] <= 720 + 8 or v["rect"][0] >= 720 - 8 for v in wins().values() if not v.get("tucked")))
    (tmp / "monitors").write_text("0 0 1440 900\n")
    msg("screens", "changed"); time.sleep(0.9)
    check("when a screen goes, its windows come onto the one left", all(0 <= v["rect"][0] < 1440 for v in wins().values()) and len(state()["monitors"]) == 1)
    # transparency: the window you are in, and the others
    (tmp / "config/wm.toml").write_text("[windows]\ntiling = true\nopacity_active = 0.95\nopacity_inactive = 0.6\nopaque_apps = [\"terminal\"]\n")
    msg("reload"); time.sleep(0.8)
    w = wins()
    foc = [v for v in w.values() if v["focused"]][0]
    others = [v for t, v in w.items() if not v["focused"] and t != "Terminal"]
    check("the focused window and the others get their own opacity", abs(foc["opacity"] - 0.95) < 0.01 and others and all(abs(v["opacity"] - 0.6) < 0.01 for v in others))
    check("an 'always solid' app stays solid", abs(w["Terminal"]["opacity"] - 1.0) < 0.01)
    if os.environ.get("OPACITY_SHOT") and shutil.which("import"):
        subprocess.run(["import", "-window", "root", os.environ["OPACITY_SHOT"]], env=env)
    msg("close"); time.sleep(0.6)
    check("close asks the focused window to leave", len(wins()) == 3)
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
