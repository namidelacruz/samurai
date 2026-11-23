// part of Samurai Settings
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

ComboBox {
    readonly property var c: core.colors
    readonly property real s: core.scale

    id: cb
    implicitWidth: 230 * s; implicitHeight: 36 * s
    font.pixelSize: 13.5 * s
    background: Rectangle { radius: 8 * s; color: c.field; border.color: cb.visualFocus ? c.accent : c.line; border.width: cb.visualFocus ? 2 : 1 }
    contentItem: Text { leftPadding: 12 * s; text: cb.displayText; color: c.text; font.pixelSize: cb.font.pixelSize; font.family: cb.font.family; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
    indicator: Icon { name: "down"; size: 15 * s; x: cb.width - width - 10 * s; anchors.verticalCenter: parent.verticalCenter }
    popup.background: Rectangle { radius: 10 * s; color: c.raised; border.color: c.line }
    delegate: ItemDelegate {
        required property var modelData
        required property int index
        width: cb.width
        text: cb.textRole ? modelData[cb.textRole] : modelData
        highlighted: cb.highlightedIndex === index
        background: Rectangle { color: parent.highlighted ? c.soft : "transparent"; radius: 6 * s }
        contentItem: Text { text: parent.text; color: c.text; font.pixelSize: 13.5 * s; elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter }
    }
}
