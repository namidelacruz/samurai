#!/usr/bin/env python3
"""Wiring check: load shell.qml and greeter/shell.qml against stand-in Quickshell modules.

The stand-ins (tests/qml/stubs, plus Process/FileView here) have the same names and
properties as the real ones but no Wayland behind them. Process really runs commands, so
Core.qml talks to the real `samurai` binary in a throwaway config folder, and to a stand-in
`niri` command (tests/bin/niri) that answers with a fixed set of windows and workspaces. This catches wrong
ids, bad bindings and broken command wiring. It cannot catch a mismatch with the real
Quickshell API or anything about how niri draws the windows.

    QT_QPA_PLATFORM=offscreen python3 tests/qml/wiring.py path/to/samurai
"""
import os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
from PySide6.QtCore import QObject, QTimer, QUrl, Property, Signal, Slot, QFileSystemWatcher, qInstallMessageHandler
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickItem  # so items come back as items
from PySide6.QtQml import QQmlApplicationEngine, QQmlExpression, qmlContext, qmlRegisterType

ROOT = Path(__file__).resolve().parents[2]
BIN = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "target/release/samurai").resolve()
tmp = Path(tempfile.mkdtemp(prefix="samurai-wiring-"))
os.environ.update(HOME=str(tmp), XDG_CONFIG_HOME=str(tmp / "config"), XDG_STATE_HOME=str(tmp / "state"),
                  PATH=str(ROOT / "tests/bin") + ":" + str(BIN.parent) + ":" + os.environ["PATH"],
                  NIRI_FAKE_LOG=str(tmp / "niri-actions.log"))
os.environ.pop("SAMURAI_CONFIG", None); os.environ.pop("SAMURAI_STATE", None)

class StdioCollector(QObject):
    textChanged = Signal()
    def __init__(self, p=None): super().__init__(p); self._t = ""
    text = Property(str, lambda s: s._t, notify=textChanged)

KEEP = []   # QML owns these objects; without a Python reference their slots stop being called
class Process(QObject):
    started = Signal(); exited = Signal(int, int); changed = Signal()
    def __init__(self, p=None):
        super().__init__(p); KEEP.append(self); self._cmd = []; self._run = False; self._out = None; self._err = None; self._stdin = False; self._p = None
    def _set_run(self, v):
        if v and not self._run and self._cmd:
            self._run = True; self._in = None
            QTimer.singleShot(0, lambda: self._go())
    def _go(self):
        self.started.emit()                      # the QML side may write() its stdin here
        try:
            r = subprocess.run(self._cmd, input=self._in, capture_output=True, text=True, timeout=20, stdin=None if self._in is not None else subprocess.DEVNULL)
            code, out, err = r.returncode, r.stdout, r.stderr
        except (OSError, subprocess.TimeoutExpired) as e:
            code, out, err = 127, "", str(e)
        if isinstance(self._out, SplitParser):
            for line in out.splitlines(): self._out.read.emit(line)
        elif self._out: self._out._t = out
        if self._err: self._err._t = err
        self._run = False; self.exited.emit(code, 0)
    @Slot(str)
    def write(self, s): self._in = s
    command = Property("QVariantList", lambda s: s._cmd, lambda s, v: setattr(s, "_cmd", [str(x) for x in v]), notify=changed)
    running = Property(bool, lambda s: s._run, _set_run, notify=changed)
    stdinEnabled = Property(bool, lambda s: s._stdin, lambda s, v: setattr(s, "_stdin", v), notify=changed)
    stdout = Property(QObject, lambda s: s._out, lambda s, v: setattr(s, "_out", v), notify=changed)
    stderr = Property(QObject, lambda s: s._err, lambda s, v: setattr(s, "_err", v), notify=changed)

class SplitParser(QObject):
    read = Signal(str)

class FileView(QObject):
    fileChanged = Signal(); loaded = Signal(); changed = Signal()
    def __init__(self, p=None):
        super().__init__(p); KEEP.append(self); self._path = ""; self._w = QFileSystemWatcher(self); self._w.directoryChanged.connect(lambda _: self.fileChanged.emit()); self._w.fileChanged.connect(lambda _: self.fileChanged.emit())
    def _set_path(self, v):
        self._path = v; d = os.path.dirname(v); os.makedirs(d, exist_ok=True); self._w.addPath(d)
        QTimer.singleShot(0, self.loaded.emit)
    @Slot(result=str)
    def text(self):
        try: return Path(self._path).read_text()
        except OSError: return ""
    @Slot()
    def reload(self): pass
    path = Property(str, lambda s: s._path, _set_path, notify=changed)
    watchChanges = Property(bool, lambda s: True, lambda s, v: None, notify=changed)
    printErrors = Property(bool, lambda s: True, lambda s, v: None, notify=changed)

class IpcHandler(QObject):
    changed = Signal()
    def __init__(self, p=None): super().__init__(p); self._t = ""
    target = Property(str, lambda s: s._t, lambda s, v: setattr(s, "_t", v), notify=changed)

for cls in (StdioCollector, SplitParser, Process, FileView, IpcHandler): qmlRegisterType(cls, "Quickshell.Io", 1, 0, cls.__name__)

# a copy of the shell with the lines only a real layer-shell window understands taken out
shell = tmp / "shell"; shutil.copytree(ROOT / "shell", shell, symlinks=True)
for f in (shell / "shell.qml", shell / "greeter/shell.qml"):
    t = f.read_text()
    t = re.sub(r"WlrLayershell\.\w+: [^;\n]*;?", "", t)
    t = re.sub(r"anchors \{ (?:top|bottom|left|right): [^}]*\}", "", t)
    f.write_text(t)

problems = []
def handler(mode, ctx, msg):
    if msg.startswith(("launch ", "greetd launch")) or "propagateSizeHints" in msg: return
    problems.append(msg); print("  ! " + msg)
qInstallMessageHandler(handler)
app = QGuiApplication(sys.argv)
checks = []
def check(name, ok):
    checks.append((name, bool(ok))); print(("  ok   " if ok else "  FAIL ") + name)

def load(path):
    e = QQmlApplicationEngine()
    e.addImportPath(str(Path(__file__).parent / "stubs"))
    e.rootContext().setContextProperty("__env", dict(os.environ))
    e.load(QUrl.fromLocalFile(str(path)))
    return e
def ev(obj, expr):
    r = QQmlExpression(qmlContext(obj), obj, expr).evaluate()
    return r[0] if isinstance(r, tuple) else r
def tree(o, seen):
    if id(o) in seen: return
    seen[id(o)] = o
    for c in o.children(): tree(c, seen)
    if hasattr(o, "childItems"):
        for c in o.childItems(): tree(c, seen)       # Repeater delegates hang here, not under children()
def find(root, name):
    seen = {}; tree(root, seen)
    return [o for o in seen.values() if o.metaObject().className().startswith(name + "_QML")]
def later(ms, fn): QTimer.singleShot(ms, fn)

subprocess.run([str(BIN), "seal", "forge"], check=True, capture_output=True)
eng = load(shell / "shell.qml")
if not eng.rootObjects(): print("shell.qml did not load"); sys.exit(1)
root = eng.rootObjects()[0]

def step1():
    check("schema loaded from `samurai schema` (24 pages)", ev(root, "kernel.schema.pages.length") == 24)
    check("settings loaded from `samurai dump`", ev(root, "Object.keys(kernel.values).length") > 150)
    check("binds loaded from `samurai binds list`", ev(root, "kernel.binds.length") > 40)
    check("status loaded (base black)", ev(root, "kernel.base") == "black")
    check("apps listed", ev(root, "kernel.apps.length") == 2)
    check("windows read from niri: one open, one minimized on the shelf",
          ev(root, "kernel.windows.length === 2 && kernel.windows[0].focused && !kernel.windows[0].minimized && kernel.windows[1].minimized && kernel.activeWorkspace === 1 && kernel.focusedOutput === 'eDP-1'") is True)
    check("bar workspace 2 maps past the shelf to niri index 3", ev(root, "kernel.workspaceIndex(2)") == 3)
    rails = find(root, "RailContent")
    if rails:
        cells = [c for c in find(rails[0], "Cell") if ev(c, "typeof clicked") == "function"]
        ev(root, 'kernel.run(["samurai", "win", "restore", "11"])')
    check("bar, rail, wall built for the screen", len(find(root, "BarContent")) == 1 and len(find(root, "RailContent")) == 1 and len(find(root, "WallContent")) == 1)
    ev(root, 'kernel.setValue("look.base", "white")')
    ev(root, 'kernel.toggle("start")')
    later(1200, step2)
def step2():
    check("a change from the shell reaches settings.json", "white" in subprocess.run([str(BIN), "get", "look.base"], capture_output=True, text=True).stdout)
    check("the file watcher brings it back: the ground is now white", ev(root, "kernel.white") is True and ev(root, "kernel.status.base") == "white")
    check("start panel opens", ev(root, "kernel.panel") == "start")
    acts = (tmp / "niri-actions.log").read_text() if (tmp / "niri-actions.log").exists() else ""
    check("restoring a minimized window asks niri to move and focus it", "move-window-to-workspace --window-id 11" in acts and "focus-window --id 11" in acts)
    ev(root, 'kernel.openHub("connect")')
    check("hub opens on a page and closes the panel", ev(root, "kernel.hubOpen && kernel.hubPage === 'connect' && kernel.panel === ''") is True)
    ev(root, 'kernel.setValue("no.such.key", 1)')
    ev(root, 'kernel.power("lock")')
    later(1000, step3)
def step3():
    check("a refused setting is reported, not kept", ev(root, "kernel.lastError !== '' && kernel.v('no.such.key') === undefined") is True)
    faces = find(root, "LockContent")
    check("locking builds the lock face", ev(root, "kernel.locked") is True and len(faces) == 1)
    if faces:
        ev(faces[0], 'submit("nami", "wrong")')
        check("a wrong password keeps it locked and says so", ev(root, "kernel.locked && pam.bad && pam.said !== ''") is True)
        ev(faces[0], 'submit("nami", "katana")')
        check("the right password unlocks", ev(root, "kernel.locked") is False)
    later(100, step4)
greeter = []
def step4():
    g = load(shell / "greeter/shell.qml"); greeter.append(g)
    if not g.rootObjects(): check("greeter/shell.qml loads", False); return finish()
    later(600, step5)
def step5():
    groot = greeter[0].rootObjects()[0]
    faces = find(groot, "LockContent")
    check("the login face is built", len(faces) == 1 and ev(faces[0], "mode") == "login")
    if faces:
        ev(faces[0], 'submit("nami", "wrong")')
        check("login: wrong password is refused", ev(groot, "root.bad && !root.busy") is True)
        ev(faces[0], 'submit("nami", "katana")')
        check("login: right password launches the session", ev(groot, "Greetd.launchedWith !== null && Greetd.launchedWith[0] === 'samurai-session'") is True)
    finish()
def finish():
    bad = [n for n, ok in checks if not ok]
    print(f"{len(checks)} checks, {len(bad)} failed, {len(problems)} QML warnings")
    shutil.rmtree(tmp, ignore_errors=True)
    os._exit(1 if bad or problems else 0)
later(1800, step1)
later(20000, lambda: (print("timed out"), os._exit(1)))
app.exec()
