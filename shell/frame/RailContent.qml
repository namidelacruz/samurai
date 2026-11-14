// The rail: pinned apps as single kanji; running, focused and minimized apps marked on the
// same cells. Minimized apps live here (a dimmed kanji with a hollow square), not on the desktop.
import QtQuick
import "../lib"

Rectangle {
    id: rail
    required property var core
    color: core.ground
    width: core.railW

    Rectangle { width: core.line; height: parent.height; color: core.ink; anchors.right: core.v("frame.rail_side") === "right" ? undefined : parent.right }

    // pinned first, then whatever else is open
    readonly property var entries: {
        var out = [], seen = {}
        var pins = core.v("frame.rail.pinned") || []
        function wins(match) { return core.windows.filter(function(w) { return (w.cls || "").toLowerCase().indexOf(match) >= 0 }) }
        for (var i = 0; i < pins.length; i++) {
            var id = (pins[i].app || "").replace(/\.desktop$/, "")
            if (!id) continue
            var key = id.toLowerCase().split(".").pop()
            seen[key] = true
            out.push({ key: key, app: id, wins: wins(key), pinned: true })
        }
        if (core.v("frame.rail.running") !== false || core.v("frame.rail.minimized") !== false) {
            for (var j = 0; j < core.windows.length; j++) {
                var c = (core.windows[j].cls || "").toLowerCase()
                if (!c || seen[c]) continue
                var hit = false
                for (var k in seen) if (c.indexOf(k) >= 0) hit = true
                if (hit) continue
                seen[c] = true
                out.push({ key: c, app: c, wins: wins(c), pinned: false })
            }
        }
        return out
    }

    Column {
        width: parent.width
        Repeater {
            model: rail.entries
            Cell {
                required property var modelData
                required property int index
                core: rail.core
                readonly property var live: modelData.wins.filter(function(w) { return !w.minimized })
                readonly property bool focused: modelData.wins.some(function(w) { return w.focused })
                readonly property bool allMin: modelData.wins.length > 0 && live.length === 0
                width: rail.width; height: rail.width; pad: 0; lineBottom: true
                kanji: core.appKanji(modelData.app); kanjiSize: core.px + 5
                filled: focused; faint: allMin
                rotation: core.tilt(40 + index)
                // running: a red square; minimized: a hollow one
                Rectangle { visible: live.length > 0 && !focused; width: 6; height: 6; color: core.red }
                Rectangle { visible: allMin; x: 3; y: 3; width: 7; height: 7; color: "transparent"; border.width: 1; border.color: core.ink }
                onClicked: {
                    var ws = modelData.wins
                    if (ws.length === 0) core.launch((modelData.app || "").replace(/\.desktop$/, ""))
                    else if (allMin) core.run(["samurai", "win", "restore", ws[ws.length - 1].address])
                    else if (focused && ws.length === 1) core.run(["samurai", "win", "minimize"])
                    else {
                        var i = ws.findIndex(function(w) { return w.focused })
                        core.run(["niri", "msg", "action", "focus-window", "--id", ws[(i + 1) % ws.length].address])
                    }
                }
                onRightClicked: core.launch((modelData.app || "").replace(/\.desktop$/, ""))
            }
        }
    }
    Column {
        anchors.bottom: parent.bottom
        width: parent.width
        Item {
            width: parent.width; height: mark.implicitWidth + 24
            Txt { id: mark; core: rail.core; anchors.centerIn: parent; rotation: 90; text: "一期一会"; jp: true; faint: true; font.pixelSize: core.small; font.letterSpacing: 4 }
        }
        Cell { core: rail.core; width: rail.width; height: rail.width; pad: 0; lineTop: true; kanji: "設"; kanjiSize: core.px + 5; filled: core.hubOpen; onClicked: core.hubOpen ? core.hubOpen = false : core.openHub("") }
        Cell { core: rail.core; width: rail.width; height: rail.width; pad: 0; lineTop: true; kanji: "電"; kanjiSize: core.px + 5; filled: core.panel === "power"; onClicked: core.toggle("power") }
    }
}
