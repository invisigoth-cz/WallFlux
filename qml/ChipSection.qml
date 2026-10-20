// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// A heading and the chips under it, wrapped onto as many rows as they need.
ColumnLayout {
    id: section

    required property string title
    /// `{ v, l }` for each: what it means and what it says.
    required property var values
    /// Which of those are on.
    required property var picked

    signal asked(var value)

    Layout.fillWidth: true
    spacing: 7
    // A heading over nothing helps nobody: a section whose values are not
    // carried by anything here does not appear at all.
    visible: section.values.length > 0

    Label {
        text: section.title
        font.pixelSize: 11
        font.letterSpacing: 1.2
        font.weight: Font.DemiBold
        opacity: 0.55
    }

    Flow {
        Layout.fillWidth: true
        spacing: 6

        Repeater {
            model: section.values

            delegate: Chip {
                required property var modelData

                label: modelData.l
                on: section.picked.indexOf(modelData.v) >= 0
                onAsked: section.asked(modelData.v)
            }
        }
    }
}
