// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Effects
import QtQuick.Layouts

/// A preview with a name under it.
///
/// What a wallpaper looks like in this program, wherever it comes from: the
/// library grid and the workshop both use this, so neither can drift from
/// the other.
///
/// The colours come from the window's palette rather than from anything named
/// here, which is what lets the whole picker change theme without a restart.
Item {
    id: tile

    required property url preview
    required property string label

    /// The band around the preview. Transparent for no band at all.
    property color band: "transparent"

    /// Whether a click means anything here.
    property bool clickable: true

    signal clicked()

    ColumnLayout {
        anchors.fill: parent
        spacing: 6

        Rectangle {
            id: frame

            Layout.fillWidth: true
            Layout.preferredHeight: Math.round(frame.width * 9 / 16)

            radius: 6
            color: tile.palette.base
            border.width: tile.band.a > 0 ? 2 : 0
            border.color: tile.band

            Image {
                id: shown

                anchors.fill: parent
                anchors.margins: frame.border.width
                source: tile.preview
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
                id: rounded

                anchors.fill: shown
                radius: frame.radius - frame.border.width
                visible: false
                // Not decoration: without this the rectangle is not a
                // texture and the effect below has nothing to mask with, so
                // it draws nothing at all and every card comes up empty.
                // Measured side by side, with and without.
                layer.enabled: true
            }

            MultiEffect {
                anchors.fill: shown
                source: shown
                maskEnabled: true
                maskSource: rounded
            }

            MouseArea {
                anchors.fill: parent
                // What draws the cursor over the preview, which is the one
                // thing this says about whether a click is worth making.
                hoverEnabled: true
                cursorShape: tile.clickable ? Qt.PointingHandCursor : Qt.ForbiddenCursor
                onClicked: {
                    if (tile.clickable)
                        tile.clicked();
                }
            }

            // A preview off the workshop is a few hundred kilobytes over
            // the network, which is long enough that an empty frame reads
            // as a wallpaper with no picture rather than as one on its way.
            BusyIndicator {
                anchors.centerIn: parent
                implicitWidth: 32
                implicitHeight: 32
                running: shown.status === Image.Loading
                visible: running
            }
        }

        // On the grid's own background rather than in a box of its own: the
        // preview is the thing being looked at and a second frame around the
        // name only competes with it.
        Label {
            Layout.fillWidth: true
            text: tile.label
            elide: Text.ElideRight
            maximumLineCount: 1
            font.weight: Font.DemiBold
        }
    }
}
