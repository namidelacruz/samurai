// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Rectangle {
    readonly property var c: core.colors
    readonly property real s: core.scale

    default property alias body: col.data
    property string title: ""
    Layout.fillWidth: true
    implicitHeight: head.height + col.implicitHeight
    color: "transparent"
    Text { id: head; text: parent.title; visible: parent.title !== ""; height: visible ? 26 * s : 0; color: c.dim; font.pixelSize: 13 * s; font.weight: Font.DemiBold; leftPadding: 4 * s }
    Rectangle {
        y: head.height; width: parent.width; height: col.implicitHeight
        radius: 14 * s; color: c.raised; border.color: c.line
        Column { id: col; width: parent.width }
    }
}
