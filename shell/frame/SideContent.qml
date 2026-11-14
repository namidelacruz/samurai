// The smaller panels that grow from the frame: widgets (clock, calendar, system), the power
// menu, and the keybind legend. One file, chosen by `mode`.
import QtQuick
import "../lib"

Rectangle {
    id: side
    required property var core
    property string mode: "widgets"
    property var binds: []
    color: core.ground
    Rectangle { width: core.line; height: parent.height; color: core.ink }
    Rectangle { anchors.right: parent.right; width: core.line; height: parent.height; color: core.ink }
    Rectangle { anchors.bottom: parent.bottom; height: core.line; width: parent.width; color: core.ink }

    // ── widgets ──
    Column {
        visible: side.mode === "widgets"
        anchors.fill: parent; anchors.margins: 20; spacing: 10
        Txt { core: side.core; text: core.clockText; heavy: true; font.pixelSize: 64 }
        Txt { core: side.core; text: core.dateText; jp: true; font.pixelSize: core.px + 3 }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Grid {
            id: cal
            columns: 7
            readonly property date first: new Date(core.now.getFullYear(), core.now.getMonth(), 1)
            readonly property int lead: (first.getDay() - (core.v("region.week_start") === "monday" ? 1 : 0) + 7) % 7
            readonly property int days: new Date(core.now.getFullYear(), core.now.getMonth() + 1, 0).getDate()
            Repeater {
                model: 7
                Txt { required property int index; core: side.core; width: Math.floor(cal.parent.width / 7); height: 26; horizontalAlignment: Text.AlignHCenter; jp: true; faint: true; text: core.weekJp[(index + (core.v("region.week_start") === "monday" ? 1 : 0)) % 7] }
            }
            Repeater {
                model: cal.lead + cal.days
                Rectangle {
                    required property int index
                    readonly property int day: index - cal.lead + 1
                    width: Math.floor(cal.parent.width / 7); height: 32
                    color: day === core.now.getDate() ? core.red : "transparent"
                    Txt { core: side.core; anchors.centerIn: parent; visible: parent.day > 0; text: "" + parent.day; color: parent.day === core.now.getDate() ? core.redInk : core.ink; heavy: parent.day === core.now.getDate() }
                }
            }
        }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Txt { core: side.core; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; text: core.label("系", "SYSTEM") }
        Txt { core: side.core; text: core.userName + " @ " + core.hostName }
        Txt { core: side.core; text: "mark  " + core.mark }
        Txt { core: side.core; text: "base  " + (core.white ? "白 white" : "黒 black"); jp: false }
    }
    // ── power ──
    Column {
        visible: side.mode === "power"
        anchors.fill: parent
        Txt { core: side.core; leftPadding: 20; topPadding: 16; bottomPadding: 10; text: "電源"; jp: true; heavy: true; font.pixelSize: 44 }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Repeater {
            model: [["施錠", "LOCK", "lock"], ["休止", "SLEEP", "sleep"], ["退出", "LOG OUT", "logout"], ["再起動", "RESTART", "restart"], ["電源", "SHUT DOWN", "shutdown"]]
            Cell {
                required property var modelData
                core: side.core; width: side.width; height: core.hit + 12; lineBottom: true
                kanji: modelData[0]; text: modelData[1]; accent: modelData[2] === "shutdown"
                onClicked: { core.panel = ""; core.power(modelData[2]) }
            }
        }
    }
    // ── keybind legend ──
    Flickable {
        visible: side.mode === "keys"
        anchors.fill: parent; anchors.margins: core.line; clip: true
        contentHeight: legend.implicitHeight
        Column {
            id: legend
            width: parent.width
            Txt { core: side.core; leftPadding: 20; topPadding: 16; bottomPadding: 10; text: "鍵 KEYS"; jp: true; heavy: true; font.pixelSize: 28 }
            Repeater {
                model: side.binds.filter(function(b) { return !b.held })
                Item {
                    required property var modelData
                    width: legend.width; height: 30
                    Txt { core: side.core; x: 20; width: 230; height: parent.height; text: modelData.keys; heavy: true; font.pixelSize: core.small }
                    Txt { core: side.core; x: 250; width: parent.width - 260; height: parent.height; text: modelData.name; font.pixelSize: core.small }
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: core.dim; opacity: 0.4 }
                }
            }
        }
    }
}
