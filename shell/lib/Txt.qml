// Text in the theme's face. `jp: true` uses the Japanese face at heavy weight (kanji as marks).
import QtQuick

Text {
    required property var core
    property bool jp: false
    property bool heavy: false
    property bool faint: false
    color: faint ? core.dim : core.ink
    font.family: jp ? core.fontJp : core.font
    font.pixelSize: core.px
    font.weight: heavy ? (jp ? Font.Black : Font.Bold) : Font.Normal
    renderType: Text.NativeRendering
    elide: Text.ElideRight
    verticalAlignment: Text.AlignVCenter
}
