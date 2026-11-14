// Samurai: the frame is the desktop.
// One Core, and per screen: the wall (seal + frame lines), the bar, the rail, and one panel
// layer that Start / Quick / Widgets / Power / Keys grow into. Plus the hub window, notice
// slips and the lock. Every surface is a thin window around a pure-QtQuick *Content file.
//@ pragma UseQApplication
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import Quickshell.Services.Pam
import "lib"
import "frame"
import "hub"
import "lock"

ShellRoot {
    id: root
    Core { id: kernel }

    Variants {
        model: Quickshell.screens
        Scope {
            id: scr
            required property var modelData
            readonly property bool here: Hyprland.focusedMonitor ? Hyprland.focusedMonitor.name === modelData.name : true

            // ── wall: under everything ──
            PanelWindow {
                screen: scr.modelData
                WlrLayershell.layer: WlrLayer.Background
                WlrLayershell.namespace: "samurai-wall"
                exclusionMode: ExclusionMode.Ignore
                anchors { top: true; bottom: true; left: true; right: true }
                color: kernel.ground
                WallContent { anchors.fill: parent; core: kernel; sealDir: kernel.sealDir; frameLeft: kernel.railW; frameTop: kernel.barH }
                // desktop clock, if asked for
                Txt {
                    core: kernel; visible: kernel.v("widgets.clock.enabled") === true
                    readonly property string place: kernel.v("widgets.clock.place") || "bottom-left"
                    x: place === "center" ? (parent.width - width) / 2 : place.indexOf("left") >= 0 ? kernel.railW + 40 : parent.width - width - kernel.inset - 40
                    y: place === "center" ? (parent.height - height) / 2 : place.indexOf("top") >= 0 ? kernel.barH + 32 : parent.height - height - kernel.inset - 56
                    heavy: true; font.pixelSize: kernel.v("widgets.clock.size") || 96
                    text: kernel.v("widgets.clock.face") === "kanji" ? kernel.kanjiNumber(kernel.now.getHours()) + "時" + kernel.kanjiNumber(kernel.now.getMinutes()) + "分" : kernel.clockText
                    jp: kernel.v("widgets.clock.face") === "kanji"
                }
            }
            // ── two invisible windows hold the space, so the order they appear in never matters ──
            PanelWindow { screen: scr.modelData; WlrLayershell.namespace: "samurai-hold"; anchors { top: true; left: true; right: true } implicitHeight: 1; exclusiveZone: kernel.barH; color: "transparent"; mask: Region {} }
            PanelWindow { screen: scr.modelData; WlrLayershell.namespace: "samurai-hold"; anchors { top: true; left: true; bottom: true } implicitWidth: 1; exclusiveZone: kernel.railW; color: "transparent"; mask: Region {} }
            // ── bar ──
            PanelWindow {
                screen: scr.modelData
                WlrLayershell.layer: WlrLayer.Top
                WlrLayershell.namespace: "samurai-bar"
                exclusionMode: ExclusionMode.Ignore
                anchors { top: true; left: true; right: true }
                implicitHeight: kernel.barH
                color: kernel.ground
                BarContent { anchors.fill: parent; core: kernel; screenName: scr.modelData.name }
            }
            // ── rail ──
            PanelWindow {
                screen: scr.modelData
                WlrLayershell.layer: WlrLayer.Top
                WlrLayershell.namespace: "samurai-rail"
                exclusionMode: ExclusionMode.Ignore
                anchors { top: true; left: true; bottom: true }
                margins.top: kernel.barH
                implicitWidth: kernel.railW
                color: kernel.ground
                RailContent { anchors.fill: parent; core: kernel }
            }
            // ── panels: one layer, shown on the focused screen while a panel is open ──
            PanelWindow {
                id: layer
                screen: scr.modelData
                visible: kernel.panel !== "" && scr.here
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.namespace: "samurai-panel"
                WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
                exclusionMode: ExclusionMode.Ignore
                anchors { top: true; bottom: true; left: true; right: true }
                color: "transparent"
                MouseArea { anchors.fill: parent; onClicked: kernel.panel = "" }
                Item { anchors.fill: parent; focus: true; Keys.onEscapePressed: kernel.panel = "" }
                StartContent {
                    id: start
                    core: kernel; visible: kernel.panel === "start"
                    x: kernel.railW; y: kernel.barH
                    width: Math.min(parent.width - kernel.railW, kernel.v("start.width") || 560); height: Math.min(parent.height - kernel.barH - kernel.inset, 620)
                    onVisibleChanged: if (visible) reset()
                    MouseArea { anchors.fill: parent; z: -1 }
                }
                QuickContent {
                    core: kernel; visible: kernel.panel === "quick"
                    x: parent.width - width - kernel.inset; y: kernel.barH
                    width: 420; height: Math.min(parent.height - kernel.barH - kernel.inset, 640)
                    MouseArea { anchors.fill: parent; z: -1 }
                }
                SideContent {
                    core: kernel; mode: kernel.panel; binds: kernel.binds
                    visible: ["widgets", "power", "keys"].indexOf(kernel.panel) >= 0
                    width: kernel.panel === "keys" ? 640 : kernel.panel === "power" ? 280 : 380
                    height: kernel.panel === "power" ? 5 * (kernel.hit + 12) + 82 : Math.min(parent.height - kernel.barH - kernel.inset, kernel.panel === "keys" ? 760 : 560)
                    x: kernel.panel === "power" ? kernel.railW : kernel.panel === "keys" ? (parent.width - width) / 2 : parent.width - width - kernel.inset
                    y: kernel.panel === "power" ? parent.height - height - kernel.inset : kernel.barH
                    MouseArea { anchors.fill: parent; z: -1 }
                }
            }
            // ── notice slips ──
            PanelWindow {
                screen: scr.modelData
                visible: kernel.popups.length > 0 && scr.here && !kernel.locked
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.namespace: "samurai-notice"
                exclusionMode: ExclusionMode.Ignore
                readonly property string corner: kernel.v("frame.notices.corner") || "bar-right"
                anchors { top: corner !== "bottom-right"; bottom: corner === "bottom-right"; right: corner !== "bar-left"; left: corner === "bar-left" }
                margins { top: kernel.barH; right: kernel.inset; left: kernel.railW; bottom: kernel.inset }
                implicitWidth: 380; implicitHeight: slips.implicitHeight
                color: "transparent"
                Column {
                    id: slips
                    width: parent.width
                    Repeater {
                        model: kernel.notices.filter(function(n) { return kernel.popups.indexOf(n.id) >= 0 }).slice(0, 4)
                        Rectangle {
                            required property var modelData
                            width: slips.width; height: txt.implicitHeight + 20; color: kernel.ground; border.width: kernel.line; border.color: kernel.ink
                            Rectangle { width: 8; height: parent.height; color: kernel.red }
                            Column { id: txt; x: 20; y: 10; width: parent.width - 32
                                Txt { core: kernel; width: parent.width; heavy: true; text: kernel.incognito && kernel.v("incognito.hide_notices") !== false ? "a notice" : (modelData.summary || modelData.app) }
                                Txt { core: kernel; width: parent.width; wrapMode: Text.Wrap; maximumLineCount: 3; visible: text !== ""; text: kernel.incognito && kernel.v("incognito.hide_notices") !== false ? "" : modelData.body } }
                            MouseArea { anchors.fill: parent; onClicked: kernel.popups = kernel.popups.filter(function(x) { return x !== modelData.id }) }
                        }
                    }
                }
            }
        }
    }

    // ── the hub: an ordinary window, so it tiles, floats and minimizes like any other ──
    FloatingWindow {
        visible: kernel.hubOpen
        title: "Samurai Settings"
        implicitWidth: 1080; implicitHeight: 720
        minimumSize: Qt.size(820, 520)
        color: kernel.ground
        onVisibleChanged: if (!visible && kernel.hubOpen) kernel.hubOpen = false
        HubContent { anchors.fill: parent; core: kernel }
    }

    // ── lock ──
    PamContext {
        id: pam
        property string secret: ""
        property string said: ""
        property bool bad: false
        onPamMessage: { if (responseRequired) { respond(secret); secret = "" } else if (message) said = message }
        onCompleted: function(result) {
            secret = ""
            if (result === PamResult.Success) { bad = false; said = ""; kernel.locked = false }
            else { bad = true; said = result === PamResult.MaxTries ? "too many tries" : "wrong password · 違います" }
        }
    }
    WlSessionLock {
        locked: kernel.locked
        WlSessionLockSurface {
            id: surf
            color: kernel.ground
            LockContent {
                anchors.fill: parent; core: kernel; mode: "lock"
                user: kernel.status.user || ""
                primary: true
                wall: kernel.incognito || !kernel.sealDir ? "" : "file://" + kernel.sealDir + "/wall-" + kernel.base + ".svg"
                busy: pam.active; message: pam.said; failed: pam.bad
                onSubmit: function(user, secret) { if (pam.active) return; pam.bad = false; pam.said = ""; pam.secret = secret; pam.start() }
                onPowerAsked: function(what) { if (what === "sleep") kernel.run(["systemctl", "suspend"]); else kernel.power(what) }
            }
        }
    }

    // ── idle: lock, screens off, sleep (minutes; 0 = never) ──
    IdleMonitor { timeout: (kernel.v("idle.lock") || 0) * 60; enabled: (kernel.v("idle.lock") || 0) > 0; onIsIdleChanged: if (isIdle) kernel.locked = true }
    IdleMonitor { timeout: (kernel.v("idle.screen_off") || 0) * 60; enabled: (kernel.v("idle.screen_off") || 0) > 0
                  onIsIdleChanged: kernel.run(["hyprctl", "dispatch", "hl.dsp.dpms({ action = \"" + (isIdle ? "off" : "on") + "\" })"]) }
    IdleMonitor { timeout: (kernel.v("idle.suspend") || 0) * 60; enabled: (kernel.v("idle.suspend") || 0) > 0; onIsIdleChanged: if (isIdle) kernel.power("sleep") }
}
