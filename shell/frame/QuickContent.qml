// Quick settings: grows down out of the status cells, sharing the bar's line and the frame's
// right line. Tiles, two sliders, notices.
import QtQuick
import "../lib"

Rectangle {
    id: quick
    required property var core
    color: core.ground
    rotation: core.tilt(6) * 0.5
    Rectangle { width: core.line; height: parent.height; color: core.ink }
    Rectangle { anchors.right: parent.right; width: core.line; height: parent.height; color: core.ink }
    Rectangle { anchors.bottom: parent.bottom; height: core.line; width: parent.width; color: core.ink }

    readonly property var net: core.status.network || ({})
    readonly property var tiles: [
        { jp: "通信", en: "WIFI", on: net.wifi_on === true, sub: net.wifi ? (core.incognito ? "????" : net.wifi.ssid) : (net.wifi_on ? "on" : "off"), cmd: ["nmcli", "radio", "wifi", net.wifi_on ? "off" : "on"] },
        { jp: "無線", en: "BT", on: core.status.bluetooth === true, sub: core.status.bluetooth ? "on" : "off", cmd: ["sh", "-c", "bluetoothctl show | grep -q 'Powered: yes' && bluetoothctl power off || bluetoothctl power on"] },
        { jp: "消音", en: "MUTE", on: core.status.muted === true, sub: core.status.muted ? "muted" : "sound on", cmd: ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"] },
        { jp: "夜", en: "NIGHT", on: core.v("displays.night.enabled") === true, sub: (core.v("displays.night.from") || "") + " – " + (core.v("displays.night.to") || ""), set: ["displays.night.enabled", !(core.v("displays.night.enabled") === true)] },
        { jp: "静", en: "QUIET", on: core.v("frame.notices.dnd") === true, sub: core.v("frame.notices.dnd") ? "no notices" : "notices on", set: ["frame.notices.dnd", !(core.v("frame.notices.dnd") === true)] },
        { jp: "地", en: "BASE", on: false, sub: core.white ? "白 WHITE" : "黒 BLACK", set: ["look.base", core.white ? "black" : "white"] },
        { jp: "隠", en: "INCOGNITO", on: core.incognito, sub: core.incognito ? "on" : "off", cmd: ["samurai", "incognito", "toggle"] },
        { jp: "撮", en: "SHOT", on: false, sub: "an area", cmd: ["samurai", "shot", "area"], close: true },
        { jp: "録", en: "RECORD", on: core.status.recording === true, sub: core.status.recording ? "recording" : "off", cmd: ["samurai", "record", "toggle"] }
    ]

    Column {
        anchors.fill: parent
        Row {
            leftPadding: 20; topPadding: 16; bottomPadding: 10; spacing: 12
            Txt { core: quick.core; text: "即"; jp: true; heavy: true; font.pixelSize: 44 }
            Txt { core: quick.core; text: "QUICK"; heavy: true; font.letterSpacing: 2; anchors.baseline: parent.children[0].baseline }
        }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Grid {
            columns: 3
            Repeater {
                model: quick.tiles
                Cell {
                    required property var modelData
                    required property int index
                    core: quick.core
                    width: Math.floor(quick.width / 3); height: 76; pad: 0; lineBottom: true; lineLeft: index % 3 !== 0
                    filled: modelData.on
                    Txt { core: quick.core; x: 10; y: 8; text: modelData.jp + " " + modelData.en; jp: true; heavy: true; color: parent.fg }
                    Txt { core: quick.core; x: 10; anchors.bottom: parent.bottom; anchors.bottomMargin: 8; width: parent.width - 20; text: modelData.sub; font.pixelSize: core.small; color: parent.fg; opacity: 0.75 }
                    onClicked: { if (modelData.set) core.setValue(modelData.set[0], modelData.set[1]); else core.run(modelData.cmd); if (modelData.close) core.panel = "" }
                }
            }
        }
        Repeater {
            model: [{ jp: "音", en: "VOL", val: core.status.volume, cmd: function(p) { return ["wpctl", "set-volume", "-l", "1", "@DEFAULT_AUDIO_SINK@", p + "%"] } },
                    { jp: "輝", en: "LUX", val: core.status.brightness, cmd: function(p) { return ["brightnessctl", "set", Math.max(1, p) + "%"] } }]
            Item {
                required property var modelData
                width: quick.width; height: core.hit
                Txt { core: quick.core; x: 20; width: 64; height: parent.height; text: modelData.jp + " " + modelData.en; jp: true; heavy: true }
                Rectangle {
                    id: track
                    x: 92; width: parent.width - 92 - 60; height: 16; anchors.verticalCenter: parent.verticalCenter
                    color: "transparent"; border.width: core.line; border.color: core.ink
                    Rectangle { width: parent.width * Math.max(0, Math.min(100, modelData.val || 0)) / 100; height: parent.height; color: core.ink }
                    MouseArea {
                        anchors.fill: parent; anchors.margins: -12
                        function put(mx) { core.run(modelData.cmd(Math.round(Math.max(0, Math.min(1, (mx - 12) / track.width)) * 100))) }
                        onPressed: function(m) { put(m.x) }
                        onPositionChanged: function(m) { if (pressed) put(m.x) }
                    }
                }
                Txt { core: quick.core; anchors.right: parent.right; anchors.rightMargin: 20; height: parent.height; text: modelData.val === undefined || modelData.val === null ? "—" : "" + modelData.val }
            }
        }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Txt { core: quick.core; leftPadding: 20; topPadding: 10; bottomPadding: 6; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; text: core.label("通知", "NOTICES") + (core.notices.length ? " · " + core.notices.length : "") }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Repeater {
            model: core.notices.slice(0, 3)
            Item {
                required property var modelData
                width: quick.width; height: body.implicitHeight + 20
                Rectangle { width: 8; height: parent.height; color: core.red }
                Column {
                    id: body
                    x: 24; y: 10; width: parent.width - 24 - core.hit - 8; spacing: 2
                    Txt { core: quick.core; width: parent.width; heavy: true; text: core.incognito && core.v("incognito.hide_notices") !== false ? "a notice" : (modelData.summary || modelData.app || "") }
                    Txt { core: quick.core; width: parent.width; faint: true; wrapMode: Text.Wrap; maximumLineCount: 2; visible: text !== ""; text: core.incognito && core.v("incognito.hide_notices") !== false ? "" : (modelData.body || "") }
                }
                Cell { core: quick.core; anchors.right: parent.right; width: core.hit; height: core.hit; pad: 0; text: "×"; onClicked: core.dismiss(modelData.id) }
                Rectangle { anchors.bottom: parent.bottom; height: core.line; width: parent.width; color: core.ink }
            }
        }
        Txt { core: quick.core; visible: core.notices.length === 0; leftPadding: 20; topPadding: 10; faint: true; text: "nothing new" }
    }
    Rectangle { anchors.bottom: parent.bottom; anchors.bottomMargin: core.hit + 4 + core.line; height: core.line; width: parent.width; color: core.ink }
    Row {
        anchors.bottom: parent.bottom; anchors.bottomMargin: core.line
        width: parent.width; height: core.hit + 4
        Txt { core: quick.core; leftPadding: 20; width: parent.width / 2; height: parent.height; text: core.status.battery ? "電 " + core.status.battery.percent + (core.status.battery.charging ? " · charging" : core.status.battery.ac ? " · plugged in" : " · on battery") : "電 AC" ; jp: false }
        Cell { core: quick.core; width: parent.width / 2 - core.line; height: parent.height; lineLeft: true; lineTop: true; kanji: "設定"; kanjiSize: core.px; text: "ALL SETTINGS"; onClicked: core.openHub("") }
    }
}
