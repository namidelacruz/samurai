// The lock face and the login face are one design: the frame closes. A red column on the left
// carries the kanji, the clock sits on the grid, one field takes the password.
// mode "lock": 施錠, the user is fixed. mode "login": 入, a user list and a session cell.
import QtQuick
import "../lib"

Rectangle {
    id: face
    required property var core
    property string mode: "lock"
    property string user: core.userName
    property var users: []
    property var sessions: []          // [{name, exec}]
    property int session: 0
    property string message: ""
    property bool failed: false
    property bool busy: false
    property bool primary: true        // only one screen shows the field
    property string wall: ""
    signal submit(string user, string secret)
    signal powerAsked(string what)
    function clear() { pw.text = ""; pw.input.forceActiveFocus() }
    color: core.ground
    onVisibleChanged: if (visible) clear()

    Image { anchors.fill: parent; source: face.wall; fillMode: Image.PreserveAspectCrop; asynchronous: true; cache: false; visible: face.wall !== "" && core.v("lock.face") !== "plain" }
    // the frame, closed: four lines
    Rectangle { x: core.inset; y: core.inset; width: parent.width - 2 * core.inset; height: parent.height - 2 * core.inset; color: "transparent"; border.width: core.line; border.color: core.ink }
    // red column
    Rectangle {
        id: col
        x: core.inset; y: core.inset; width: Math.round(parent.width * 0.16); height: parent.height - 2 * core.inset; color: core.red
        Column {
            anchors.horizontalCenter: parent.horizontalCenter; y: 40; spacing: 0
            Repeater { model: face.mode === "login" ? ["入", "城"] : ["施", "錠"]
                Txt { required property var modelData; core: face.core; text: modelData; jp: true; heavy: true; color: core.redInk; font.pixelSize: Math.round(col.width * 0.5) } }
        }
        Txt { core: face.core; anchors.bottom: parent.bottom; anchors.bottomMargin: 24; anchors.horizontalCenter: parent.horizontalCenter; color: core.redInk; heavy: true; font.letterSpacing: 4; text: face.mode === "login" ? "ENTER" : "LOCKED" }
    }
    // clock block
    Column {
        anchors.left: col.right; anchors.leftMargin: 48; y: core.inset + 40
        Txt { core: face.core; text: core.clockText; heavy: true; font.pixelSize: Math.round(face.height * 0.16) }
        Txt { core: face.core; text: core.dateText; jp: true; font.pixelSize: core.px + 8 }
    }
    // the box: user, field, message
    Column {
        visible: face.primary
        anchors.left: col.right; anchors.leftMargin: 48; anchors.bottom: parent.bottom; anchors.bottomMargin: core.inset + 96
        width: 460; spacing: 0
        Row {
            visible: face.mode === "login" && face.users.length > 1 && core.v("greeter.show_users") !== false
            Repeater { model: face.users
                Cell { required property var modelData; core: face.core; box: true; text: modelData; filled: face.user === modelData; onClicked: { face.user = modelData; face.clear() } } }
        }
        Field {
            core: face.core; width: parent.width; visible: face.mode === "login" && !(face.users.length > 1 && core.v("greeter.show_users") !== false)
            placeholder: "user"; text: face.user; onEdited: function(t) { face.user = t }
            input.onAccepted: pw.input.forceActiveFocus()
        }
        Txt { core: face.core; visible: face.mode === "lock" && core.v("lock.show_user") !== false; bottomPadding: 8; heavy: true; text: face.user }
        Row {
            Field { id: pw; core: face.core; width: 460 - core.hit - 24; secret: true; placeholder: face.busy ? "checking…" : "password"
                    input.enabled: !face.busy
                    onCommit: function(t) { if (t !== "" && !face.busy && input.activeFocus) { face.submit(face.user, t); pw.text = "" } } }
            Cell { core: face.core; box: true; filled: true; width: core.hit + 24; kanji: "開"; enabled: !face.busy; onClicked: { if (pw.text !== "") { face.submit(face.user, pw.text); pw.text = "" } } }
        }
        Rectangle { width: parent.width; height: core.hit; color: face.failed ? core.red : "transparent"; visible: face.message !== ""
            Txt { core: face.core; x: 10; height: parent.height; width: parent.width - 20; color: face.failed ? core.redInk : core.ink; heavy: true; text: face.message } }
    }
    // foot: session (login), power, mark
    Row {
        anchors.right: parent.right; anchors.rightMargin: core.inset; anchors.bottom: parent.bottom; anchors.bottomMargin: core.inset
        Cell { core: face.core; visible: face.mode === "login" && face.sessions.length > 0; lineLeft: true; lineTop: true; kanji: "座"; kanjiSize: core.px
               text: face.sessions.length ? ("" + face.sessions[face.session].name).toUpperCase() : ""; onClicked: face.session = (face.session + 1) % face.sessions.length }
        Repeater { model: face.primary ? [["休止", "sleep"], ["再起", "restart"], ["電源", "shutdown"]] : []
            Cell { required property var modelData; core: face.core; lineLeft: true; lineTop: true; kanji: modelData[0]; kanjiSize: core.px; onClicked: face.powerAsked(modelData[1]) } }
    }
    Txt { core: face.core; anchors.left: col.right; anchors.leftMargin: 48; anchors.bottom: parent.bottom; anchors.bottomMargin: core.inset + 14; faint: true; font.pixelSize: core.small; font.letterSpacing: 2
          text: core.mark ? "一期一会  " + core.mark : "SAMURAI" }
}
