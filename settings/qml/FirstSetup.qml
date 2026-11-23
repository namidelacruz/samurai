// The first setup: once per user, at the first login (it can be skipped, and opened again from
// Settings › Keyboard). Four short steps: shortcuts, look, Wi-Fi, done.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Window

Window {
    id: win
    readonly property var c: core.colors
    readonly property real s: core.scale
    property int step: 0
    property string preset: core.values["keys.preset"] || "samurai"
    property var rows: core.keySet(preset)
    property var twice: core.clashes(rows)
    readonly property var steps: ["Shortcuts", "Look", "Wi-Fi", "Done"]
    width: 920 * s; height: 640 * s
    minimumWidth: 720 * s; minimumHeight: 520 * s
    visible: true
    title: "Welcome to Samurai"
    color: c.ground
    function say(text, bad) { note.text = text; note.bad = bad; noteTimer.restart() }
    function choose(p) { preset = p; rows = core.keySet(p); twice = core.clashes(rows) }
    function edit(i, k) { var r = rows.slice(); r[i] = Object.assign({}, r[i], { keys: k }); rows = r; twice = core.clashes(r) }

    // ── top: the steps ───────────────────────────────────────────────────────────────────
    Row {
        id: top
        x: 32 * s; y: 26 * s; spacing: 8 * s
        Repeater {
            model: win.steps
            Row {
                required property string modelData
                required property int index
                spacing: 8 * s
                Rectangle {
                    width: 22 * s; height: 22 * s; radius: 11 * s
                    color: index <= win.step ? c.accent : "transparent"; border.color: index <= win.step ? c.accent : c.line
                    Text { anchors.centerIn: parent; text: index + 1; color: index <= win.step ? c.onAccent : c.dim; font.pixelSize: 11 * s; font.weight: Font.DemiBold }
                }
                Text { text: modelData; color: index === win.step ? c.text : c.dim; font.pixelSize: 13 * s; font.weight: index === win.step ? Font.DemiBold : Font.Normal; anchors.verticalCenter: parent.verticalCenter }
                Rectangle { visible: index < win.steps.length - 1; width: 28 * s; height: 1; color: c.line; anchors.verticalCenter: parent.verticalCenter }
            }
        }
    }

    // ── the step ─────────────────────────────────────────────────────────────────────────
    Item {
        id: body
        x: 32 * s; y: top.y + top.height + 22 * s
        width: parent.width - 64 * s; height: parent.height - y - 78 * s

        // 1. shortcuts
        ColumnLayout {
            visible: win.step === 0
            anchors.fill: parent; spacing: 12 * s
            Text { text: "Which shortcuts feel like home?"; color: c.text; font.pixelSize: 22 * s; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 13 * s
                   text: "Pick a set, then change any key below. Super is the Windows key. You can change all of this later in Settings › Keyboard." }
            RowLayout {
                spacing: 10 * s
                Repeater {
                    model: [["samurai", "Samurai", "Super opens everything · Super+Q closes · Super+arrows move windows"],
                            ["hyprland", "Hyprland-style", "Super+Q terminal · Super+C closes · Super+R apps · Super+arrows move focus"]]
                    Button {
                        required property var modelData
                        Layout.preferredWidth: 380 * s; Layout.preferredHeight: 66 * s
                        Accessible.name: modelData[1]
                        onClicked: win.choose(modelData[0])
                        background: Rectangle { radius: 12 * s; color: win.preset === modelData[0] ? c.soft : (parent.hovered ? c.chrome : c.raised); border.color: win.preset === modelData[0] ? c.accent : c.line; border.width: win.preset === modelData[0] ? 1.5 : 1 }
                        contentItem: Column {
                            leftPadding: 14 * s; topPadding: 4 * s; spacing: 3 * s
                            Text { text: modelData[1]; color: c.text; font.pixelSize: 14 * s; font.weight: Font.DemiBold }
                            Text { text: modelData[2]; color: c.dim; font.pixelSize: 11.5 * s; width: 350 * s; elide: Text.ElideRight }
                        }
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true
                radius: 12 * s; color: c.raised; border.color: c.line; clip: true
                ListView {
                    id: keyList
                    anchors.fill: parent; anchors.margins: 6 * s
                    model: win.rows
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar {}
                    delegate: Item {
                        required property var modelData
                        required property int index
                        width: keyList.width - 10 * s; height: 38 * s
                        readonly property bool clash: win.twice.indexOf(modelData.keys) >= 0
                        Text { x: 10 * s; anchors.verticalCenter: parent.verticalCenter; text: modelData.label; color: c.text; font.pixelSize: 13 * s }
                        Text { visible: parent.clash; anchors.right: kf.left; anchors.rightMargin: 10 * s; anchors.verticalCenter: parent.verticalCenter
                               text: "used twice"; color: "#E0705A"; font.pixelSize: 11.5 * s; font.weight: Font.DemiBold }
                        TextField {
                            id: kf
                            anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                            width: 220 * s; height: 30 * s
                            text: modelData.keys
                            placeholderText: "no key"
                            font.family: "JetBrains Mono"; font.pixelSize: 12 * s
                            color: c.text; placeholderTextColor: c.dim
                            Accessible.name: "Keys for " + modelData.label
                            background: Rectangle { radius: 7 * s; color: c.chrome; border.color: parent.parent.clash ? "#E0705A" : (kf.activeFocus ? c.accent : c.line) }
                            onEditingFinished: if (text !== modelData.keys) win.edit(index, text)
                        }
                    }
                }
            }
        }

        // 2. look
        ColumnLayout {
            visible: win.step === 1
            anchors.fill: parent; spacing: 14 * s
            Text { text: "How should it look?"; color: c.text; font.pixelSize: 22 * s; font.weight: Font.DemiBold }
            Repeater {
                model: [["look.ground", "Ground", [["night", "Night"], ["paper", "Paper"], ["sun", "Follow the sun"]], "night"],
                        ["look.apps_view", "Apps in the overview", [["icons", "Icons"], ["list", "List"]], "icons"],
                        ["look.motion", "Motion", [["gentle", "Gentle"], ["full", "Full"], ["off", "Off"]], "gentle"]]
                RowLayout {
                    required property var modelData
                    spacing: 14 * s
                    Text { Layout.preferredWidth: 200 * s; text: modelData[1]; color: c.text; font.pixelSize: 13.5 * s }
                    Repeater {
                        model: modelData[2]
                        Button {
                            required property var modelData
                            readonly property string key: parent.modelData[0]
                            readonly property bool on: (core.values[key] || parent.modelData[3]) === modelData[0]
                            implicitWidth: 120 * s; implicitHeight: 34 * s
                            onClicked: core.set(key, modelData[0])
                            Accessible.name: modelData[1]
                            background: Rectangle { radius: 8 * s; color: parent.on ? c.accent : (parent.hovered ? c.chrome : c.raised); border.color: parent.on ? c.accent : c.line }
                            contentItem: Text { text: modelData[1]; color: parent.on ? c.onAccent : c.text; font.pixelSize: 12.5 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                        }
                    }
                }
            }
            RowLayout {
                spacing: 14 * s
                Text { Layout.preferredWidth: 200 * s; text: "Size of everything"; color: c.text; font.pixelSize: 13.5 * s }
                Slider {
                    id: size
                    Layout.preferredWidth: 300 * s
                    from: 1.0; to: 2.5; stepSize: 0.25; snapMode: Slider.SnapAlways
                    value: core.values["look.scale"] || 1.0
                    onPressedChanged: if (!pressed) core.set("look.scale", value)
                    Accessible.name: "Size of everything"
                }
                Text { text: Math.round(size.value * 100) + " %"; color: c.dim; font.pixelSize: 13 * s }
            }
            Item { Layout.fillHeight: true }
        }

        // 3. Wi-Fi: the same page as in Settings
        Flickable {
            visible: win.step === 2
            anchors.fill: parent; contentHeight: wifiPage.implicitHeight; clip: true
            boundsBehavior: Flickable.StopAtBounds
            Loader { id: wifiPage; width: parent.width; active: win.step === 2; source: "pages/network.qml" }
        }

        // 4. done
        ColumnLayout {
            visible: win.step === 3
            anchors.fill: parent; spacing: 10 * s
            Text { text: "All set."; color: c.text; font.pixelSize: 26 * s; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; wrapMode: Text.Wrap; color: c.dim; font.pixelSize: 14 * s
                   text: "Press " + (win.rows.length ? win.rows[0].keys : "Super") + " for your windows and apps. Everything here is in Settings (" + (win.rows.length > 5 ? win.rows[5].keys : "Super+comma") + ")." }
            Item { Layout.fillHeight: true }
        }
    }

    // ── bottom: skip, back, next ─────────────────────────────────────────────────────────
    Text {
        id: note
        property bool bad: false
        x: 32 * s; anchors.bottom: parent.bottom; anchors.bottomMargin: 80 * s
        color: bad ? "#E0705A" : c.dim; font.pixelSize: 12 * s
        Timer { id: noteTimer; interval: 5000; onTriggered: note.text = "" }
    }
    RowLayout {
        x: 32 * s; width: parent.width - 64 * s
        anchors.bottom: parent.bottom; anchors.bottomMargin: 26 * s
        spacing: 10 * s
        Button {
            text: "Skip setup"
            onClicked: core.finishSetup()
            background: Rectangle { radius: 8 * s; color: parent.hovered ? c.chrome : "transparent" }
            contentItem: Text { text: parent.text; color: c.dim; font.pixelSize: 13 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
            implicitWidth: 110 * s; implicitHeight: 38 * s
        }
        Text { Layout.fillWidth: true; visible: win.step === 0 && win.twice.length > 0; text: "Two actions share a key: give one of them another."; color: "#E0705A"; font.pixelSize: 12.5 * s; horizontalAlignment: Text.AlignRight }
        Item { Layout.fillWidth: true; visible: !(win.step === 0 && win.twice.length > 0) }
        Button {
            visible: win.step > 0
            text: "Back"
            onClicked: win.step--
            implicitWidth: 100 * s; implicitHeight: 38 * s
            background: Rectangle { radius: 8 * s; color: parent.hovered ? c.chrome : c.raised; border.color: c.line }
            contentItem: Text { text: parent.text; color: c.text; font.pixelSize: 13 * s; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
        }
        Button {
            text: win.step === win.steps.length - 1 ? "Start" : "Next"
            enabled: !(win.step === 0 && win.twice.length > 0)
            implicitWidth: 120 * s; implicitHeight: 38 * s
            onClicked: {
                if (win.step === 0) core.saveKeys(win.preset, win.rows)
                if (win.step === win.steps.length - 1) core.finishSetup()
                else win.step++
            }
            background: Rectangle { radius: 8 * s; color: c.accent; opacity: parent.enabled ? 1 : 0.45 }
            contentItem: Text { text: parent.text; color: c.onAccent; font.pixelSize: 13 * s; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
        }
    }
}
