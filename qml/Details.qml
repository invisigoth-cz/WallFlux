// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts

/// What is known about one wallpaper, in a panel beside the grid.
///
/// The same panel in the library and in the workshop: a picture, a name,
/// a list of what is known, and whatever can be done about it -- which is
/// handed in as children, since that is the one thing the two tabs do not
/// share. What is in the list is the caller's business too: the two know
/// different things about a wallpaper and neither has to pretend
/// otherwise.
Rectangle {
    id: panel

    required property url preview
    required property string title

    /// What is known, as `{ label, value }` in the order it should read.
    /// Whoever builds the list leaves out what it cannot answer rather
    /// than filling a row with a dash.
    required property var facts

    /// Whatever can be done about this one. One button, so far.
    default property alias actions: doing.data

    signal closed()

    implicitWidth: 300
    // Shown when something is clicked, which is the only way in.
    visible: false

    color: panel.palette.base
    border.color: panel.palette.mid
    border.width: 1
    radius: 8

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        PreviewTile {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.round(width * 9 / 16)
            preview: panel.preview
            // The name goes under this in its own type, where it can wrap.
            label: ""
            clickable: false
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            // As wide as the button on the other side, so the name sits in
            // the middle of the panel rather than in the middle of what is
            // left of it once the cross has taken its room.
            Item {
                implicitWidth: closer.implicitWidth
                implicitHeight: 1
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: panel.title
                font.pixelSize: 15
                font.weight: Font.Bold
                wrapMode: Text.Wrap
                maximumLineCount: 3
                elide: Text.ElideRight
            }

            RoundedTool {
                id: closer

                Layout.alignment: Qt.AlignTop
                icon.name: "window-close"
                icon.color: panel.palette.windowText
                ToolTip.visible: closer.hovered
                ToolTip.text: qsTr("Close")
                onClicked: panel.closed()
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: told.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller {}

            ColumnLayout {
                id: told

                // Short of the scrollbar, as the filter panels are.
                width: parent.width - 18
                spacing: 8

                Repeater {
                    model: panel.facts

                    delegate: ColumnLayout {
                        id: fact

                        required property var modelData

                        Layout.fillWidth: true
                        spacing: 1

                        Label {
                            text: fact.modelData.label
                            font.pixelSize: 11
                            font.letterSpacing: 1.1
                            font.weight: Font.Bold
                            font.capitalization: Font.AllUppercase
                            opacity: 0.6
                        }

                        Label {
                            Layout.fillWidth: true
                            text: fact.modelData.value
                            wrapMode: Text.Wrap
                        }
                    }
                }
            }
        }

        // Outside the Flickable, like Clear in the filters: what there is
        // to do about a wallpaper is not something to scroll for.
        ColumnLayout {
            id: doing

            Layout.fillWidth: true
            spacing: 6
        }
    }
}
