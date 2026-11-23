// Settings › Look: this machine's ink, made once.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    readonly property var c: core.colors
    readonly property real s: core.scale
    property bool asking: false
    spacing: 0
    Card {
        title: "Wallpaper"
        ColumnLayout {
            x: 18 * s; width: parent.width - 36 * s
            spacing: 14 * s
            Item { Layout.preferredHeight: 4 * s }
            RowLayout {
                spacing: 12 * s
                Repeater {
                    model: ["night", "paper"]
                    Image {
                        required property string modelData
                        Layout.fillWidth: true
                        Layout.preferredHeight: width * 10 / 16
                        // no seal yet (before `samurai seal forge`): an empty frame, not an error
                        source: core.sealHas("wall-" + modelData + ".svg") ? core.sealDir + "/wall-" + modelData + ".svg" : ""
                        sourceSize.width: 640
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                        cache: false
                        Accessible.name: modelData + " wallpaper"
                        Rectangle { anchors.fill: parent; color: "transparent"; radius: 8 * s; border.color: c.line }
                    }
                }
            }
            Text { text: "Your ink, made once"; color: c.text; font.pixelSize: 14 * s; font.weight: Font.DemiBold }
            Text {
                Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 13 * s
                text: "Marbled for this machine from random numbers that were thrown away after drawing. Nobody, including Samurai, can make this one again." + (core.status.seal && core.status.seal.mark ? "  Mark: " + core.status.seal.mark : "")
            }
            RowLayout {
                spacing: 12 * s
                Btn { visible: !asking; text: "Make a new one…"; onClicked: asking = true }
                Btn { visible: asking; primary: true; text: "Throw this one away and make a new one"; onClicked: { asking = false; core.run("forge", ["samurai", "seal", "forge", "--erase-current"]) } }
                Btn { visible: asking; text: "Keep it"; onClicked: asking = false }
                Text { visible: !asking; text: "Asks first: the current one is lost for good."; color: c.dim; font.pixelSize: 12.5 * s }
            }
            Item { Layout.preferredHeight: 6 * s }
        }
    }
    Connections {
        target: core
        function onDone(tag, code, text) { if (tag === "forge") { core.reload(); win.say(code === 0 ? "A new ink is made. The old one is gone." : text, code !== 0) } }
    }
}
