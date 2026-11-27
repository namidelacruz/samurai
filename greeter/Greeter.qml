// The Samurai login screen, in the Sakura look: the ink on the left with the time, the sign-in
// on the right (like the Shadow Sakura website's sign-in page). The lock uses the same face.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Window
import "../settings/qml"

Window {
    id: w
    readonly property var c: core.colors
    readonly property real s: core.scale
    property int user: 0
    property int session: core.lastSession
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

    function signIn() {
        w.msg = ""; w.bad = false
        if (w.question !== "") { var a = pw.text; pw.text = ""; core.answer(a) }
        else { var p = pw.text; pw.text = ""; core.login(core.users.length ? core.users[w.user] : name.text, p, w.session) }
    }

    // the ink, over every screen
    Image { anchors.fill: parent; source: core.wall; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: source !== "" }

    // everything else on the main screen
    Item {
        id: mon
        x: Screen.virtualX; y: Screen.virtualY; width: Screen.width; height: Screen.height
        // narrow or upright screens get the sign-in alone
        readonly property bool wide: width >= 980 * s
        readonly property real side: wide ? Math.min(520 * s, width * 0.42) : width

        // ── left: the mark and the time ────────────────────────────────────────────────
        Row {
            visible: mon.wide
            x: 40 * s; y: 36 * s; spacing: 12 * s
            Rectangle {
                width: 40 * s; height: width; radius: width / 2; color: c.accent
                Text { anchors.centerIn: parent; text: "侍"; color: c.onAccent; font.pixelSize: 18 * s; font.weight: Font.DemiBold }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Text { text: "Samurai"; color: c.text; font.pixelSize: 17 * s; font.weight: Font.DemiBold }
                Text { text: "サムライ"; color: c.dim; font.pixelSize: 11 * s; font.letterSpacing: 2 * s }
            }
        }
        Rectangle {
            visible: mon.wide
            x: 40 * s; anchors.bottom: parent.bottom; anchors.bottomMargin: 40 * s
            width: Math.max(timeCol.implicitWidth + 52 * s, 360 * s); height: timeCol.implicitHeight + 44 * s
            radius: 14 * s; color: Qt.alpha(c.chrome, 0.94); border.color: c.line
            Column {
                id: timeCol
                x: 26 * s; anchors.verticalCenter: parent.verticalCenter; spacing: 4 * s
                Text { id: clock; text: core.clock(); color: c.text; font.pixelSize: 64 * s; font.weight: Font.DemiBold; font.letterSpacing: -1 * s }
                Text { id: date; text: core.date(); color: c.dim; font.pixelSize: 15 * s }
                Text { visible: wifi.available; text: wifi.current ? "Wi-Fi · " + wifi.current : "Wi-Fi · not connected"; color: c.dim; font.pixelSize: 13 * s }
            }
            Timer { interval: 10000; running: true; repeat: true; onTriggered: { clock.text = core.clock(); date.text = core.date() } }
        }

        // ── right: signing in ──────────────────────────────────────────────────────────
        Rectangle {
            id: panel
            anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom
            width: mon.side
            color: mon.wide ? Qt.alpha(c.surf, 0.97) : Qt.alpha(c.ground, 0.9)
            Rectangle { visible: mon.wide; width: 1; height: parent.height; color: c.line }

            Item {
                id: card
                width: Math.min(400 * s, parent.width - 64 * s); height: col.implicitHeight
                x: (parent.width - width) / 2
                y: Math.max(24 * s, (parent.height - height) / 2 - 40 * s)
                SequentialAnimation {
                    id: shake
                    NumberAnimation { target: card; property: "x"; to: card.x - 10 * s; duration: 50 }
                    NumberAnimation { target: card; property: "x"; to: card.x + 10 * s; duration: 60 }
                    NumberAnimation { target: card; property: "x"; to: (panel.width - card.width) / 2; duration: 50 }
                }
                Column {
                    id: col
                    width: parent.width
                    spacing: 18 * s
                    // who
                    Row {
                        spacing: 16 * s
                        Rectangle {
                            width: 64 * s; height: width; radius: width / 2; color: c.soft
                            Text { anchors.centerIn: parent; text: (core.users[w.user] || name.text || "?").charAt(0).toUpperCase(); color: c.accent; font.pixelSize: 26 * s; font.weight: Font.DemiBold }
                        }
                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2 * s
                            Row {
                                spacing: 6 * s
                                Text { visible: core.users.length > 0; text: core.users[w.user] || ""; color: c.text; font.pixelSize: 26 * s; font.weight: Font.DemiBold }
                                TextInput {
                                    id: name
                                    visible: core.users.length === 0
                                    width: 220 * s; color: c.text; font.pixelSize: 24 * s; font.weight: Font.DemiBold
                                    Accessible.name: "User name"
                                    onAccepted: pw.forceActiveFocus()
                                    Text { anchors.fill: parent; visible: !parent.text; text: "user name"; color: c.dim; font: parent.font }
                                }
                            }
                            Text { text: "おかえりなさい · Welcome back"; color: c.dim; font.pixelSize: 13 * s }
                        }
                    }
                    Row {
                        visible: core.users.length > 1
                        spacing: 6 * s
                        Repeater {
                            model: [["‹  Previous", -1], ["Next  ›", 1]]
                            Button {
                                required property var modelData
                                height: 36 * s; width: implicitContentWidth + 28 * s
                                Accessible.name: modelData[1] < 0 ? "Previous user" : "Next user"
                                onClicked: w.user = (w.user + core.users.length + modelData[1]) % core.users.length
                                background: Rectangle { radius: height / 2; color: parent.hovered ? c.raised : "transparent"; border.color: c.line }
                                contentItem: Text { text: modelData[0]; color: c.text; font.pixelSize: 13 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                            }
                        }
                    }
                    // the password (or what the system asks)
                    Column {
                        width: parent.width; spacing: 6 * s
                        Text { text: w.question !== "" ? w.question : "パスワード Password"; color: c.text; font.pixelSize: 13 * s; font.weight: Font.DemiBold; elide: Text.ElideRight; width: parent.width }
                        Field {
                            id: pw
                            width: parent.width; height: 52 * s
                            echoMode: w.question !== "" && !w.questionSecret ? TextInput.Normal : TextInput.Password
                            placeholderText: core.busy ? "Checking…" : ""
                            focus: true
                            enabled: !core.busy
                            font.pixelSize: 16 * s
                            leftPadding: 16 * s
                            Accessible.name: "Password"
                            Component.onCompleted: forceActiveFocus()
                            onAccepted: w.signIn()
                            background: Rectangle { radius: 10 * s; color: c.chrome; border.color: w.bad ? "#C8503F" : c.accent; border.width: 2 }
                        }
                    }
                    Button {
                        id: go
                        width: parent.width; height: 52 * s
                        Accessible.name: "Sign in"
                        enabled: !core.busy
                        onClicked: w.signIn()
                        background: Rectangle { radius: height / 2; color: c.accent; opacity: parent.enabled ? (parent.hovered ? 0.92 : 1) : 0.6 }
                        contentItem: Text { text: core.busy ? "確認中 Checking…" : "ログイン Sign in"; color: c.onAccent; font.pixelSize: 15 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                    Text {
                        width: parent.width; wrapMode: Text.Wrap
                        text: w.msg || "Type your password and press Enter, or touch your security key."
                        color: w.bad ? c.text : c.dim; font.pixelSize: 13 * s; font.weight: w.bad ? Font.DemiBold : Font.Normal
                    }
                    // which desktop: pills, the one picked last is picked
                    Column {
                        visible: core.sessions.length > 1
                        width: parent.width; spacing: 8 * s
                        Text { text: "セッション Session"; color: c.text; font.pixelSize: 13 * s; font.weight: Font.DemiBold }
                        Rectangle {
                            width: parent.width; height: sessFlow.implicitHeight + 8 * s
                            radius: Math.min(height / 2, 28 * s); color: c.chrome; border.color: c.line
                            Flow {
                                id: sessFlow
                                x: 4 * s; y: 4 * s; width: parent.width - 8 * s; spacing: 4 * s
                                Repeater {
                                    model: core.sessions
                                    Button {
                                        required property var modelData
                                        required property int index
                                        readonly property bool on: w.session === index
                                        height: 44 * s
                                        width: core.sessions.length <= 2 ? (sessFlow.width - 4 * s) / 2 : implicitContentWidth + 32 * s
                                        Accessible.name: "Session: " + modelData
                                        Accessible.checked: on
                                        onClicked: w.session = index
                                        background: Rectangle { radius: height / 2; color: parent.on ? c.accent : (parent.hovered ? c.raised : "transparent") }
                                        contentItem: Text { text: modelData; color: parent.on ? c.onAccent : c.text; font.pixelSize: 14 * s; font.weight: Font.DemiBold; elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // power and Wi-Fi: words in pills, at the foot
            Row {
                id: corner
                anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; anchors.bottomMargin: 32 * s
                spacing: 8 * s
                Button {
                    id: wifiBtn
                    visible: wifi.available
                    height: 44 * s; width: visible ? implicitContentWidth + 32 * s : 0
                    Accessible.name: "Wi-Fi"
                    onClicked: { wifiPanel.visible = !wifiPanel.visible; if (wifiPanel.visible) wifi.refresh() }
                    background: Rectangle { radius: height / 2; color: parent.hovered || wifiPanel.visible ? c.raised : "transparent"; border.color: c.line }
                    contentItem: Text { text: "Wi-Fi"; color: wifi.current ? c.accent : c.text; font.pixelSize: 14 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                Repeater {
                    model: [["restart", "Restart 再起動"], ["poweroff", "Power off 電源"]]
                    Button {
                        required property var modelData
                        height: 44 * s; width: implicitContentWidth + 32 * s
                        Accessible.name: modelData[1]
                        onClicked: core.power(modelData[0])
                        background: Rectangle { radius: height / 2; color: parent.hovered ? c.raised : "transparent"; border.color: c.line }
                        contentItem: Text { text: modelData[1]; color: c.text; font.pixelSize: 14 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                    }
                }
            }
        // Wi-Fi before logging in: compact, above its button
        Rectangle {
            id: wifiPanel
            visible: false
            anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: corner.top; anchors.bottomMargin: 10 * s
            width: 300 * s; height: wcol.implicitHeight + 20 * s
            radius: 14 * s; color: c.raised; border.color: c.line
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
}
