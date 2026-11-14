pragma Singleton
import QtQuick
QtObject {
    property var applications: ({ values: [{ id: "kitty", name: "Kitty", noDisplay: false }, { id: "firefox", name: "Firefox", noDisplay: false }] })
    function byId(id) { return null }
    function heuristicLookup(id) { return null }
}
