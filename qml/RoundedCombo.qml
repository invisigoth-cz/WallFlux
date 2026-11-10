// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl

/// A drop down with its corners taken off, as RoundedButton is a button:
/// the same face, edge and height, an arrow that turns over while the list
/// is open, and the list itself rounded. Fusion's square box was the one
/// sharp thing left in the panel (the user's word of 10 November).
///
/// Only the popup's background is replaced, never the popup: Fusion's popup
/// is what takes the drop down's width, and replacing a Menu's background
/// once left it twelve pixels wide.
ComboBox {
    id: box

    leftPadding: 12
    rightPadding: 34

    background: Rectangle {
        implicitWidth: 120
        implicitHeight: 34
        radius: 7
        color: box.down ? Qt.darker(box.palette.button, 1.25)
             : box.hovered ? Qt.lighter(box.palette.button, 1.12)
                           : box.palette.button
        border.width: 1
        border.color: box.visualFocus || box.popup.visible ? box.palette.highlight
                                                          : box.palette.mid
        opacity: box.enabled ? 1 : 0.45
    }

    indicator: IconImage {
        x: box.width - width - 12
        y: (box.height - height) / 2
        width: 12
        height: 12
        name: "arrow-down"
        color: box.palette.buttonText
        rotation: box.popup.visible ? 180 : 0
        Behavior on rotation { NumberAnimation { duration: 150 } }
    }

    popup.padding: 4
    popup.topMargin: 6
    popup.bottomMargin: 6
    popup.background: Rectangle {
        radius: 8
        color: box.palette.window
        border.color: box.palette.mid
    }
}
