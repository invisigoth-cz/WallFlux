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

    /// Whether this is the one the arrow keys are on. Drawn as a ring
    /// round the whole card: the band along the preview already says
    /// whether it can be played, and a second band would be a riddle.
    property bool current: false

    /// Whether the tick box shows without the pointer being on the card --
    /// what Add wallpaper in the playlist strip switches on, so several
    /// can be ticked without hunting for the box on each one.
    property bool showTick: false

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
        //
        // Blue instead while the arrow keys are on this one. The band
        // changes colour rather than a ring being drawn round the card: a
        // ring is a second frame round the first and it takes the name in
        // with it, which looked like the card had been put in a box.
        // Nothing is lost by borrowing the band -- a wallpaper that cannot
        // be played is drawn at four tenths and says so that way too.
        band: card.current ? card.palette.highlight
                           : card.usable ? "#2ecc71" : "#e8a33d"
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
        visible: card.usable && (pointer.hovered || card.ticked || card.showTick)
        // White with a black edge, blue with a black edge when it is
        // ticked -- fixed colours rather than the palette's, because this
        // box sits on a picture and not on the window: against a dark
        // preview the window's own colours left it all but invisible.
        color: card.ticked ? "#3daee9" : "#ffffff"
        border.width: 1
        border.color: "#000000"

        Label {
            anchors.centerIn: parent
            text: "✓"
            font.pixelSize: 13
            font.weight: Font.Bold
            visible: card.ticked
            color: "#ffffff"
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: card.tickAsked()
        }
    }
}
