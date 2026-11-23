// The Samurai login screen: the public ink, the time, and one card. Same face as the lock.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Window
import "../settings/qml"

Window {
    id: w
    readonly property var c: core.colors
    readonly property real s: core.scale
    property int user: 0
    property int session: 0
    property string msg: ""
    property bool bad: false
    visible: true
    visibility: Window.FullScreen
    flags: Qt.FramelessWindowHint
    color: c.ground
    title: "Samurai login"

    property string question: ""
    property bool questionSecret: true
    Connections {
        target: core
        function onSaid(text, isBad) { w.msg = text; w.bad = isBad; w.question = ""; pw.text = ""; pw.forceActiveFocus(); if (isBad && !safeMode) shake.start() }
        function onAsked(text, secret) { w.question = text; w.questionSecret = secret; w.msg = ""; w.bad = false; pw.text = ""; pw.forceActiveFocus() }
    }

    Image { anchors.fill: parent; source: core.wall; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: source !== "" }
    Rectangle { anchors.fill: parent; color: c.ground; opacity: 0.45 }

    // the card, on the main screen
    Item {
        id: mon
        x: Screen.virtualX; y: Screen.virtualY; width: Screen.width; height: Screen.height
        Rectangle {
            id: card
            width: 360 * s; height: col.implicitHeight + 56 * s
            x: (parent.width - width) / 2
            y: (parent.height - height) / 2
            radius: 16 * s
            color: Qt.alpha(c.surf, 0.88)
            border.color: c.line
            SequentialAnimation {
                id: shake
                NumberAnimation { target: card; property: "x"; to: card.x - 10 * s; duration: 50 }
                NumberAnimation { target: card; property: "x"; to: card.x + 10 * s; duration: 60 }
                NumberAnimation { target: card; property: "x"; to: (mon.width - card.width) / 2; duration: 50 }
            }
            Column {
                id: col
                anchors.centerIn: parent
                width: parent.width - 56 * s
                spacing: 16 * s
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 76 * s; height: width; radius: width / 2; color: c.raised; border.color: c.line
                    Text { anchors.centerIn: parent; text: (core.users[w.user] || "?").charAt(0).toUpperCase(); color: c.accent; font.pixelSize: 30 * s; font.weight: Font.DemiBold }
                }
                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: 10 * s
                    Button {
                        visible: core.users.length > 1; width: 32 * s; height: 32 * s
                        Accessible.name: "Previous user"
                        onClicked: w.user = (w.user + core.users.length - 1) % core.users.length
                        background: Rectangle { radius: 8 * s; color: parent.hovered ? c.raised : "transparent" }
                        contentItem: Text { text: "‹"; color: c.dim; font.pixelSize: 20 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                    TextInput {
                        id: name
                        visible: core.users.length === 0
                        width: 200 * s; color: c.text; font.pixelSize: 18 * s; horizontalAlignment: Text.AlignHCenter
                        Text { anchors.fill: parent; visible: !parent.text; text: "user name"; color: c.dim; font.pixelSize: 18 * s; horizontalAlignment: Text.AlignHCenter }
                    }
                    Text { visible: core.users.length > 0; text: core.users[w.user] || ""; color: c.text; font.pixelSize: 18 * s; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                    Button {
                        visible: core.users.length > 1; width: 32 * s; height: 32 * s
                        Accessible.name: "Next user"
                        onClicked: w.user = (w.user + 1) % core.users.length
                        background: Rectangle { radius: 8 * s; color: parent.hovered ? c.raised : "transparent" }
                        contentItem: Text { text: "›"; color: c.dim; font.pixelSize: 20 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                }
                Field {
                    id: pw
                    width: parent.width; height: 48 * s
                    echoMode: w.question !== "" && !w.questionSecret ? TextInput.Normal : TextInput.Password
                    placeholderText: core.busy ? "Checking…" : (w.question !== "" ? w.question : "Password")
                    focus: true
                    enabled: !core.busy
                    rightPadding: 48 * s
                    font.pixelSize: 16 * s
                    Accessible.name: "Password"
                    Component.onCompleted: forceActiveFocus()
                    onAccepted: go.clicked()
                    background: Rectangle { radius: 12 * s; color: c.field; border.color: w.bad ? "#C8503F" : c.accent; border.width: 2 }
                    Button {
                        id: go
                        anchors.right: parent.right; anchors.rightMargin: 5 * s; anchors.verticalCenter: parent.verticalCenter
                        width: 38 * s; height: 38 * s
                        Accessible.name: "Sign in"
                        enabled: !core.busy
                        onClicked: {
                            w.msg = ""; w.bad = false
                            if (w.question !== "") { var a = pw.text; pw.text = ""; core.answer(a) }
                            else { var p = pw.text; pw.text = ""; core.login(core.users.length ? core.users[w.user] : name.text, p, w.session) }
                        }
                        background: Rectangle { radius: 9 * s; color: c.accent }
                        contentItem: Icon { name: "right"; tint: c.onAccent; size: 18 * s; anchors.centerIn: parent }
                    }
                }
                Row {
                    width: parent.width; spacing: 8 * s
                    Button {
                        width: parent.width; height: 36 * s
                        Accessible.name: "Session: " + core.sessions[w.session]
                        onClicked: w.session = (w.session + 1) % core.sessions.length
                        background: Rectangle { radius: 9 * s; color: parent.hovered ? c.raised : "transparent"; border.color: c.line }
                        contentItem: Text { text: core.sessions[w.session] + (core.sessions.length > 1 ? "   ›" : ""); color: c.dim; font.pixelSize: 13 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                }
                Text {
                    width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                    text: w.msg || (w.question !== "" ? w.question : "Type your password and press Enter.")
                    color: w.bad ? c.text : c.dim; font.pixelSize: 13 * s; font.weight: w.bad ? Font.DemiBold : Font.Normal
                }
            }
        }
        // time, bottom left
        Column {
            x: 56 * s; anchors.bottom: parent.bottom; anchors.bottomMargin: 52 * s
            Text { id: clock; text: core.clock(); color: c.text; font.pixelSize: 76 * s; font.weight: Font.DemiBold }
            Text { id: date; text: core.date(); color: c.text; font.pixelSize: 18 * s }
            Timer { interval: 10000; running: true; repeat: true; onTriggered: { clock.text = core.clock(); date.text = core.date() } }
        }
        // power and Wi-Fi, bottom right
        Rectangle {
            id: corner
            anchors.right: parent.right; anchors.rightMargin: 40 * s; anchors.bottom: parent.bottom; anchors.bottomMargin: 44 * s
            width: pr.implicitWidth + 12 * s; height: 56 * s; radius: 12 * s; color: Qt.alpha(c.surf, 0.88); border.color: c.line
            Row {
                id: pr; anchors.centerIn: parent; spacing: 4 * s
                Button {
                    id: wifiBtn
                    visible: wifi.available
                    width: visible ? 44 * s : 0; height: 44 * s
                    Accessible.name: "Wi-Fi"
                    ToolTip.visible: hovered; ToolTip.text: wifi.current ? "Wi-Fi: " + wifi.current : "Wi-Fi"
                    onClicked: { wifiPanel.visible = !wifiPanel.visible; if (wifiPanel.visible) wifi.refresh() }
                    background: Rectangle { radius: 9 * s; color: parent.hovered || wifiPanel.visible ? c.raised : "transparent" }
                    contentItem: Icon { name: "wifi"; tint: wifi.current ? c.accent : c.text; size: 19 * s; anchors.centerIn: parent }
                }
                Repeater {
                    model: [["restart", "Restart", "restart"], ["poweroff", "Power off", "power"]]
                    Button {
                        required property var modelData
                        width: 44 * s; height: 44 * s
                        Accessible.name: modelData[1]
                        ToolTip.visible: hovered; ToolTip.text: modelData[1]
                        onClicked: core.power(modelData[0])
                        background: Rectangle { radius: 9 * s; color: parent.hovered ? c.raised : "transparent" }
                        contentItem: Icon { name: modelData[2]; tint: c.text; size: 19 * s; anchors.centerIn: parent }
                    }
                }
            }
        }
        // Wi-Fi before logging in: compact, above its button
        Rectangle {
            id: wifiPanel
            visible: false
            anchors.right: corner.right; anchors.bottom: corner.top; anchors.bottomMargin: 8 * s
            width: 300 * s; height: wcol.implicitHeight + 20 * s
            radius: 12 * s; color: Qt.alpha(c.surf, 0.96); border.color: c.line
            property string pick: ""
            Component.onCompleted: if (openAtStart.indexOf("wifi") === 0) { visible = true; wifi.refresh(); pickLater.start() }
            Timer { id: pickLater; interval: 400; onTriggered: { var p = openAtStart.split(":")[1]; if (p) { for (var i = 0; i < wifi.nets.length; i++) if (wifi.nets[i].ssid === p) { wifiPanel.pick = p; wifiPanel.preset = wifi.nets[i].campus } } } }
            property string preset: ""
            property bool secure: false
            Keys.onEscapePressed: visible = false
            Column {
                id: wcol
                x: 10 * s; y: 10 * s; width: parent.width - 20 * s; spacing: 2 * s
                Row {
                    width: parent.width; height: 26 * s; spacing: 6 * s
                    Text { text: "Wi-Fi"; color: c.text; font.pixelSize: 13 * s; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: wifi.current ? "· " + wifi.current : ""; color: c.dim; font.pixelSize: 12 * s; elide: Text.ElideRight; width: parent.width - 110 * s; anchors.verticalCenter: parent.verticalCenter }
                    Button {
                        width: 40 * s; height: 24 * s; anchors.verticalCenter: parent.verticalCenter
                        Accessible.name: "Look again"
                        onClicked: wifi.refresh()
                        background: Rectangle { radius: 6 * s; color: parent.hovered ? c.raised : "transparent" }
                        contentItem: Text { text: "↻"; color: c.dim; font.pixelSize: 14 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                }
                Text { visible: wifi.nets.length === 0; text: wifi.busy ? "" : "Looking for networks…"; color: c.dim; font.pixelSize: 12 * s; height: visible ? 26 * s : 0; verticalAlignment: Text.AlignVCenter }
                Repeater {
                    model: wifi.nets
                    Column {
                        required property var modelData
                        width: wcol.width
                        Button {
                            width: parent.width; height: 28 * s
                            Accessible.name: modelData.ssid
                            onClicked: {
                                wifiPanel.pick = modelData.ssid; wifiPanel.preset = modelData.campus; wifiPanel.secure = modelData.secure
                                if (!modelData.secure && !modelData.campus) { wifi.join(modelData.ssid, ""); wifiPanel.pick = "" }
                                else (modelData.campus ? wuser : wpass).forceActiveFocus()
                            }
                            background: Rectangle { radius: 6 * s; color: parent.hovered || wifiPanel.pick === modelData.ssid ? c.raised : "transparent" }
                            contentItem: Row {
                                spacing: 6 * s; leftPadding: 6 * s
                                Row {
                                    spacing: 1.5 * s; anchors.verticalCenter: parent.verticalCenter
                                    Repeater {
                                        model: 3
                                        Rectangle { required property int index; width: 2.5 * s; height: (4 + 3 * index) * s; radius: 1; anchors.bottom: parent.bottom; color: modelData.signal >= [1, 40, 70][index] ? c.text : c.line }
                                    }
                                }
                                Text {
                                    text: modelData.ssid + (modelData.secure ? "  🔒" : "")
                                    width: wcol.width - 110 * s; elide: Text.ElideRight
                                    color: modelData.active ? c.accent : c.text; font.pixelSize: 12.5 * s; font.weight: modelData.active ? Font.DemiBold : Font.Normal
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                                Text { text: modelData.active ? "Connected" : (modelData.campus ? "Campus" : ""); color: c.dim; font.pixelSize: 11 * s; anchors.verticalCenter: parent.verticalCenter }
                            }
                        }
                        // sign-in for the picked network, inline
                        Column {
                            visible: wifiPanel.pick === modelData.ssid && (modelData.secure || modelData.campus)
                            width: parent.width; spacing: 4 * s; topPadding: 2 * s; bottomPadding: 6 * s
                            TextField {
                                id: wuser
                                visible: modelData.campus !== ""
                                width: parent.width; height: 30 * s; font.pixelSize: 12.5 * s
                                placeholderText: modelData.campus === "ucalgary" ? "UCalgary username" : "you@your-university"
                                color: c.text; placeholderTextColor: c.dim
                                background: Rectangle { radius: 7 * s; color: c.field || c.raised; border.color: wuser.activeFocus ? c.accent : c.line }
                                onAccepted: wpass.forceActiveFocus()
                            }
                            Row {
                                width: parent.width; spacing: 6 * s
                                TextField {
                                    id: wpass
                                    width: parent.width - 60 * s; height: 30 * s; font.pixelSize: 12.5 * s
                                    echoMode: TextInput.Password; placeholderText: "Password"
                                    color: c.text; placeholderTextColor: c.dim
                                    background: Rectangle { radius: 7 * s; color: c.field || c.raised; border.color: wpass.activeFocus ? c.accent : c.line }
                                    onAccepted: joinBtn.clicked()
                                }
                                Button {
                                    id: joinBtn
                                    width: 54 * s; height: 30 * s; enabled: !wifi.busy
                                    Accessible.name: "Join"
                                    onClicked: {
                                        var p = wpass.text; wpass.text = ""
                                        if (modelData.campus) { wifi.campus(modelData.campus, wuser.text, p) } else { wifi.join(modelData.ssid, p) }
                                    }
                                    background: Rectangle { radius: 7 * s; color: c.accent; opacity: parent.enabled ? 1 : 0.5 }
                                    contentItem: Text { text: "Join"; color: c.onAccent; font.pixelSize: 12 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                                }
                            }
                        }
                    }
                }
                Text {
                    visible: wifi.note !== ""
                    width: parent.width; wrapMode: Text.Wrap; topPadding: 4 * s
                    text: wifi.note; color: wifi.bad ? "#E0705A" : c.dim; font.pixelSize: 11.5 * s
                }
            }
        }
    }
}
