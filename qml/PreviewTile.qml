// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Effects

/// A preview with its name across the bottom of it.
///
/// What a wallpaper looks like in this program, wherever it comes from: the
/// library grid and the workshop both use this, so neither can drift from
/// the other.
///
/// The shape is whoever draws it. The grid gives a square, which is what
/// Wallpaper Engine's own picker draws and what fits five across a window
/// rather than three; the details panel and the playlist strip give the
/// wallpaper's own sixteen by nine, where there is room for the whole of
/// the picture. Nothing in here asks for a height of its own any more --
/// it used to ask for nine sixteenths of its width whatever height it was
/// given, so `Layout.preferredHeight` did not shorten the picture, it let
/// the name be drawn over the bottom of it.
///
/// The colours come from the window's palette rather than from anything named
/// here, which is what lets the whole picker change theme without a restart.
Item {
    id: tile

    required property url preview
    required property string label

    /// The band around the preview. Transparent for no band at all.
    property color band: "transparent"

    /// Whether the pointer is over the preview. What the tick box on a
    /// card and the cross on a playlist's strip are shown by.
    readonly property alias hovered: hover.containsMouse

    /// Whether a click means anything here.
    property bool clickable: true

    signal clicked()
    /// Two clicks, which in the library means "put this one on the desktop".
    /// A double click delivers `clicked` first, so one picks the wallpaper
    /// out and the other acts on it, in that order.
    signal doubleClicked()

    /// The colour the card is drawn on, which the name's gradient fades
    /// into. Read once rather than in each of the three stops.
    readonly property color card: tile.palette.base

    Rectangle {
        id: frame

        anchors.fill: parent

        radius: 6
        color: tile.card
        border.width: tile.band.a > 0 ? 2 : 0
        border.color: tile.band

        // The picture goes in a box of its own, which crops it: that is
        // what lets it grow under the pointer without the card moving or
        // anything being pushed about. A child's scale goes into the
        // texture an effect reads, where the source item's own scale does
        // not, so the zoom has to be on something inside this.
        Item {
            id: picture

            anchors.fill: parent
            anchors.margins: frame.border.width
            clip: true
            // Drawn by the effect below instead, which is what rounds the
            // corners off; a clip on a rectangle is square whatever its
            // radius says.
            visible: false

            Image {
                id: shown

                anchors.fill: parent
                source: tile.preview
                fillMode: Image.PreserveAspectCrop
                // Fifty of these at once, read off a spinning disk: loaded in
                // the background so the grid scrolls while they arrive.
                asynchronous: true

                // The picture leans in under the pointer, a twentieth over
                // three tenths of a second. Only where a click means
                // something: the big tile in the details panel is a
                // picture and not an offer.
                scale: tile.clickable && hover.containsMouse ? 1.05 : 1
                Behavior on scale {
                    NumberAnimation {
                        duration: 300
                        easing.type: Easing.OutCubic
                    }
                }
            }
        }

        Rectangle {
            id: rounded

            anchors.fill: picture
            radius: frame.radius - frame.border.width
            visible: false
            // Not decoration: without this the rectangle is not a
            // texture and the effect below has nothing to mask with, so
            // it draws nothing at all and every card comes up empty.
            // Measured side by side, with and without.
            layer.enabled: true
        }

        MultiEffect {
            anchors.fill: picture
            source: picture
            maskEnabled: true
            maskSource: rounded
        }

        // The name across the bottom of the picture rather than in a row
        // under it, which is what makes a card a square and nothing else.
        // It lies over a gradient of the window's own base colour, so the
        // words are legible over a bright preview and the card still ends
        // in the colour it is drawn on.
        //
        // Outside the mask on purpose: text drawn into a texture and
        // masked comes back softened, and the square corners this leaves
        // over the rounded ones are the base colour on the base colour.
        //
        // Each stop names the base colour and changes only its alpha. A
        // stop of "transparent" is a transparent *black*, which the
        // gradient then interpolates through -- a dark bruise across the
        // bottom of every card in the light theme.
        Rectangle {
            id: scrim

            anchors.left: picture.left
            anchors.right: picture.right
            anchors.bottom: picture.bottom
            height: Math.min(64, Math.round(picture.height / 3))
            visible: tile.label !== ""
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: Qt.rgba(tile.card.r, tile.card.g, tile.card.b, 0)
                }
                GradientStop {
                    position: 0.45
                    color: Qt.rgba(tile.card.r, tile.card.g, tile.card.b, 0.6)
                }
                GradientStop { position: 1; color: tile.card }
            }
        }

        Label {
            anchors.left: scrim.left
            anchors.right: scrim.right
            anchors.bottom: scrim.bottom
            anchors.margins: 8
            // No name, no room taken: the details panel shows the picture
            // with the name in its own type underneath.
            visible: tile.label !== ""
            text: tile.label
            elide: Text.ElideRight
            maximumLineCount: 1
            font.weight: Font.DemiBold
        }

        MouseArea {
            id: hover

            anchors.fill: parent
            // What draws the cursor over the preview, which is the one
            // thing this says about whether a click is worth making.
            hoverEnabled: true
            // An ordinary arrow where a click means nothing, rather
            // than the crossed-out one: nothing is being refused any
            // more, since every card in the grid can at least be asked
            // about. The one tile that takes no click is the big one
            // in the details panel, which is a picture and not an
            // offer.
            cursorShape: tile.clickable ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: {
                if (tile.clickable)
                    tile.clicked();
            }
            onDoubleClicked: {
                if (tile.clickable)
                    tile.doubleClicked();
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
}
