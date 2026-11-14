import QtQuick
// stand-in: the password "katana" is right, anything else is wrong
QtObject {
    id: p
    property bool active: false; property string message: ""; property bool responseRequired: false; property string user
    signal pamMessage(); signal completed(int result)
    function start() { active = true; responseRequired = true; pamMessage(); return true }
    function respond(r) { responseRequired = false; active = false; completed(r === "katana" ? 0 : 1) }
    function abort() { active = false }
}
