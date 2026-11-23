#!/usr/bin/env python3
"""The Samurai login screen: a full-screen window in its own X server, talking to greetd.

greetd runs /opt/samurai/bin/samurai-greeter-run as its greeter user; that starts X with this
program. You pick a user and a session and type the password; greetd checks it (PAM) and, once
this program exits, starts the session on the same screen. Nothing here sees a password after
it is sent, and nothing is written to disk.

Config: /etc/samurai/greeter.json  { "ground": "night", "users": [...], "wall": "...svg" }
Test without greetd: SAMURAI_GREETER_FAKE=1 (password "samurai" works).
"""
import json, os, socket, struct, subprocess, sys
from pathlib import Path

from PySide6.QtCore import QObject, QTimer, QUrl, Property, Signal, Slot
from PySide6.QtGui import QGuiApplication, QCursor
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow  # noqa: F401  (windows come back as QQuickWindow)

HERE = Path(__file__).resolve().parent
SETTINGS_QML = Path(os.environ.get("SAMURAI_SETTINGS_QML", HERE.parent / "settings" / "qml"))
CONF = Path(os.environ.get("SAMURAI_GREETER_CONF", "/etc/samurai/greeter.json"))

sys.path.insert(0, str(SETTINGS_QML.parent))
try:
    from samurai_settings import NIGHT, PAPER, ACCENT, ICONS
except ImportError:  # the greeter must start even if Settings is missing
    NIGHT = dict(ground="#101114", surf="#1A1C21", chrome="#1F2228", raised="#252930", line="#30343C", text="#ECEBE6", dim="#A3A098", onAccent="#0E1424", field="#14161A")
    PAPER = NIGHT
    ACCENT = {"indigo": ("#2F4B8F", "#93ABEB")}
    ICONS = {}


class Greetd:
    """greetd's IPC: a u32 length (native order) and a JSON message, each way."""
    def __init__(self, path):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(path)

    def ask(self, msg):
        data = json.dumps(msg).encode()
        self.s.sendall(struct.pack("=I", len(data)) + data)
        n = struct.unpack("=I", self._read(4))[0]
        return json.loads(self._read(n))

    def _read(self, n):
        b = b""
        while len(b) < n:
            chunk = self.s.recv(n - len(b))
            if not chunk:
                raise ConnectionError("greetd closed the connection")
            b += chunk
        return b


class Fake:
    """Stands in for greetd when testing: password "samurai" is right."""
    def __init__(self):
        self.pw = None
    def ask(self, msg):
        t = msg["type"]
        if t == "create_session":
            return {"type": "auth_message", "auth_message_type": "secret", "auth_message": "Password:"}
        if t == "post_auth_message_response":
            if msg.get("response") == "samurai":
                return {"type": "success"}
            return {"type": "error", "error_type": "auth_error", "description": "auth failed"}
        if t == "start_session":
            print("would start:", msg["cmd"], flush=True)
            return {"type": "success"}
        return {"type": "success"}


def sessions():
    out = [dict(name="Samurai", cmd=["/opt/samurai/bin/samurai-session"])]
    for d in ("/usr/share/xsessions",):
        p = Path(d)
        if not p.is_dir():
            continue
        for f in sorted(p.glob("*.desktop")):
            if f.name == "samurai.desktop":
                continue
            kv = {}
            for l in f.read_text(errors="replace").splitlines():
                if "=" in l and not l.startswith("#"):
                    k, v = l.split("=", 1)
                    kv.setdefault(k.strip(), v.strip())
            if kv.get("Exec") and kv.get("Hidden") != "true":
                out.append(dict(name=kv.get("Name", f.stem), cmd=["/opt/samurai/bin/samurai-session", "--run", kv["Exec"]]))
    return out


class Core(QObject):
    changed = Signal()
    said = Signal(str, bool)

    def __init__(self):
        super().__init__()
        try:
            self.conf = json.loads(CONF.read_text())
        except (OSError, ValueError):
            self.conf = {}
        self._users = self.conf.get("users") or []
        self._sessions = sessions()
        self._busy = False
        fake = os.environ.get("SAMURAI_GREETER_FAKE")
        sock = os.environ.get("GREETD_SOCK")
        self.g = Fake() if fake or not sock else None
        self.sock = sock

    def _g(self):
        if self.g is None:
            self.g = Greetd(self.sock)
        return self.g

    @Property("QVariant", constant=True)
    def colors(self):
        night = self.conf.get("ground", "night") != "paper"
        c = dict(NIGHT if night else PAPER)
        acc = ACCENT.get("indigo")
        c["accent"] = acc[1] if night else acc[0]
        c["soft"] = "#" + ("29" if night else "1A") + c["accent"].lstrip("#")
        return c

    @Property(float, constant=True)
    def scale(self):
        try:
            return float(self.conf.get("scale", 1.0))
        except (TypeError, ValueError):
            return 1.0

    @Property("QVariant", constant=True)
    def users(self):
        return self._users

    @Property("QVariant", constant=True)
    def sessions(self):
        return [s["name"] for s in self._sessions]

    @Property(str, constant=True)
    def wall(self):
        w = self.conf.get("wall") or ""
        return QUrl.fromLocalFile(w).toString() if w and Path(w).exists() else ""

    @Property(bool, notify=changed)
    def busy(self):
        return self._busy

    @Slot(str, str, result=str)
    def icon(self, name, color):
        color = color[:7] if color.startswith("#") else "#888888"
        cache = Path(os.environ.get("XDG_RUNTIME_DIR") or "/tmp") / "samurai-greeter-icons"
        cache.mkdir(parents=True, exist_ok=True)
        f = cache / f"{name}-{color.lstrip('#')}.svg"
        if not f.exists():
            body = ICONS.get(name, '<circle cx="12" cy="12" r="8"/>')
            f.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{body}</svg>')
        return QUrl.fromLocalFile(str(f)).toString()

    asked = Signal(str, bool)  # a further question from the login rules (a code…), secret or not

    @Slot(str, str, int)
    def login(self, user, password, session):
        """Ask greetd. On success the session starts and this program leaves."""
        if self._busy or not user:
            return
        self._busy = True
        self._user, self._pw, self._session = user, password, session
        self._sent_user = self._sent_pw = False
        self.changed.emit()
        QTimer.singleShot(10, lambda: self._step(lambda g: g.ask({"type": "create_session", "username": user})))

    @Slot(str)
    def answer(self, text):
        """The answer to a question shown with `asked`."""
        self._busy = True
        self.changed.emit()
        QTimer.singleShot(10, lambda: self._step(lambda g: g.ask({"type": "post_auth_message_response", "response": text})))

    def _step(self, first):
        try:
            g = self._g()
            r = first(g)
            for _ in range(12):
                if r.get("type") != "auth_message":
                    break
                kind, text = r.get("auth_message_type"), r.get("auth_message", "")
                low = text.lower()
                if kind == "visible" and not self._sent_user and any(w in low for w in ("user", "login", "name")):
                    self._sent_user = True
                    r = g.ask({"type": "post_auth_message_response", "response": self._user})
                elif kind == "secret" and not self._sent_pw:
                    self._sent_pw = True
                    r = g.ask({"type": "post_auth_message_response", "response": self._pw})
                    self._pw = ""
                elif kind in ("secret", "visible"):
                    # something else is asked (a one-time code…): show it and wait
                    self._busy = False
                    self.changed.emit()
                    self.asked.emit(text.strip() or "Answer:", kind == "secret")
                    return
                else:
                    if text:
                        self.said.emit(text, kind == "error")
                    r = g.ask({"type": "post_auth_message_response"})
            if r.get("type") == "success":
                cmd = self._sessions[self._session]["cmd"] if 0 <= self._session < len(self._sessions) else self._sessions[0]["cmd"]
                env = ["XDG_SESSION_TYPE=x11"] + (["XDG_SESSION_DESKTOP=samurai", "XDG_CURRENT_DESKTOP=Samurai"] if self._session <= 0 else [])
                r = g.ask({"type": "start_session", "cmd": cmd, "env": env})
                if r.get("type") == "success":
                    self.said.emit("Starting…", False)
                    # tells samurai-greeter-run this was a login, not a crash
                    done = os.environ.get("SAMURAI_GREETER_DONE")
                    if done:
                        try:
                            Path(done).write_text("ok\n")
                        except OSError:
                            pass
                    QGuiApplication.quit()
                    return
            if r.get("type") == "error":
                try:
                    g.ask({"type": "cancel_session"})
                except (OSError, ValueError, ConnectionError):
                    self.g = None
            msg = "Wrong user or password." if r.get("error_type") == "auth_error" else (r.get("description") or "The session could not start.")
            self.said.emit(msg, True)
        except (OSError, ValueError, ConnectionError) as e:
            self.g = None
            self.said.emit(f"The login service did not answer ({e}).", True)
        finally:
            self._pw = ""
            self._busy = False
            self.changed.emit()

    @Slot(str)
    def power(self, what):
        cmd = {"restart": ["systemctl", "reboot"], "poweroff": ["systemctl", "poweroff"], "sleep": ["systemctl", "suspend"]}.get(what)
        if cmd:
            subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    @Slot(result=str)
    def clock(self):
        import time
        return time.strftime("%H:%M")

    @Slot(result=str)
    def date(self):
        import time
        t = time.localtime()
        return time.strftime("%A, ") + str(t.tm_mday) + time.strftime(" %B")


def main():
    os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
    app = QGuiApplication(sys.argv)
    app.setOverrideCursor(QCursor())
    core = Core()
    eng = QQmlApplicationEngine()
    eng.addImportPath(str(SETTINGS_QML))
    eng.rootContext().setContextProperty("core", core)
    eng.rootContext().setContextProperty("partsDir", QUrl.fromLocalFile(str(SETTINGS_QML) + "/").toString())
    eng.load(QUrl.fromLocalFile(str(HERE / "Greeter.qml")))
    if not eng.rootObjects():
        sys.exit(1)
    shot = os.environ.get("SAMURAI_GREETER_SHOT")
    if shot:  # tests: picture the screen, then leave
        win = eng.rootObjects()[0]
        def snap():
            from PySide6.QtQuick import QQuickWindow  # noqa: F401
            win.grabWindow().save(shot)
            app.quit()
        QTimer.singleShot(900, snap)
    code = app.exec()
    del eng
    sys.exit(code)


if __name__ == "__main__":
    main()
