// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion

/// An icon on its own, with its corners taken off.
///
/// The same corners as RoundedButton, for the buttons that are a picture
/// and nothing else: Fusion shows nothing at all until one is pointed at
/// and then draws a square.
///
/// Note that Fusion leaves `icon.color` unset on a ToolButton, which draws
/// the icon in nothing at all -- so every one of these still has to say
/// what colour it is.
ToolButton {
    id: tool

    // The same room around the icon that RoundedButton leaves around its
    // words, so a row of the two does not look like two different sizes.
    padding: 8

    background: Rectangle {
        implicitHeight: 34
        radius: 7
        visible: tool.down || tool.hovered || tool.checked
        color: tool.down ? Qt.alpha(tool.palette.highlight, 0.5)
             : tool.checked ? Qt.alpha(tool.palette.highlight, 0.32)
                            : Qt.alpha(tool.palette.windowText, 0.14)
    }
}
