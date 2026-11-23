#!/usr/bin/env python3
"""The login screen's Wi-Fi, with stand-ins for nmcli and shadow (tests/x11/net): the list, a
wrong and a right password (sent on stdin, never as an argument), a campus sign-in, and no
NetworkManager at all."""
import os, subprocess, sys, tempfile, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
tmp = Path(tempfile.mkdtemp())
os.environ["NET_CALLS"] = str(tmp / "calls")
os.environ["PATH"] = str(ROOT / "tests/x11/net") + ":" + os.environ["PATH"]
os.environ["SAMURAI_SHADOW"] = str(ROOT / "tests/x11/net/shadow")
os.environ["SAMURAI_GREETER_CONF"] = "/dev/null"
sys.path.insert(0, str(ROOT / "greeter"))
import greeter
from PySide6.QtGui import QGuiApplication
app = QGuiApplication(sys.argv)
checks = []
def check(name, ok):
    checks.append(bool(ok)); print(("  ok   " if ok else "  FAIL ") + name)
def wait(cond, t=8):
    end = time.time() + t
    while not cond() and time.time() < end:
        app.processEvents(); time.sleep(0.02)
w = greeter.Wifi()
check("NetworkManager found: the Wi-Fi button shows", w.available)
w.refresh(); wait(lambda: w.nets)
names = [n["ssid"] for n in w.nets]
check("networks listed, the connected one first, a name with ':' intact, hidden ones left out",
      names[:1] == ["Home 5G"] and "Neighbour:Net" in names and "" not in names and w.current == "Home 5G")
check("campus networks are recognised", {n["ssid"]: n["campus"] for n in w.nets}.get("UC-Secure") == "ucalgary")
w.join("Neighbour:Net", "wrong"); wait(lambda: not w.busy and w.note and w.note != "Connecting…")
check("a wrong password: a plain message", w.bad and "Wrong password" in w.note)
w.join("Neighbour:Net", "right"); wait(lambda: not w.busy and w.note and w.note != "Connecting…")
check("the right password connects", not w.bad and w.note.startswith("Connected"))
w.campus("ucalgary", "nami.d", "right"); wait(lambda: not w.busy and w.note and "Setting up" not in w.note)
check("campus sign-in runs the campus setup", not w.bad and "campus" in w.note.lower())
calls = (tmp / "calls").read_text()
check("passwords never appear on a command line", "wrong" not in calls and "right" not in calls and "password length 5" in calls)
os.environ["PATH"] = "/nonexistent"
check("no NetworkManager: no Wi-Fi button (the login screen is unaffected)", not greeter.Wifi().available)
print(f"{len(checks)} checks, {checks.count(False)} failed")
sys.exit(0 if all(checks) else 1)
