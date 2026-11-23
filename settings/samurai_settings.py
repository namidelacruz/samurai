#!/usr/bin/env python3
"""Samurai Settings: ten plain pages over the `samurai` command.

Every change goes through `samurai set`, which checks it against the settings list and applies it
at once (the window manager, the terminal, GTK and Qt apps follow within a second). Nothing here
writes a config file itself.

    samurai-settings [--page ID]
"""
import json, os, re, shutil, subprocess, sys
from pathlib import Path

from PySide6.QtCore import QObject, QProcess, QTimer, QUrl, Property, Signal, Slot, QFileSystemWatcher, QByteArray
from PySide6.QtGui import QGuiApplication, QFontDatabase, QIcon
from PySide6.QtQml import QQmlApplicationEngine

HERE = Path(__file__).resolve().parent
SAMURAI = os.environ.get("SAMURAI_BIN", "samurai")


def config_dir() -> Path:
    if os.environ.get("SAMURAI_CONFIG"):
        return Path(os.environ["SAMURAI_CONFIG"])
    return Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "samurai"


def sh(cmd, inp=None, timeout=20):
    try:
        r = subprocess.run(cmd, input=inp, capture_output=True, text=True, timeout=timeout)
        return r.returncode, (r.stdout + r.stderr).strip()
    except (OSError, subprocess.TimeoutExpired) as e:
        return 127, str(e)


NIGHT = dict(ground="#101114", surf="#1A1C21", chrome="#1F2228", raised="#252930", line="#30343C", text="#ECEBE6", dim="#A3A098", onAccent="#0E1424", field="#14161A")
PAPER = dict(ground="#ECE8DF", surf="#FAF8F4", chrome="#F2EFE9", raised="#FFFFFF", line="#DCD6CA", text="#1B1B1D", dim="#5F5C56", onAccent="#FFFFFF", field="#FFFFFF")
ACCENT = {"indigo": ("#2F4B8F", "#93ABEB"), "moss": ("#4A6B3A", "#9CC28A"), "rust": ("#9A4A2A", "#E39A78"), "plum": ("#6B3F73", "#C9A0D6"), "slate": ("#44505E", "#A9B6C6")}


ICONS = {
    "bt": '<path d="M7 7.5l10 9-5 4.5V3l5 4.5-10 9"/>',
    "refresh": '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
    "eye-off": '<path d="M3 3l18 18"/><path d="M10.6 6.1A9.8 9.8 0 0 1 12 6c5 0 8.5 4.5 9.5 6-.5.8-1.6 2.2-3.1 3.5M6.4 7.6C4.7 8.8 3.4 10.6 2.5 12c1 1.5 4.5 6 9.5 6 1.4 0 2.7-.3 3.8-.9"/>',
    "search": '<circle cx="11" cy="11" r="7"/><path d="M16.5 16.5L21 21"/>',
    "wifi": '<path d="M2.5 8.8a14 14 0 0 1 19 0"/><path d="M5.6 12.2a9.5 9.5 0 0 1 12.8 0"/><path d="M8.8 15.5a5 5 0 0 1 6.4 0"/><circle cx="12" cy="19" r="0.9"/>',
    "sound": '<path d="M4 9.5h3.5L12 6v12l-4.5-3.5H4z"/><path d="M15.5 9.5a3.5 3.5 0 0 1 0 5"/><path d="M18 7a7 7 0 0 1 0 10"/>',
    "mic": '<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0"/><path d="M12 17.5V21"/>',
    "ink": '<path d="M12 3c4.2 5 6.3 8.2 6.3 11.2a6.3 6.3 0 0 1-12.6 0C5.7 11.2 7.8 8 12 3z"/>',
    "display": '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8.5 20h7M12 16v4"/>',
    "keyboard": '<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6 10h.01M9.5 10h.01M13 10h.01M16.5 10h.01M7 14h10"/>',
    "mouse": '<rect x="7" y="3" width="10" height="18" rx="5"/><path d="M12 7v3"/>',
    "apps": '<rect x="4" y="4" width="7" height="7" rx="1.5"/><rect x="13" y="4" width="7" height="7" rx="1.5"/><rect x="4" y="13" width="7" height="7" rx="1.5"/><rect x="13" y="13" width="7" height="7" rx="1.5"/>',
    "plug": '<path d="M9 2.5v5M15 2.5v5"/><path d="M6.5 7.5h11v3.5a5.5 5.5 0 0 1-11 0z"/><path d="M12 16.5v5"/>',
    "bell": '<path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>',
    "info": '<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><path d="M12 7.6v.2"/>',
    "shield": '<path d="M12 3l7.5 3v6c0 4.8-3.3 7.8-7.5 9-4.2-1.2-7.5-4.2-7.5-9V6z"/>',
    "lock": '<rect x="5" y="11" width="14" height="9.5" rx="2"/><path d="M8.5 11V8a3.5 3.5 0 0 1 7 0v3"/>',
    "check": '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    "close": '<path d="M7 7l10 10M17 7L7 17"/>',
    "down": '<path d="M6.5 9.5L12 15l5.5-5.5"/>',
    "right": '<path d="M9.5 6.5L15 12l-5.5 5.5"/>',
    "repair": '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
    "doc": '<path d="M6.5 3h8l4 4v14h-12z"/><path d="M14.5 3v4h4"/><path d="M9 12.5h6M9 16h6"/>',
    "download": '<path d="M12 4v11"/><path d="M7 10.5l5 5 5-5"/><path d="M5 20h14"/>',
    "terminal": '<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M7 9.5l3 2.5-3 2.5"/><path d="M12.5 15h4.5"/>',
    "power": '<path d="M12 3.5v8"/><path d="M6.6 7a7.5 7.5 0 1 0 10.8 0"/>',
    "restart": '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
}


# ── shortcut sets (the first setup shows and edits these) ─────────────────────────────────────
# What each set binds for the actions people look for first. Samurai's are the window manager's
# built-ins; the Hyprland-style set is samurai/src/x11.rs HYPRLAND_KEYS laid over them.
KEY_ACTIONS = [
    ("overview", "Open the overview and apps"), ("terminal", "Terminal"), ("close", "Close the window"),
    ("files", "Files"), ("browser", "Web browser"), ("settings", "Settings"), ("fullscreen", "Full screen"),
    ("fill", "Fill the screen"), ("float", "Float or tile this window"), ("tuck", "Minimize (tuck away)"),
    ("taskbar", "Show minimized windows"), ("focus left", "Focus left"), ("focus right", "Focus right"),
    ("move left", "Move window left"), ("move right", "Move window right"), ("lock", "Lock the screen"),
    ("quit", "Log out"),
]
KEY_SETS = {
    "samurai": {"overview": "Super", "terminal": "Super+Return", "close": "Super+Q", "files": "Super+E",
                "browser": "Super+B", "settings": "Super+comma", "fullscreen": "Super+F", "fill": "Super+Up",
                "float": "Super+V", "tuck": "Super+M", "taskbar": "Super+T", "focus left": "Super+Alt+Left",
                "focus right": "Super+Alt+Right", "move left": "Super+Left", "move right": "Super+Right",
                "lock": "Super+L", "quit": "Super+Shift+Escape"},
    "hyprland": {"overview": "Super+R", "terminal": "Super+Q", "close": "Super+C", "files": "Super+E",
                 "browser": "Super+B", "settings": "Super+comma", "fullscreen": "Super+F", "fill": "Super+Shift+F",
                 "float": "Super+V", "tuck": "Super+H", "taskbar": "Super+T", "focus left": "Super+Left",
                 "focus right": "Super+Right", "move left": "Super+Shift+Left", "move right": "Super+Shift+Right",
                 "lock": "Super+L", "quit": "Super+M"},
}


def key_norm(k):
    """'shift+super+q' and 'Super+Shift+Q' are the same keys."""
    parts = [p.strip().lower() for p in (k or "").split("+") if p.strip()]
    if not parts:
        return ""
    mods = sorted(p for p in parts[:-1])
    return "+".join(mods + [parts[-1]])


def plain_error(text):
    """A network or Bluetooth tool's error, in plain words where the usual ones are known."""
    low = (text or "").lower()
    if "secrets were required" in low or "wrong password" in low or "invalid key" in low:
        return "Wrong password, or the network refused it."
    if "no network with ssid" in low:
        return "That network is out of range now."
    if "not authorized" in low or "insufficient privileges" in low:
        return "Not allowed: your account may not change network settings."
    if "is not installed" in low:
        return text
    lines = [l.strip() for l in (text or "").splitlines() if l.strip()]
    last = (lines[-1] if lines else "That did not work.").replace("Error:", "").strip()
    return last[:120]


def data_dirs():
    v = [Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")]
    v += [Path(d) for d in (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":") if d]
    return v


def desktop_entries():
    seen, out = set(), []
    for d in data_dirs():
        a = d / "applications"
        if not a.is_dir():
            continue
        for f in sorted(a.glob("*.desktop")):
            if f.name in seen:
                continue
            seen.add(f.name)
            try:
                text = f.read_text(errors="replace")
            except OSError:
                continue
            main = text.split("[Desktop Entry]", 1)[-1].split("\n[", 1)[0]
            kv = dict(l.split("=", 1) for l in main.splitlines() if "=" in l and not l.startswith("#"))
            if kv.get("Type") != "Application" or kv.get("NoDisplay") == "true" or kv.get("Hidden") == "true":
                continue
            out.append(dict(id=f.name, name=kv.get("Name", f.stem), mime=kv.get("MimeType", ""), cats=kv.get("Categories", "")))
    return out


class Core(QObject):
    changed = Signal()
    themeChanged = Signal()
    done = Signal(str, int, str)  # tag, exit code, text
    message = Signal(str, bool)   # text, is an error

    def __init__(self, page):
        super().__init__()
        self._schema = {}
        self._values = {}
        self._status = {}
        self._page = page or "look"
        self._procs = []
        self._after = {}
        self.load_schema()
        self.reload()
        self.watch = QFileSystemWatcher(self)
        cd = config_dir()
        cd.mkdir(parents=True, exist_ok=True)
        self.watch.addPath(str(cd))
        self.watch.directoryChanged.connect(lambda _: self._debounce.start())
        self._debounce = QTimer(self, singleShot=True, interval=150)
        self._debounce.timeout.connect(self.reload)

    # ── data ────────────────────────────────────────────────────────────────────────────────
    def load_schema(self):
        code, out = sh([SAMURAI, "schema"])
        if code == 0:
            self._schema = json.loads(out)
        else:
            p = HERE / "schema.json"
            self._schema = json.loads(p.read_text()) if p.exists() else {"pages": []}

    @Slot()
    def reload(self):
        code, out = sh([SAMURAI, "dump"])
        if code == 0:
            self._values = json.loads(out)
        code, out = sh([SAMURAI, "status"])
        if code == 0:
            try:
                self._status = json.loads(out)
            except ValueError:
                pass
        self.changed.emit()
        self.themeChanged.emit()

    @Property("QVariant", notify=changed)
    def schema(self):
        return self._schema

    @Property("QVariant", notify=changed)
    def values(self):
        return self._values

    @Property("QVariant", notify=changed)
    def status(self):
        return self._status

    @Property(str, constant=True)
    def startPage(self):
        return self._page

    @Property(bool, notify=themeChanged)
    def night(self):
        return self._status.get("ground", "night") != "paper"

    @Property("QVariant", notify=themeChanged)
    def colors(self):
        night = self._status.get("ground", "night") != "paper"
        c = dict(NIGHT if night else PAPER)
        acc = ACCENT.get(self._values.get("look.accent", "indigo"), ACCENT["indigo"])
        c["accent"] = acc[1] if night else acc[0]
        # Qt reads #AARRGGBB: alpha first
        c["soft"] = "#" + ("29" if night else "1A") + c["accent"].lstrip("#")
        return c

    @Slot(str, result=bool)
    def sealHas(self, name):
        return (config_dir() / "seal" / name).is_file()

    @Property(str, constant=True)
    def sealDir(self):
        return QUrl.fromLocalFile(str(config_dir() / "seal")).toString()

    @Property(float, notify=changed)
    def scale(self):
        try:
            return float(self._values.get("look.scale", 1.0))
        except (TypeError, ValueError):
            return 1.0

    # ── changes ─────────────────────────────────────────────────────────────────────────────
    def _run(self, tag, cmd, inp=None):
        p = QProcess(self)
        p.setProcessChannelMode(QProcess.MergedChannels)
        def fin(code, _status, p=p):
            text = bytes(p.readAll()).decode(errors="replace").strip()
            if p in self._procs:
                self._procs.remove(p)
            cb = self._after.pop(tag, None)
            if cb:
                cb(code, text)
            if code != 0 and (tag.startswith("net:") or tag.startswith("bt:")) and tag != "net:log":
                text = plain_error(text)
            self.done.emit(tag, code, text)
            p.deleteLater()
        p.finished.connect(fin)
        p.errorOccurred.connect(lambda e, p=p: (self.done.emit(tag, 127, f"{cmd[0]} is not installed"), self._procs.remove(p) if p in self._procs else None) if e == QProcess.FailedToStart else None)
        self._procs.append(p)
        p.start(cmd[0], cmd[1:])
        if inp is not None:
            p.write(QByteArray(inp.encode()))
            p.closeWriteChannel()

    @Slot(str, "QVariant")
    def set(self, key, value):
        # show it at once; the file watcher confirms, an error puts the old value back
        old = self._values.get(key)
        self._values[key] = value
        self.changed.emit()
        if key in ("look.ground", "look.accent", "look.sun_from", "look.sun_to"):
            self.themeChanged.emit()
        def back(code, text, key=key, old=old):
            if code != 0:
                self._values[key] = old
                self.message.emit(text or "That was not saved.", True)
                self.changed.emit()
            else:
                self.reload()
        self._after["set:" + key] = back
        self._run("set:" + key, [SAMURAI, "set", key, json.dumps(value)])

    @Slot(str)
    def reset(self, what):
        self._after["reset"] = lambda code, text: (self.reload(), self.message.emit(text, code != 0))
        self._run("reset", [SAMURAI, "reset", what])

    @Slot(str, "QVariant")
    def run(self, tag, cmd):
        self._run(tag, [str(c) for c in cmd])

    @Slot(str, "QVariant", str)
    def runInput(self, tag, cmd, inp):
        """For passwords: they go to the command's stdin, never on its command line."""
        self._run(tag, [str(c) for c in cmd], inp)

    @Slot(str, str, result=str)
    def icon(self, name, color):
        """A stroke icon in a colour, as a file URL (cached)."""
        color = (color or "#888888")[:7] if color.startswith("#") and len(color) >= 7 else "#888888"
        cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache") / "samurai" / "icons"
        f = cache / f"{name}-{color.lstrip('#')}.svg"
        if not f.exists():
            cache.mkdir(parents=True, exist_ok=True)
            body = ICONS.get(name, '<rect x="4" y="4" width="16" height="16" rx="4"/>')
            f.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{body}</svg>')
        return QUrl.fromLocalFile(str(f)).toString()

    @Slot(str, result=bool)
    def have(self, prog):
        return shutil.which(prog) is not None

    # ── lists for custom parts ──────────────────────────────────────────────────────────────
    @Slot(str, result="QVariant")
    def appsFor(self, kind):
        """Installed apps that can open this kind of thing, for Settings › Apps."""
        mimes = {"apps.browser": ["text/html", "x-scheme-handler/http"], "apps.files": ["inode/directory"], "apps.editor": ["text/plain"],
                 "apps.images": ["image/png", "image/jpeg"], "apps.pdf": ["application/pdf"], "apps.video": ["video/mp4", "video/x-matroska"],
                 "apps.music": ["audio/mpeg", "audio/flac"], "apps.mail": ["x-scheme-handler/mailto"]}.get(kind, [])
        out = [dict(id=e["id"], name=e["name"]) for e in desktop_entries() if any(m in e["mime"] for m in mimes)]
        cur = self._values.get(kind, "")
        if cur and not any(o["id"] == cur for o in out):
            out.insert(0, dict(id=cur, name=cur.replace(".desktop", "") + " (not installed)"))
        return out

    @Slot(result="QVariant")
    def allApps(self):
        return [dict(id=e["id"], name=e["name"]) for e in desktop_entries()]

    @Slot(result="QVariant")
    def shortcuts(self):
        code, out = sh(["samurai-wm", "--keys"])
        rows = []
        if code == 0:
            for l in out.splitlines():
                if "\t" in l:
                    k, a = l.split("\t", 1)
                    rows.append(dict(keys=k, action=a))
        return rows

    @Slot(result="QVariant")
    def wifi(self):
        code, out = sh(["nmcli", "-t", "-f", "IN-USE,SIGNAL,SECURITY,SSID", "dev", "wifi", "list", "--rescan", "auto"], timeout=15)
        rows, seen = [], set()
        if code != 0:
            return rows
        # saved profiles: the network's name → the profile (Samurai names its own "SN <ssid>")
        _, saved = sh(["nmcli", "-t", "-f", "NAME,TYPE", "connection", "show"], timeout=10)
        known = {}
        for l in saved.splitlines():
            f = re.split(r"(?<!\\):", l)
            if len(f) >= 2 and "wireless" in f[-1]:
                name = ":".join(f[:-1]).replace("\\:", ":")
                known[name[3:] if name.startswith("SN ") else name] = name
        for l in out.splitlines():
            parts = re.split(r"(?<!\\):", l, maxsplit=3)
            if len(parts) < 4:
                continue
            ssid = parts[3].replace("\\:", ":")
            if not ssid or ssid in seen:
                continue
            seen.add(ssid)
            rows.append(dict(on=parts[0] == "*", signal=int(parts[1] or 0), secure=parts[2] not in ("", "--"), ssid=ssid,
                             profile=known.get(ssid, ""), campus=bool(re.search(r"uc-secure|eduroam", ssid, re.I))))
        rows.sort(key=lambda r: (not r["on"], r["profile"] == "", -r["signal"]))
        return rows

    # ── first setup ────────────────────────────────────────────────────────────────────────
    @Slot(str, result="QVariant")
    def keySet(self, name):
        """The rows of a shortcut set, in the order the first setup shows them."""
        base = KEY_SETS.get(name, KEY_SETS["samurai"])
        return [{"action": a, "label": label, "keys": base.get(a, "")} for a, label in KEY_ACTIONS]

    @Slot("QVariant", result="QVariant")
    def clashes(self, rows):
        """Keys used for more than one action."""
        seen, twice = {}, set()
        for r in rows or []:
            n = key_norm(r.get("keys", ""))
            if not n:
                continue
            if n in seen:
                twice.add(n)
            seen[n] = True
        return [r["keys"] for r in rows if key_norm(r.get("keys", "")) in twice]

    @Slot(str, "QVariant")
    def saveKeys(self, preset, rows):
        """Keep the chosen set, plus the keys you changed in it (and switch off what they replaced)."""
        base = KEY_SETS.get(preset, KEY_SETS["samurai"])
        custom = [r for r in self._values.get("keys.custom", []) if isinstance(r, dict)]
        for r in rows or []:
            a, k = r.get("action", ""), (r.get("keys") or "").strip()
            old = base.get(a, "")
            if not k or key_norm(k) == key_norm(old):
                continue
            custom = [c for c in custom if key_norm(c.get("keys", "")) not in (key_norm(k), key_norm(old))]
            custom.append({"keys": k, "action": a})
            if old:
                custom.append({"keys": old, "action": "none"})
        self.set("keys.preset", preset)
        self.set("keys.custom", custom)

    @Slot()
    def finishSetup(self):
        try:
            d = config_dir()
            d.mkdir(parents=True, exist_ok=True)
            (d / "first-setup.done").write_text("done\n")
        except OSError:
            pass
        QTimer.singleShot(600, QGuiApplication.quit)  # let the last settings be written

    @Slot(result="QVariant")
    def bluetooth(self):
        """Bluetooth: on or off, and the paired devices (connected first)."""
        if not shutil.which("bluetoothctl"):
            return {"have": False, "on": False, "devices": []}
        _, show = sh(["bluetoothctl", "show"], timeout=5)
        _, paired = sh(["bluetoothctl", "devices", "Paired"], timeout=5)
        _, conn = sh(["bluetoothctl", "devices", "Connected"], timeout=5)
        connected = {l.split(" ", 2)[1] for l in conn.splitlines() if l.startswith("Device ") and len(l.split(" ")) > 1}
        devs = []
        for l in paired.splitlines():
            parts = l.split(" ", 2)
            if len(parts) >= 2 and parts[0] == "Device":
                devs.append({"mac": parts[1], "name": parts[2].strip() if len(parts) > 2 and parts[2].strip() else parts[1], "on": parts[1] in connected})
        devs.sort(key=lambda d: not d["on"])
        return {"have": True, "on": "Powered: yes" in show, "devices": devs}

    @Slot(result="QVariant")
    def audio(self):
        """Outputs and inputs from PipeWire (wpctl status)."""
        code, out = sh(["wpctl", "status"])
        res = dict(sinks=[], sources=[])
        if code != 0:
            return res
        section, audio = None, False
        for l in out.splitlines():
            t = l.strip("│├└─ ").strip()
            if l.startswith("Audio"):
                audio = True
                continue
            if l.startswith("Video") or l.startswith("Settings"):
                audio = False
            if not audio:
                continue
            if t.startswith("Sinks:"):
                section = "sinks"; continue
            if t.startswith("Sources:"):
                section = "sources"; continue
            if t.endswith(":"):
                section = None; continue
            m = re.match(r"(\*)?\s*(\d+)\.\s+(.*?)(?:\s+\[vol: ([\d.]+)( MUTED)?\])?$", t)
            if section and m:
                res[section].append(dict(default=bool(m.group(1)), id=m.group(2), name=m.group(3), volume=int(float(m.group(4) or 0) * 100), muted=bool(m.group(5))))
        return res

    @Slot(result="QVariant")
    def screens(self):
        """Every connected screen: where it is, its modes with their refresh rates, rotation."""
        code, out = sh(["xrandr", "--query"])
        rows = []
        if code != 0:
            return rows
        cur = None
        for l in out.splitlines():
            m = re.match(r"^(\S+) (connected|disconnected)( primary)?\s*(?:(\d+)x(\d+)\+(\d+)\+(\d+))?\s*(normal|left|right|inverted)?", l)
            if m:
                cur = None
                if m.group(2) == "connected":
                    on = m.group(4) is not None
                    cur = dict(name=m.group(1), primary=bool(m.group(3)), on=on,
                               x=int(m.group(6) or 0), y=int(m.group(7) or 0),
                               w=int(m.group(4) or 0), h=int(m.group(5) or 0),
                               rotate=m.group(8) or "normal", modes=[], mode="", rate="",
                               geometry=(f"{m.group(4)}x{m.group(5)}+{m.group(6)}+{m.group(7)}" if on else "off"))
                    rows.append(cur)
                continue
            if cur is not None and l.startswith("   "):
                f = l.split()
                rates = []
                for r in f[1:]:
                    rate = r.rstrip("*+")
                    if re.match(r"^\d+(\.\d+)?$", rate):
                        rates.append(rate)
                        if "*" in r:
                            cur["mode"], cur["rate"] = f[0], rate
                cur["modes"].append({"mode": f[0], "rates": rates})
        for r in rows:
            if not r["mode"] and r["modes"]:
                r["mode"] = r["modes"][0]["mode"]
                r["rate"] = (r["modes"][0]["rates"] or [""])[0]
            # rotated screens are taller than wide on the desktop
            if r["rotate"] in ("left", "right") and r["w"] > r["h"]:
                r["w"], r["h"] = r["h"], r["w"]
        return rows

    @Slot("QVariant", result=str)
    def arrangeRows(self, screens):
        """Turn the arrangement on the Displays page into displays.monitors rows (fixed positions)."""
        rows = []
        for sc in screens or []:
            if not sc.get("on", True):
                rows.append({"output": sc["name"], "mode": "off", "position": "", "rotate": "normal", "primary": "no"})
                continue
            mode = sc.get("mode", "") + (("@" + sc["rate"]) if sc.get("rate") else "")
            rows.append({"output": sc["name"], "mode": mode, "position": f"{int(sc.get('x', 0))}x{int(sc.get('y', 0))}",
                         "rotate": sc.get("rotate", "normal"), "primary": "yes" if sc.get("primary") else "no"})
        return json.dumps(rows)

    @Slot(str)
    def setJson(self, key_and_json):
        """set(), with the value given as JSON text (lists of rows from QML)."""
        key, _, text = key_and_json.partition("=")
        try:
            self.set(key, json.loads(text))
        except ValueError:
            self.message.emit("That could not be saved.", True)

    @Slot(result="QVariant")
    def system(self):
        info = {}
        code, out = sh([SAMURAI, "xserver"])
        if code == 0:
            try:
                info["xserver"] = json.loads(out)
            except ValueError:
                pass
        code, out = sh([SAMURAI, "version"])
        info["version"] = out if code == 0 else "not installed"
        info["greeter"] = Path("/etc/samurai/greeter.json").exists()
        return info


def main():
    page = None
    if "--page" in sys.argv:
        i = sys.argv.index("--page")
        page = sys.argv[i + 1] if i + 1 < len(sys.argv) else None
    os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
    app = QGuiApplication(sys.argv)
    app.setApplicationName("Samurai Settings")
    app.setDesktopFileName("samurai-settings")
    app.setWindowIcon(QIcon.fromTheme("preferences-system"))
    core = Core(page)
    eng = QQmlApplicationEngine()
    eng.rootContext().setContextProperty("core", core)
    first = "--first-setup" in sys.argv
    eng.load(QUrl.fromLocalFile(str(HERE / "qml" / ("FirstSetup.qml" if first else "Main.qml"))))
    if not eng.rootObjects():
        sys.exit(1)
    code = app.exec()
    del eng  # the pages go before the data they read
    sys.exit(code)


if __name__ == "__main__":
    main()
