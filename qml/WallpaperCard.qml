// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import QtQuick.Controls.Fusion
import WallFlux

/// One wallpaper in the picker's grid: the preview, and the click that
/// picks it out.
///
/// The look is PreviewTile's, shared with the workshop so that a wallpaper
/// is drawn the same whether it is installed or not. What belongs to this
/// one is the band around it and what a click does.
Item {
    id: card

    /// The wallpaper this card is for.
    required property var wallpaper

    /// Asked for when this card is clicked, whether it can be played or
    /// not: one that cannot be put on the desktop can still be read about
    /// and given back.
    signal chose(string id, int modifiers)

    /// Asked for when it is clicked twice, which is what puts a wallpaper
    /// on the desktop. One click used to do it, and with a panel to fill
    /// that made reading about a wallpaper change the desktop.
    signal applied(string id)

    /// Whether this one is in the playlist being put together.
    property bool ticked: false

    /// Whether its author has published a newer version than the one on
    /// the disk.
    property bool outdated: false

    /// Asked for when the tick box is clicked.
    signal tickAsked()

    /// Whether it is one of the account's favourites on Steam, and whether
    /// it can be one at all: only what came from the workshop, and only
    /// while Steam can be asked.
    property bool favorite: false
    property bool canFavorite: false

    /// Asked for when the heart is clicked.
    signal favoriteAsked()

    /// Whether this is the one the arrow keys are on. Drawn as a ring
    /// round the whole card: the band along the preview already says
    /// whether it can be played, and a second band would be a riddle.
    property bool current: false

    /// Among several chosen at once -- with Ctrl, Shift or a band -- to be
    /// removed together.
    property bool selected: false

    /// Whether the tick box shows without the pointer being on the card --
    /// what Add wallpaper in the playlist strip switches on, so several
    /// can be ticked without hunting for the box on each one.
    property bool showTick: false

    /// Whether this is one a click can do something with. One that is not
    /// is listed and nothing more.
    readonly property bool usable: card.wallpaper.playable

    // The whole card dims, not just the preview: a wallpaper that cannot be
    // played is here to say it exists, not to be chosen.
    opacity: card.usable ? 1 : 0.4

    /// The colour of the band when the pointer is somewhere else.
    readonly property color edge: card.current ? (Look.ring === "glow" ? Qt.lighter(Look.highlight, 1.25)
                                                : Look.ring === "focus" ? Qt.lighter(Look.highlight, 1.4)
                                                : Look.highlight)
                                : card.usable ? Look.playable : Look.listed

    /// How the theme marks the card the keyboard is on; see `ring` in Look.
    readonly property string ring: card.current ? Look.ring : ""

    // Grown, for the two styles that raise the card rather than ring it.
    scale: card.ring === "focus" ? 1.06 : card.ring === "lift" ? 1.03 : 1
    Behavior on scale { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

    /// Whether the pointer is anywhere on this card.
    ///
    /// A handler rather than the tile's own hover: the tick box has a
    /// mouse area of its own, and the moment the pointer crossed onto it
    /// the tile stopped being hovered, which hid the box, which put the
    /// pointer back on the tile -- a flicker several times a second.
    /// Handlers do not take the pointer away from one another.
    HoverHandler {
        id: pointer
    }

    // What marks the card the keyboard is on, in the theme's own way: the
    // band alone, two pixels of blue among a grid of green ones, was not
    // enough to find it by (the user's word of 10 November). Only that one
    // card has any of it, so only one effect is ever made.
    Loader {
        anchors.fill: parent
        active: card.ring !== ""
        sourceComponent: card.ring === "lift" ? lift
                       : card.ring === "outline" || card.ring === "double" ? line
                       : glow
    }

    // A halo in the accent: Dark's, and Steam's brighter and closer.
    Component {
        id: glow

        Item {
            Rectangle {
                id: halo

                anchors.fill: parent
                anchors.margins: card.ring === "focus" ? -3 : -5
                radius: 10
                // Lighter rather than the effect's `brightness`, which
                // lights its whole padding as a faint box.
                color: Qt.lighter(Look.highlight, card.ring === "focus" ? 1.6 : 1.3)
                visible: false
            }

            MultiEffect {
                anchors.fill: halo
                source: halo
                blurEnabled: true
                blur: 1
                blurMax: card.ring === "focus" ? 28 : 40
            }
        }
    }

    // Raised on a shadow: Light's, where a glow is lost against the window.
    Component {
        id: lift

        Item {
            Rectangle {
                id: slab

                anchors.fill: parent
                radius: 8
                color: "#000000"
                visible: false
            }

            MultiEffect {
                anchors.fill: slab
                source: slab
                shadowEnabled: true
                shadowColor: "#000000"
                shadowOpacity: 0.45
                shadowBlur: 1
                shadowVerticalOffset: 8
            }
        }
    }

    // A second line a gap outside the band: System's, as KDE draws focus,
    // and the high contrast pair's, hard and in the window's text colour.
    Component {
        id: line

        Rectangle {
            anchors.fill: parent
            anchors.margins: -5
            radius: 11
            color: "transparent"
            border.width: card.ring === "double" ? 3 : 2
            border.color: card.ring === "double" ? card.palette.windowText : Look.highlight
        }
    }

    PreviewTile {
        id: tile

        anchors.fill: parent
        preview: card.wallpaper.preview
        label: card.wallpaper.title
        // The band says whether this one can be played: green for yes, amber
        // for the ones that are only listed. It replaces a dot in
        // the corner, which said the same thing in a place nobody looks.
        //
        // Blue instead while the arrow keys are on this one. The band
        // changes colour rather than a ring being drawn round the card: a
        // ring is a second frame round the first and it takes the name in
        // with it, which looked like the card had been put in a box.
        // Nothing is lost by borrowing the band -- a wallpaper that cannot
        // be played is drawn at four tenths and says so that way too.
        // Lit up under the pointer, which is the card saying it is
        // ready to be clicked -- the preview is already leaning in, and
        // this is the rest of the same gesture. The band is borrowed for
        // it rather than a shadow being drawn, which would cost every
        // card a layer of its own.
        band: pointer.hovered ? Qt.lighter(card.edge, 1.4) : card.edge
        bandWidth: card.ring === "double" ? 5 : card.ring === "glow" ? 4
                   : card.ring === "" || card.ring === "outline" ? 2 : 3
        Behavior on band {
            ColorAnimation { duration: 150 }
        }
        onClicked: (modifiers) => card.chose(card.wallpaper.id, modifiers)
        onDoubleClicked: card.applied(card.wallpaper.id)
    }

    // Chosen with several others: the accent over the whole picture and a
    // tick in its corner, plainly another thing than the arrows' band and
    // the playlist's box.
    Rectangle {
        anchors.fill: parent
        visible: card.selected
        radius: 8
        color: Qt.alpha(Look.highlight, 0.28)
        border.width: 3
        border.color: Look.highlight

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 8
            width: 22
            height: 22
            radius: 11
            color: Look.highlight

            Label {
                anchors.centerIn: parent
                text: "✓"
                font.pixelSize: 13
                font.weight: Font.Bold
                color: Look.ink(Look.highlight)
            }
        }
    }

    /// The tick that puts this one in a playlist.
    ///
    /// Out of the way until the pointer is on the card, and then it stays
    /// as long as it is ticked. On every card, so that they all look alike
    /// -- the user's word of 24 November -- but greyed on a wallpaper that
    /// cannot be played, and saying so: a playlist of ones that cannot
    /// would be a quarter of an hour of nothing happening.
    Rectangle {
        id: box

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.margins: 10
        // Small, as tick boxes are; the click reaches a little past it.
        width: 16
        height: 16
        radius: 4
        visible: pointer.hovered || card.ticked || card.showTick
        opacity: card.usable ? 1 : 0.4
        // White with a black edge, blue with a black edge when it is
        // ticked -- fixed colours rather than the palette's, because this
        // box sits on a picture and not on the window: against a dark
        // preview the window's own colours left it all but invisible.
        color: card.ticked ? Look.highlight : "#ffffff"
        border.width: 1
        border.color: "#000000"

        Label {
            anchors.centerIn: parent
            text: "✓"
            font.pixelSize: 11
            font.weight: Font.Bold
            visible: card.ticked
            color: Look.ink(Look.highlight)
        }

        MouseArea {
            id: boxArea

            anchors.fill: parent
            anchors.margins: -5
            hoverEnabled: true
            cursorShape: card.usable ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: if (card.usable) card.tickAsked()

            ToolTip.visible: !card.usable && boxArea.containsMouse
            ToolTip.text: qsTr("WallFlux cannot play this one, so it cannot go in a playlist.")
        }
    }

    /// The heart, in the corner opposite the tick box: there while the
    /// pointer is on the card, and for good on a favourite.
    Heart {
        id: heart

        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 6
        visible: card.canFavorite && (pointer.hovered || card.favorite)
        on: card.favorite
        onAsked: card.favoriteAsked()
    }

    /// Said on the picture, in the corner the tick box leaves free, so a
    /// wallpaper with a newer version can be found by looking at the grid.
    Rectangle {
        anchors.top: parent.top
        anchors.right: heart.visible ? heart.left : parent.right
        anchors.topMargin: 10
        anchors.rightMargin: heart.visible ? 6 : 10
        visible: card.outdated
        implicitWidth: updateWord.implicitWidth + 14
        implicitHeight: 20
        radius: 10
        // The accent, which every theme keeps strong enough for a picture.
        color: Look.highlight

        Label {
            id: updateWord

            anchors.centerIn: parent
            text: qsTr("Update")
            font.pixelSize: 11
            font.weight: Font.Bold
            color: Look.ink(Look.highlight)
        }
    }
}
