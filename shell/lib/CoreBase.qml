// The state every surface reads: settings, theme, status, windows, which panel is open.
// Pure QtQuick, so the same file runs in Quickshell (through Core.qml, which feeds it) and in the
// offscreen render check (tests/qml), which feeds it canned data.
import QtQuick

Item {
    id: core
    visible: false

    // ── fed by Core.qml ────────────────────────────────────────────────────────────────────
    property var schema: ({ groups: [], pages: [] })
    property var values: ({})          // every setting, defaults included
    property var status: ({})          // samurai status
    property var windows: []           // [{address, cls, title, workspace, focused, minimized}]
    property int activeWorkspace: 1
    property var notices: []           // [{id, app, summary, body}]
    property var apps: []              // [{id, name, exec, kanji}]
    property date now: new Date()

    // ── what the user is doing ──────────────────────────────────────────────────────────────
    property string panel: ""          // "", start, quick, widgets, power, keys
    property bool hubOpen: false
    property string hubPage: "displays"
    property bool locked: false
    function toggle(name) { panel = (panel === name ? "" : name) }
    function openHub(page) { if (page) hubPage = page; hubOpen = true; panel = "" }

    // ── settings ────────────────────────────────────────────────────────────────────────────
    function v(key) { return values[key] }
    // overridden by Core.qml: write a setting / run a command
    property var setValue: function(key, val) { var c = Object.assign({}, values); c[key] = val; values = c }
    property var run: function(cmd, done, input) { console.log("run", cmd) }
    property var launch: function(appId) { console.log("launch", appId) }
    property var power: function(what) { console.log("power", what) }
    property var dismiss: function(id) { notices = notices.filter(function(n) { return n.id !== id }) }
    property var binds: []             // samurai binds list

    // ── theme: two grounds, one red ────────────────────────────────────────────────────────
    readonly property string base: status.base || (v("look.base") === "white" ? "white" : "black")
    readonly property bool white: base === "white"
    readonly property color ground: white ? "#FFFFFF" : "#000000"
    readonly property color ink: white ? "#000000" : "#FFFFFF"
    readonly property color dim: white ? "#5A5A5A" : "#A3A3A3"
    readonly property color red: status.accent || "#C8102E"
    readonly property color redInk: "#FFFFFF"
    readonly property real scale: v("look.scale") || 1
    readonly property int px: Math.round((v("look.font_px") || 13) * scale)
    readonly property int small: Math.round(px * 0.85)
    readonly property string font: v("look.font") || "IBM Plex Mono"
    readonly property string fontJp: v("look.font_jp") || "Noto Sans CJK JP"
    readonly property int line: v("frame.line") || 1
    readonly property int barH: Math.round((v("frame.bar_height") || 44) * scale)
    readonly property int railW: Math.round((v("frame.rail_width") || 48) * scale)
    readonly property int inset: v("frame.inset") === undefined ? 12 : v("frame.inset")
    readonly property int hit: Math.round(44 * scale)   // smallest thing you can click

    // ── incognito ───────────────────────────────────────────────────────────────────────────
    readonly property bool incognito: status.incognito === true
    readonly property bool chaos: incognito && v("incognito.chaos") !== false
    // a fixed tilt per slot: chaotic to look at, the same every time, so nothing jumps around
    function tilt(slot) {
        if (!chaos) return 0
        var k = v("incognito.chaos_strength") === undefined ? 6 : v("incognito.chaos_strength")
        var x = Math.sin(slot * 12.9898) * 43758.5453
        return ((x - Math.floor(x)) * 2 - 1) * k
    }
    readonly property string userName: incognito && v("incognito.hide_user") !== false ? "someone" : (status.user || "")
    readonly property string hostName: incognito && v("incognito.hide_user") !== false ? "somewhere" : (status.host || "")
    readonly property string mark: {
        var m = status.seal && status.seal.mark ? status.seal.mark : ""
        return incognito && v("incognito.hide_seal") !== false ? "████·████·████·████" : m
    }

    // ── language ────────────────────────────────────────────────────────────────────────────
    function label(jp, en) {
        var m = v("region.labels")
        return m === "japanese" ? jp : m === "english" ? en : jp + " " + en
    }
    readonly property var kanjiDigits: ["零", "壱", "弐", "参", "肆", "伍", "陸", "漆", "捌", "玖", "拾"]
    function kanjiNumber(n) {
        var d = ["", "一", "二", "三", "四", "五", "六", "七", "八", "九"]
        if (n <= 0) return "〇"
        if (n < 10) return d[n]
        var tens = Math.floor(n / 10), ones = n % 10
        return (tens > 1 ? d[tens] : "") + "十" + d[ones]
    }
    readonly property var weekJp: ["日", "月", "火", "水", "木", "金", "土"]
    function pad(n) { return n < 10 ? "0" + n : "" + n }
    readonly property string clockText: {
        var h = now.getHours(), m = now.getMinutes()
        var sec = v("region.seconds") ? ":" + pad(now.getSeconds()) : ""
        if (v("region.clock24") === false) return ((h + 11) % 12 + 1) + ":" + pad(m) + sec + (h < 12 ? " AM" : " PM")
        return pad(h) + ":" + pad(m) + sec
    }
    readonly property string dateText: {
        var mode = v("region.date") || "kanji"
        if (mode === "off") return ""
        if (mode === "iso") return now.getFullYear() + "-" + pad(now.getMonth() + 1) + "-" + pad(now.getDate())
        if (mode === "long") return now.toLocaleDateString(Qt.locale("en_US"), "ddd d MMM")
        return kanjiNumber(now.getMonth() + 1) + "月" + kanjiNumber(now.getDate()) + "日 " + weekJp[now.getDay()]
    }

    // ── apps as single kanji ───────────────────────────────────────────────────────────────
    readonly property var kanjiTable: [
        ["端", ["term", "kitty", "foot", "alacritty", "ghostty", "wezterm", "konsole"]],
        ["蔵", ["files", "nautilus", "thunar", "dolphin", "yazi", "nemo", "pcmanfm"]],
        ["網", ["firefox", "chrom", "zen", "brave", "browser", "librewolf", "vivaldi"]],
        ["設", ["samurai-hub", "settings"]],
        ["書", ["code", "nvim", "vim", "editor", "zed", "kate", "gedit", "emacs", "writer", "libreoffice"]],
        ["話", ["discord", "vesktop", "telegram", "signal", "element", "slack", "teams", "zoom"]],
        ["信", ["mail", "thunderbird", "geary", "evolution"]],
        ["音", ["spotify", "music", "mpv", "vlc", "rhythmbox", "audacious"]],
        ["遊", ["steam", "lutris", "heroic", "game"]],
        ["画", ["gimp", "krita", "inkscape", "image", "eog", "loupe", "photo"]],
        ["読", ["zathura", "okular", "evince", "pdf", "calibre", "foliate"]],
        ["線", ["network", "nm-connection", "iwgtk"]],
        ["務", ["task", "monitor", "htop", "btop"]],
        ["算", ["calc"]]
    ]
    function appKanji(name) {
        var n = (name || "").toLowerCase()
        for (var i = 0; i < kanjiTable.length; i++)
            for (var j = 0; j < kanjiTable[i][1].length; j++)
                if (n.indexOf(kanjiTable[i][1][j]) >= 0) return kanjiTable[i][0]
        return "具"
    }
    readonly property var focusedWindow: {
        for (var i = 0; i < windows.length; i++) if (windows[i].focused) return windows[i]
        return null
    }
    readonly property string focusedTitle: {
        if (!focusedWindow) return ""
        return incognito && v("incognito.hide_titles") !== false ? "window" : (focusedWindow.title || "")
    }

    Timer { interval: 1000; running: true; repeat: true; onTriggered: core.now = new Date() }
}
