// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

// The delegate below is a component of its own, and without this it may not
// name the grid or the window it sits in.
pragma ComponentBehavior: Bound

import QtQuick
// The Fusion style by name rather than through QQuickStyle: it is the one
// that takes a palette, and asking for it here keeps the choice beside the
// colours it is set with.
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// The window for choosing a wallpaper: everything installed, in a grid of
/// the preview images Wallpaper Engine ships beside each one.
///
/// A window of its own rather than a panel inside the wallpaper. The
/// wallpaper is a layer-shell surface in the compositor's background that
/// lets every click fall through to the desktop, so nothing drawn there can
/// be clicked on -- by design.
Window {
    id: picker

    /// This one was clicked. Handled outside, because a choice is also
    /// something to remember, and remembering is not this window's business.
    signal chose(string id)

    /// Whether the sound is off. Read by the players in Main.qml: the tick
    /// in this window is the setting itself, so there is one place it lives
    /// and nothing to keep in step.
    readonly property alias muted: mute.checked

    width: 960
    height: 640
    // Wide enough for the whole of the bar along the bottom. Narrower than
    // this and a layout starts squeezing what is in it, which for a Controls
    // item means squeezing it down to nothing at all rather than clipping it.
    minimumWidth: 720
    minimumHeight: 320
    color: "#1b1b1b"
    title: qsTr("WallFlux — choose a wallpaper")

    // What every other window on this desktop does, and the one key a person
    // reaches for without being told.
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: picker.hide()
    }

    GridView {
        id: grid

        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
            bottom: bar.top
            margins: 12
        }
        // As many columns of about 240 as fit, then share the width out
        // between them: a fixed cell width leaves a ragged empty strip down
        // the side of every window that is not an exact multiple of it.
        readonly property int columns: Math.max(1, Math.floor(grid.width / 240))

        cellWidth: Math.floor(grid.width / grid.columns)
        cellHeight: Math.round(grid.cellWidth * 2 / 3)
        clip: true
        model: Wallpapers.wallpapers

        delegate: Item {
            id: tile

            required property var modelData

            readonly property bool current: tile.modelData.id === Wallpapers.current.id

            width: grid.cellWidth
            height: grid.cellHeight

            Rectangle {
                anchors.fill: parent
                anchors.margins: 6
                radius: 3
                color: "#262626"
                // The one that is playing says so with its frame; a grid of
                // fifty needs somewhere for the eye to come back to.
                border.width: tile.current ? 2 : 1
                border.color: tile.current ? "#4d9fd6" : "#3a3a3a"

                Image {
                    anchors.fill: parent
                    anchors.margins: 1
                    source: tile.modelData.preview
                    // Decoded at the size it is drawn at, not at the 1080p it
                    // is stored at: fifty of those at once is a lot of memory
                    // for thumbnails. A GIF preview shows its first frame,
                    // which is all a thumbnail is for.
                    sourceSize.width: grid.cellWidth
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    // Dimmed rather than hidden: a scene wallpaper is worth
                    // seeing in the list even though nothing here can play it.
                    opacity: tile.modelData.playable ? 1 : 0.3
                }

                // Over the bottom of the preview, on a dark band, so the name
                // stays readable whatever the image underneath happens to be.
                Rectangle {
                    anchors {
                        left: parent.left
                        right: parent.right
                        bottom: parent.bottom
                        margins: 1
                    }
                    height: name.implicitHeight + 10
                    color: "#cc000000"

                    Text {
                        id: name

                        anchors.fill: parent
                        anchors.margins: 5
                        color: tile.modelData.playable ? "#f0f0f0" : "#8c8c8c"
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        text: tile.modelData.playable
                              ? tile.modelData.title
                              : qsTr("%1 — cannot be played yet").arg(tile.modelData.title)
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    // Clicking a scene wallpaper would only blank the screen,
                    // so it does nothing at all.
                    enabled: tile.modelData.playable
                    cursorShape: Qt.PointingHandCursor
                    onClicked: picker.chose(tile.modelData.id)
                }
            }
        }
    }

    // Everything that is not choosing a wallpaper. It lives here rather than
    // in the tray icon's menu: the menu is for reaching this window, and
    // what the wallpaper does belongs with the wallpapers.
    Pane {
        id: bar

        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        // Fusion draws itself from the palette, so the dark colours of this
        // window are a few names rather than a style sheet. Set on the Pane
        // because a palette passes down to what is inside it.
        palette {
            window: "#222222"
            windowText: "#e0e0e0"
            base: "#1b1b1b"
            button: "#333333"
            buttonText: "#e0e0e0"
            text: "#e0e0e0"
            highlight: "#4d9fd6"
            mid: "#3a3a3a"
        }

        // A layout rather than anchors, and the only child: a Pane takes its
        // height from one child that has a height of its own, and anchored
        // children have none to give -- which left the row sliced in half.
        RowLayout {
            // Stretched across the pane, which is what gives the spacer
            // below something to fill; the height stays its own, so the pane
            // can still work out how tall it has to be.
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: 8

            CheckBox {
                id: mute

                text: qsTr("Mute the sound")
                // Without a floor of its own a layout may give this a width
                // of zero, and a tick nobody can see is worse than one that
                // is in the way.
                Layout.minimumWidth: implicitWidth
            }

            // Pushes the buttons to the far end.
            Item {
                Layout.fillWidth: true
            }

            Button {
                text: qsTr("Look for new wallpapers")
                onClicked: Wallpapers.rescan()
            }

            Button {
                text: qsTr("Turn the wallpaper off")
                // Nothing to turn off until something plays, which is the
                // state the program starts in.
                enabled: Wallpapers.current.playable
                onClicked: Wallpapers.clear()
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: Wallpapers.wallpapers.length === 0
        color: "#787878"
        font.pixelSize: 15
        horizontalAlignment: Text.AlignHCenter
        text: qsTr("Nothing installed.\nWallpaper Engine's workshop folder is empty or missing.")
    }
}
