pragma Singleton
import QtQuick
// stand-in: user "nami" with password "katana" gets in
QtObject {
    id: g
    property string user; property var launchedWith: null
    signal authMessage(string message, bool error, bool responseRequired, bool echoResponse)
    signal authFailure(string message); signal readyToLaunch(); signal error(string error)
    function createSession(u) { user = u; authMessage("Password:", false, true, false) }
    function respond(r) { if (user === "nami" && r === "katana") readyToLaunch(); else authFailure("auth failed") }
    function launch(cmd) { launchedWith = cmd; console.log("greetd launch " + cmd) }
}
