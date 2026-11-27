// Samurai Settings. Ten plain pages; every row says what it does; every change applies at once.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Window

ApplicationWindow {
    id: win
    width: 1100 * s
    height: 760 * s
    minimumWidth: 760
    minimumHeight: 520
    visible: true
    title: "Settings"
    color: c.surf

    readonly property var c: core.colors
    readonly property real s: core.scale
    readonly property var pages: core.schema.pages || []
    // the Sakura look: every page also has its Japanese name
    readonly property var jp: ({ look: "外観", displays: "ディスプレイ", network: "ネットワーク", sound: "音", keyboard: "キー",
                                 mouse: "マウス", apps: "アプリ", power: "電源", notices: "通知", system: "システム" })
    property string pageId: core.startPage
    property string query: ""
    readonly property var page: { for (var i = 0; i < pages.length; i++) if (pages[i].id === pageId) return pages[i]; return pages[0] }
    font.family: "Inter"
    font.pixelSize: 13.5 * s

    function v(key) { return core.values[key] }
    function isDefault(item) { return JSON.stringify(v(item.key)) === JSON.stringify(item["default"]) }
    function say(text, bad) { toast.text = text; toast.bad = bad; toast.opacity = 1; toastTimer.restart() }

    Connections {
        target: core
        function onMessage(text, bad) { win.say(text, bad) }
    }

    // ── small parts ──────────────────────────────────────────────────────────────────────────
    component RowLine: Item {
        property bool first: false
        width: parent ? parent.width : 0
        Rectangle { visible: !parent.first; width: parent.width; height: 1; color: c.line }
    }

    // ── one setting, drawn from the list ─────────────────────────────────────────────────────
    component Setting: Item {
        id: st
        required property var item
        property bool first: false
        width: parent ? parent.width : 0
        height: Math.max(58 * s, lay.implicitHeight + 20 * s) + (item.type === "list" ? listBox.height + 8 * s : 0)
        readonly property var val: win.v(item.key)
        readonly property bool appPick: item.key.indexOf("apps.") === 0 && item.type === "text" && win.pageId === "apps"
        opacity: item.soon ? 0.45 : 1
        enabled: !item.soon
        Rectangle { visible: !st.first; width: parent.width; height: 1; color: c.line }
        RowLayout {
            id: lay
            x: 18 * s; width: parent.width - 36 * s; y: 10 * s
            spacing: 16 * s
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2 * s
                Text { text: st.item.label; color: c.text; font.pixelSize: 14 * s; font.weight: Font.Medium; Layout.fillWidth: true; wrapMode: Text.Wrap }
                Text { visible: !!st.item.help; text: st.item.help || ""; color: c.dim; font.pixelSize: 12.5 * s; Layout.fillWidth: true; wrapMode: Text.Wrap }
            }
            Button {
                visible: !win.isDefault(st.item) && st.item.type !== "list"
                text: "↺"
                implicitWidth: 30 * s; implicitHeight: 30 * s
                ToolTip.visible: hovered; ToolTip.text: "Back to the default"
                Accessible.name: "Back to the default"
                onClicked: core.reset(st.item.key)
                background: Rectangle { radius: height / 2; color: parent.hovered ? c.chrome : "transparent" }
                contentItem: Text { text: "↺"; color: c.dim; font.pixelSize: 16 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
            }
            Loader {
                Layout.alignment: Qt.AlignVCenter
                sourceComponent: {
                    if (st.item.type === "toggle") return tog
                    if (st.item.type === "enum") return st.item.options.length <= 3 ? segc : combo
                    if (st.item.type === "int" || st.item.type === "float") return slide
                    if (st.appPick) return apppick
                    if (st.item.type === "text") return text
                    return null
                }
            }
        }
        Component { id: tog; Toggle { on: st.val === true; onFlipped: (to) => core.set(st.item.key, to); Accessible.name: st.item.label } }
        Component { id: segc; Seg { options: st.item.options; current: st.val; onPicked: (x) => core.set(st.item.key, x) } }
        Component {
            id: combo
            Choice {
                model: st.item.options; textRole: "label"
                currentIndex: { for (var i = 0; i < st.item.options.length; i++) if (st.item.options[i].value === st.val) return i; return -1 }
                onActivated: (i) => core.set(st.item.key, st.item.options[i].value)
                Accessible.name: st.item.label
            }
        }
        Component {
            id: apppick
            Choice {
                property var apps: core.appsFor(st.item.key)
                model: apps; textRole: "name"
                currentIndex: { for (var i = 0; i < apps.length; i++) if (apps[i].id === st.val) return i; return -1 }
                displayText: currentIndex < 0 ? "Nothing installed for this" : currentText
                onActivated: (i) => core.set(st.item.key, apps[i].id)
                Accessible.name: st.item.label
            }
        }
        Component {
            id: text
            Field {
                text: st.val === undefined ? "" : st.val
                onEditingFinished: if (text !== st.val) core.set(st.item.key, text)
                Accessible.name: st.item.label
            }
        }
        Component {
            id: slide
            Row {
                spacing: 12 * s
                Slider {
                    id: sl
                    width: 200 * s; height: 30 * s
                    from: st.item.min; to: st.item.max; stepSize: st.item.step || (st.item.type === "int" ? 1 : 0.05); snapMode: Slider.SnapAlways
                    value: st.val === undefined ? (st.item.default || 0) : st.val
                    Accessible.name: st.item.label
                    onPressedChanged: if (!pressed && value !== st.val) core.set(st.item.key, st.item.type === "int" ? Math.round(value) : Math.round(value * 100) / 100)
                    Keys.onReleased: if (value !== st.val) core.set(st.item.key, st.item.type === "int" ? Math.round(value) : Math.round(value * 100) / 100)
                    background: Rectangle {
                        x: sl.leftPadding; y: sl.topPadding + sl.availableHeight / 2 - height / 2
                        width: sl.availableWidth; height: 6 * s; radius: 3 * s; color: c.line
                        Rectangle { width: sl.visualPosition * parent.width; height: parent.height; radius: 3 * s; color: c.accent }
                    }
                    handle: Rectangle {
                        x: sl.leftPadding + sl.visualPosition * (sl.availableWidth - width); y: sl.topPadding + sl.availableHeight / 2 - height / 2
                        width: 18 * s; height: 18 * s; radius: 9 * s; color: c.raised; border.color: c.accent; border.width: sl.visualFocus ? 3 : 2
                    }
                }
                Text {
                    width: 64 * s; anchors.verticalCenter: parent.verticalCenter
                    text: (st.item.type === "int" ? Math.round(sl.value) : (Math.round(sl.value * 100) / 100)) + (st.item.unit ? " " + st.item.unit : "")
                    color: c.text; font.pixelSize: 13 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignRight
                }
            }
        }
        // a list: one line per row, ✕ to remove, one field to add a row in the same shape
        Column {
            id: listBox
            visible: st.item.type === "list"
            x: 18 * s; width: parent.width - 36 * s
            y: lay.y + lay.implicitHeight + 8 * s
            spacing: 6 * s
            Repeater {
                model: st.item.type === "list" ? (st.val || []) : []
                Rectangle {
                    required property var modelData
                    required property int index
                    width: listBox.width; height: 38 * s; radius: 8 * s; color: c.chrome
                    Text {
                        x: 12 * s; width: parent.width - 60 * s; anchors.verticalCenter: parent.verticalCenter
                        text: st.item.columns.map(function (k) { return modelData[k] || "—" }).join("   ·   ")
                        color: c.text; font.pixelSize: 13 * s; elide: Text.ElideRight
                    }
                    Button {
                        anchors.right: parent.right; anchors.rightMargin: 4 * s; anchors.verticalCenter: parent.verticalCenter
                        width: 32 * s; height: 32 * s
                        Accessible.name: "Remove this line"
                        onClicked: { var l = (st.val || []).slice(); l.splice(index, 1); core.set(st.item.key, l) }
                        background: Rectangle { radius: 7 * s; color: parent.hovered ? c.raised : "transparent" }
                        contentItem: Icon { name: "close"; size: 14 * s; anchors.centerIn: parent }
                    }
                }
            }
            Row {
                spacing: 8 * s
                Field {
                    id: add
                    width: listBox.width - addBtn.width - 8 * s
                    placeholderText: st.item.example || ""
                    Accessible.name: "Add a line to " + st.item.label
                    onAccepted: addBtn.clicked()
                }
                Btn {
                    id: addBtn
                    text: "Add"
                    enabled: add.text.trim() !== ""
                    onClicked: {
                        var parts = add.text.split("·").map(function (x) { return x.trim() })
                        var row = {}
                        for (var i = 0; i < st.item.columns.length; i++) row[st.item.columns[i]] = parts[i] === undefined ? "" : parts[i]
                        // a shortcut's action is everything after the keys
                        if (st.item.columns.length === 2 && parts.length > 2) row[st.item.columns[1]] = parts.slice(1).join(" · ")
                        core.set(st.item.key, (st.val || []).concat([row]))
                        add.text = ""
                    }
                }
            }
        }
    }

    // ── the window: sidebar and page ─────────────────────────────────────────────────────────
    RowLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.preferredWidth: 236 * s
            Layout.fillHeight: true
            color: c.chrome
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: c.line }
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12 * s
                spacing: 4 * s
                Field {
                    id: search
                    Layout.fillWidth: true
                    Layout.bottomMargin: 10 * s
                    placeholderText: "検索 Search settings"
                    leftPadding: 36 * s
                    background: Rectangle { radius: height / 2; color: c.field; border.color: search.activeFocus ? c.accent : c.line; border.width: search.activeFocus ? 2 : 1 }
                    onTextChanged: win.query = text.trim().toLowerCase()
                    Icon { name: "search"; size: 16 * s; x: 12 * s; anchors.verticalCenter: parent.verticalCenter }
                    Accessible.name: "Search settings"
                }
                Repeater {
                    model: win.pages
                    Button {
                        required property var modelData
                        readonly property bool sel: win.pageId === modelData.id && win.query === ""
                        Layout.fillWidth: true
                        implicitHeight: 38 * s
                        onClicked: { win.pageId = modelData.id; search.text = "" }
                        Accessible.name: modelData.title
                        background: Rectangle { radius: height / 2; color: parent.sel ? c.soft : (parent.hovered ? Qt.alpha(c.text, 0.05) : "transparent"); border.width: parent.visualFocus ? 2 : 0; border.color: c.accent }
                        contentItem: Item {
                            Text { x: 16 * s; width: parent.width - 16 * s - jpName.width - 22 * s; elide: Text.ElideRight; text: modelData.title; color: parent.parent.sel ? c.accent : c.text; font.pixelSize: 13.5 * s; font.weight: parent.parent.sel ? Font.DemiBold : Font.Normal; anchors.verticalCenter: parent.verticalCenter }
                            Text { id: jpName; anchors.right: parent.right; anchors.rightMargin: 14 * s; text: win.jp[modelData.id] || ""; color: parent.parent.sel ? c.accent : c.dim; font.pixelSize: 11 * s; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                }
                Item { Layout.fillHeight: true }
            }
        }

        ScrollView {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: scroller.availableWidth
                spacing: 20 * s
                Item { Layout.preferredHeight: 10 * s }
                // heading
                ColumnLayout {
                    Layout.leftMargin: 40 * s; Layout.rightMargin: 40 * s
                    spacing: 6 * s
                    Text { text: win.query ? "検索" : (win.jp[win.page.id] || win.page.title); color: c.text; font.pixelSize: 28 * s; font.weight: Font.DemiBold }
                    Text { text: win.query ? "Search" : win.page.title; color: c.dim; font.pixelSize: 14 * s; visible: win.query !== "" || !!win.jp[win.page.id] }
                    Text { text: win.query ? "Every setting whose name has “" + search.text + "”." : win.page.lead; color: c.dim; font.pixelSize: 14 * s; wrapMode: Text.Wrap; Layout.maximumWidth: 640 * s }
                }
                // the hand-made part of this page
                Loader {
                    Layout.leftMargin: 40 * s; Layout.rightMargin: 40 * s
                    Layout.fillWidth: true
                    active: win.query === "" && !!win.page.custom
                    source: active ? "pages/" + win.page.custom + ".qml" : ""
                }
                // the settings, by section
                Repeater {
                    model: win.query === "" ? win.page.sections : []
                    Card {
                        required property var modelData
                        Layout.leftMargin: 40 * s; Layout.rightMargin: 40 * s
                        title: modelData.title
                        Repeater {
                            model: modelData.items
                            Setting { required property var modelData; required property int index; item: modelData; first: index === 0 }
                        }
                    }
                }
                // search results across every page
                Repeater {
                    model: {
                        if (!win.query) return []
                        var out = []
                        for (var p = 0; p < win.pages.length; p++)
                            for (var x = 0; x < win.pages[p].sections.length; x++)
                                for (var i = 0; i < win.pages[p].sections[x].items.length; i++) {
                                    var it = win.pages[p].sections[x].items[i]
                                    if ((it.label + " " + (it.help || "")).toLowerCase().indexOf(win.query) >= 0) out.push({ item: it, where: win.pages[p].title })
                                }
                        return out
                    }
                    Card {
                        required property var modelData
                        Layout.leftMargin: 40 * s; Layout.rightMargin: 40 * s
                        title: modelData.where
                        Setting { item: modelData.item; first: true }
                    }
                }
                Item { Layout.preferredHeight: 30 * s }
            }
        }
    }

    // a line at the bottom for what happened
    Rectangle {
        id: toast
        property alias text: tt.text
        property bool bad: false
        opacity: 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 160 } }
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom; anchors.bottomMargin: 24 * s
        width: Math.min(tt.implicitWidth + 36 * s, win.width - 80 * s); height: Math.max(40 * s, tt.implicitHeight + 16 * s)
        radius: 20 * s
        color: bad ? c.accent : c.raised
        border.color: c.line
        Text { id: tt; anchors.centerIn: parent; width: parent.width - 36 * s; color: toast.bad ? c.onAccent : c.text; font.pixelSize: 13 * s; wrapMode: Text.Wrap; horizontalAlignment: Text.AlignHCenter }
        Timer { id: toastTimer; interval: 5000; onTriggered: toast.opacity = 0 }
    }
}
