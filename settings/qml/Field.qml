// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

TextField {
    readonly property var c: core.colors
    readonly property real s: core.scale

    implicitWidth: 230 * s; implicitHeight: 36 * s
    color: c.text; placeholderTextColor: c.dim; selectionColor: c.accent; selectedTextColor: c.onAccent
    font.pixelSize: 13.5 * s
    leftPadding: 12 * s
    background: Rectangle { radius: 10 * s; color: c.field; border.color: parent.activeFocus ? c.accent : c.line; border.width: parent.activeFocus ? 2 : 1 }
}
