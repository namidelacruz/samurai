// Settings › Apps: a line on what "make every app match" changes.
import QtQuick
import QtQuick.Layouts

Text {
    readonly property var c: core.colors
    readonly property real s: core.scale
    Layout.fillWidth: true
    wrapMode: Text.Wrap
    color: c.dim
    font.pixelSize: 13 * s
    text: "Each choice below lists the apps on this machine that can open that kind of file. Samurai sets it for every app at once (the same way a file manager and a browser ask)."
}
