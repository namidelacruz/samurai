// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Button {
    readonly property var c: core.colors
    readonly property real s: core.scale

    id: sw
    property bool on: false
    signal flipped(bool to)
    checkable: false
    implicitWidth: 42 * s; implicitHeight: 24 * s
    onClicked: flipped(!on)
    Accessible.role: Accessible.CheckBox
    Accessible.checked: on
    background: Rectangle {
        radius: height / 2
        color: sw.on ? c.accent : c.line
        border.width: sw.visualFocus ? 2 : 0; border.color: c.text
        Rectangle {
            width: parent.height - 4 * s; height: width; radius: width / 2
            y: 2 * s; x: sw.on ? parent.width - width - 2 * s : 2 * s
            color: sw.on ? c.onAccent : c.raised
            Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        }
    }
    contentItem: Item {}
}
