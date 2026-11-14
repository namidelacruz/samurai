// One setting, drawn from its schema entry. Label on the left, the control on the right,
// one line under it. Changed-from-default settings carry a red tick and a reset cell.
import QtQuick
import "../lib"

Item {
    id: row
    required property var core
    required property var item
    property string result: ""
    readonly property var val: item.key ? core.v(item.key) : undefined
    readonly property bool changed: item.key !== "" && item.key !== undefined && item["default"] !== undefined && JSON.stringify(val) !== JSON.stringify(item["default"])
    width: parent ? parent.width : 600
    height: body.implicitHeight + core.line
    function put(x) { core.setValue(item.key, x) }
    function clamp(x) {
        if (item.min !== undefined) x = Math.max(item.min, x)
        if (item.max !== undefined) x = Math.min(item.max, x)
        return item.type === "int" ? Math.round(x) : Math.round(x * 1000) / 1000
    }

    Rectangle { visible: row.changed; width: 4; height: parent.height - core.line; color: core.red }
    Column {
        id: body
        width: parent.width
        // ── note ──
        Txt { core: row.core; visible: item.type === "note"; width: parent.width; leftPadding: 20; rightPadding: 20; topPadding: 10; bottomPadding: 10; faint: true; wrapMode: Text.Wrap; text: item.label }
        // ── everything with a label and a control ──
        Item {
            visible: item.type !== "note" && item.type !== "list"
            width: parent.width; height: visible ? core.hit + 8 : 0
            Txt { core: row.core; x: 20; width: parent.width - ctl.width - 40 - (soon.visible ? soon.width + 12 : 0); height: parent.height; text: item.label; faint: item.soon === true; elide: Text.ElideRight }
            Txt { id: soon; core: row.core; visible: item.soon === true; anchors.right: ctl.left; anchors.rightMargin: 12; height: parent.height; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; text: "未 NOT BUILT YET"; jp: true }
            Row {
                id: ctl
                enabled: item.soon !== true; opacity: item.soon === true ? 0.4 : 1
                anchors.right: parent.right; anchors.rightMargin: 20; anchors.verticalCenter: parent.verticalCenter
                spacing: 0
                Cell { core: row.core; visible: row.changed; text: "↺"; box: true; width: core.hit; pad: 0; onClicked: core.run(["samurai", "reset", item.key]) }
                Item { width: row.changed ? 8 : 0; height: 1 }
                // toggle: two cells, the live one filled
                Cell { core: row.core; visible: item.type === "toggle"; box: true; kanji: "切"; kanjiSize: core.px; text: "OFF"; filled: row.val !== true; onClicked: row.put(false) }
                Cell { core: row.core; visible: item.type === "toggle"; box: true; kanji: "入"; kanjiSize: core.px; text: "ON"; filled: row.val === true; onClicked: row.put(true) }
                // number: − value +
                Cell { core: row.core; visible: item.type === "int" || item.type === "float"; box: true; width: core.hit; pad: 0; text: "−"; onClicked: row.put(row.clamp((row.val || 0) - (item.step || 1))) }
                Field {
                    core: row.core; visible: item.type === "int" || item.type === "float"; width: 88
                    text: row.val === undefined ? "" : "" + row.val
                    onCommit: function(t) { var n = Number(t); if (t !== "" && isFinite(n) && row.clamp(n) !== row.val) row.put(row.clamp(n)) }
                }
                Cell { core: row.core; visible: item.type === "int" || item.type === "float"; box: true; width: core.hit; pad: 0; text: "+"; onClicked: row.put(row.clamp((row.val || 0) + (item.step || 1))) }
                // enum: every choice shown when they fit, else step through them
                Repeater {
                    model: item.type === "enum" && item.options.length <= 4 ? item.options : []
                    Cell { required property var modelData; core: row.core; box: true; text: ("" + modelData).toUpperCase(); filled: row.val === modelData; onClicked: row.put(modelData) }
                }
                Cell { core: row.core; visible: item.type === "enum" && item.options.length > 4; box: true; width: core.hit; pad: 0; text: "‹"; onClicked: { var o = item.options, i = o.indexOf(row.val); row.put(o[(i - 1 + o.length) % o.length]) } }
                Cell { core: row.core; visible: item.type === "enum" && item.options.length > 4; box: true; filled: true; width: 200; text: ("" + row.val).toUpperCase() }
                Cell { core: row.core; visible: item.type === "enum" && item.options.length > 4; box: true; width: core.hit; pad: 0; text: "›"; onClicked: { var o = item.options, i = o.indexOf(row.val); row.put(o[(i + 1) % o.length]) } }
                // text / colour
                Rectangle { visible: item.type === "color"; width: core.hit; height: core.hit; color: /^#[0-9a-fA-F]{6}$/.test(row.val || "") ? row.val : "transparent"; border.width: core.line; border.color: core.ink }
                Field {
                    core: row.core; visible: item.type === "text" || item.type === "color"; width: item.type === "color" ? 120 : 280
                    secret: item.secret === true
                    placeholder: item.secret === true ? "kept in the keyring" : ""
                    text: item.secret === true ? "" : (row.val === undefined ? "" : "" + row.val)
                    onCommit: function(t) {
                        if (item.secret === true) { if (t !== "") { core.run(["secret-tool", "store", "--label=Samurai " + item.key, "samurai", item.key], function(o, c) { row.result = c === 0 ? "kept in the keyring" : "the keyring is not available (secret-tool)" }, t); text = "" } return }
                        if (item.type === "color" && !/^#[0-9a-fA-F]{6}$/.test(t)) return
                        if (t !== row.val) row.put(t)
                    }
                }
                // action
                Cell { core: row.core; visible: item.type === "action"; box: true; text: "RUN"; kanji: "行"; kanjiSize: core.px
                    onClicked: { row.result = "…"; core.run(item.cmd, function(o, c) { row.result = (o.trim() || (c === 0 ? "done" : "failed")) }) } }
            }
        }
        // ── list: rows of fields, one add line in the example's shape ──
        Column {
            visible: item.type === "list"
            width: parent.width
            Txt { core: row.core; leftPadding: 20; topPadding: 12; bottomPadding: 4; text: item.label; heavy: true }
            Txt { core: row.core; leftPadding: 20; bottomPadding: 6; faint: true; font.pixelSize: core.small; text: item.columns ? item.columns.join(" · ") : "" }
            Repeater {
                model: item.type === "list" && row.val ? row.val : []
                Item {
                    required property var modelData
                    required property int index
                    width: body.width; height: core.hit
                    Txt { core: row.core; x: 20; width: parent.width - 40 - core.hit; height: parent.height
                          text: (item.columns || []).map(function(c) { var x = modelData[c]; return x === undefined || x === "" ? "—" : "" + x }).join("  ·  ") }
                    Cell { core: row.core; anchors.right: parent.right; anchors.rightMargin: 20; width: core.hit; pad: 0; box: true; text: "×"
                           onClicked: { var l = row.val.slice(); l.splice(index, 1); row.put(l) } }
                }
            }
            Row {
                leftPadding: 20; topPadding: 4; bottomPadding: 10; spacing: 0
                Field { id: add; core: row.core; width: body.width - 40 - 88; placeholder: item.example || ""
                        onCommit: function(t) {} }
                Cell { core: row.core; box: true; width: 88; kanji: "加"; kanjiSize: core.px; text: "ADD"
                    onClicked: {
                        var parts = add.text.split("·").map(function(s) { return s.trim() })
                        if (add.text.trim() === "") return
                        var o = {}; for (var i = 0; i < item.columns.length; i++) o[item.columns[i]] = parts[i] === undefined ? "" : parts[i]
                        row.put((row.val || []).concat([o])); add.text = ""
                    } }
            }
        }
        Txt { core: row.core; visible: row.result !== ""; width: parent.width; leftPadding: 20; rightPadding: 20; bottomPadding: 10; wrapMode: Text.Wrap; font.pixelSize: core.small; text: row.result }
    }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: core.line; color: core.dim; opacity: 0.5 }
}
