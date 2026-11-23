// Settings › Network: Wi-Fi, and the campus networks (UC-Secure, AirUC-Secure, eduroam), which
// need a certificate and a sign-in. Passwords go to the command's input, never its arguments.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    id: net
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var nets: []
    property string open: ""
    property bool busy: false
    property string log: ""
    spacing: 20 * s
    function scan() { nets = core.wifi() }
    Component.onCompleted: scan()
    Connections {
        target: core
        function onDone(tag, code, text) {
            if (tag.indexOf("net:") !== 0) return
            net.busy = false
            if (tag === "net:log") { net.log = text; return }
            win.say(text || (code === 0 ? "Done." : "That did not work."), code !== 0)
            net.scan()
        }
    }
    RowLayout {
        spacing: 20 * s
        Layout.fillWidth: true
        // networks
        Card {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 1
            Item {
                width: parent.width; height: 58 * s
                Text { x: 18 * s; anchors.verticalCenter: parent.verticalCenter; text: "Wi-Fi"; color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium }
                Toggle {
                    anchors.right: parent.right; anchors.rightMargin: 18 * s; anchors.verticalCenter: parent.verticalCenter
                    on: core.status.network ? core.status.network.wifi_on !== false : true
                    onFlipped: (to) => core.run("net:radio", ["nmcli", "radio", "wifi", to ? "on" : "off"])
                    Accessible.name: "Wi-Fi"
                }
            }
            Repeater {
                model: nets
                Item {
                    required property var modelData
                    width: parent.width; height: (net.open === modelData.ssid ? 110 : 58) * s
                    Rectangle { width: parent.width; height: 1; color: c.line }
                    Icon { name: "wifi"; x: 18 * s; y: 20 * s; tint: modelData.on ? c.accent : c.dim; opacity: 0.4 + modelData.signal / 160 }
                    Column {
                        x: 52 * s; y: 11 * s
                        Text { text: modelData.ssid; color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium }
                        Text { text: (modelData.on ? "Connected · " : "") + (modelData.secure ? "secured" : "open") + " · " + modelData.signal + "%"; color: c.dim; font.pixelSize: 12.5 * s }
                    }
                    Btn {
                        anchors.right: parent.right; anchors.rightMargin: 14 * s; y: 10 * s
                        text: modelData.on ? "Disconnect" : (net.open === modelData.ssid ? "Cancel" : "Connect")
                        onClicked: {
                            if (modelData.on) core.run("net:down", ["nmcli", "connection", "down", "id", modelData.ssid])
                            else if (/UC-Secure|eduroam/i.test(modelData.ssid)) win.say("Use the campus setup on the right for " + modelData.ssid + ".", false)
                            else net.open = net.open === modelData.ssid ? "" : modelData.ssid
                        }
                    }
                    RowLayout {
                        visible: net.open === modelData.ssid
                        x: 52 * s; y: 62 * s; width: parent.width - 66 * s
                        Field { id: pw; Layout.fillWidth: true; echoMode: TextInput.Password; placeholderText: modelData.secure ? "Password" : "No password needed"; enabled: modelData.secure; Accessible.name: "Password for " + modelData.ssid }
                        Btn {
                            primary: true; text: "Join"; enabled: !net.busy
                            onClicked: { net.busy = true; core.runInput("net:join", ["shadow", "wifi", "connect", modelData.ssid, "--password-stdin"], pw.text); pw.text = ""; net.open = "" }
                        }
                    }
                }
            }
            Text { visible: nets.length === 0; x: 18 * s; height: 52 * s; verticalAlignment: Text.AlignVCenter; text: "No networks seen. Wi-Fi may be off."; color: c.dim; font.pixelSize: 13 * s }
        }
        // campus
        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 1
            Layout.fillWidth: true
            spacing: 20 * s
            Card {
                ColumnLayout {
                    x: 18 * s; width: parent.width - 36 * s
                    spacing: 12 * s
                    Item { Layout.preferredHeight: 4 * s }
                    RowLayout {
                        spacing: 12 * s
                        Icon { name: "shield"; tint: c.accent; size: 22 * s; Layout.alignment: Qt.AlignTop }
                        ColumnLayout {
                            spacing: 2 * s
                            Text { text: "Campus Wi-Fi"; color: c.text; font.pixelSize: 14 * s; font.weight: Font.DemiBold }
                            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 13 * s; text: "UCalgary and eduroam use a certificate and your campus sign-in. Samurai sets both up the way the university's IT expects, checks the certificate, and repairs the connection if it breaks after an update." }
                        }
                    }
                    Field { id: user; Layout.fillWidth: true; placeholderText: "Campus username (eduroam: name@ucalgary.ca)"; Accessible.name: "Campus username" }
                    Field { id: pass; Layout.fillWidth: true; echoMode: TextInput.Password; placeholderText: "Password (kept by NetworkManager, never in a file of ours)"; Accessible.name: "Campus password" }
                    Flow {
                        Layout.fillWidth: true; spacing: 10 * s
                        Btn { primary: true; text: "Set up UC-Secure + AirUC-Secure"; enabled: !net.busy && user.text !== "" && pass.text !== ""
                              onClicked: { net.busy = true; core.runInput("net:campus", ["shadow", "wifi", "campus", "ucalgary", "--user", user.text, "--password-stdin"], pass.text); pass.text = "" } }
                        Btn { text: "Set up eduroam"; enabled: !net.busy && user.text !== "" && pass.text !== ""
                              onClicked: { net.busy = true; core.runInput("net:campus", ["shadow", "wifi", "campus", "eduroam", "--user", user.text, "--password-stdin"], pass.text); pass.text = "" } }
                        Btn { text: "Repair this connection"; iconName: "repair"; enabled: !net.busy; onClicked: { net.busy = true; core.run("net:repair", ["shadow", "wifi", "repair"]) } }
                    }
                    Item { Layout.preferredHeight: 6 * s }
                }
            }
            Card {
                title: "More"
                Repeater {
                    model: [["doc", "Problem log", "What the last connection attempt said, in plain words", ["shadow", "wifi", "log"]],
                            ["repair", "Device check", "Is the Wi-Fi card there, with its firmware and driver", ["shadow", "wifi", "device"]],
                            ["plug", "Wired and VPN", "Opens NetworkManager's editor", ["nm-connection-editor"]]]
                    Button {
                        required property var modelData
                        required property int index
                        width: parent.width; height: 58 * s
                        onClicked: { if (modelData[3][0] === "nm-connection-editor") core.run("app", modelData[3]); else core.run("net:log", modelData[3]) }
                        background: Rectangle { color: parent.hovered ? Qt.alpha(c.text, 0.04) : "transparent"; Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line } }
                        contentItem: Item {
                            Icon { name: modelData[0]; x: 18 * s; anchors.verticalCenter: parent.verticalCenter }
                            Column {
                                x: 52 * s; width: parent.width - 52 * s - 44 * s; anchors.verticalCenter: parent.verticalCenter
                                Text { width: parent.width; text: modelData[1]; color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium; elide: Text.ElideRight }
                                Text { width: parent.width; text: modelData[2]; color: c.dim; font.pixelSize: 12.5 * s; elide: Text.ElideRight }
                            }
                            Icon { name: "right"; anchors.right: parent.right; anchors.rightMargin: 18 * s; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                }
            }
            Rectangle {
                visible: net.log !== ""
                Layout.fillWidth: true; implicitHeight: logText.implicitHeight + 24 * s
                radius: 10 * s; color: c.chrome; border.color: c.line
                Text { id: logText; x: 12 * s; y: 12 * s; width: parent.width - 24 * s; text: net.log; color: c.text; font.family: "JetBrains Mono"; font.pixelSize: 12 * s; wrapMode: Text.WrapAnywhere }
            }
        }
    }
}
