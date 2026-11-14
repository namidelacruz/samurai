import QtQuick
// stand-in for PanelWindow and FloatingWindow: holds the same properties, shows nothing
Item {
    id: w
    width: 1600; height: 900
    property var screen
    property Margins margins: Margins {}
    property int exclusiveZone: 0
    property int exclusionMode: 0
    property bool aboveWindows: true
    property bool focusable: false
    property color color: "white"
    property var mask
    property string title: ""
    property size minimumSize
}
