// The bar: the upper (or lower) edge of the frame. Cells on a visible grid, left to right:
// 影 start · workspaces · focused window · update · network · sound · battery · date and clock.
import QtQuick
import "../lib"

Rectangle {
    id: bar
    required property var core
    property string screenName: ""
    color: core.ground
    height: core.barH
    rotation: core.tilt(1) * 0.25

    Rectangle { width: parent.width; height: core.line; color: core.ink; anchors.bottom: core.v("frame.bar_edge") === "bottom" ? undefined : parent.bottom; anchors.top: core.v("frame.bar_edge") === "bottom" ? parent.top : undefined }

    Row {
        id: left
        height: parent.height
        Cell { core: bar.core; kanji: "影"; kanjiSize: core.px + 9; width: core.railW; height: parent.height; pad: 0; lineRight: true; filled: core.panel === "start"; rotation: core.tilt(2); onClicked: core.toggle("start") }
        Repeater {
            model: core.v("frame.bar.workspaces") === false ? 0 : (core.v("frame.bar.workspace_count") || 4)
            Cell {
                required property int index
                core: bar.core
                readonly property int n: index + 1
                readonly property bool has: core.windows.some(function(w) { return w.workspace === n && !w.minimized && (w.output === core.focusedOutput || !w.output) })
                width: core.hit; height: bar.height; pad: 0; lineRight: true
                kanji: core.v("frame.bar.workspace_marks") === "digits" ? "" : core.kanjiDigits[n]
                text: core.v("frame.bar.workspace_marks") === "digits" ? "" + n : ""
                accent: core.activeWorkspace === n
                faint: !has && core.activeWorkspace !== n
                rotation: core.tilt(10 + index)
                onClicked: core.run(["niri", "msg", "action", "focus-workspace", "" + core.workspaceIndex(n)])
                onWheel: function(d) { core.run(["niri", "msg", "action", d < 0 ? "focus-workspace-down" : "focus-workspace-up"]) }
            }
        }
    }
    Row {
        visible: core.v("frame.bar.title") !== false && core.focusedWindow !== null
        anchors.left: left.right; anchors.leftMargin: 16; anchors.right: right.left; anchors.rightMargin: 16
        height: parent.height; spacing: 12; clip: true
        Txt { core: bar.core; text: core.focusedWindow ? core.appKanji(core.focusedWindow.cls) : ""; jp: true; heavy: true; height: parent.height }
        Txt { core: bar.core; text: core.focusedWindow ? (core.incognito ? "WINDOW" : (core.focusedWindow.cls || "").toUpperCase()) : ""; heavy: true; font.letterSpacing: 1; height: parent.height }
        Txt { core: bar.core; text: core.focusedTitle; faint: true; height: parent.height; width: Math.max(0, parent.width - x); }
    }
    Row {
        id: right
        anchors.right: parent.right
        height: parent.height
        Cell {
            core: bar.core; visible: core.v("frame.bar.update") !== false && !!core.status.update && core.status.update.available === true
            height: parent.height; lineLeft: true; kanji: "更新"; text: "1"; accent: true
            onClicked: core.openHub("updates")
        }
        Cell {
            core: bar.core; visible: core.status.recording === true; height: parent.height; lineLeft: true; kanji: "録"; accent: true
            onClicked: core.run(["samurai", "record", "stop"])
        }
        Cell {
            core: bar.core; visible: core.v("frame.bar.network") !== false; height: parent.height; lineLeft: true; kanji: "通信"; rotation: core.tilt(20)
            readonly property var n: core.status.network || ({})
            text: core.incognito ? "????" : n.wired ? "WIRED" : n.wifi ? "WIFI" : n.wifi_on ? "—" : "OFF"
            filled: core.panel === "quick"; onClicked: core.toggle("quick")
        }
        Cell {
            core: bar.core; visible: core.v("frame.bar.sound") !== false; height: parent.height; lineLeft: true; kanji: "音"; rotation: core.tilt(21)
            text: core.status.muted ? "×" : (core.status.volume === undefined || core.status.volume === null ? "—" : "" + core.status.volume)
            onClicked: core.toggle("quick")
            onWheel: function(d) { core.run(["wpctl", "set-volume", "-l", "1", "@DEFAULT_AUDIO_SINK@", d > 0 ? "5%+" : "5%-"]) }
        }
        Cell {
            core: bar.core; visible: core.v("frame.bar.battery") !== false && !!core.status.battery; height: parent.height; lineLeft: true; kanji: "電"; rotation: core.tilt(22)
            text: core.status.battery ? "" + core.status.battery.percent : ""
            accent: !!core.status.battery && core.status.battery.percent <= 10 && !core.status.battery.charging
            onClicked: core.toggle("quick")
        }
        Cell {
            core: bar.core; visible: core.v("frame.bar.clock") !== false; height: parent.height; lineLeft: true; rotation: core.tilt(23)
            kanji: core.dateText; kanjiSize: core.px; text: core.clockText; heavy: true
            filled: core.panel === "widgets"; onClicked: core.toggle("widgets")
        }
    }
}
