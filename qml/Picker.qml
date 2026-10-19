// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
// The Fusion style by name rather than through QQuickStyle: it is the one
// that takes a palette, and asking for it here keeps the choice beside the
// colours it is set with.
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// The window for choosing a wallpaper: everything installed, in a grid of
/// previews, and what the one that is playing sounds like.
///
/// Says what was clicked and nothing more -- it neither plays a wallpaper nor
/// remembers one. Which of the two is on screen is the wallpaper window's
/// business, and remembering is nobody's, by design.
Window {
    id: picker

    /// What was clicked. Never emitted for a wallpaper that cannot be played.
    signal chose(string id)

    /// Whether the sound is off. Read by the players in Main.qml, together
    /// with `volume`: the controls in this window are the setting itself, so
    /// there is one place it lives and nothing to keep in step.
    property bool muted: false

    /// How loud a video wallpaper plays, 0 to 1.
    ///
    /// One volume rather than one per wallpaper: a volume per wallpaper would
    /// have to be remembered between runs, and this program deliberately
    /// remembers nothing.
    property real volume: 1

    /// Which way the colours go. Dark to start with, because a wallpaper
    /// picker is a window full of pictures.
    property bool dark: true

    /// Whether the grid is in name order or reversed. The scan hands them
    /// over in name order already, so reversing is all there is to do.
    property bool byNameDescending: false

    /// Whether the scene wallpapers are left out. This is what the filter
    /// button does today; it is the only filter the backend can answer.
    property bool playableOnly: false

    /// The wallpapers the grid shows: what was typed, in the order asked for.
    readonly property var shown: {
        const needle = search.text.trim().toLowerCase();
        const all = Wallpapers.wallpapers;
        const kept = [];
        for (let i = 0; i < all.length; ++i) {
            const one = all[i];
            if (picker.playableOnly && !one.playable)
                continue;
            if (needle && !one.title.toLowerCase().includes(needle))
                continue;
            kept.push(one);
        }
        return picker.byNameDescending ? kept.reverse() : kept;
    }

    width: 1100
    height: 720
    // Wide enough for the whole of the header. Narrower than this and a
    // layout starts squeezing what is in it, which for a Controls item means
    // squeezing it down to nothing at all rather than clipping it.
    minimumWidth: 900
    minimumHeight: 420
    title: qsTr("WallFlux — choose a wallpaper")

    // Breeze's own two sets, so the window belongs on this desktop rather
    // than looking like a visitor. Set on the window because a palette
    // passes down to everything inside it -- which is the whole of the theme
    // switch: nothing below names a colour of its own.
    palette {
        window: picker.dark ? "#1b1e20" : "#eff0f1"
        windowText: picker.dark ? "#fcfcfc" : "#232629"
        base: picker.dark ? "#232629" : "#fcfcfc"
        text: picker.dark ? "#fcfcfc" : "#232629"
        button: picker.dark ? "#31363b" : "#fcfcfc"
        buttonText: picker.dark ? "#fcfcfc" : "#232629"
        mid: picker.dark ? "#3a4045" : "#c4c8cc"
        placeholderText: "#7f8c8d"
        highlight: "#3daee9"
        highlightedText: "#ffffff"
    }
    color: picker.palette.window

    // What every other window on this desktop does, and the one key a person
    // reaches for without being told.
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: picker.hide()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 24
            Layout.bottomMargin: 12
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                ColumnLayout {
                    spacing: 0

                    Label {
                        text: qsTr("Installed")
                        font.pixelSize: 30
                        font.weight: Font.DemiBold
                    }
                    Label {
                        text: qsTr("Wallpapers downloaded to your system")
                        opacity: 0.65
                    }
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: qsTr("Refresh")
                    icon.name: "view-refresh"
                    onClicked: Wallpapers.rescan()
                }

                Button {
                    // The icon is what the click will do, not what is on
                    // screen now: turn the lights up, or down.
                    //
                    // These two rather than a sun and a moon out of the
                    // weather icons: those are drawn for a 48-pixel applet
                    // and all that survives at 16 is a white disc. Looked
                    // at, not assumed. A Button rather than a ToolButton for
                    // the same reason the one beside it is: an icon alone on
                    // the window background says nothing about being a
                    // control at all.
                    icon.name: picker.dark ? "brightness-high" : "brightness-low"
                    ToolTip.visible: hovered
                    ToolTip.text: picker.dark ? qsTr("Light theme") : qsTr("Dark theme")
                    onClicked: picker.dark = !picker.dark
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                TextField {
                    id: search

                    Layout.fillWidth: true
                    Layout.maximumWidth: 420
                    placeholderText: qsTr("Search wallpapers…")
                    // Room for the magnifier that sits in it.
                    leftPadding: 34

                    ToolButton {
                        x: 4
                        anchors.verticalCenter: parent.verticalCenter
                        icon.name: "system-search"
                        // Fusion leaves a ToolButton's icon colour unset and
                        // the icon is then drawn in nothing at all. Measured
                        // against a Button, which sets it and shows up.
                        icon.color: search.palette.text
                        flat: true
                        focusPolicy: Qt.NoFocus
                        // Not decoration for its own sake: it is where
                        // somebody who clicks the magnifier expects to end up.
                        onClicked: search.forceActiveFocus()
                    }
                }

                Button {
                    text: qsTr("Filters")
                    icon.name: "view-filter"
                    checkable: true
                    checked: picker.playableOnly
                    // One filter, because one is all there is an answer for:
                    // leaving out what cannot be played. A button that opens
                    // a menu of nothing would be worse than this.
                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Show only wallpapers that can be played")
                    onToggled: picker.playableOnly = checked
                }

                Button {
                    text: qsTr("Name")
                    icon.name: picker.byNameDescending ? "view-sort-descending"
                                                       : "view-sort-ascending"
                    onClicked: picker.byNameDescending = !picker.byNameDescending
                }
            }
        }

        GridView {
            id: grid

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            clip: true

            // As many columns of about 240 as fit, then share the width out
            // between them: a fixed cell width leaves a ragged empty strip
            // down the side of every window that is not an exact multiple
            // of it.
            readonly property int columns: Math.max(1, Math.floor(grid.width / 240))

            cellWidth: Math.floor(grid.width / grid.columns)
            // The preview is sixteen by nine, and the name sits under it.
            cellHeight: Math.round((cellWidth - 16) * 9 / 16) + 46

            model: picker.shown

            delegate: Item {
                id: cell

                required property var modelData

                width: grid.cellWidth
                height: grid.cellHeight

                WallpaperCard {
                    anchors.fill: parent
                    anchors.margins: 8
                    wallpaper: cell.modelData
                    volume: picker.volume
                    muted: picker.muted
                    onChose: (id) => picker.chose(id)
                    onVolumeAsked: (volume) => picker.volume = volume
                    onMuteAsked: picker.muted = !picker.muted
                }
            }

            ScrollBar.vertical: ScrollBar {}
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            Layout.margins: 24
            visible: picker.shown.length === 0
            horizontalAlignment: Text.AlignHCenter
            opacity: 0.65
            text: Wallpapers.wallpapers.length === 0
                  ? qsTr("Nothing installed.\nWallpaper Engine's workshop folder is empty or missing.")
                  : qsTr("Nothing here by that name.")
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 38
            color: picker.palette.base

            // A hairline rather than a border, so the line is only along the
            // top where the grid ends.
            Rectangle {
                width: parent.width
                height: 1
                color: picker.palette.mid
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 12
                spacing: 8

                Label {
                    // The screen's own name, which is what the compositor
                    // calls the output this window is on.
                    text: qsTr("%1  |  %2").arg(Screen.name).arg(
                              Wallpapers.current.playable ? Wallpapers.current.title
                                                          : qsTr("No active wallpaper"))
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    opacity: 0.8
                }

                Button {
                    text: qsTr("Turn off")
                    flat: true
                    // Nothing to turn off until something plays, which is the
                    // state the program starts in.
                    enabled: Wallpapers.current.playable
                    onClicked: Wallpapers.clear()
                }

                ToolButton {
                    icon.name: picker.muted ? "audio-volume-muted" : "audio-volume-high"
                    // See the magnifier above: without this it is invisible.
                    icon.color: picker.palette.windowText
                    ToolTip.visible: hovered
                    ToolTip.text: picker.muted ? qsTr("Turn the sound on")
                                               : qsTr("Turn the sound off")
                    onClicked: picker.muted = !picker.muted
                }
            }
        }
    }
}
