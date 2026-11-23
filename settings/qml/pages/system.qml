// Settings › System & updates: version and updates, the login screen, the X server.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    id: sys
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var info: core.system()
    property bool busy: false
    property string doctor: ""
    spacing: 20 * s
    Connections {
        target: core
        function onDone(tag, code, text) {
            if (tag.indexOf("sys:") !== 0) return
            sys.busy = false
            if (tag === "sys:doctor") { sys.doctor = text; return }
            win.say(text || (code === 0 ? "Done." : "That did not work."), code !== 0)
            sys.info = core.system(); core.reload()
        }
    }
    readonly property var xs: info.xserver || ({})
    Card {
        Repeater {
            model: [
                ["info", "Samurai", info.version + (core.status.update && core.status.update.available ? "  ·  an update is ready" : ""), [["Look for updates", ["samurai", "update", "check"], false], ["Install the update", ["samurai", "update", "install"], true]]],
                ["lock", "Login screen", info.greeter ? "Samurai's login screen is set up." : "Not set up: your previous login screen is used.", info.greeter ? [["Put the previous one back", ["samurai", "greeter", "remove"], false]] : [["Set it up", ["samurai", "greeter", "apply"], true]]],
                ["display", "X server", (xs.server === "xlibre" ? "XLibre " : xs.server === "xorg" ? "X.Org " : "Unknown ") + (xs.version || "") + "  ·  Samurai works the same on both.", xs.server === "xlibre" ? [] : [["Switch to XLibre…", ["alacritty", "--hold", "-e", "/opt/samurai/bin/samurai-xserver", "xlibre"], false]]]
            ]
            Item {
                required property var modelData
                required property int index
                width: parent.width; height: 76 * s
                Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line }
                Icon { name: modelData[0]; x: 18 * s; anchors.verticalCenter: parent.verticalCenter }
                Column {
                    x: 52 * s; width: parent.width - 52 * s - acts.width - 30 * s; anchors.verticalCenter: parent.verticalCenter
                    Text { text: modelData[1]; color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium }
                    Text { width: parent.width; text: modelData[2]; color: c.dim; font.pixelSize: 12.5 * s; elide: Text.ElideRight }
                }
                Row {
                    id: acts
                    anchors.right: parent.right; anchors.rightMargin: 16 * s; anchors.verticalCenter: parent.verticalCenter
                    spacing: 8 * s
                    Repeater {
                        model: modelData[3]
                        Btn { required property var modelData; text: modelData[0]; primary: modelData[2]; enabled: !sys.busy; onClicked: { sys.busy = true; core.run("sys:" + modelData[0], modelData[1]) } }
                    }
                }
            }
        }
    }
    Card {
        title: "Check and repair"
        RowLayout {
            x: 16 * s; width: parent.width - 32 * s; height: 64 * s; spacing: 12 * s
            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 12.5 * s
                   text: "Looks for what commonly goes wrong (a damaged settings file, a helper that stopped, sound, Wi-Fi, disk space) and puts right what it safely can. Nothing of yours is deleted." }
            Btn { text: sys.busy ? "Checking…" : "Check and repair"; primary: true; enabled: !sys.busy; onClicked: { sys.busy = true; sys.doctor = ""; core.run("sys:doctor", ["samurai", "doctor"]) } }
        }
    }
    Rectangle {
        visible: sys.doctor !== ""
        Layout.fillWidth: true; implicitHeight: docText.implicitHeight + 20 * s
        radius: 10 * s; color: c.chrome; border.color: c.line
        Text { id: docText; x: 10 * s; y: 10 * s; width: parent.width - 20 * s; text: sys.doctor; color: c.text; font.family: "JetBrains Mono"; font.pixelSize: 11.5 * s; wrapMode: Text.Wrap }
    }
    Text {
        Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 12.5 * s
        text: "Remove Samurai: run ./install.sh --uninstall from its folder. Your settings, your ink and your previous login screen come back as they were."
    }
}
