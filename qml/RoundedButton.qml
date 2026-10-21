// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion

/// A button with its corners taken off.
///
/// Fusion draws one as a near-square box, which next to the round chips and
/// the rounded previews made the buttons the only sharp thing in the
/// window. Only the background is ours; what the button does with an icon,
/// how it takes a click and what it is called are all still Fusion's.
Button {
    id: button

    // Room around the words. Fusion sets the text almost against the edge,
    // which on a rounded button looks like the corners are eating it.
    leftPadding: 16
    rightPadding: 16
    topPadding: 7
    bottomPadding: 7

    background: Rectangle {
        implicitHeight: 34
        radius: 7
        // A flat button is a word on the window background until it is
        // pointed at, which is the whole of being flat.
        visible: !button.flat || button.down || button.hovered
        color: button.down ? Qt.darker(button.palette.button, 1.25)
             : button.hovered ? Qt.lighter(button.palette.button, 1.12)
                              : button.palette.button
        border.width: button.flat ? 0 : 1
        border.color: button.visualFocus ? button.palette.highlight
                                         : button.palette.mid
        opacity: button.enabled ? 1 : 0.45
    }
}
