// The hand-made part of a page, shown above its generated settings. Chosen by `page`.
import QtQuick
import "../lib"

Column {
    id: c
    required property var core
    property string page: ""
    property string out: ""            // what the last command said
    property bool busy: false
    width: parent ? parent.width : 600
    function sh(cmd, input, after) { busy = true; out = "…"; core.run(cmd, function(o, code) { busy = false; out = o.trim() || (code === 0 ? "done" : "failed (" + code + ")"); if (after) after(o, code) }, input) }
    onPageChanged: { out = ""; if (page === "connect") scan(); if (page === "displays") mons(); if (page === "look") seal() }
    Component.onCompleted: pageChanged()

    // ═════ CONNECT ═════ the campus setups are fixed here and cannot be turned off
    property var nets: []
    property string pick: ""
    function scan() { core.run(["nmcli", "-t", "-f", "IN-USE,SIGNAL,SECURITY,SSID", "dev", "wifi", "list"], function(o) {
        var seen = {}, list = []
        o.split("\n").forEach(function(l) { var p = l.split(":"); if (p.length < 4) return; var ssid = p.slice(3).join(":").replace(/\\:/g, ":"); if (!ssid || seen[ssid]) return; seen[ssid] = 1
            list.push({ on: p[0] === "*", signal: Number(p[1]) || 0, sec: p[2] || "open", ssid: ssid }) })
        list.sort(function(a, b) { return (b.on - a.on) || (b.signal - a.signal) }); nets = list }) }
    Column {
        visible: c.page === "connect"; width: parent.width
        Row {
            leftPadding: 20; topPadding: 12; bottomPadding: 12; spacing: 0
            Cell { core: c.core; box: true; kanji: "探"; kanjiSize: core.px; text: "SCAN"; onClicked: c.scan() }
            Cell { core: c.core; box: true; kanji: "修"; kanjiSize: core.px; text: "REPAIR THIS CONNECTION"; onClicked: c.sh(["shadow", "wifi", "repair"], "", c.scan) }
            Cell { core: c.core; box: true; kanji: "診"; kanjiSize: core.px; text: "DEVICE CHECK"; onClicked: c.sh(["shadow", "wifi", "device"]) }
            Cell { core: c.core; box: true; kanji: "録"; kanjiSize: core.px; text: "EAP / TLS LOG"; visible: core.v("net.show_log") !== false; onClicked: c.sh(["shadow", "wifi", "log"]) }
        }
        // campus: two fixed cards
        Row {
            x: 20; width: parent.width - 40; spacing: 12
            Repeater {
                model: [{ id: "ucalgary", jp: "学", title: "UC-SECURE + AIRUC-SECURE", sub: "UCalgary · sets up both networks, pins the campus certificate, repairs on failure", realm: false },
                        { id: "eduroam", jp: "遊学", title: "EDUROAM", sub: "user@realm · same certificate checks and repairs", realm: true }]
                Rectangle {
                    required property var modelData
                    width: (parent.width - 12) / 2; height: card.implicitHeight + 24
                    color: "transparent"; border.width: core.line; border.color: core.ink
                    Rectangle { width: 8; height: parent.height; color: core.red }
                    Column {
                        id: card
                        x: 20; y: 12; width: parent.width - 32; spacing: 8
                        Row { spacing: 10
                            Txt { core: c.core; text: modelData.jp; jp: true; heavy: true; font.pixelSize: 26 }
                            Txt { core: c.core; text: modelData.title; heavy: true; anchors.verticalCenter: parent.verticalCenter } }
                        Txt { core: c.core; width: parent.width; faint: true; wrapMode: Text.Wrap; font.pixelSize: core.small; text: modelData.sub }
                        Field { id: user; core: c.core; width: parent.width; placeholder: modelData.realm ? "username@ucalgary.ca" : "username" }
                        Field { id: pass; core: c.core; width: parent.width; secret: true; placeholder: "password (goes to the keyring, never to a file)" }
                        Cell { core: c.core; box: true; filled: true; enabled: !c.busy && user.text !== "" && pass.text !== ""; kanji: "繋"; kanjiSize: core.px; text: "SET UP AND CONNECT"
                            onClicked: { c.sh(["shadow", "wifi", "campus", modelData.id, "--user", user.text, "--password-stdin"], pass.text, c.scan); pass.text = "" } }
                    }
                }
            }
        }
        Item { width: 1; height: 12 }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Repeater {
            model: c.nets
            Column {
                required property var modelData
                width: c.width
                Cell {
                    core: c.core; width: parent.width; height: core.hit; pad: 0; filled: modelData.on
                    Txt { core: c.core; x: 20; width: 60; height: parent.height; color: parent.fg; text: "▂▄▆█".slice(0, Math.max(1, Math.min(4, Math.floor(modelData.signal / 25) + 1))) }
                    Txt { core: c.core; x: 84; width: parent.width - 300; height: parent.height; color: parent.fg; heavy: true; text: modelData.ssid }
                    Txt { core: c.core; anchors.right: parent.right; anchors.rightMargin: 20; height: parent.height; color: parent.fg; text: (modelData.on ? "connected · " : "") + modelData.sec + " · " + modelData.signal }
                    onClicked: c.pick = (c.pick === modelData.ssid ? "" : modelData.ssid)
                }
                Row {
                    visible: c.pick === modelData.ssid; leftPadding: 84; bottomPadding: 8; spacing: 0
                    Field { id: pw; core: c.core; width: 300; secret: true; placeholder: "password (empty if saved or open)" }
                    Cell { core: c.core; box: true; text: "CONNECT"; onClicked: { if (pw.text !== "") c.sh(["shadow", "wifi", "connect", modelData.ssid, "--password-stdin"], pw.text, c.scan); else c.sh(["shadow", "wifi", "connect", modelData.ssid], "", c.scan); pw.text = "" } }
                    Cell { core: c.core; box: true; text: "FORGET"; onClicked: c.sh(["shadow", "wifi", "forget", modelData.ssid], "", c.scan) }
                }
                Rectangle { width: parent.width; height: core.line; color: core.dim; opacity: 0.5 }
            }
        }
        Txt { core: c.core; visible: c.nets.length === 0; leftPadding: 20; topPadding: 10; bottomPadding: 10; faint: true; text: "no networks seen · press SCAN" }
    }

    // ═════ KEYBINDS ═════ every bind, where it comes from, and an editor that refuses clashes
    property var check: null
    Column {
        visible: c.page === "keybinds"; width: parent.width
        Txt { core: c.core; leftPadding: 20; topPadding: 12; bottomPadding: 6; heavy: true; text: core.label("新", "NEW BIND") }
        Row {
            leftPadding: 20; spacing: 0
            Field { id: bk; core: c.core; width: 210; placeholder: "SUPER + SHIFT + T"
                onEdited: function(t) { if (t.trim() === "") { c.check = null; return } core.run(["samurai", "binds", "check", t], function(o) { try { c.check = JSON.parse(o) } catch (e) { c.check = null } }) } }
            Cell { core: c.core; box: true; width: 124; text: ba.list[ba.at].toUpperCase(); property alias at: ba.at; onClicked: ba.at = (ba.at + 1) % ba.list.length
                   Item { id: ba; property int at: 0; property var list: ["spawn", "frame", "window", "focus", "move", "workspace", "window_to", "special", "layout", "nothing"] } }
            Field { id: bg; core: c.core; width: 190; placeholder: ["command", "start|quick|widgets|settings|keys|base|incognito|lock|power", "close|float|fullscreen|maximize|minimize|restore|next|prev", "left|right|up|down", "left|right|up|down", "1…10 · e+1 · e-1", "1…10 · e+1 · e-1", "name", "master|dwindle|…", ""][ba.at] }
            Field { id: bn; core: c.core; width: Math.max(120, c.width - 40 - 210 - 124 - 190 - 96); placeholder: "what it does" }
            Cell { core: c.core; box: true; filled: true; width: 96; pad: 0; kanji: "加"; kanjiSize: core.px; text: "ADD"
                readonly property bool clash: c.check !== null && !!c.check.conflicts && c.check.conflicts.length > 0
                enabled: bk.text.trim() !== "" && c.check !== null && !!c.check.keys
                onClicked: {
                    var rowv = { keys: c.check.keys || bk.text, action: ba.list[ba.at], arg: bg.text, name: bn.text, flags: clash ? "replace" : "" }
                    core.setValue("binds.custom", (core.v("binds.custom") || []).concat([rowv])); bk.text = ""; bg.text = ""; bn.text = ""; c.check = null
                } }
        }
        Txt { core: c.core; leftPadding: 20; topPadding: 6; bottomPadding: 10; width: parent.width; wrapMode: Text.Wrap
              visible: c.check !== null
              heavy: !!c.check && c.check.ok === false
              text: !c.check ? "" : !c.check.keys ? "✕ " + (c.check.error || "not a key combination") : c.check.conflicts.length > 0 ? "▲ " + c.check.keys + " is already used by “" + c.check.conflicts[0].name + "”. Adding it replaces that bind." : "○ free · " + c.check.keys }
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Repeater {
            model: core.binds
            Item {
                required property var modelData
                width: c.width; height: core.hit
                readonly property bool off: (core.v("binds.disabled") || []).some(function(d) { return d.id === modelData.id })
                Rectangle { visible: modelData.custom || !!modelData.held; width: 4; height: parent.height; color: modelData.held ? core.red : core.ink }
                Txt { core: c.core; x: 20; width: 250; height: parent.height; heavy: true; text: modelData.keys }
                Txt { core: c.core; x: 280; width: parent.width - 280 - 330; height: parent.height; text: modelData.name + (modelData.held ? "   ▲ HELD: " + modelData.held : "") }
                Txt { core: c.core; anchors.right: act.left; anchors.rightMargin: 12; width: 190; horizontalAlignment: Text.AlignRight; height: parent.height; faint: true; font.pixelSize: core.small; text: modelData.action + " " + modelData.arg }
                Cell { id: act; core: c.core; anchors.right: parent.right; anchors.rightMargin: 20; box: true; width: 96; pad: 0
                    text: modelData.custom ? "REMOVE" : "TURN OFF"
                    onClicked: {
                        if (modelData.custom) core.setValue("binds.custom", (core.v("binds.custom") || []).filter(function(b) { return !(b.keys === modelData.keys && b.action === modelData.action && b.arg === modelData.arg) }))
                        else core.setValue("binds.disabled", (core.v("binds.disabled") || []).concat([{ id: modelData.id }]))
                    } }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: core.line; color: core.dim; opacity: 0.5 }
            }
        }
        Item { width: 1; height: 12 }
    }

    // ═════ LOOK ═════ the seal: drawn once, for this machine, and the seed is gone
    property var sealInfo: ({})
    property bool sure: false
    function seal() { core.run(["samurai", "seal", "show"], function(o) { try { sealInfo = JSON.parse(o) } catch (e) {} }) }
    Row {
        visible: c.page === "look"; leftPadding: 20; topPadding: 12; bottomPadding: 12; spacing: 16
        Repeater {
            model: ["black", "white"]
            Rectangle {
                required property var modelData
                width: 240; height: 135; color: modelData === "black" ? "#000" : "#fff"; border.width: core.base === modelData ? 3 : core.line; border.color: core.base === modelData ? core.red : core.ink
                Image { anchors.fill: parent; anchors.margins: 3; fillMode: Image.PreserveAspectCrop; asynchronous: true; cache: false
                        source: core.sealDir ? "file://" + core.sealDir + "/wall-" + modelData + ".svg" : ""; visible: !core.incognito }
                MouseArea { anchors.fill: parent; onClicked: core.setValue("look.base", modelData) }
            }
        }
        Column {
            spacing: 6; width: c.width - 2 * 240 - 2 * 16 - 40
            Txt { core: c.core; text: "一期一会"; jp: true; heavy: true; font.pixelSize: 26 }
            Txt { core: c.core; width: parent.width; wrapMode: Text.Wrap; text: "One wallpaper pair, drawn once from this machine's randomness. The seed was destroyed after drawing: it cannot be drawn again, here or anywhere." }
            Txt { core: c.core; text: "mark  " + core.mark; heavy: true }
            Txt { core: c.core; faint: true; font.pixelSize: core.small; text: c.sealInfo.forged ? "forged " + (c.sealInfo.made || "") : "not forged yet (it is drawn at first login)" }
            Row { spacing: 0
                Cell { core: c.core; box: true; visible: !c.sure; text: "FORGE A NEW ONE…"; onClicked: c.sure = true }
                Cell { core: c.core; box: true; visible: c.sure; accent: true; text: "DESTROY THIS ONE FOR GOOD AND FORGE"; onClicked: { c.sure = false; c.sh(["samurai", "seal", "forge", "--erase-current"], "", c.seal) } }
                Cell { core: c.core; box: true; visible: c.sure; text: "KEEP IT"; onClicked: c.sure = false }
            }
        }
    }

    // ═════ DISPLAYS ═════ what is plugged in now
    property var monitors: []
    function mons() { core.run(["hyprctl", "-j", "monitors", "all"], function(o) { try { monitors = JSON.parse(o) } catch (e) { monitors = [] } }) }
    Column {
        visible: c.page === "displays"; width: parent.width
        Repeater {
            model: c.monitors
            Item {
                required property var modelData
                width: c.width; height: core.hit + 12
                Txt { core: c.core; x: 20; height: parent.height; heavy: true; text: modelData.name }
                Txt { core: c.core; x: 140; width: parent.width - 420; height: parent.height; text: (modelData.description || "") }
                Txt { core: c.core; anchors.right: parent.right; anchors.rightMargin: 20; height: parent.height; text: modelData.width + "×" + modelData.height + " @ " + Math.round(modelData.refreshRate) + " · ×" + modelData.scale + " · " + modelData.x + "," + modelData.y }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: core.line; color: core.dim; opacity: 0.5 }
            }
        }
        Txt { core: c.core; leftPadding: 20; rightPadding: 20; topPadding: 8; bottomPadding: 8; width: parent.width; wrapMode: Text.Wrap; faint: true
              text: "Add a row under Monitors to fix a mode, scale, place or rotation for an output. Outputs without a row use their preferred mode." }
    }

    // ═════ STORE · PLUGINS · UPDATES · GPU ═════ commands with a plain result
    Row {
        visible: ["store", "plugins", "updates", "gpu"].indexOf(c.page) >= 0
        leftPadding: 20; topPadding: 12; bottomPadding: 12; spacing: 0
        Repeater {
            model: ({ store: [["更", "REFRESH", ["samurai", "store", "refresh"]], ["覧", "LIST", ["samurai", "store", "list"]]],
                      plugins: [["覧", "LIST", ["samurai", "plugins"]]],
                      updates: [["確", "CHECK NOW", ["samurai", "update", "check"]], ["入", "INSTALL", ["samurai", "update", "install"]]],
                      gpu: [["診", "CHECK READINESS", ["samurai", "gpu", "check"]]] })[c.page] || []
            Cell { required property var modelData; core: c.core; box: true; enabled: !c.busy; kanji: modelData[0]; kanjiSize: core.px; text: modelData[1]; onClicked: c.sh(modelData[2]) }
        }
        Field { id: sid; core: c.core; visible: c.page === "store"; width: 200; placeholder: "id from the list" }
        Cell { core: c.core; visible: c.page === "store"; box: true; text: "INSTALL"; enabled: sid.text !== "" && !c.busy; onClicked: c.sh(["samurai", "store", "install", sid.text]) }
        Cell { core: c.core; visible: c.page === "store"; box: true; text: "REMOVE"; enabled: sid.text !== "" && !c.busy; onClicked: c.sh(["samurai", "store", "remove", sid.text]) }
    }
    Txt { core: c.core; visible: c.page === "updates" && !!core.status.update && core.status.update.available === true; leftPadding: 20; bottomPadding: 10; heavy: true; text: "▲ an update is ready: " + ((core.status.update || {}).commit || "") }

    // what the last command said, verbatim
    Rectangle {
        visible: c.out !== ""; x: 20; width: parent.width - 40; height: say.implicitHeight + 20
        color: "transparent"; border.width: core.line; border.color: core.ink
        Txt { id: say; core: c.core; x: 12; y: 10; width: parent.width - 24; wrapMode: Text.WrapAnywhere; font.pixelSize: core.small; text: c.out }
    }
    Item { visible: c.out !== ""; width: 1; height: 12 }
}
