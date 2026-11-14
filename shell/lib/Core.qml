// The live Core: feeds CoreBase from the samurai command, Hyprland and the notification bus.
// This is the only file besides shell.qml and the window wrappers that needs Quickshell.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import Quickshell.Services.Notifications

CoreBase {
    id: core
    readonly property string home: Quickshell.env("HOME")
    readonly property string configDir: (Quickshell.env("XDG_CONFIG_HOME") || home + "/.config") + "/samurai"
    readonly property string stateDir: (Quickshell.env("XDG_STATE_HOME") || home + "/.local/state") + "/samurai"
    // empty for a moment after a new seal is forged, so every image lets go of the old file and reloads
    readonly property string sealDir: sealBlink.running ? "" : configDir + "/seal"
    Timer { id: sealBlink; interval: 80 }

    // ── run a command; `done(text, code)` is optional ──────────────────────────────────────
    Component {
        id: job
        Process {
            id: p
            property var done: null
            property string input: ""
            stdinEnabled: input !== ""
            onStarted: if (input !== "") { write(input + "\n"); input = "" }
            stdout: StdioCollector { id: so }
            stderr: StdioCollector { id: se }
            onExited: function(code) { if (p.done) p.done(so.text + se.text, code); p.destroy() }
        }
    }
    // `input` goes to the command's stdin and nowhere else (passwords never ride on argv)
    run: function(cmd, done, input) { var p = job.createObject(core, { command: cmd, done: done || null, input: input || "" }); p.running = true }
    launch: function(appId) {
        var e = DesktopEntries.byId(appId) || DesktopEntries.heuristicLookup(appId)
        if (e) e.execute(); else Quickshell.execDetached([appId])
    }
    setValue: function(key, val) {
        var c = Object.assign({}, values); c[key] = val; values = c   // show it at once
        run(["samurai", "set", key, JSON.stringify(val)], function(text, code) { if (code !== 0) { core.lastError = text.trim() || (key + " was not saved"); loadValues() } })
    }
    property string lastError: ""
    power: function(what) {
        if (what === "lock") { locked = true; return }
        if (what === "sleep") { if (v("idle.lock_before_sleep") !== false) locked = true; run(["systemctl", "suspend"]); return }
        if (what === "logout") { run(["hyprctl", "dispatch", "hl.dsp.exit()"]); return }
        if (what === "restart") { run(["systemctl", "reboot"]); return }
        if (what === "shutdown") { run(["systemctl", "poweroff"]); return }
    }

    // ── settings, status, binds ────────────────────────────────────────────────────────────
    function parse(text, fallback) { try { return JSON.parse(text) } catch (e) { return fallback } }
    function loadValues() { run(["samurai", "dump"], function(t) { var o = parse(t, null); if (o) values = o }) }
    function loadStatus() { run(["samurai", "status"], function(t) { var o = parse(t, null); if (o) { status = o } }) }
    function loadBinds() { run(["samurai", "binds", "list"], function(t) { binds = parse(t, []) }) }
    Component.onCompleted: {
        run(["samurai", "schema"], function(t) { schema = parse(t, schema) })
        loadValues(); loadStatus(); loadBinds(); loadWindows()
    }
    FileView { path: core.configDir + "/settings.json"; watchChanges: true; printErrors: false; onFileChanged: { core.loadValues(); core.loadBinds(); core.loadStatus() } }
    FileView { path: core.stateDir + "/state.json"; watchChanges: true; printErrors: false; onFileChanged: core.loadStatus() }
    FileView { path: core.configDir + "/seal/seal.json"; watchChanges: true; printErrors: false; onFileChanged: { sealBlink.restart(); core.loadStatus() } }
    Timer { interval: 5000; running: true; repeat: true; onTriggered: core.loadStatus() }

    // ── night light (hyprsunset) and the background update check ─────────────────────────
    property string nightNow: ""
    function mins(t) { var m = /^(\d{1,2}):(\d{2})$/.exec(t || ""); return m ? Number(m[1]) * 60 + Number(m[2]) : -1 }
    function nightTick() {
        var want = "off"
        if (v("displays.night.enabled") === true) {
            var a = mins(v("displays.night.from")), b = mins(v("displays.night.to")), n = now.getHours() * 60 + now.getMinutes()
            var inside = a < 0 || b < 0 ? true : a <= b ? (n >= a && n < b) : (n >= a || n < b)
            if (inside) want = "" + (v("displays.night.kelvin") || 4000)
        }
        if (want === nightNow) return
        nightNow = want
        run(["sh", "-c", want === "off" ? "pkill -x hyprsunset; true" : "pkill -x hyprsunset; command -v hyprsunset >/dev/null && setsid -f hyprsunset -t " + Number(want) + " >/dev/null 2>&1; true"])
    }
    Timer { interval: 30000; running: true; repeat: true; triggeredOnStart: true; onTriggered: core.nightTick() }
    onValuesChanged: nightTick()
    property double lastUpdateCheck: 0
    Timer {
        interval: 600000; running: true; repeat: true
        onTriggered: {
            if (core.v("updates.auto") === "off" || core.incognito) return
            var every = ({ hourly: 1, daily: 24, weekly: 168 })[core.v("updates.every")] || 24
            if (Date.now() - core.lastUpdateCheck < every * 3600000) return
            core.lastUpdateCheck = Date.now()
            core.run(["samurai", "update", "check"], function() { core.loadStatus() })
        }
    }

    // ── windows ────────────────────────────────────────────────────────────────────────────
    activeWorkspace: Hyprland.focusedWorkspace ? Hyprland.focusedWorkspace.id : 1
    function loadWindows() {
        run(["hyprctl", "-j", "clients"], function(t) {
            var list = parse(t, []), out = []
            for (var i = 0; i < list.length; i++) {
                var c = list[i]
                if (!c.mapped && c.mapped !== undefined) continue
                var ws = c.workspace || {}
                out.push({ address: c.address, cls: c["class"] || c.initialClass || "", title: c.title || "",
                           workspace: ws.id, focused: c.focusHistoryID === 0 && String(ws.name || "").indexOf("special:min") !== 0,
                           minimized: String(ws.name || "").indexOf("special:min") === 0 })
            }
            windows = out
        })
    }
    Timer { id: settle; interval: 60; onTriggered: core.loadWindows() }
    Connections { target: Hyprland; function onRawEvent(ev) { settle.restart() } }

    // ── apps ───────────────────────────────────────────────────────────────────────────────
    apps: {
        var out = [], list = DesktopEntries.applications.values
        for (var i = 0; i < list.length; i++) if (!list[i].noDisplay) out.push({ id: list[i].id, name: list[i].name })
        out.sort(function(a, b) { return a.name.localeCompare(b.name) })
        return out
    }

    // ── notices ────────────────────────────────────────────────────────────────────────────
    property var popups: []            // the ones still showing as a slip
    NotificationServer {
        id: server
        bodySupported: true; actionsSupported: false; imageSupported: false; keepOnReload: true
        onNotification: function(n) {
            n.tracked = true
            if (core.v("frame.notices.dnd") === true || core.locked) return
            var id = n.id
            core.popups = core.popups.concat([id])
            var t = Qt.createQmlObject('import QtQuick; Timer { running: true }', core)
            t.interval = (core.v("frame.notices.seconds") || 6) * 1000
            t.triggered.connect(function() { core.popups = core.popups.filter(function(x) { return x !== id }); t.destroy() })
        }
    }
    notices: {
        var out = [], list = server.trackedNotifications.values
        for (var i = list.length - 1; i >= 0; i--) out.push({ id: list[i].id, app: list[i].appName, summary: list[i].summary, body: list[i].body })
        return out
    }
    dismiss: function(id) {
        var list = server.trackedNotifications.values
        for (var i = 0; i < list.length; i++) if (list[i].id === id) { list[i].dismiss(); return }
    }

    // ── samurai shell <verb> lands here ────────────────────────────────────────────────────
    IpcHandler {
        target: "frame"
        function start(): void { core.toggle("start") }
        function quick(): void { core.toggle("quick") }
        function widgets(): void { core.toggle("widgets") }
        function keys(): void { core.toggle("keys") }
        function power(): void { core.toggle("power") }
        function settings(page: string): void { core.openHub(page) }
        function base(): void { core.setValue("look.base", core.white ? "black" : "white") }
        function incognito(): void { core.run(["samurai", "incognito", "toggle"]) }
        function lock(): void { core.locked = true }
        function close(): void { core.panel = ""; core.hubOpen = false }
        function reload(): void { core.loadValues(); core.loadStatus(); core.loadBinds() }
    }
}
