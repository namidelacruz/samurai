// Settings › Sound: where sound goes and comes from (PipeWire).
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var audio: core.audio()
    spacing: 20 * s
    Connections { target: core; function onDone(tag, code, text) { if (tag === "snd") audio = core.audio() } }
    Repeater {
        model: [["sinks", "Sound goes to", "sound"], ["sources", "Microphone", "mic"]]
        Card {
            required property var modelData
            title: modelData[1]
            Repeater {
                model: audio[modelData[0]] || []
                Button {
                    required property var modelData
                    required property int index
                    width: parent.width; height: 58 * s
                    Accessible.name: modelData.name
                    onClicked: core.run("snd", ["wpctl", "set-default", modelData.id])
                    background: Rectangle { color: parent.hovered ? Qt.alpha(c.text, 0.04) : "transparent"; Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line } }
                    contentItem: Item {
                        Icon { name: parent.parent.parent.parent.modelData ? "sound" : "sound"; x: 18 * s; anchors.verticalCenter: parent.verticalCenter; tint: modelData.default ? c.accent : c.dim }
                        Column {
                            x: 52 * s; anchors.verticalCenter: parent.verticalCenter
                            Text { text: modelData.name; color: c.text; font.pixelSize: 14 * s; font.weight: modelData.default ? Font.DemiBold : Font.Medium }
                            Text { text: (modelData.default ? "In use · " : "") + modelData.volume + "%" + (modelData.muted ? " · muted" : ""); color: c.dim; font.pixelSize: 12.5 * s }
                        }
                        Icon { visible: modelData.default; name: "check"; tint: c.accent; anchors.right: parent.right; anchors.rightMargin: 18 * s; anchors.verticalCenter: parent.verticalCenter }
                    }
                }
            }
            Text { visible: (audio[modelData[0]] || []).length === 0; x: 18 * s; height: 52 * s; verticalAlignment: Text.AlignVCenter; text: "Nothing found (is PipeWire running?)"; color: c.dim; font.pixelSize: 13 * s }
        }
    }
}
