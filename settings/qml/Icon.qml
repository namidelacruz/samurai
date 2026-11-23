// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Image {
    readonly property var c: core.colors
    readonly property real s: core.scale

    property string name: "apps"
    property color tint: c.dim
    property int size: 18 * s
    width: size; height: size
    sourceSize: Qt.size(size * 2, size * 2)
    source: core.icon(name, tint.toString())
    fillMode: Image.PreserveAspectFit
}
