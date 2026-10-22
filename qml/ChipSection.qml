// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import QtQuick.Layouts
import WallFlux

/// A heading and the values under it, one to a row, and the heading folds
/// them away.
///
/// Rows rather than a flow of chips side by side: a flow puts a different
/// number on every line and every line ends somewhere else, and five
/// sections of that is a panel with no straight edge anywhere in it. Each
/// value keeps the shape of a chip and takes the width of the panel.
///
/// The workshop's filter carries fifty-nine of them, which is longer than
/// any window, so the heading is a switch: it turns to point down, the
/// chips slide out from under it, and a number beside it says how many of
/// them are ticked while they are folded away. A section nobody has opened
/// still says that much about itself.
ColumnLayout {
    id: section

    required property string title
    /// `{ v, l }` for each: what it means and what it says.
    required property var values
    /// Which of those are on.
    required property var picked

    /// Whether the chips are out.
    ///
    /// The short sections start open and the long ones folded, which is
    /// what makes the panel short without making it empty. Half a dozen
    /// is the line: the workshop's kinds and age ratings are six and
    /// three, which are the two anybody reaches for, and its nineteen
    /// picture sizes and twenty-five genres are what the folding is for.
    property bool open: section.values.length <= 6

    /// How many of this section's values are ticked, which is what a
    /// folded section has to be able to say.
    readonly property int chosenHere: {
        let count = 0;
        for (let i = 0; i < section.values.length; ++i) {
            if (section.picked.indexOf(section.values[i].v) >= 0)
                ++count;
        }
        return count;
    }

    signal asked(var value)



    Layout.fillWidth: true
    spacing: 4
    // A heading over nothing helps nobody: a section whose values are not
    // carried by anything here does not appear at all.
    visible: section.values.length > 0

    // The heading, which is also the switch. Handlers rather than a
    // MouseArea: a mouse area takes hover away from whatever is under it,
    // and the chips below have their own.
    Item {
        id: head

        Layout.fillWidth: true
        implicitHeight: 30

        HoverHandler { id: pointer }

        TapHandler {
            onTapped: section.open = !section.open
        }

        Rectangle {
            anchors.fill: parent
            radius: 6
            color: pointer.hovered ? Qt.alpha(head.palette.highlight, 0.16) : "transparent"
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 4
            anchors.rightMargin: 6
            spacing: 8

            // Turned rather than swapped for a second icon: a quarter turn
            // reads as the same thing moving, and two icons read as two
            // things.
            IconImage {
                name: "arrow-right"
                color: head.palette.windowText
                sourceSize.width: 14
                sourceSize.height: 14
                opacity: 0.8
                rotation: section.open ? 90 : 0

                Behavior on rotation {
                    NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
                }
            }

            Label {
                // Bigger and heavier than what it is a heading for. At the
                // same size as the values, and half faded, it sank into
                // them -- which in a panel of five sections is the one
                // thing worth seeing at a glance.
                Layout.fillWidth: true
                text: section.title
                font.pixelSize: 14
                font.letterSpacing: 1.1
                font.weight: Font.Bold
                font.capitalization: Font.AllUppercase
                opacity: 0.85
                elide: Text.ElideRight
            }

            // How many are ticked, for a section that is folded away. Not
            // shown while it is open, where the chips say it themselves.
            Rectangle {
                visible: section.chosenHere > 0 && !section.open
                implicitWidth: Math.max(20, count.implicitWidth + 12)
                implicitHeight: 20
                radius: 10
                color: head.palette.highlight

                Label {
                    id: count

                    anchors.centerIn: parent
                    text: section.chosenHere
                    font.pixelSize: 11
                    font.weight: Font.Bold
                    color: head.palette.highlightedText
                }
            }
        }
    }

    // The chips, and the room they take. Clipped and given a height so
    // that folding is something sliding out of sight rather than a panel
    // jumping to a new shape.
    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: section.open ? chips.implicitHeight : 0
        clip: true

        Behavior on Layout.preferredHeight {
            NumberAnimation { duration: 140; easing.type: Easing.OutCubic }
        }

        ColumnLayout {
            id: chips

            width: parent.width
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
}
