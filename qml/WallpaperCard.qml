// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Effects
import QtQuick.Layouts
import WallFlux

/// One wallpaper in the picker's grid: its preview, its name underneath, and
/// what is known about it once somebody looks.
///
/// The colours come from the window's palette rather than from anything named
/// here, which is what lets the whole picker change theme without a restart.
Item {
    id: card

    /// The wallpaper this card is for.
    required property var wallpaper

    /// How loud a video wallpaper plays, 0 to 1. Owned by the picker: a
    /// delegate is thrown away as soon as it scrolls out of sight, so
    /// nothing worth keeping can live in one.
    property real volume: 1

    /// Asked for when this card is clicked, and only ever for a playable one.
    signal chose(string id)
    /// Asked for while the slider in this card is being dragged.
    signal volumeAsked(real volume)

    /// Whether the sound is off altogether. Owned by the picker, like the
    /// volume, and shown here so the panel says the whole truth about what
    /// this wallpaper will sound like.
    property bool muted: false
    /// Asked for when the speaker in this card is clicked.
    signal muteAsked()

    /// Whether this is one a click can do something with. A scene wallpaper
    /// is listed and nothing more.
    readonly property bool usable: card.wallpaper.playable

    /// Whether the panel over the preview is showing.
    readonly property bool telling: card.usable && hover.containsMouse

    // The whole card dims, not just the preview: a scene wallpaper is here
    // to say it exists, not to be chosen.
    opacity: card.usable ? 1 : 0.4

    ColumnLayout {
        anchors.fill: parent
        spacing: 6

        Rectangle {
            id: frame

            Layout.fillWidth: true
            Layout.preferredHeight: Math.round(frame.width * 9 / 16)

            radius: 6
            color: card.palette.base
            // The band around the preview is what says whether this one can
            // be played: green for yes, amber for the scene wallpapers that
            // are only listed. It replaces a dot in the corner, which said
            // the same thing in a place nobody looks.
            border.width: 2
            border.color: card.usable ? "#2ecc71" : "#e8a33d"

            Image {
                id: preview

                anchors.fill: parent
                anchors.margins: frame.border.width
                source: card.wallpaper.preview
                fillMode: Image.PreserveAspectCrop
                // Fifty of these at once, read off a spinning disk: loaded in
                // the background so the grid scrolls while they arrive.
                asynchronous: true
                // Drawn by the effect below instead, which is what rounds the
                // corners off; a clip on the rectangle is square whatever its
                // radius says.
                visible: false
            }

            Rectangle {
                id: roundedPreview

                anchors.fill: preview
                radius: frame.radius - frame.border.width
                visible: false
                // Not decoration: without this the rectangle is not a
                // texture and the effect below has nothing to mask with, so
                // it draws nothing at all and every card comes up empty.
                // Measured side by side, with and without.
                layer.enabled: true
            }

            MultiEffect {
                anchors.fill: preview
                source: preview
                maskEnabled: true
                maskSource: roundedPreview
            }

            // Underneath the panel, so that the slider in it gets the mouse
            // first and a click anywhere else falls through to here.
            MouseArea {
                id: hover

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: card.usable ? Qt.PointingHandCursor : Qt.ForbiddenCursor
                onClicked: {
                    if (card.usable)
                        card.chose(card.wallpaper.id);
                }
            }

            Rectangle {
                anchors.fill: parent
                anchors.margins: frame.border.width
                radius: roundedPreview.radius
                color: Qt.alpha(card.palette.window, 0.88)

                opacity: card.telling ? 1 : 0
                visible: opacity > 0
                Behavior on opacity {
                    NumberAnimation { duration: 120 }
                }

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
                        // project.json carries no resolution -- checked
                        // across every one of them -- so a video's is read
                        // off the file by ffprobe shortly after the scan,
                        // and this says so plainly until it arrives or where
                        // there is no one answer to give.
                        text: card.wallpaper.resolution ? card.wallpaper.resolution
                                                        : qsTr("Native")
                        opacity: 0.75
                    }
                    Label {
                        text: card.wallpaper.sizeOnDisk
                        opacity: 0.75
                    }

                    Item { Layout.fillHeight: true }

                    // Only a video has a sound of its own to set. A web
                    // wallpaper makes whatever noise its JavaScript makes,
                    // and there is no volume on a page to turn down.
                    RowLayout {
                        Layout.fillWidth: true
                        visible: card.wallpaper.kind === WallpaperKind.Video
                        spacing: 6

                        ToolButton {
                            icon.name: card.muted ? "audio-volume-muted"
                                                  : "audio-volume-high"
                            // Fusion leaves a ToolButton's icon colour unset,
                            // which draws it in nothing at all. Measured.
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

        // On the grid's own background rather than in a box of its own: the
        // preview is the thing being looked at and a second frame around the
        // name only competes with it.
        Label {
            Layout.fillWidth: true
            text: card.wallpaper.title
            elide: Text.ElideRight
            maximumLineCount: 1
            font.weight: Font.DemiBold
        }
    }
}
