// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts

/// The playlist being put together, along the bottom of the window.
///
/// Ten wallpapers to a row, as many rows as it takes, and under them what
/// can be done with the lot. It appears when the first wallpaper is ticked
/// and goes when the last is unticked -- the same as the panel down the
/// side, which appears when there is something to say.
Rectangle {
    id: bar

    /// The wallpapers in it, in the order they were ticked.
    required property var items

    /// How many go in a row. Ten, as asked for.
    readonly property int across: 10

    signal dropped(string id)
    signal saveAsked()
    signal configureAsked()

    /// One preview is a tenth of the room, less the gaps between them.
    readonly property int small:
        Math.max(40, Math.floor((bar.width - 24 - (bar.across - 1) * 6) / bar.across))

    /// Three rows at most before it scrolls: this is a strip along the
    /// bottom, not a second grid.
    readonly property int rows:
        Math.max(1, Math.ceil(bar.items.length / bar.across))

    implicitHeight: 12 + Math.min(3, bar.rows) * Math.round(bar.small * 9 / 16 + 6)
                    + 10 + buttons.implicitHeight + 12

    color: bar.palette.base
    border.color: bar.palette.mid
    border.width: 1
    radius: 8

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

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

        RowLayout {
            id: buttons

            Layout.fillWidth: true
            spacing: 10

            Label {
                text: qsTr("Playlist (%1)").arg(bar.items.length)
                font.pixelSize: 15
                font.weight: Font.Bold
            }

            Item { Layout.fillWidth: true }

            RoundedButton {
                text: qsTr("Configure")
                icon.name: "settings-configure"
                onClicked: bar.configureAsked()
            }

            RoundedButton {
                text: qsTr("Save playlist")
                icon.name: "document-save"
                accent: "#2e9e5b"
                onClicked: bar.saveAsked()
            }
        }
    }
}
