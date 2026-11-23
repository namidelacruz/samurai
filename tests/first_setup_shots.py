#!/usr/bin/env python3
"""Pictures of the first setup's four steps (offscreen), at the given scales, and a check that
choosing the Hyprland-style set and editing a key into a clash is caught."""
import os, sys, tempfile, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
os.environ.setdefault("SAMURAI_CONFIG", tempfile.mkdtemp())
sys.path.insert(0, str(ROOT / "settings"))
import samurai_settings as ss
from PySide6.QtCore import QUrl
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow  # noqa: F401
out = Path(os.environ.get("SHOTS", "/tmp/first-setup-shots")); out.mkdir(exist_ok=True)
app = QGuiApplication(sys.argv)
warn = []
from PySide6.QtCore import qInstallMessageHandler
qInstallMessageHandler(lambda mode, ctx, msg: warn.append(msg) if "propagate" not in msg else None)
checks = []
def check(name, ok):
    checks.append(bool(ok)); print(("  ok   " if ok else "  FAIL ") + name)
core = ss.Core(None)
r = core.keySet("hyprland")
check("the Hyprland-style set: Super+Q opens a terminal, Super+C closes", {x["action"]: x["keys"] for x in r}["terminal"] == "Super+Q" and {x["action"]: x["keys"] for x in r}["close"] == "Super+C")
r[0]["keys"] = "super+c"   # overview on the close key
check("a key used twice is caught, whatever its spelling", set(core.clashes(r)) == {"super+c", "Super+C"})
for scale in [float(x) for x in os.environ.get("SCALES", "1.0").split(",")]:
    core._values["look.scale"] = scale
    eng = QQmlApplicationEngine()
    eng.rootContext().setContextProperty("core", core)
    eng.load(QUrl.fromLocalFile(str(ROOT / "settings/qml/FirstSetup.qml")))
    win = eng.rootObjects()[0]
    for step in range(4):
        win.setProperty("step", step)
        end = time.time() + 1.0
        while time.time() < end:
            app.processEvents(); time.sleep(0.02)
        win.grabWindow().save(str(out / f"step{step + 1}-{scale}.png"))
    win.close(); del eng
bad = [w for w in warn if "qml" in w.lower()]
check("the window loads without QML warnings", not bad)
for w in bad[:5]: print("   ", w)
print(f"{len(checks)} checks, {checks.count(False)} failed -> {out}")
sys.exit(0 if all(checks) else 1)
