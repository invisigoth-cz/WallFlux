// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
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
    /// not: a scene wallpaper cannot be put on the desktop but can still
    /// be read about and given back.
    signal chose(string id)

    /// Asked for when it is clicked twice, which is what puts a wallpaper
    /// on the desktop. One click used to do it, and with a panel to fill
    /// that made reading about a wallpaper change the desktop.
    signal applied(string id)

    /// Whether this one is in the playlist being put together.
    property bool ticked: false

    /// Asked for when the tick box is clicked.
    signal tickAsked()

    /// Whether this is one a click can do something with. A scene wallpaper
    /// is listed and nothing more.
    readonly property bool usable: card.wallpaper.playable

    // The whole card dims, not just the preview: a scene wallpaper is here
    // to say it exists, not to be chosen.
    opacity: card.usable ? 1 : 0.4

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

    PreviewTile {
        id: tile

        anchors.fill: parent
        preview: card.wallpaper.preview
        label: card.wallpaper.title
        // The band says whether this one can be played: green for yes, amber
        // for the scene wallpapers that are only listed. It replaces a dot in
        // the corner, which said the same thing in a place nobody looks.
        band: card.usable ? "#2ecc71" : "#e8a33d"
        onClicked: card.chose(card.wallpaper.id)
        onDoubleClicked: card.applied(card.wallpaper.id)
    }

    /// The tick that puts this one in a playlist.
    ///
    /// Out of the way until the pointer is on the card, and then it stays
    /// as long as it is ticked. Only on a wallpaper that can be played: a
    /// playlist of scene wallpapers would be a quarter of an hour of
    /// nothing happening.
    Rectangle {
        id: box

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.margins: 14
        width: 22
        height: 22
        radius: 6
        visible: card.usable && (pointer.hovered || card.ticked)
        color: card.ticked ? card.palette.highlight
                           : Qt.alpha(card.palette.window, 0.8)
        border.width: 1
        border.color: card.ticked ? card.palette.highlight : card.palette.mid

        Label {
            anchors.centerIn: parent
            text: "✓"
            font.pixelSize: 13
            visible: card.ticked
            color: card.palette.highlightedText
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: card.tickAsked()
        }
    }
}
