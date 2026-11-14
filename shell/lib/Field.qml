// A one-line text box: a 1px box that thickens on focus. Commits on Enter and on leaving.
import QtQuick

Rectangle {
    id: f
    required property var core
    property alias text: input.text
    property string placeholder: ""
    property bool secret: false
    property alias input: input
    signal commit(string text)
    signal edited(string text)
    implicitHeight: core.hit; implicitWidth: 220
    color: core.ground; border.width: input.activeFocus ? 2 : core.line; border.color: core.ink
    TextInput {
        id: input
        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
        verticalAlignment: TextInput.AlignVCenter; clip: true; selectByMouse: true
        color: f.core.ink; selectionColor: f.core.red; selectedTextColor: f.core.redInk
        font.family: f.core.font; font.pixelSize: f.core.px
        echoMode: f.secret ? TextInput.Password : TextInput.Normal
        onAccepted: f.commit(text)
        onEditingFinished: f.commit(text)
        onTextEdited: f.edited(text)
    }
    Txt { core: f.core; visible: input.text === "" && !input.activeFocus; x: 10; height: parent.height; faint: true; text: f.placeholder; width: parent.width - 20 }
}
