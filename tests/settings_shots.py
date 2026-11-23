#!/usr/bin/env python3
"""Render every Settings page offscreen, on both grounds, and fail on any QML warning.

    QT_QPA_PLATFORM=offscreen python3 tests/settings_shots.py OUT_DIR
Needs `samurai` (and `samurai-wm` for the shortcut list) on PATH and SAMURAI_CONFIG set to a
throwaway folder. Proves the pages load and lay out; not that the commands behind them work on
a real machine.
"""
import os, sys, json, subprocess, faulthandler
faulthandler.dump_traceback_later(int(os.environ.get("SHOTS_TIMEOUT", "100")), exit=True)
from pathlib import Path
from PySide6.QtCore import QTimer, QUrl, qInstallMessageHandler
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow  # so the window comes back as a QQuickWindow

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "settings"))
import samurai_settings as ss

out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/settings-shots")
out.mkdir(parents=True, exist_ok=True)
problems = []
def handler(mode, ctx, msg):
    if "propagateSizeHints" in msg or "QFont::" in msg:
        return
    problems.append(msg); print("  ! " + msg)
qInstallMessageHandler(handler)
app = QGuiApplication(sys.argv)
core = ss.Core(None)
eng = QQmlApplicationEngine()
eng.rootContext().setContextProperty("core", core)
eng.load(QUrl.fromLocalFile(str(ROOT / "settings/qml/Main.qml")))
if not eng.rootObjects():
    print("Main.qml did not load"); sys.exit(1)
win = eng.rootObjects()[0]
pages = [p["id"] for p in core._schema["pages"]]
plan = [(p, g) for g in ("night", "paper") for p in pages]
def step(i=0):
    if i >= len(plan):
        print(f"{len(plan)} pages, {len(problems)} warnings -> {out}")
        app.exit(1 if problems else 0); return
    pid, g = plan[i]
    if win.property("pageId") != pid or (i and plan[i - 1][1] != g):
        core._status["ground"] = g
        core.themeChanged.emit()
        win.setProperty("pageId", pid)
    def snap():
        win.grabWindow().save(str(out / f"{g}-{pid}.png"))
        step(i + 1)
    QTimer.singleShot(250, snap)
QTimer.singleShot(300, step)
code = app.exec()
del eng
sys.exit(code)
