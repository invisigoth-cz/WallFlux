// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import WallFlux

/// One wallpaper in the picker's grid: the preview, and what a click on it
/// means.
///
/// The look is PreviewTile's, shared with the workshop so that a wallpaper
/// is drawn the same whether it is installed or not. What belongs to this
/// one is the band around it and what a click does.
Item {
    id: card

    /// The wallpaper this card is for.
    required property var wallpaper

    /// Asked for when this card is clicked, and only ever for a playable one.
    signal chose(string id)

    /// Whether this is one a click can do something with. A scene wallpaper
    /// is listed and nothing more.
    readonly property bool usable: card.wallpaper.playable

    // The whole card dims, not just the preview: a scene wallpaper is here
    // to say it exists, not to be chosen.
    opacity: card.usable ? 1 : 0.4

    PreviewTile {
        anchors.fill: parent
        preview: card.wallpaper.preview
        label: card.wallpaper.title
        clickable: card.usable
        // The band says whether this one can be played: green for yes, amber
        // for the scene wallpapers that are only listed. It replaces a dot in
        // the corner, which said the same thing in a place nobody looks.
        band: card.usable ? "#2ecc71" : "#e8a33d"
        onClicked: card.chose(card.wallpaper.id)
    }
}
