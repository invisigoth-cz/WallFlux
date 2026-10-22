// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts

/// The playlist being put together, along the bottom of the window.
///
/// It is there from the moment the window opens, empty or not: the strip is
/// where a playlist is made, and a place to put something has to be visible
/// before there is anything in it. Ten wallpapers to a row, as many rows as
/// it takes, and above them what can be done with the lot.
Rectangle {
    id: bar

    /// The wallpapers in it, in the order they were ticked.
    required property var items

    /// Whether the grid is showing every tick box rather than the one
    /// under the pointer -- which is what Add wallpaper switches.
    required property bool adding

    /// Whether there is a playlist written down somewhere. Handed in
    /// rather than read here: this strip draws what it is given and knows
    /// nothing about where playlists are kept.
    required property bool anySaved

    /// How many go in a row. Twenty, which is half the preview the strip
    /// used to draw -- asked for after looking at ten.
    readonly property int across: 20

    signal dropped(string id)
    signal loadAsked()
    signal saveAsked()
    signal configureAsked()
    signal addAsked()

    /// One preview is a tenth of the room, less the gaps between them.
    readonly property int small:
        Math.max(40, Math.floor((bar.width - 24 - (bar.across - 1) * 6) / bar.across))

    /// Three rows at most before it scrolls: this is a strip along the
    /// bottom, not a second grid.
    readonly property int rows:
        Math.max(1, Math.ceil(bar.items.length / bar.across))

    /// The strip is its words and its buttons until something is ticked;
    /// the room for previews comes with the first one and goes with the
    /// last. An empty tray of a fixed height is a gap with nothing to say.
    implicitHeight: bar.items.length === 0
        ? 12 + buttons.implicitHeight + 12
        : 12 + buttons.implicitHeight + 10
          + Math.min(3, bar.rows) * Math.round(bar.small * 9 / 16 + 6) + 12

    color: bar.palette.base
    border.color: bar.palette.mid
    border.width: 1
    radius: 8

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        RowLayout {
            id: buttons

            Layout.fillWidth: true
            spacing: 8

            Label {
                text: qsTr("Playlists")
                font.pixelSize: 15
                font.weight: Font.Bold
            }

            RoundedButton {
                text: qsTr("Load")
                icon.name: "document-open"
                onClicked: bar.loadAsked()
            }

            RoundedButton {
                text: qsTr("Save")
                icon.name: "document-save"
                accent: "#2e9e5b"
                // Nothing to write down until something is in it.
                enabled: bar.items.length > 0
                onClicked: bar.saveAsked()
            }

            RoundedButton {
                text: qsTr("Configure")
                icon.name: "settings-configure"
                // Nothing to set up until there is a playlist to set up:
                // either one being put together here, or one already
                // saved that Load can bring back.
                enabled: bar.items.length > 0 || bar.anySaved
                onClicked: bar.configureAsked()
            }

            RoundedButton {
                text: qsTr("Add wallpaper")
                icon.name: "list-add"
                // Held in while it is on, so the grid full of tick boxes
                // has something saying why.
                accent: bar.adding ? bar.palette.highlight : "transparent"
                onClicked: bar.addAsked()
            }

            Item { Layout.fillWidth: true }

            Label {
                font.pixelSize: 12
                opacity: 0.8
                text: qsTr("Wallpapers: %1").arg(bar.items.length)
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            visible: bar.items.length > 0
            contentWidth: width
            contentHeight: held.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller {}

            Flow {
                id: held

                width: parent.width - 18
                spacing: 6

                Repeater {
                    model: bar.items

                    delegate: PreviewTile {
                        id: little

                        required property var modelData

                        width: bar.small
                        height: Math.round(bar.small * 9 / 16)
                        preview: little.modelData.preview
                        // The name would not fit in a tenth of the window,
                        // and the picture is what somebody recognises.
                        label: ""
                        onClicked: bar.dropped(little.modelData.id)

                        ToolTip.visible: little.hovered
                        ToolTip.text: qsTr("%1 — click to take it out")
                                          .arg(little.modelData.title)

                        // A cross over the picture while the pointer is on
                        // it, so the click has something to say for itself.
                        Rectangle {
                            anchors.top: parent.top
                            anchors.right: parent.right
                            anchors.margins: 3
                            width: 16
                            height: 16
                            radius: 4
                            visible: little.hovered
                            color: Qt.alpha(little.palette.window, 0.85)

                            Label {
                                anchors.centerIn: parent
                                text: "✕"
                                font.pixelSize: 10
                                color: little.palette.windowText
                            }
                        }
                    }
                }
            }
        }
    }
}
