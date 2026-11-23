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
    // security key
    property var sticks: []
    property string keyStatus: ""
    property bool tested: false
    property string pickedStick: ""
    function keyRefresh() { core.run("key:list", ["samurai", "key", "list"]); core.run("key:status", ["samurai-key-pam", "status"]) }
    Component.onCompleted: keyRefresh()
    spacing: 20 * s
    Connections {
        target: core
        function onDone(tag, code, text) {
            if (tag.indexOf("key:") === 0) {
                if (tag === "key:list") { var l = []; text.split("\n").forEach(function (x) { try { if (x.trim()) l.push(JSON.parse(x)) } catch (e) {} }); sys.sticks = l; return }
                if (tag === "key:status") { sys.keyStatus = text; return }
                if (tag === "key:test") sys.tested = code === 0
                win.say(text || (code === 0 ? "Done." : "That did not work."), code !== 0)
                if (tag !== "key:test") sys.keyRefresh()
                return
            }
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
        title: "Security key"
        ColumnLayout {
            x: 16 * s; width: parent.width - 32 * s; spacing: 10 * s
            Item { Layout.preferredHeight: 4 * s }
            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.text; font.pixelSize: 13 * s; font.weight: Font.DemiBold
                   text: sys.keyStatus || "key login: off" }
            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 12 * s
                   text: "Log in and unlock with a key. A FIDO2 security key (YubiKey, SoloKey…) cannot be copied: use one if you can. A plain USB stick works too, but it is weaker: anyone who copies it can use the copy once, until you next use yours (its secret changes every time). Your password keeps working unless you choose key and password; the console (Ctrl+Alt+F2) always takes your password." }
            // a plain USB stick
            Text { text: "USB stick"; color: c.text; font.pixelSize: 13 * s; font.weight: Font.Medium }
            Flow {
                Layout.fillWidth: true; spacing: 6 * s
                Repeater {
                    model: sys.sticks
                    Btn { required property var modelData; text: (modelData.label || modelData.uuid) + (modelData.mounted ? "" : " (not opened)"); primary: sys.pickedStick === modelData.uuid; onClicked: sys.pickedStick = modelData.uuid }
                }
                Text { visible: sys.sticks.length === 0; text: "Plug a USB stick in (and open it once in Files)."; color: c.dim; font.pixelSize: 12 * s }
                Btn { text: "Look again"; onClicked: sys.keyRefresh() }
            }
            Flow {
                Layout.fillWidth: true; spacing: 6 * s
                Btn { text: "Make it my key"; enabled: sys.pickedStick !== ""; onClicked: { sys.tested = false; core.run("key:enroll", ["samurai", "key", "enroll", sys.pickedStick]) } }
                Btn { text: "Test it"; onClicked: core.run("key:test", ["samurai", "key", "test"]) }
                Btn { text: "Key or password"; primary: true; onClicked: core.run("key:on", ["pkexec", "/opt/samurai/bin/samurai-key-pam", "enable", "usb", "either"]) }
                Btn { text: "Key and password"; enabled: sys.tested; onClicked: core.run("key:on", ["pkexec", "/opt/samurai/bin/samurai-key-pam", "enable", "usb", "both", "--tested", core.status.user || ""]) }
                Btn { text: "Forget the stick"; onClicked: core.run("key:forget", ["samurai", "key", "forget"]) }
            }
            Text { visible: !sys.tested; Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 11.5 * s; text: "Key and password needs a test that passed just now, so a key that does not work can never lock you out." }
            // a FIDO2 security key
            Text { text: "Security key (FIDO2)"; color: c.text; font.pixelSize: 13 * s; font.weight: Font.Medium }
            Flow {
                Layout.fillWidth: true; spacing: 6 * s
                Btn { text: "Register a key…"; onClicked: core.run("app", ["alacritty", "--hold", "-e", "sh", "-c", "mkdir -p ~/.config/Yubico && echo 'Touch your security key when it blinks.' && pamu2fcfg >> ~/.config/Yubico/u2f_keys && echo 'Registered. Close this window.'"]) }
                Btn { text: "Key or password"; primary: true; onClicked: core.run("key:on", ["pkexec", "/opt/samurai/bin/samurai-key-pam", "enable", "u2f", "either"]) }
            }
            Btn { text: "Turn key login off"; onClicked: core.run("key:off", ["pkexec", "/opt/samurai/bin/samurai-key-pam", "disable"]) }
            Item { Layout.preferredHeight: 6 * s }
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
