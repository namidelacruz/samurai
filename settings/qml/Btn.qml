// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Button {
    readonly property var c: core.colors
    readonly property real s: core.scale

    id: b
    property bool primary: false
    property string iconName: ""
    implicitHeight: 38 * s
    leftPadding: 16 * s; rightPadding: 16 * s
    font.pixelSize: 13.5 * s
    background: Rectangle { radius: 9 * s; color: b.primary ? (b.down ? Qt.darker(c.accent, 1.1) : c.accent) : (b.hovered ? c.raised : c.field); border.color: b.primary ? c.accent : c.line; border.width: b.visualFocus ? 2 : 1; opacity: b.enabled ? 1 : 0.45 }
    contentItem: Row {
        spacing: 8 * s
        Icon { visible: b.iconName !== ""; name: b.iconName; size: 16 * s; tint: b.primary ? c.onAccent : c.text; anchors.verticalCenter: parent.verticalCenter }
        Text { text: b.text; color: b.primary ? c.onAccent : c.text; font.family: b.font.family; font.pixelSize: b.font.pixelSize; font.weight: b.primary ? Font.DemiBold : Font.Normal; anchors.verticalCenter: parent.verticalCenter }
    }
}
