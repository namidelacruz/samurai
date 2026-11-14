// The one control: a square-cornered cell on the grid. It can carry a line on any side, be
// filled (pressed / selected = ink on ground swapped), or be red (the single accent).
import QtQuick

Rectangle {
    id: cell
    required property var core
    property string text: ""
    property string kanji: ""
    property bool filled: false       // selected: ink ground, ground text
    property bool accent: false       // red ground, white text
    property bool faint: false
    property bool heavy: false
    property bool enabled: true
    property bool lineLeft: false
    property bool lineRight: false
    property bool lineTop: false
    property bool lineBottom: false
    property bool box: false          // all four lines
    property int pad: 12
    property int kanjiSize: core.px + 3
    property alias hovered: area.containsMouse
    property var onWheel: null
    signal clicked()
    signal rightClicked()

    readonly property color fg: accent ? core.redInk : filled ? core.ground : faint ? core.dim : core.ink
    color: accent ? core.red : filled ? core.ink : (area.containsMouse && enabled ? Qt.rgba(core.ink.r, core.ink.g, core.ink.b, 0.14) : "transparent")
    implicitHeight: core.hit
    implicitWidth: row.implicitWidth + 2 * pad
    opacity: enabled ? 1 : 0.4
    activeFocusOnTab: true
    Keys.onReturnPressed: if (enabled) clicked()
    Keys.onSpacePressed: if (enabled) clicked()

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8
        Txt { core: cell.core; visible: cell.kanji !== ""; text: cell.kanji; jp: true; heavy: true; color: cell.fg; font.pixelSize: cell.kanjiSize; anchors.verticalCenter: parent.verticalCenter }
        Txt { core: cell.core; visible: cell.text !== ""; text: cell.text; heavy: cell.heavy || cell.filled || cell.accent; color: cell.fg; anchors.verticalCenter: parent.verticalCenter }
    }
    // the focus ring is a second, inner line: visible on both grounds
    Rectangle { anchors.fill: parent; anchors.margins: 3; color: "transparent"; border.width: cell.activeFocus ? 1 : 0; border.color: cell.fg }
    Rectangle { visible: cell.lineLeft || cell.box; width: core.line; height: parent.height; color: core.ink }
    Rectangle { visible: cell.lineRight || cell.box; width: core.line; height: parent.height; anchors.right: parent.right; color: core.ink }
    Rectangle { visible: cell.lineTop || cell.box; height: core.line; width: parent.width; color: core.ink }
    Rectangle { visible: cell.lineBottom || cell.box; height: core.line; width: parent.width; anchors.bottom: parent.bottom; color: core.ink }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: function(m) { if (!cell.enabled) return; cell.forceActiveFocus(); if (m.button === Qt.RightButton) cell.rightClicked(); else cell.clicked() }
        onWheel: function(w) { if (cell.onWheel) cell.onWheel(w.angleDelta.y) }
    }
}
