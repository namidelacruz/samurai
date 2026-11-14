#!/usr/bin/env python3
"""Offscreen render check for the pure-QtQuick parts of the shell.

Loads each *Content.qml with CoreBase and canned data (no Quickshell, no Hyprland), saves a
PNG per scene, and fails on any QML warning. This proves the files parse, bind and lay out;
it does not prove anything about Quickshell windows or Hyprland.

    QT_QPA_PLATFORM=offscreen python3 tests/qml/render.py OUT_DIR
"""
import json, os, sys
from pathlib import Path
from PySide6.QtCore import QUrl, QTimer, qInstallMessageHandler
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickView

ROOT = Path(__file__).resolve().parents[2]
SHELL = ROOT / "shell"
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/samurai-render")
OUT.mkdir(parents=True, exist_ok=True)
schema = json.loads((SHELL / "settings.schema.json").read_text())
values = {it["key"]: it["default"] for p in schema["pages"] for s in p["sections"] for it in s["items"] if it.get("key") and "default" in it}
binds = json.loads(os.environ.get("SAMURAI_BINDS", "[]"))
seal_dir = os.environ.get("SAMURAI_SEAL", "")

def status(base, incognito):
    return {"battery": {"percent": 82, "charging": False, "ac": False}, "network": {"wifi_on": True, "wifi": {"ssid": "UC-Secure", "signal": 74}, "wired": False},
            "volume": 45, "muted": False, "brightness": 70, "user": "nami", "host": "katana", "base": base, "accent": "#C8102E", "incognito": incognito,
            "update": {"available": True, "commit": "a1b2c3d4e5f6"}, "seal": {"forged": True, "mark": "15A2·2B54·1239·066B"}, "recording": False, "bluetooth": True}

windows = [
    {"address": "0x1", "cls": "kitty", "title": "nami@katana: ~/samurai", "workspace": 1, "focused": True, "minimized": False},
    {"address": "0x2", "cls": "firefox", "title": "Hyprland Wiki", "workspace": 1, "focused": False, "minimized": False},
    {"address": "0x3", "cls": "org.gnome.Nautilus", "title": "Home", "workspace": 2, "focused": False, "minimized": True},
    {"address": "0x4", "cls": "thunderbird", "title": "Inbox", "workspace": 3, "focused": False, "minimized": True},
]
notices = [{"id": 1, "app": "Samurai", "summary": "Update ready", "body": "a1b2c3d4e5f6 was fetched and checked. Install from Settings › Updates."},
           {"id": 2, "app": "Wi-Fi", "summary": "UC-Secure repaired", "body": "The certificate check failed once; the connection was repaired."}]
apps = [{"id": "kitty", "name": "Kitty"}, {"id": "firefox", "name": "Firefox"}, {"id": "org.gnome.Nautilus", "name": "Files"}, {"id": "thunderbird", "name": "Thunderbird"}]

HARNESS = """
import QtQuick
import "file://%(shell)s/lib"
import "file://%(shell)s/frame"
import "file://%(shell)s/hub"
import "file://%(shell)s/lock"
Rectangle {
    id: root
    width: %(w)d; height: %(h)d; color: core.ground
    CoreBase { id: core; panel: "%(panel)s"; hubPage: "%(page)s"
        property string sealDir: "%(seal)s"; property string lastError: ""; property var popups: [] }
    function feed(o) { core.schema = o.schema; core.values = o.values; core.status = o.status; core.windows = o.windows; core.notices = o.notices; core.apps = o.apps; core.binds = o.binds }
    %(body)s
}
"""
DESK = """
    WallContent { anchors.fill: parent; core: core; sealDir: core.sealDir; frameLeft: core.railW; frameTop: core.barH }
    BarContent { width: parent.width; height: core.barH; core: core; screenName: "eDP-1" }
    RailContent { y: core.barH; width: core.railW; height: parent.height - core.barH; core: core }
    StartContent { core: core; visible: core.panel === "start"; x: core.railW; y: core.barH; width: 560; height: 620 }
    QuickContent { core: core; visible: core.panel === "quick"; x: parent.width - width - core.inset; y: core.barH; width: 420; height: 640 }
    SideContent { core: core; mode: core.panel; binds: core.binds; visible: ["widgets", "power", "keys"].indexOf(core.panel) >= 0
        width: core.panel === "keys" ? 640 : core.panel === "power" ? 280 : 380; height: core.panel === "power" ? 362 : 560
        x: core.panel === "power" ? core.railW : parent.width - width - core.inset; y: core.panel === "power" ? parent.height - height - core.inset : core.barH }
"""
HUB = 'HubContent { anchors.fill: parent; core: core }'
LOCK = 'LockContent { anchors.fill: parent; core: core; mode: "%s"; user: "nami"; users: ["nami", "guest"]; sessions: [{ name: "Samurai", exec: "start-hyprland" }]; wall: core.sealDir ? "file://" + core.sealDir + "/wall-" + core.base + ".svg" : ""; message: "%s"; failed: %s }'

scenes = [("desk-black", DESK, "black", False, "", "displays", 1600, 900), ("desk-white", DESK, "white", False, "", "displays", 1600, 900),
          ("start-black", DESK, "black", False, "start", "displays", 1600, 900), ("quick-white", DESK, "white", False, "quick", "displays", 1600, 900),
          ("widgets-black", DESK, "black", False, "widgets", "displays", 1600, 900), ("power-white", DESK, "white", False, "power", "displays", 1600, 900),
          ("keys-black", DESK, "black", False, "keys", "displays", 1600, 900), ("incognito-black", DESK, "black", True, "quick", "displays", 1600, 900),
          ("lock-black", LOCK % ("lock", "", "false"), "black", False, "", "displays", 1600, 900), ("lock-white-wrong", LOCK % ("lock", "wrong password · 違います", "true"), "white", False, "", "displays", 1600, 900),
          ("login-black", LOCK % ("login", "", "false"), "black", False, "", "displays", 1600, 900)]
for p in schema["pages"]:
    scenes.append(("hub-" + p["id"], HUB, "white" if p["id"] in ("look", "region") else "black", False, "", p["id"], 1080, 1400 if p["id"] in ("look", "keybinds") else 820))

warnings = []
def handler(mode, ctx, msg):
    if "Painter" in msg or "propagateSizeHints" in msg or msg.startswith(("run [", "launch ", "power ")): return
    warnings.append(msg); print("  ! " + msg)
qInstallMessageHandler(handler)
app = QGuiApplication(sys.argv)
only = os.environ.get("ONLY", "")
todo = [s for s in scenes if only in s[0]]
views = []
def run(i=0):
    if i >= len(todo): app.quit(); return
    name, body, base, inc, panel, page, w, h = todo[i]
    print(name)
    src = HARNESS % {"shell": SHELL, "w": w, "h": h, "panel": panel, "page": page, "seal": seal_dir, "body": body}
    f = OUT / (name + ".qml"); f.write_text(src)
    v = QQuickView(); views.append(v)
    v.setSource(QUrl.fromLocalFile(str(f)))
    if v.status() != QQuickView.Status.Ready:
        for e in v.errors(): warnings.append(e.toString()); print("  ! " + e.toString())
        QTimer.singleShot(0, lambda: run(i + 1)); return
    v.rootObject().feed({"schema": schema, "values": values, "status": status(base, inc), "windows": windows, "notices": notices, "apps": apps, "binds": binds})
    v.resize(w, h); v.show()
    def snap():
        v.grabWindow().save(str(OUT / (name + ".png"))); f.unlink(); v.hide(); run(i + 1)
    QTimer.singleShot(350, snap)
QTimer.singleShot(0, run)
app.exec()
print(f"{len(todo)} scenes, {len(warnings)} warnings -> {OUT}")
sys.exit(1 if warnings else 0)
