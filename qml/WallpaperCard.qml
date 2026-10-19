// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// One wallpaper in the picker's grid: the preview, and what is known about
/// it once somebody looks.
///
/// The look is PreviewTile's, shared with the workshop so that a wallpaper
/// is drawn the same whether it is installed or not. What belongs to this
/// one is what the panel says and what a click means.
Item {
    id: card

    /// The wallpaper this card is for.
    required property var wallpaper

    /// How loud a video wallpaper plays, 0 to 1. Owned by the picker: a
    /// delegate is thrown away as soon as it scrolls out of sight, so
    /// nothing worth keeping can live in one.
    property real volume: 1

    /// Whether the sound is off altogether. Owned by the picker, like the
    /// volume, and shown here so the panel says the whole truth about what
    /// this wallpaper will sound like.
    property bool muted: false

    /// Asked for when this card is clicked, and only ever for a playable one.
    signal chose(string id)
    /// Asked for while the slider in this card is being dragged.
    signal volumeAsked(real volume)
    /// Asked for when the speaker in this card is clicked.
    signal muteAsked()

    /// Whether this is one a click can do something with. A scene wallpaper
    /// is listed and nothing more.
    readonly property bool usable: card.wallpaper.playable

    // The whole card dims, not just the preview: a scene wallpaper is here
    // to say it exists, not to be chosen.
    opacity: card.usable ? 1 : 0.4

    PreviewTile {
        id: tile

        anchors.fill: parent
        preview: card.wallpaper.preview
        label: card.wallpaper.title
        clickable: card.usable
        // The band says whether this one can be played: green for yes, amber
        // for the scene wallpapers that are only listed. It replaces a dot in
        // the corner, which said the same thing in a place nobody looks.
        band: card.usable ? "#2ecc71" : "#e8a33d"
        telling: card.usable && tile.hovered
        onClicked: card.chose(card.wallpaper.id)

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 2

            Label {
                text: card.wallpaper.kind === WallpaperKind.Web
                      ? qsTr("Web") : qsTr("Video")
                font.weight: Font.DemiBold
            }
            Label {
                // project.json carries no resolution -- checked across every
                // one of them -- so a video's is read off the file by ffprobe
                // shortly after the scan, and this says so plainly until it
                // arrives or where there is no one answer to give.
                text: card.wallpaper.resolution ? card.wallpaper.resolution
                                                : qsTr("Native")
                opacity: 0.75
            }
            Label {
                text: card.wallpaper.sizeOnDisk
                opacity: 0.75
            }

            Item { Layout.fillHeight: true }

            // Only a video has a sound of its own to set. A web wallpaper
            // makes whatever noise its JavaScript makes, and there is no
            // volume on a page to turn down.
            RowLayout {
                Layout.fillWidth: true
                visible: card.wallpaper.kind === WallpaperKind.Video
                spacing: 6

                ToolButton {
                    icon.name: card.muted ? "audio-volume-muted"
                                          : "audio-volume-high"
                    // Fusion leaves a ToolButton's icon colour unset, which
                    // draws it in nothing at all. Measured.
                    icon.color: card.palette.windowText
                    onClicked: card.muteAsked()
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: 1
                    value: card.volume
                    onMoved: card.volumeAsked(value)
                }
            }
        }
    }
}
