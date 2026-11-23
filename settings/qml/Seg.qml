// part of Samurai Settings: a choice of two or three, the chosen one raised
import QtQuick
import QtQuick.Controls.Basic

Rectangle {
    id: seg
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var options: []
    property var current
    signal picked(var value)
    implicitWidth: row.implicitWidth + 6 * s
    implicitHeight: row.implicitHeight + 6 * s
    radius: 10 * s
    color: c.chrome
    border.color: c.line
    Row {
        id: row
        x: 3 * s; y: 3 * s
        spacing: 2 * s
        Repeater {
            model: seg.options
            Button {
                required property var modelData
                readonly property bool sel: JSON.stringify(seg.current) === JSON.stringify(modelData.value)
                text: modelData.label
                height: 32 * s
                leftPadding: 14 * s; rightPadding: 14 * s
                onClicked: seg.picked(modelData.value)
                Accessible.role: Accessible.RadioButton
                Accessible.checked: sel
                Accessible.name: modelData.label
                background: Rectangle { radius: 7 * s; color: parent.sel ? c.raised : (parent.hovered ? Qt.alpha(c.text, 0.06) : "transparent"); border.width: parent.visualFocus ? 2 : 0; border.color: c.accent }
                contentItem: Text { text: parent.text; color: parent.sel ? c.text : c.dim; font.pixelSize: 13 * s; font.weight: parent.sel ? Font.DemiBold : Font.Normal; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
            }
        }
    }
}
