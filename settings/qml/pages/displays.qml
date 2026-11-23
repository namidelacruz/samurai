// Settings › Displays: arrange the screens by dragging them, pick each one's resolution, refresh
// rate, rotation and the main screen, then apply. A changed setup reverts by itself after 15
// seconds unless you keep it, so a screen that goes dark never leaves you stuck.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    id: page
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var screens: core.screens()
    property var work: JSON.parse(JSON.stringify(screens))   // what you are arranging
    property int pick: 0
    property int countdown: 0
    property string before: ""                                 // displays.monitors as it was
    spacing: 14 * s

    function refresh() { screens = core.screens(); work = JSON.parse(JSON.stringify(screens)); pick = Math.min(pick, Math.max(0, work.length - 1)) }
    function setField(i, k, v) { var w = work.slice(); w[i] = Object.assign({}, w[i]); w[i][k] = v; if (k === "primary" && v) for (var j = 0; j < w.length; j++) if (j !== i) { w[j] = Object.assign({}, w[j]); w[j].primary = false } work = w }
    function modeOf(i) { var sc = work[i]; for (var k = 0; k < sc.modes.length; k++) if (sc.modes[k].mode === sc.mode) return sc.modes[k]; return { mode: sc.mode, rates: [] } }
    function changed() { return JSON.stringify(work) !== JSON.stringify(screens) }
    function apply() {
        before = JSON.stringify(core.values["displays.monitors"] || [])
        // sizes follow the chosen mode (and rotation) before saving positions
        core.setJson("displays.monitors=" + core.arrangeRows(work))
        core.run("screens", ["samurai", "apply"])
        countdown = 15
    }
    function revert() {
        countdown = 0
        core.setJson("displays.monitors=" + before)
        core.run("screens", ["samurai", "apply"])
        revertLater.start()
    }
    Timer { id: revertLater; interval: 1500; onTriggered: page.refresh() }
    Timer { running: page.countdown > 0; interval: 1000; repeat: true; onTriggered: { page.countdown--; if (page.countdown === 0) page.revert() } }
    Connections { target: core; function onDone(tag, code, text) { if (tag === "screens") { if (code !== 0) win.say(text || "The screens could not be set.", true); arrangeLater.start() } } }
    Timer { id: arrangeLater; interval: 1200; onTriggered: if (page.countdown === 0) page.refresh() }

    // ── keep or go back ──────────────────────────────────────────────────────────────────
    Rectangle {
        visible: page.countdown > 0
        Layout.fillWidth: true; implicitHeight: 52 * s; radius: 10 * s
        color: c.soft; border.color: c.accent
        RowLayout {
            anchors.fill: parent; anchors.margins: 10 * s; spacing: 10 * s
            Text { Layout.fillWidth: true; text: "Keep these display settings? Going back in " + page.countdown + " s."; color: c.text; font.pixelSize: 13.5 * s; font.weight: Font.DemiBold }
            Btn { text: "Go back"; onClicked: page.revert() }
            Btn { text: "Keep"; primary: true; onClicked: { page.countdown = 0; page.refresh() } }
        }
    }

    // ── the arrangement ──────────────────────────────────────────────────────────────────
    Card {
        title: "Arrangement: drag a screen to move it"
        Item {
            id: board
            width: parent.width; height: 230 * s
            readonly property real minX: Math.min.apply(null, page.work.map(function (w) { return w.x }).concat([0]))
            readonly property real minY: Math.min.apply(null, page.work.map(function (w) { return w.y }).concat([0]))
            readonly property real spanW: Math.max.apply(null, page.work.map(function (w) { return w.x + w.w }).concat([1])) - minX
            readonly property real spanH: Math.max.apply(null, page.work.map(function (w) { return w.y + w.h }).concat([1])) - minY
            readonly property real k: Math.min((width - 60 * s) / Math.max(spanW * 1.6, 1), (height - 40 * s) / Math.max(spanH * 1.2, 1))
            readonly property real ox: (width - spanW * k) / 2
            readonly property real oy: (height - spanH * k) / 2
            Repeater {
                model: page.work
                Rectangle {
                    id: scr
                    required property var modelData
                    required property int index
                    visible: modelData.on
                    x: board.ox + (modelData.x - board.minX) * board.k
                    y: board.oy + (modelData.y - board.minY) * board.k
                    width: Math.max(30 * s, modelData.w * board.k); height: Math.max(20 * s, modelData.h * board.k)
                    radius: 8 * s
                    color: index === page.pick ? c.soft : c.chrome
                    border.color: index === page.pick ? c.accent : c.line; border.width: index === page.pick ? 2 : 1
                    Column {
                        anchors.centerIn: parent
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: (index + 1) + (modelData.primary ? " ★" : ""); color: c.text; font.pixelSize: 18 * s; font.weight: Font.DemiBold }
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.name; color: c.dim; font.pixelSize: 11 * s }
                    }
                    MouseArea {
                        anchors.fill: parent
                        drag.target: scr; drag.threshold: 2
                        onPressed: page.pick = index
                        onReleased: {
                            // back to screen pixels, then snap edges to a neighbour's edges
                            var nx = Math.round((scr.x - board.ox) / board.k + board.minX)
                            var ny = Math.round((scr.y - board.oy) / board.k + board.minY)
                            var me = page.work[index], best = 40 / board.k
                            for (var j = 0; j < page.work.length; j++) {
                                if (j === index || !page.work[j].on) continue
                                var o = page.work[j]
                                var xs = [o.x + o.w, o.x - me.w, o.x], ys = [o.y, o.y + o.h - me.h, o.y + o.h, o.y - me.h]
                                for (var a = 0; a < xs.length; a++) if (Math.abs(nx - xs[a]) < best) nx = xs[a]
                                for (var b = 0; b < ys.length; b++) if (Math.abs(ny - ys[b]) < best) ny = ys[b]
                            }
                            page.setField(index, "x", nx); page.setField(index, "y", ny)
                        }
                    }
                }
            }
            Text { visible: page.work.length === 0; anchors.centerIn: parent; text: "No screens found (is this an X session?)"; color: c.dim }
        }
    }

    // ── the picked screen ────────────────────────────────────────────────────────────────
    Card {
        visible: page.work.length > 0
        title: page.work.length ? "Screen " + (page.pick + 1) + " · " + page.work[page.pick].name : ""
        GridLayout {
            x: 16 * s; width: parent.width - 32 * s
            columns: 2; rowSpacing: 10 * s; columnSpacing: 16 * s
            Item { Layout.columnSpan: 2; Layout.preferredHeight: 2 * s }
            Text { text: "On"; color: c.text; font.pixelSize: 13 * s }
            Toggle {
                on: page.work.length ? page.work[page.pick].on : false
                onFlipped: (to) => page.setField(page.pick, "on", to)
                Accessible.name: "Screen on"
            }
            Text { text: "Resolution"; color: c.text; font.pixelSize: 13 * s }
            Choice {
                id: modeBox
                Layout.preferredWidth: 220 * s
                model: page.work.length ? page.work[page.pick].modes.map(function (m) { return m.mode }) : []
                currentIndex: page.work.length ? Math.max(0, model.indexOf(page.work[page.pick].mode)) : -1
                onActivated: (i) => {
                    var m = page.work[page.pick].modes[i]
                    page.setField(page.pick, "mode", m.mode); page.setField(page.pick, "rate", m.rates[0] || "")
                    var wh = m.mode.split("x"), rot = page.work[page.pick].rotate === "left" || page.work[page.pick].rotate === "right"
                    page.setField(page.pick, "w", parseInt(rot ? wh[1] : wh[0])); page.setField(page.pick, "h", parseInt(rot ? wh[0] : wh[1]))
                }
                Accessible.name: "Resolution"
            }
            Text { text: "Refresh rate"; color: c.text; font.pixelSize: 13 * s }
            Choice {
                Layout.preferredWidth: 220 * s
                model: page.work.length ? page.modeOf(page.pick).rates.map(function (r) { return Math.round(parseFloat(r)) + " Hz  (" + r + ")" }) : []
                currentIndex: page.work.length ? Math.max(0, page.modeOf(page.pick).rates.indexOf(page.work[page.pick].rate)) : -1
                onActivated: (i) => page.setField(page.pick, "rate", page.modeOf(page.pick).rates[i])
                Accessible.name: "Refresh rate"
            }
            Text { text: "Rotation"; color: c.text; font.pixelSize: 13 * s }
            Row {
                spacing: 6 * s
                Repeater {
                    model: [["normal", "Normal"], ["left", "Left"], ["right", "Right"], ["inverted", "Upside down"]]
                    Btn {
                        required property var modelData
                        text: modelData[1]
                        primary: page.work.length && page.work[page.pick].rotate === modelData[0]
                        onClicked: {
                            var sc = page.work[page.pick], was = sc.rotate === "left" || sc.rotate === "right", now = modelData[0] === "left" || modelData[0] === "right"
                            page.setField(page.pick, "rotate", modelData[0])
                            if (was !== now) { var w = sc.w; page.setField(page.pick, "w", sc.h); page.setField(page.pick, "h", w) }
                        }
                    }
                }
            }
            Text { text: "Main screen"; color: c.text; font.pixelSize: 13 * s }
            Toggle {
                on: page.work.length ? page.work[page.pick].primary : false
                onFlipped: (to) => page.setField(page.pick, "primary", to)
                Accessible.name: "Main screen"
            }
            Item { Layout.columnSpan: 2; Layout.preferredHeight: 4 * s }
        }
    }
    RowLayout {
        spacing: 10 * s
        Btn { text: "Apply"; primary: true; enabled: page.changed() && page.countdown === 0; onClicked: page.apply() }
        Btn { text: "Undo changes"; enabled: page.changed() && page.countdown === 0; onClicked: page.refresh() }
        Btn { text: "Look again"; iconName: "repair"; onClicked: page.refresh() }
        Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 12 * s
               text: "The size of everything (Settings › Look) is one for all screens: X11 has no size per screen." }
    }
}
