// Settings › Keyboard & shortcuts: every shortcut there is now.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var keys: core.shortcuts()
    Connections { target: core; function onChanged() { keys = core.shortcuts() } }
    Card {
        title: "Shortcuts now"
        GridLayout {
            x: 18 * s; width: parent.width - 36 * s
            columns: 2; rowSpacing: 0; columnSpacing: 24 * s
            Item { Layout.preferredHeight: 6 * s; Layout.columnSpan: 2 }
            Repeater {
                model: keys
                Item {
                    required property var modelData
                    Layout.fillWidth: true; Layout.preferredHeight: 34 * s
                    Rectangle {
                        id: chip
                        anchors.verticalCenter: parent.verticalCenter
                        width: kt.implicitWidth + 16 * s; height: 26 * s; radius: 6 * s; color: c.chrome; border.color: c.line
                        Text { id: kt; anchors.centerIn: parent; text: modelData.keys; color: c.text; font.family: "JetBrains Mono"; font.pixelSize: 12 * s }
                    }
                    Text { x: Math.max(chip.width + 12 * s, 190 * s); anchors.verticalCenter: parent.verticalCenter; width: parent.width - x; text: modelData.action; color: c.dim; font.pixelSize: 13 * s; elide: Text.ElideRight }
                }
            }
            Item { Layout.preferredHeight: 6 * s; Layout.columnSpan: 2 }
        }
        Text { visible: keys.length === 0; x: 18 * s; height: 52 * s; verticalAlignment: Text.AlignVCenter; text: "The window manager is not running here."; color: c.dim; font.pixelSize: 13 * s }
    }
}
