#!/usr/bin/env python3
"""The Samurai login screen: a full-screen window in its own X server, talking to greetd.

greetd runs /opt/samurai/bin/samurai-greeter-run as its greeter user; that starts X with this
program. You pick a user and a session and type the password; greetd checks it (PAM) and, once
this program exits, starts the session on the same screen. Nothing here sees a password after
it is sent, and nothing is written to disk.

Config: /etc/samurai/greeter.json  { "ground": "night", "users": [...], "wall": "...svg" }
Test without greetd: SAMURAI_GREETER_FAKE=1 (password "samurai" works).
SAMURAI_GREETER_SAFE=1: safe mode (no wallpaper, no animations; the launcher also picks software
drawing). SAMURAI_GREETER_CHECK=1: load the screen, then leave with 0 (the setup's self-check).

Any failure ends this program with a non-zero code, so samurai-greeter-run can try safe mode or
again: it never hangs and never shows a half-drawn screen.
"""
import json, os, socket, struct, subprocess, sys, traceback
from pathlib import Path

try:
    from PySide6.QtCore import QObject, QProcess, QProcessEnvironment, QTimer, QUrl, Property, Signal, Slot
    from PySide6.QtGui import QGuiApplication, QCursor
    from PySide6.QtQml import QQmlApplicationEngine
    from PySide6.QtQuick import QQuickWindow  # noqa: F401  (windows come back as QQuickWindow)
except Exception as e:  # PySide6 missing or broken: say so and let the launcher handle it
    print(f"samurai-greeter: PySide6 cannot be loaded ({e})", file=sys.stderr)
    sys.exit(4)

SAFE = os.environ.get("SAMURAI_GREETER_SAFE") == "1"
GREETD_TIMEOUT = 90  # seconds; PAM may wait a while (a fingerprint, a key), but never forever

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
        self.s.settimeout(GREETD_TIMEOUT)
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


SHADOW = os.environ.get("SAMURAI_SHADOW", "/opt/samurai/bin/shadow")
CAMPUS = {"uc-secure": "ucalgary", "airuc-secure": "ucalgary", "eduroam": "eduroam"}


def nm_fields(line):
    """nmcli -t: fields split on ':', a ':' inside a field written as '\\:'."""
    out, cur, esc = [], "", False
    for ch in line:
        if esc:
            cur += ch; esc = False
        elif ch == "\\":
            esc = True
        elif ch == ":":
            out.append(cur); cur = ""
        else:
            cur += ch
    out.append(cur)
    return out


def plain_error(text):
    low = text.lower()
    if "secrets were required" in low or "wrong password" in low or "invalid key" in low:
        return "Wrong password, or the network refused it."
    if "not authorized" in low or "permission" in low or "insufficient privileges" in low:
        return "The login screen is not allowed to change Wi-Fi here (see Samurai's README)."
    if "no network with ssid" in low:
        return "That network is out of range now."
    lines = [l.strip() for l in text.splitlines() if l.strip()]
    last = (lines[-1] if lines else "It did not work.").replace("Error:", "").strip()
    return last[:90] + ("…" if len(last) > 90 else "")


class Wifi(QObject):
    """Wi-Fi before logging in, through NetworkManager. Connections made here are for the whole
    machine (a polkit rule written by samurai-greeter-setup allows exactly that for the login
    screen's user). Passwords go to the tool on its standard input, never on a command line,
    and are never logged."""
    changed = Signal()

    def __init__(self):
        super().__init__()
        self._nets, self._note, self._bad, self._busy = [], "", False, False
        self._nmcli = next((d + "/nmcli" for d in os.environ.get("PATH", "/usr/bin").split(":") if os.access(d + "/nmcli", os.X_OK)), "")
        self._current = ""

    def _env(self):
        e = QProcessEnvironment.systemEnvironment()
        # the login screen's user may have / as its home: tools get a folder they can write to
        e.insert("HOME", os.environ.get("XDG_RUNTIME_DIR") or "/tmp")
        # networks joined here belong to the whole machine, so they stay up after someone logs in
        e.insert("SHADOW_SYSTEM", "1")
        return e

    @Property(bool, constant=True)
    def available(self):
        return bool(self._nmcli)

    @Property("QVariant", notify=changed)
    def nets(self):
        return self._nets

    @Property(str, notify=changed)
    def note(self):
        return self._note

    @Property(bool, notify=changed)
    def bad(self):
        return self._bad

    @Property(bool, notify=changed)
    def busy(self):
        return self._busy

    @Property(str, notify=changed)
    def current(self):
        return self._current

    def _run(self, args, done, stdin=None, timeout=45000):
        p = QProcess(self)
        p.setProcessEnvironment(self._env())
        p.setProcessChannelMode(QProcess.MergedChannels)
        timer = QTimer(self, singleShot=True, interval=timeout)
        timer.timeout.connect(p.kill)

        def finished(code, _status):
            try:
                timer.stop()
                out = bytes(p.readAll()).decode(errors="replace")
                p.deleteLater()
            except RuntimeError:  # the screen is closing (a login went through): nothing to show
                return
            try:
                done(code, out)
            except Exception as e:  # a parsing surprise must not take the login screen down
                print(f"samurai-greeter: wifi: {e}", file=sys.stderr)

        def failed(err):
            if err == QProcess.FailedToStart:  # finished() never comes for this one
                timer.stop()
                try:
                    done(127, f"{os.path.basename(args[0])} could not start")
                except Exception as e:
                    print(f"samurai-greeter: wifi: {e}", file=sys.stderr)
        p.finished.connect(finished)
        p.errorOccurred.connect(failed)
        p.start(args[0], args[1:])
        if stdin is not None:
            p.write((stdin + "\n").encode())
            p.closeWriteChannel()
        timer.start()

    @Slot()
    def refresh(self):
        if not self._nmcli:
            return
        def got(code, out):
            nets = {}
            for line in out.splitlines():
                f = nm_fields(line)
                if len(f) < 4 or not f[1]:
                    continue
                n = {"ssid": f[1], "signal": int(f[2]) if f[2].isdigit() else 0, "secure": f[3].strip() not in ("", "--"),
                     "active": f[0].strip() == "*", "campus": CAMPUS.get(f[1].lower(), "")}
                o = nets.get(n["ssid"])
                if o is None or n["signal"] > o["signal"]:
                    if o is not None:
                        n["active"] = n["active"] or o["active"]
                    nets[n["ssid"]] = n
                elif n["active"]:
                    o["active"] = True
            self._nets = sorted(nets.values(), key=lambda n: (not n["active"], -n["signal"]))[:10]
            self._current = next((n["ssid"] for n in self._nets if n["active"]), "")
            if code != 0 and not self._nets:
                self._note, self._bad = plain_error(out), True
            self.changed.emit()
        self._run([self._nmcli, "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "auto"], got, timeout=20000)

    def _finish(self, ok_text):
        def done(code, out):
            self._busy = False
            if code == 0:
                self._note, self._bad = ok_text, False
            else:
                self._note, self._bad = plain_error(out), True
            self.changed.emit()
            self.refresh()
        return done

    @Slot(str, str)
    def join(self, ssid, password):
        if self._busy or not ssid:
            return
        self._busy, self._note, self._bad = True, "Connecting…", False
        self.changed.emit()
        if password:
            self._run([SHADOW, "wifi", "connect", ssid, "--password-stdin"], self._finish(f"Connected to {ssid}"), stdin=password)
        else:
            self._run([self._nmcli, "device", "wifi", "connect", ssid], self._finish(f"Connected to {ssid}"))

    @Slot(str, str, str)
    def campus(self, preset, user, password):
        if self._busy or not user:
            return
        self._busy, self._note, self._bad = True, "Setting up the campus network…", False
        self.changed.emit()
        self._run([SHADOW, "wifi", "campus", preset, "--user", user, "--password-stdin"], self._finish("Campus Wi-Fi is set up"), stdin=password, timeout=90000)


def load_conf(path):
    """The login screen's settings, checked one by one: anything missing or wrong becomes the
    default, so a damaged file can never stop the login screen."""
    try:
        raw = json.loads(path.read_text())
    except (OSError, ValueError, UnicodeDecodeError) as e:
        if path.exists():
            print(f"samurai-greeter: {path} unreadable ({e}); using defaults", file=sys.stderr)
        raw = {}
    if not isinstance(raw, dict):
        raw = {}
    conf = {"ground": "night", "users": [], "wall": "", "scale": None}  # None: from the screen
    if raw.get("ground") in ("night", "paper"):
        conf["ground"] = raw["ground"]
    if isinstance(raw.get("users"), list):
        conf["users"] = [u for u in raw["users"] if isinstance(u, str) and u and len(u) < 64][:50]
    if isinstance(raw.get("wall"), str) and not SAFE:
        conf["wall"] = raw["wall"]
    if raw.get("scale") not in (None, "auto"):
        try:
            conf["scale"] = min(3.0, max(0.75, float(raw["scale"])))
        except (TypeError, ValueError):
            pass
    return conf


def screen_scale():
    """The same rule as Samurai's first login: about 110 dots per inch is 100 %; quarter steps,
    100 % to 250 %. A screen that reports no believable size stays at 100 %."""
    try:
        scr = QGuiApplication.primaryScreen()
        mm = scr.physicalSize().width()
        px = scr.geometry().width() * scr.devicePixelRatio()
        if mm <= 50 or px <= 0:
            return 1.0
        dpi = px / (mm / 25.4)
        return min(2.5, max(1.0, round(dpi / 110 * 4) / 4))
    except Exception:
        return 1.0


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
        self.conf = load_conf(CONF)
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
        v = self.conf.get("scale")
        return float(v) if v else screen_scale()

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
        try:
            cache = Path(os.environ.get("XDG_RUNTIME_DIR") or "/tmp") / f"samurai-greeter-icons-{os.getuid()}"
            cache.mkdir(parents=True, exist_ok=True)
            f = cache / f"{name}-{color.lstrip('#')}.svg"
            if not f.exists():
                body = ICONS.get(name, '<circle cx="12" cy="12" r="8"/>')
                f.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{body}</svg>')
            return QUrl.fromLocalFile(str(f)).toString()
        except OSError:
            return ""  # no icon is better than no login screen

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
        except (OSError, ValueError, ConnectionError, KeyError, TypeError, struct.error) as e:
            # greetd went away or answered nonsense: drop the connection, say so, stay usable
            try:
                if self.g is not None and hasattr(self.g, "s"):
                    self.g.s.close()
            except OSError:
                pass
            self.g = None
            self.said.emit(f"The login service did not answer ({e.__class__.__name__}). Try again.", True)
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
    if SAFE:
        os.environ.setdefault("QT_QUICK_BACKEND", "software")
    app = QGuiApplication(sys.argv)
    app.setOverrideCursor(QCursor())
    core = Core()
    eng = QQmlApplicationEngine()
    eng.addImportPath(str(SETTINGS_QML))
    eng.rootContext().setContextProperty("core", core)
    wifi = Wifi()
    eng.rootContext().setContextProperty("wifi", wifi)
    # tests and pictures: open a panel at start ("wifi", or "wifi:SSID" to pick a network too)
    eng.rootContext().setContextProperty("openAtStart", os.environ.get("SAMURAI_GREETER_OPEN", ""))
    eng.rootContext().setContextProperty("safeMode", SAFE)
    eng.rootContext().setContextProperty("partsDir", QUrl.fromLocalFile(str(SETTINGS_QML) + "/").toString())
    eng.load(QUrl.fromLocalFile(str(HERE / "Greeter.qml")))
    if not eng.rootObjects():
        print("samurai-greeter: the screen could not be loaded", file=sys.stderr)
        sys.exit(3)
    if os.environ.get("SAMURAI_GREETER_CHECK") == "1":
        print("samurai-greeter: check ok" + (" (safe mode)" if SAFE else ""))
        del eng
        sys.exit(0)
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


def _fatal(kind, value, tb):
    # an error inside a button press would otherwise be printed and ignored, leaving a screen
    # that may no longer work: leave instead, so the launcher starts a fresh one
    traceback.print_exception(kind, value, tb)
    print("samurai-greeter: stopped after an error", file=sys.stderr)
    os._exit(5)


if __name__ == "__main__":
    sys.excepthook = _fatal
    try:
        main()
    except SystemExit:
        raise
    except BaseException:
        _fatal(*sys.exc_info())
