pragma Singleton
import QtQuick
QtObject {
    property var screens: [{ name: "eDP-1" }]
    property var envs: ({})
    function env(k) { return __env[k] }
    function execDetached(cmd) { console.log("launch " + cmd) }
}
