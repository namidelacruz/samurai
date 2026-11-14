// The settings hub. Left: every page, grouped. Right: the page, drawn from the schema.
// The same grid as the desktop: lines, cells, one red.
import QtQuick
import "../lib"

Rectangle {
    id: hub
    required property var core
    color: core.ground
    property string query: ""
    readonly property var pages: core.schema.pages || []
    readonly property var page: { for (var i = 0; i < pages.length; i++) if (pages[i].id === core.hubPage) return pages[i]; return pages[0] || null }
    function changedOn(p) { var n = 0; p.sections.forEach(function(s) { s.items.forEach(function(it) { if (it.key && it["default"] !== undefined && JSON.stringify(core.v(it.key)) !== JSON.stringify(it["default"])) n++ }) }); return n }
    readonly property var hits: {
        var q = query.trim().toLowerCase(), out = []
        if (!q) return out
        pages.forEach(function(p) { p.sections.forEach(function(s) { s.items.forEach(function(it) {
            if (it.type !== "note" && ((it.label || "").toLowerCase().indexOf(q) >= 0 || (it.key || "").indexOf(q) >= 0)) out.push({ page: p, item: it }) }) }) })
        return out.slice(0, 40)
    }

    // ── sidebar ──
    Rectangle {
        id: sidebar
        width: 232; height: parent.height; color: "transparent"
        Rectangle { anchors.right: parent.right; width: core.line; height: parent.height; color: core.ink }
        Flickable {
            anchors.fill: parent; anchors.rightMargin: core.line; clip: true; contentHeight: nav.implicitHeight
            Column {
                id: nav
                width: parent.width
                Row { leftPadding: 16; topPadding: 14; bottomPadding: 10; spacing: 10
                    Txt { core: hub.core; text: "設定"; jp: true; heavy: true; font.pixelSize: 30 }
                    Txt { core: hub.core; text: "SETTINGS"; heavy: true; font.letterSpacing: 2; anchors.baseline: parent.children[0].baseline } }
                Field { core: hub.core; x: 16; width: parent.width - 32; placeholder: "探 search every setting"; onEdited: function(t) { hub.query = t } }
                Item { width: 1; height: 10 }
                Repeater {
                    model: core.schema.groups || []
                    Column {
                        id: grp
                        required property var modelData
                        width: nav.width
                        Rectangle { width: parent.width; height: core.line; color: core.ink }
                        Txt { core: hub.core; leftPadding: 16; topPadding: 8; bottomPadding: 4; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; text: grp.modelData.jp + " " + grp.modelData.en; jp: true }
                        Repeater {
                            model: hub.pages.filter(function(p) { return p.group === grp.modelData.id })
                            Cell {
                                required property var modelData
                                core: hub.core; width: nav.width; height: Math.round(core.hit * 0.82); pad: 0
                                filled: hub.query === "" && hub.page && hub.page.id === modelData.id
                                accent: modelData.id === "incognito" && core.incognito
                                Txt { core: hub.core; x: 16; width: 46; height: parent.height; text: modelData.jp; jp: true; heavy: true; color: parent.fg }
                                Txt { core: hub.core; x: 64; height: parent.height; text: modelData.en; color: parent.fg; heavy: parent.filled }
                                Txt { core: hub.core; anchors.right: parent.right; anchors.rightMargin: 12; height: parent.height; color: parent.fg; font.pixelSize: core.small
                                      readonly property int n: hub.changedOn(modelData); visible: n > 0; text: "" + n }
                                onClicked: { core.hubPage = modelData.id; hub.query = "" }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── page ──
    Flickable {
        id: view
        anchors.left: sidebar.right; anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: foot.top
        clip: true; contentHeight: content.implicitHeight + 24
        Column {
            id: content
            width: view.width
            // header
            Row { visible: hub.query === "" && hub.page; leftPadding: 20; topPadding: 14; spacing: 14
                Txt { core: hub.core; text: hub.page ? hub.page.jp : ""; jp: true; heavy: true; font.pixelSize: 44 }
                Txt { core: hub.core; text: hub.page ? hub.page.en : ""; heavy: true; font.letterSpacing: 2; anchors.baseline: parent.children[0].baseline } }
            Txt { core: hub.core; visible: hub.query === "" && hub.page; leftPadding: 20; rightPadding: 20; topPadding: 4; bottomPadding: 12; width: parent.width; wrapMode: Text.Wrap; faint: true; text: hub.page ? hub.page.blurb : "" }
            Rectangle { width: parent.width; height: core.line; color: core.ink }
            Txt { core: hub.core; visible: core.lastError !== undefined && core.lastError !== ""; leftPadding: 20; topPadding: 8; bottomPadding: 8; heavy: true; text: "✕ " + (core.lastError || "") }
            // the hand-made part
            Custom { core: hub.core; width: parent.width; visible: page !== ""; page: hub.query === "" && hub.page && hub.page.custom ? hub.page.custom : "" }
            // the generated part
            Repeater {
                model: hub.query === "" && hub.page ? hub.page.sections : []
                Column {
                    id: sec
                    required property var modelData
                    width: content.width
                    Rectangle { width: parent.width; height: core.hit * 0.75; color: "transparent"; visible: !!sec.modelData.en
                        Txt { core: hub.core; x: 20; height: parent.height; faint: true; font.pixelSize: core.small; font.letterSpacing: 2; jp: true; text: (sec.modelData.jp || "") + " " + (sec.modelData.en || "") }
                        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: core.line; color: core.ink } }
                    Repeater { model: sec.modelData.items; ItemRow { required property var modelData; core: hub.core; item: modelData; width: content.width } }
                }
            }
            // search results
            Txt { core: hub.core; visible: hub.query !== ""; leftPadding: 20; topPadding: 14; bottomPadding: 10; heavy: true; text: hub.hits.length + " settings match “" + hub.query + "”" }
            Repeater {
                model: hub.hits
                Column {
                    required property var modelData
                    width: content.width
                    Txt { core: hub.core; leftPadding: 20; topPadding: 6; faint: true; font.pixelSize: core.small; jp: true; text: modelData.page.jp + " " + modelData.page.en }
                    ItemRow { core: hub.core; item: modelData.item; width: content.width }
                }
            }
        }
    }

    // ── foot: what this page changed, and the way back ──
    Rectangle {
        id: foot
        anchors.left: sidebar.right; anchors.right: parent.right; anchors.bottom: parent.bottom; height: core.hit + core.line; color: core.ground
        Rectangle { width: parent.width; height: core.line; color: core.ink }
        Txt { core: hub.core; x: 20; y: core.line; height: core.hit; faint: true
              text: hub.page ? (hub.changedOn(hub.page) + " changed on this page · saved as you go · applied live") : "" }
        Row { anchors.right: parent.right; y: core.line
            Cell { core: hub.core; lineLeft: true; text: "RESET THIS PAGE"; enabled: hub.page && hub.changedOn(hub.page) > 0; onClicked: core.run(["samurai", "reset", hub.page.id]) }
            Cell { core: hub.core; lineLeft: true; kanji: "閉"; kanjiSize: core.px; text: "CLOSE"; onClicked: core.hubOpen = false }
        }
    }
}
