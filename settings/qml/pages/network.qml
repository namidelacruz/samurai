// Settings › Network: Wi-Fi, Bluetooth, and the campus networks (UC-Secure, AirUC-Secure,
// eduroam), which need a certificate and a sign-in. Compact: small tiles on top, slim rows below.
// Passwords go to the command's input, never its arguments.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import ".."

ColumnLayout {
    id: net
    readonly property var c: core.colors
    readonly property real s: core.scale
    property var nets: []
    property var bt: ({ have: false, on: false, devices: [] })
    property string open: ""          // the network whose sign-in row is open ("\u0001hidden" for a hidden one)
    property bool busy: false
    property string log: ""
    readonly property bool wifiOn: core.status.network ? core.status.network.wifi_on !== false : true
    spacing: 14 * s
    function scan() { nets = core.wifi(); bt = core.bluetooth() }
    Component.onCompleted: scan()
    Timer { interval: 8000; running: true; repeat: true; onTriggered: if (!net.busy && net.open === "") net.scan() }
    Connections {
        target: core
        function onDone(tag, code, text) {
            if (tag.indexOf("net:") !== 0 && tag.indexOf("bt:") !== 0) return
            net.busy = false
            if (tag === "net:log") { net.log = text; return }
            if (code === 0 && (tag === "net:join" || tag === "net:hidden")) net.open = ""
            win.say(code === 0 ? (text.split("\n").pop() || "Done.") : text, code !== 0)
            net.scan()
        }
    }

    // ── quick tiles ──────────────────────────────────────────────────────────────────────
    component QuickTile: Button {
        id: qt
        property string glyph: ""
        property string word: ""
        property string state2: ""
        property bool on: false
        implicitWidth: 132 * s; implicitHeight: 48 * s
        background: Rectangle { radius: 10 * s; color: qt.on ? c.accent : (qt.hovered ? c.chrome : c.raised); border.color: qt.on ? c.accent : c.line }
        contentItem: Row {
            spacing: 9 * s; leftPadding: 4 * s
            Icon { name: qt.glyph; tint: qt.on ? c.onAccent : c.text; size: 16 * s; anchors.verticalCenter: parent.verticalCenter }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Text { text: qt.word; color: qt.on ? c.onAccent : c.text; font.pixelSize: 12.5 * s; font.weight: Font.DemiBold }
                Text { text: qt.state2; color: qt.on ? c.onAccent : c.dim; font.pixelSize: 11 * s; width: 90 * s; elide: Text.ElideRight }
            }
        }
    }
    Flow {
        Layout.fillWidth: true; spacing: 8 * s
        QuickTile {
            glyph: "wifi"; word: "Wi-Fi"; on: net.wifiOn
            state2: !net.wifiOn ? "Off" : (net.nets.length && net.nets[0].on ? net.nets[0].ssid : "Not connected")
            Accessible.name: "Wi-Fi " + (on ? "on" : "off")
            onClicked: core.run("net:radio", ["nmcli", "radio", "wifi", net.wifiOn ? "off" : "on"])
        }
        QuickTile {
            visible: net.bt.have
            glyph: "bt"; word: "Bluetooth"; on: net.bt.on
            state2: !net.bt.on ? "Off" : (net.bt.devices.length && net.bt.devices[0].on ? net.bt.devices[0].name : "On")
            Accessible.name: "Bluetooth " + (on ? "on" : "off")
            onClicked: core.run("bt:power", ["bluetoothctl", "power", net.bt.on ? "off" : "on"])
        }
        QuickTile {
            glyph: "refresh"; word: "Look again"; state2: net.nets.length + " networks"
            onClicked: net.scan()
        }
    }

    // ── a slim row, used by every list here ─────────────────────────────────────────────
    component Row2: Button {
        id: r2
        property string label: ""
        property string sub: ""
        property bool strong: false
        property int bars: -1          // 0..3 signal bars; -1 none
        property string iconName: ""
        property bool lock: false
        default property alias trailing: tail.data
        width: parent ? parent.width : 0; height: 40 * s
        background: Rectangle { color: r2.hovered ? Qt.alpha(c.text, 0.04) : "transparent" }
        contentItem: Item {
            Row {
                x: 14 * s; anchors.verticalCenter: parent.verticalCenter; spacing: 1.5 * s; visible: r2.bars >= 0
                Repeater { model: 3; Rectangle { required property int index; width: 2.6 * s; height: (5 + 3 * index) * s; radius: 1; anchors.bottom: parent.bottom; color: r2.bars > index ? c.text : c.line } }
            }
            Icon { visible: r2.iconName !== ""; name: r2.iconName; x: 12 * s; size: 15 * s; tint: r2.strong ? c.accent : c.dim; anchors.verticalCenter: parent.verticalCenter }
            Row {
                x: 38 * s; anchors.verticalCenter: parent.verticalCenter; spacing: 6 * s
                Text { text: r2.label; color: r2.strong ? c.accent : c.text; font.pixelSize: 13 * s; font.weight: r2.strong ? Font.DemiBold : Font.Normal; elide: Text.ElideRight; width: Math.min(implicitWidth, r2.width - 38 * s - tail.width - 90 * s) }
                Icon { visible: r2.lock; name: "lock"; size: 11 * s; tint: c.dim; anchors.verticalCenter: parent.verticalCenter }
                Text { text: r2.sub; color: c.dim; font.pixelSize: 11.5 * s; anchors.verticalCenter: parent.verticalCenter }
            }
            Row { id: tail; anchors.right: parent.right; anchors.rightMargin: 10 * s; anchors.verticalCenter: parent.verticalCenter; spacing: 4 * s }
        }
    }
    component Small: Button {
        id: sm
        property string word: ""
        property bool primary: false
        implicitHeight: 26 * s; implicitWidth: lab.implicitWidth + 20 * s
        background: Rectangle { radius: 7 * s; color: sm.primary ? c.accent : (sm.hovered ? c.chrome : "transparent"); border.color: sm.primary ? c.accent : c.line }
        contentItem: Text { id: lab; text: sm.word; color: sm.primary ? c.onAccent : c.text; font.pixelSize: 11.5 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    }

    RowLayout {
        spacing: 16 * s
        Layout.fillWidth: true
        // ── Wi-Fi networks ───────────────────────────────────────────────────────────────
        Card {
            title: "Wi-Fi networks"
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 1
            Repeater {
                model: net.nets
                Column {
                    id: nrow
                    required property var modelData
                    required property int index
                    width: parent.width
                    Rectangle { visible: nrow.index > 0; width: parent.width; height: 1; color: c.line }
                    Row2 {
                        label: modelData.ssid; strong: modelData.on; lock: modelData.secure
                        bars: modelData.signal >= 70 ? 3 : modelData.signal >= 40 ? 2 : 1
                        sub: modelData.on ? "Connected" : (modelData.profile ? "Saved" : (modelData.campus ? "Campus" : ""))
                        Accessible.name: modelData.ssid + (modelData.on ? ", connected" : "")
                        onClicked: {
                            if (modelData.on) return
                            if (modelData.profile) { net.busy = true; core.run("net:join", ["nmcli", "connection", "up", "id", modelData.profile]); return }
                            if (modelData.campus) { win.say("Use Campus Wi-Fi on the right for " + modelData.ssid + ".", false); return }
                            if (!modelData.secure) { net.busy = true; core.run("net:join", ["nmcli", "device", "wifi", "connect", modelData.ssid]); return }
                            net.open = net.open === modelData.ssid ? "" : modelData.ssid
                        }
                        Small { visible: modelData.on; word: "Disconnect"; onClicked: core.run("net:down", ["nmcli", "connection", "down", "id", modelData.profile || modelData.ssid]) }
                        Small { visible: modelData.profile !== "" && !modelData.on; word: "Forget"; onClicked: core.run("net:forget", ["nmcli", "connection", "delete", "id", modelData.profile]) }
                    }
                    RowLayout {
                        visible: net.open === modelData.ssid
                        x: 38 * s; width: parent.width - 48 * s; height: visible ? 44 * s : 0; spacing: 6 * s
                        Field { id: pw; Layout.fillWidth: true; echoMode: TextInput.Password; placeholderText: "Password"; Accessible.name: "Password for " + modelData.ssid
                                onAccepted: go.clicked() }
                        Small { id: go; primary: true; word: "Join"; enabled: !net.busy && pw.text !== ""
                                onClicked: { net.busy = true; core.runInput("net:join", ["shadow", "wifi", "connect", modelData.ssid, "--password-stdin"], pw.text); pw.text = "" } }
                    }
                }
            }
            Text { visible: net.nets.length === 0; x: 14 * s; height: 40 * s; verticalAlignment: Text.AlignVCenter; text: net.wifiOn ? "No networks seen yet." : "Wi-Fi is off."; color: c.dim; font.pixelSize: 12.5 * s }
            Rectangle { width: parent.width; height: 1; color: c.line }
            // a network that does not show its name
            Row2 {
                iconName: "eye-off"; label: "Join a hidden network…"
                onClicked: net.open = net.open === "\u0001hidden" ? "" : "\u0001hidden"
            }
            ColumnLayout {
                visible: net.open === "\u0001hidden"
                x: 38 * s; width: parent.width - 48 * s; spacing: 6 * s
                Field { id: hname; Layout.fillWidth: true; placeholderText: "Network name"; Accessible.name: "Hidden network name" }
                RowLayout {
                    Layout.fillWidth: true; spacing: 6 * s
                    Field { id: hpw; Layout.fillWidth: true; echoMode: TextInput.Password; placeholderText: "Password (empty if open)"; Accessible.name: "Hidden network password" }
                    Small { primary: true; word: "Join"; enabled: !net.busy && hname.text !== ""
                            onClicked: { net.busy = true; core.runInput("net:hidden", ["shadow", "wifi", "connect", hname.text, "--password-stdin", "--hidden"], hpw.text); hpw.text = "" } }
                }
                Item { Layout.preferredHeight: 4 * s }
            }
        }
        // ── campus, Bluetooth, more ──────────────────────────────────────────────────────
        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 1
            Layout.fillWidth: true
            spacing: 14 * s
            Card {
                title: "Campus Wi-Fi"
                ColumnLayout {
                    x: 14 * s; width: parent.width - 28 * s
                    spacing: 8 * s
                    Item { Layout.preferredHeight: 2 * s }
                    Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 12 * s
                           text: "UC-Secure, AirUC-Secure and eduroam: set up with the university's certificate, and repaired if an update breaks them." }
                    Field { id: user; Layout.fillWidth: true; placeholderText: "Campus username (eduroam: name@ucalgary.ca)"; Accessible.name: "Campus username" }
                    Field { id: pass; Layout.fillWidth: true; echoMode: TextInput.Password; placeholderText: "Password"; Accessible.name: "Campus password" }
                    Flow {
                        Layout.fillWidth: true; spacing: 6 * s
                        Small { primary: true; word: "UC-Secure + AirUC-Secure"; enabled: !net.busy && user.text !== "" && pass.text !== ""
                                onClicked: { net.busy = true; core.runInput("net:campus", ["shadow", "wifi", "campus", "ucalgary", "--user", user.text, "--password-stdin"], pass.text); pass.text = "" } }
                        Small { word: "eduroam"; enabled: !net.busy && user.text !== "" && pass.text !== ""
                                onClicked: { net.busy = true; core.runInput("net:campus", ["shadow", "wifi", "campus", "eduroam", "--user", user.text, "--password-stdin"], pass.text); pass.text = "" } }
                        Small { word: "Repair this connection"; enabled: !net.busy; onClicked: { net.busy = true; core.run("net:repair", ["shadow", "wifi", "repair"]) } }
                    }
                    Item { Layout.preferredHeight: 4 * s }
                }
            }
            Card {
                title: "Bluetooth"
                visible: net.bt.have
                Repeater {
                    model: net.bt.devices
                    Column {
                        required property var modelData
                        required property int index
                        width: parent.width
                        Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line }
                        Row2 {
                            iconName: "bt"; label: modelData.name; strong: modelData.on; sub: modelData.on ? "Connected" : ""
                            onClicked: core.run("bt:conn", ["bluetoothctl", modelData.on ? "disconnect" : "connect", modelData.mac])
                            Small { word: modelData.on ? "Disconnect" : "Connect"; onClicked: core.run("bt:conn", ["bluetoothctl", modelData.on ? "disconnect" : "connect", modelData.mac]) }
                        }
                    }
                }
                Text { visible: net.bt.devices.length === 0; x: 14 * s; height: 40 * s; verticalAlignment: Text.AlignVCenter; color: c.dim; font.pixelSize: 12.5 * s
                       text: net.bt.on ? "No paired devices. Put a device in pairing mode, then pair it with: bluetoothctl" : "Bluetooth is off." }
            }
            Card {
                title: "More"
                Repeater {
                    model: [["doc", "Problem log", ["shadow", "wifi", "log"]],
                            ["repair", "Device check", ["shadow", "wifi", "device"]],
                            ["plug", "Wired and VPN", ["nm-connection-editor"]]]
                    Column {
                        required property var modelData
                        required property int index
                        width: parent.width
                        Rectangle { visible: index > 0; width: parent.width; height: 1; color: c.line }
                        Row2 {
                            iconName: modelData[0]; label: modelData[1]
                            onClicked: { if (modelData[2][0] === "nm-connection-editor") core.run("app", modelData[2]); else core.run("net:log", modelData[2]) }
                        }
                    }
                }
            }
            Rectangle {
                visible: net.log !== ""
                Layout.fillWidth: true; implicitHeight: logText.implicitHeight + 20 * s
                radius: 10 * s; color: c.chrome; border.color: c.line
                Text { id: logText; x: 10 * s; y: 10 * s; width: parent.width - 20 * s; text: net.log; color: c.text; font.family: "JetBrains Mono"; font.pixelSize: 11.5 * s; wrapMode: Text.WrapAnywhere }
            }
        }
    }
}
