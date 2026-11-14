// Behind everything: the one-time wallpaper for the ground in use (or the incognito decoy), the
// frame's right and bottom lines, this machine's cut lines, and the corner mark.
import QtQuick
import "../lib"

Rectangle {
    id: wall
    required property var core
    property string sealDir: ""
    // how far the frame's lines sit from this window's own edges (the bar and rail are other windows)
    property int frameLeft: 0
    property int frameTop: 0
    color: core.ground
    readonly property bool hideSeal: core.incognito && core.v("incognito.hide_seal") !== false
    readonly property bool safe: core.v("fix.safe_mode") === true

    Image {
        anchors.fill: parent
        visible: !wall.safe && core.v("look.wallpaper") !== "plain"
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: wall.width; sourceSize.height: wall.height
        cache: false
        source: {
            if (core.v("look.wallpaper") === "image" && core.v("look.wallpaper_image") && !wall.hideSeal) return "file://" + core.v("look.wallpaper_image")
            if (!wall.sealDir) return ""
            return "file://" + wall.sealDir + "/" + (wall.hideSeal ? "decoy-" : "wall-") + core.base + ".svg"
        }
        rotation: core.chaos ? 7 : 0
        scale: core.chaos ? 1.25 : 1
    }
    // the frame: right and bottom lines close the rectangle the bar and the rail begin
    Item {
        id: frame
        x: wall.frameLeft; y: wall.frameTop
        width: wall.width - wall.frameLeft - core.inset; height: wall.height - wall.frameTop - core.inset
        rotation: core.tilt(3) * 0.3
        Rectangle { anchors.right: parent.right; width: core.line; height: parent.height; color: core.ink }
        Rectangle { anchors.bottom: parent.bottom; height: core.line; width: parent.width; color: core.ink }
        // this machine's cuts: four short red marks whose places came from the seal
        Repeater {
            model: core.v("frame.cuts") !== false && !wall.hideSeal && core.status.seal && core.status.seal.frame_cuts ? core.status.seal.frame_cuts : []
            Rectangle {
                required property real modelData
                required property int index
                color: core.red
                width: index % 2 === 0 ? 3 : 18; height: index % 2 === 0 ? 18 : 3
                x: index % 2 === 0 ? frame.width - 2 : Math.round(modelData * (frame.width - 18))
                y: index % 2 === 0 ? Math.round(modelData * (frame.height - 18)) : frame.height - 2
            }
        }
    }
    // the corner mark
    Row {
        visible: core.v("look.corner_mark") !== false && core.mark !== ""
        anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.rightMargin: core.inset + 16; anchors.bottomMargin: core.inset + 16
        spacing: 12
        rotation: core.tilt(4)
        Rectangle {
            anchors.bottom: parent.bottom
            color: core.ground; border.width: core.line; border.color: core.ink
            width: col.implicitWidth + 16; height: col.implicitHeight + 12
            Column {
                id: col
                anchors.centerIn: parent
                Txt { core: wall.core; text: "ONE TIME · ONE MACHINE"; font.pixelSize: core.small; font.letterSpacing: 1.5; anchors.right: parent.right }
                Txt { core: wall.core; text: core.mark; heavy: true; font.pixelSize: core.small; font.letterSpacing: 1.5; anchors.right: parent.right }
            }
        }
        Rectangle {
            width: 64; height: 64; color: core.red
            Txt { core: wall.core; anchors.centerIn: parent; text: core.incognito ? "隠" : "影"; jp: true; heavy: true; color: core.redInk; font.pixelSize: 40 }
        }
    }
    // incognito: the strips that repeat themselves, and the way out, which stays straight
    Rectangle {
        visible: core.chaos
        x: -40; y: 40; width: parent.width + 80; height: 34; color: core.red; rotation: -1.5
        Txt { core: wall.core; anchors.fill: parent; color: core.redInk; heavy: true; font.letterSpacing: 3; text: "隠 INCOGNITO ".repeat(40); elide: Text.ElideNone; clip: true }
    }
    Rectangle {
        visible: core.incognito
        anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.leftMargin: wall.frameLeft + 16; anchors.bottomMargin: core.inset + 16
        color: core.ground; border.width: core.line; border.color: core.ink
        width: out.implicitWidth; height: core.hit + 12
        Row {
            id: out
            height: parent.height
            Column {
                anchors.verticalCenter: parent.verticalCenter; leftPadding: 14; rightPadding: 14
                Txt { core: wall.core; text: "隠 INCOGNITO IS ON"; heavy: true }
                Txt { core: wall.core; text: "Name, host, mark, wallpaper, titles and notices are hidden."; faint: true; font.pixelSize: core.small }
            }
            Cell { core: wall.core; height: parent.height; filled: true; text: "LEAVE · Mod+Shift+I"; onClicked: core.run(["samurai", "incognito", "off"]) }
        }
    }
}
