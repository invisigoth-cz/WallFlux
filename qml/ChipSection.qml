// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// A heading and the values under it, one to a row.
///
/// Rows rather than a flow of chips side by side: a flow puts a different
/// number on every line and every line ends somewhere else, and five
/// sections of that is a panel with no straight edge anywhere in it. Each
/// value keeps the shape of a chip and takes the width of the panel.
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
        // Bigger and heavier than what it is a heading for. At the same
        // size as the values, and half faded, it sank into them -- which
        // in a panel of five sections is the one thing worth seeing at a
        // glance.
        text: section.title
        font.pixelSize: 14
        font.letterSpacing: 1.1
        font.weight: Font.Bold
        font.capitalization: Font.AllUppercase
        opacity: 0.85
        bottomPadding: 2
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 5

        Repeater {
            model: section.values

            delegate: Chip {
                required property var modelData

                Layout.fillWidth: true
                label: modelData.l
                on: section.picked.indexOf(modelData.v) >= 0
                onAsked: section.asked(modelData.v)
            }
        }
    }
}
