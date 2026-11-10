// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import WallFlux

/// A button with its corners taken off.
///
/// Fusion draws one as a near-square box, which next to the round chips and
/// the rounded previews made the buttons the only sharp thing in the
/// window. Only the background is ours; what the button does with an icon,
/// how it takes a click and what it is called are all still Fusion's.
Button {
    id: button

    /// The colour of a button that is the thing to do on the page it is
    /// on -- Download, and nothing else so far. Transparent leaves it
    /// looking like every other button, which is what all the rest want.
    property color accent: "transparent"

    // Room around the words. Fusion sets the text almost against the edge,
    // which on a rounded button looks like the corners are eating it.
    leftPadding: 16
    rightPadding: 16
    topPadding: 7
    bottomPadding: 7

    // Fusion's own content, with words that read on the accent: white on
    // Breeze's green, black on High Contrast Dark's.
    contentItem: IconLabel {
        readonly property color ink: button.accent.a > 0 ? Look.ink(button.accent)
                                                         : button.palette.buttonText

        spacing: button.spacing
        mirrored: button.mirrored
        display: button.display
        icon: button.icon
        defaultIconColor: ink
        text: button.text
        font: button.font
        color: ink
    }

    background: Rectangle {
        implicitHeight: 34
        radius: 7
        // A flat button is a word on the window background until it is
        // pointed at, which is the whole of being flat.
        visible: !button.flat || button.down || button.hovered
        readonly property color face: button.accent.a > 0 ? button.accent
                                                          : button.palette.button
        color: button.down ? Qt.darker(face, 1.25)
             : button.hovered ? Qt.lighter(face, 1.12)
                              : face
        border.width: button.flat ? 0 : 1
        border.color: button.visualFocus ? button.palette.highlight
                                         : button.palette.mid
        opacity: button.enabled ? 1 : 0.45
    }
}
