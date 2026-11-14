import QtQuick
QtObject {
    property bool bodySupported; property bool actionsSupported; property bool imageSupported; property bool keepOnReload
    property var trackedNotifications: ({ values: [] })
    signal notification(var n)
}
