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
        // power, bottom right
        Rectangle {
            anchors.right: parent.right; anchors.rightMargin: 40 * s; anchors.bottom: parent.bottom; anchors.bottomMargin: 44 * s
            width: pr.implicitWidth + 12 * s; height: 56 * s; radius: 12 * s; color: Qt.alpha(c.surf, 0.88); border.color: c.line
            Row {
                id: pr; anchors.centerIn: parent; spacing: 4 * s
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
    }
}
