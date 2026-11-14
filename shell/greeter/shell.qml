// The Samurai login screen: the lock face in "login" mode, talking to greetd.
// Runs as the greeter user inside a bare niri started by greetd (see samurai-greeter-setup).
// It reads one small file written by the installer: /etc/samurai/greeter.json
//   { "base": "black", "users": ["nami"], "sessions": [{ "name": "Samurai", "exec": "samurai-session" }], "wall": "/usr/share/samurai/greeter-black.svg" }
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Greetd
import "lib"
import "lock"

ShellRoot {
    id: root
    property var conf: ({})
    property string secret: ""
    property string said: ""
    property bool bad: false
    property bool busy: false
    property var launchCmd: ["samurai-session"]

    CoreBase {
        id: kernel
        status: ({ base: root.conf.base || "black", user: (root.conf.users || [""])[0], host: "" })
        values: ({ "greeter.show_users": root.conf.show_users !== false })
    }
    FileView {
        path: "/etc/samurai/greeter.json"; printErrors: false
        onLoaded: { try { root.conf = JSON.parse(text()) } catch (e) {} }
    }
    Connections {
        target: Greetd
        function onAuthMessage(message, error, responseRequired, echoResponse) {
            if (responseRequired) { Greetd.respond(root.secret); root.secret = "" } else { root.said = message; root.bad = error; Greetd.respond("") }
        }
        function onAuthFailure(message) { root.busy = false; root.bad = true; root.said = "wrong user or password · 違います"; root.secret = "" }
        function onReadyToLaunch() { Greetd.launch(root.launchCmd) }
        function onError(error) { root.busy = false; root.bad = true; root.said = error }
    }
    Process { id: sys }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            id: win
            required property var modelData
            screen: modelData
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "samurai-greeter"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            color: kernel.ground
            LockContent {
                anchors.fill: parent; core: kernel; mode: "login"
                users: root.conf.users || []
                sessions: root.conf.sessions || []
                wall: root.conf.wall ? "file://" + root.conf.wall : ""
                primary: Quickshell.screens[0] === win.modelData
                busy: root.busy; message: root.said; failed: root.bad
                onSubmit: function(user, secret) {
                    if (root.busy || user === "") return
                    var s = sessions.length ? sessions[session] : null
                    root.launchCmd = s ? String(s.exec).split(" ") : ["samurai-session"]
                    root.busy = true; root.bad = false; root.said = ""; root.secret = secret
                    Greetd.createSession(user)
                }
                onPowerAsked: function(what) { sys.command = ["systemctl", what === "sleep" ? "suspend" : what === "restart" ? "reboot" : "poweroff"]; sys.running = true }
            }
        }
    }
}
