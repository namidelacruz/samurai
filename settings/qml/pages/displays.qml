// Settings › Displays: what is plugged in now.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var screens: core.screens()
    Card {
        title: "Plugged in now"
        Repeater {
            model: screens
            Item {
                required property var modelData
                required property int index
                width: parent.width; height: 64 * s
                Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line }
                Icon { name: "display"; x: 18 * s; anchors.verticalCenter: parent.verticalCenter }
                Column {
                    x: 50 * s; anchors.verticalCenter: parent.verticalCenter
                    Text { text: modelData.name + (modelData.primary ? "  · main screen" : ""); color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium }
                    Text { text: modelData.geometry + "   ·   best: " + (modelData.modes[0] || "—"); color: c.dim; font.pixelSize: 12.5 * s }
                }
            }
        }
        Text { visible: screens.length === 0; x: 18 * s; height: 52 * s; verticalAlignment: Text.AlignVCenter; text: "No screens found (is this an X session?)"; color: c.dim }
    }
    RowLayout {
        Layout.topMargin: 10 * s
        Btn { text: "Look again"; iconName: "repair"; onClicked: screens = core.screens() }
        Btn { text: "Arrange screens now"; onClicked: core.run("screens", ["samurai", "apply"]) }
    }
}
