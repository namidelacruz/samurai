// Start: grows out of the 影 cell, sharing the bar's line and the rail's line. Search (apps,
// settings, = calculator, > command), pinned apps, power row.
import QtQuick
import "../lib"

Rectangle {
    id: start
    required property var core
    color: core.ground
    rotation: core.tilt(5) * 0.5
    property int sel: 0
    function reset() { input.text = ""; sel = 0; input.forceActiveFocus() }

    Rectangle { anchors.right: parent.right; width: core.line; height: parent.height; color: core.ink }
    Rectangle { anchors.bottom: parent.bottom; height: core.line; width: parent.width; color: core.ink }

    readonly property string q: input.text.trim()
    readonly property var results: {
        var out = [], ql = q.toLowerCase()
        if (!ql) return out
        if (q[0] === "=" && core.v("start.search.calc") !== false) {
            var expr = q.slice(1)
            if (/^[0-9+\-*/().,%\s^]+$/.test(expr)) {
                try { var r = Function("return (" + expr.replace(/\^/g, "**").replace(/,/g, ".") + ")")(); if (isFinite(r)) out.push({ kanji: "算", name: "" + r, detail: "copy", kind: "calc", cmd: ["sh", "-c", "printf %s '" + r + "' | wl-copy"] }) } catch (e) {}
            }
            return out
        }
        if (q[0] === ">" && core.v("start.search.commands") !== false) {
            if (q.length > 1) out.push({ kanji: "端", name: q.slice(1).trim(), detail: "run in a terminal", kind: "cmd", cmd: ["samurai", "term", "sh", "-c", q.slice(1).trim()] })
            return out
        }
        if (core.v("start.search.apps") !== false) {
            for (var i = 0; i < core.apps.length && out.length < 6; i++) {
                var a = core.apps[i]
                if ((a.name || "").toLowerCase().indexOf(ql) >= 0) out.push({ kanji: core.appKanji(a.id + " " + a.name), name: a.name, detail: "app", kind: "app", app: a.id })
            }
        }
        if (core.v("start.search.settings") !== false) {
            var pages = core.schema.pages || []
            for (var p = 0; p < pages.length && out.length < 9; p++) {
                var secs = pages[p].sections
                for (var s = 0; s < secs.length && out.length < 9; s++)
                    for (var k = 0; k < secs[s].items.length && out.length < 9; k++) {
                        var it = secs[s].items[k]
                        if (it.label.toLowerCase().indexOf(ql) >= 0) out.push({ kanji: "設", name: "Settings › " + pages[p].en + " › " + it.label, detail: "setting", kind: "page", page: pages[p].id })
                    }
            }
        }
        return out
    }
    function go(r) {
        if (!r) return
        if (r.kind === "page") core.openHub(r.page); else if (r.kind === "app") core.launch(r.app); else core.run(r.cmd)
        core.panel = ""
    }

    Column {
        anchors.fill: parent
        Row {
            leftPadding: 20; topPadding: 16; bottomPadding: 10; spacing: 12
            Txt { core: start.core; text: "起動"; jp: true; heavy: true; font.pixelSize: 44 }
            Txt { core: start.core; text: "START"; heavy: true; font.letterSpacing: 2; anchors.baseline: parent.children[0].baseline }
        }
        Rectangle {
            x: 20; width: parent.width - 40; height: core.hit; color: "transparent"; border.width: input.activeFocus ? 2 : core.line; border.color: core.ink
            Txt { id: mag; core: start.core; x: 12; height: parent.height; text: "探"; jp: true; heavy: true }
            TextInput {
                id: input
                anchors.left: mag.right; anchors.leftMargin: 10; anchors.right: hint.left; anchors.rightMargin: 8
                height: parent.height; verticalAlignment: TextInput.AlignVCenter; clip: true
                color: core.ink; selectionColor: core.red; selectedTextColor: core.redInk
                font.family: core.font; font.pixelSize: core.px
                onTextChanged: start.sel = 0
                Keys.onDownPressed: start.sel = Math.min(start.results.length - 1, start.sel + 1)
                Keys.onUpPressed: start.sel = Math.max(0, start.sel - 1)
                Keys.onReturnPressed: start.go(start.results[start.sel])
                Keys.onEscapePressed: core.panel = ""
            }
            Txt { id: hint; core: start.core; anchors.right: parent.right; anchors.rightMargin: 12; height: parent.height; faint: true; font.pixelSize: core.small; text: "APPS · SETTINGS · = CALC · > RUN" }
        }
        Item { width: 1; height: 14 }
        Rectangle { width: parent.width; height: core.line; color: core.ink; visible: start.results.length > 0 }
        Repeater {
            model: start.results
            Cell {
                required property var modelData
                required property int index
                core: start.core
                width: start.width - core.line; height: core.hit + 4; pad: 0; lineBottom: true
                filled: index === start.sel
                Txt { core: start.core; width: core.hit + 4; height: parent.height; horizontalAlignment: Text.AlignHCenter; text: modelData.kanji; jp: true; heavy: true; font.pixelSize: core.px + 6; color: parent.fg }
                Txt { core: start.core; x: core.hit + 4; width: parent.width - x - 90; height: parent.height; text: modelData.name; heavy: true; color: parent.fg }
                Txt { core: start.core; anchors.right: parent.right; anchors.rightMargin: 16; height: parent.height; text: modelData.detail; color: parent.fg; opacity: 0.7 }
                onClicked: start.go(modelData)
            }
        }
        Txt { core: start.core; visible: start.q !== "" && start.results.length === 0; leftPadding: 20; topPadding: 8; faint: true; text: "nothing matches · 該当なし" }
        Txt { core: start.core; visible: start.q === ""; leftPadding: 20; topPadding: 4; bottomPadding: 8; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; text: (core.v("frame.rail.pinned") || []).length ? core.label("固定", "PINNED") : core.label("全", "APPS") }
        Rectangle { width: parent.width; height: core.line; color: core.ink; visible: start.q === "" }
        Grid {
            id: grid
            visible: start.q === ""
            columns: core.v("start.columns") || 5
            Repeater {
                model: (core.v("frame.rail.pinned") || []).length ? core.v("frame.rail.pinned") : core.apps.slice(0, grid.columns * 4).map(function(a) { return { app: a.id, name: a.name } })
                Cell {
                    required property var modelData
                    required property int index
                    core: start.core
                    readonly property string app: (modelData.app || "").replace(/\.desktop$/, "")
                    width: Math.floor((start.width - core.line) / grid.columns); height: 96; pad: 0
                    lineLeft: index % grid.columns !== 0; lineBottom: true
                    Column {
                        anchors.centerIn: parent; spacing: 4
                        Txt { core: start.core; anchors.horizontalCenter: parent.horizontalCenter; text: core.appKanji(app); jp: true; heavy: true; font.pixelSize: 32 }
                        Txt { core: start.core; anchors.horizontalCenter: parent.horizontalCenter; text: (modelData.name || app.split(".").pop()).toUpperCase(); font.pixelSize: core.small; elide: Text.ElideRight; width: Math.min(implicitWidth, grid.width / grid.columns - 8) }
                    }
                    onClicked: { core.launch(app); core.panel = "" }
                }
            }
        }
    }
    Rectangle { visible: core.v("start.power_row") !== false; anchors.bottom: parent.bottom; anchors.bottomMargin: core.hit + 12 + core.line; height: core.line; width: parent.width; color: core.ink }
    // power row
    Row {
        visible: core.v("start.power_row") !== false
        anchors.bottom: parent.bottom; anchors.bottomMargin: core.line
        width: parent.width - core.line; height: core.hit + 12
        Txt { core: start.core; leftPadding: 20; height: parent.height; width: parent.width - 5 * 72; elide: Text.ElideRight; text: core.userName + " @ " + core.hostName }
        Repeater {
            model: [["施錠", "lock"], ["休止", "sleep"], ["退出", "logout"], ["再起", "restart"], ["電源", "shutdown"]]
            Cell {
                required property var modelData
                core: start.core; width: 72; height: parent.height; pad: 0; lineLeft: true; lineTop: true
                kanji: modelData[0]; kanjiSize: core.px; accent: modelData[1] === "shutdown"
                onClicked: { core.panel = ""; core.power(modelData[1]) }
            }
        }
    }
}
