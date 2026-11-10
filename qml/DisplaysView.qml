// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Window
import WallFlux

/// Which screens the wallpaper goes on.
///
/// One wallpaper, drawn on the screens that are ticked. Not a wallpaper
/// per screen: that needs a player and a web view per screen at once, and
/// `Wallpapers.current` stops being one answer.
///
/// The screens are Qt's own -- `Qt.application.screens` -- so there is no
/// `xrandr`, no `kscreen-doctor` and nobody's output to parse. Qt names a
/// screen after its connector, which is the same name KDE's display
/// settings show and the same one under /sys/class/drm.
Item {
    id: view

    /// The names of the screens the wallpaper is wanted on. Every screen
    /// to begin with, which is what somebody who never opens this tab
    /// expects to happen.
    property var wanted: []

    /// Everything Qt can see, in the order it hands them over.
    readonly property var screens: Qt.application.screens

    function toggle(name) {
        view.wanted = view.wanted.indexOf(name) >= 0
            ? view.wanted.filter((kept) => kept !== name)
            : view.wanted.concat([name]);
    }

    /// How the screens sit next to each other, drawn to scale.
    ///
    /// Read off the virtual desktop they share: a screen's `virtualX` and
    /// `virtualY` are where the compositor has put it, so laying the
    /// rectangles out by those is the arrangement somebody set up in their
    /// display settings, without asking anybody about it.
    readonly property rect bounds: {
        if (view.screens.length === 0)
            return Qt.rect(0, 0, 1, 1);

        let left = view.screens[0].virtualX;
        let top = view.screens[0].virtualY;
        let right = left + view.screens[0].width;
        let bottom = top + view.screens[0].height;

        for (let i = 1; i < view.screens.length; ++i) {
            const s = view.screens[i];
            left = Math.min(left, s.virtualX);
            top = Math.min(top, s.virtualY);
            right = Math.max(right, s.virtualX + s.width);
            bottom = Math.max(bottom, s.virtualY + s.height);
        }
        return Qt.rect(left, top, Math.max(1, right - left), Math.max(1, bottom - top));
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 24
        anchors.rightMargin: 24
        anchors.bottomMargin: 12
        spacing: 16

        // The screens as they are arranged, which is the quickest way to
        // say which rectangle is which monitor -- a name on its own means
        // nothing to somebody looking at two identical screens.
        Item {
            id: picture

            Layout.fillWidth: true
            Layout.preferredHeight: 190

            /// What one pixel of desktop is worth here.
            readonly property real scale: Math.min(picture.width / view.bounds.width,
                                                   picture.height / view.bounds.height)

            Repeater {
                model: view.screens

                delegate: Rectangle {
                    id: shape

                    required property var modelData

                    readonly property bool on: view.wanted.indexOf(shape.modelData.name) >= 0

                    x: (picture.width - view.bounds.width * picture.scale) / 2
                       + (shape.modelData.virtualX - view.bounds.x) * picture.scale
                    y: (picture.height - view.bounds.height * picture.scale) / 2
                       + (shape.modelData.virtualY - view.bounds.y) * picture.scale
                    width: Math.max(2, shape.modelData.width * picture.scale - 4)
                    height: Math.max(2, shape.modelData.height * picture.scale - 4)

                    radius: 6
                    color: shape.on ? Qt.alpha(view.palette.highlight, 0.28)
                                    : Qt.alpha(view.palette.windowText, 0.08)
                    border.width: 2
                    border.color: shape.on ? view.palette.highlight : view.palette.mid

                    Label {
                        anchors.centerIn: parent
                        width: parent.width - 12
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: shape.modelData.name
                        font.weight: Font.DemiBold
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: view.toggle(shape.modelData.name)
                    }
                }
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: rows.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: rows

                width: parent.width - 18
                spacing: 8

                Repeater {
                    model: view.screens

                    delegate: Rectangle {
                        id: row

                        required property var modelData

                        readonly property bool on: view.wanted.indexOf(row.modelData.name) >= 0

                        Layout.fillWidth: true
                        implicitHeight: 62
                        radius: 8
                        color: row.palette.base
                        border.width: 1
                        border.color: row.palette.mid

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: view.toggle(row.modelData.name)
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 12

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Label {
                                    text: row.modelData.name
                                    font.pixelSize: 15
                                    font.weight: Font.Bold
                                }

                                Label {
                                    font.pixelSize: 12
                                    opacity: 0.75
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                    // What it is and where it is. The model
                                    // is what a person reads off the back of
                                    // the monitor; the position is what tells
                                    // two of the same thing apart.
                                    text: row.modelData.width + " × " + row.modelData.height
                                          + "  ·  " + qsTr("at %1, %2")
                                                          .arg(row.modelData.virtualX)
                                                          .arg(row.modelData.virtualY)
                                          + (row.modelData.model
                                                 ? "  ·  " + row.modelData.model : "")
                                }
                            }

                            // The same switch as Settings', told and answering.
                            Toggle {
                                value: row.on
                                onChanged: view.toggle(row.modelData.name)
                            }
                        }
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font.pixelSize: 11
            opacity: 0.8
            visible: view.wanted.length === 0
            // Said rather than refused: turning every screen off is a way
            // of turning the wallpaper off, and it should not look broken.
            text: qsTr("No screen is ticked, so no wallpaper is drawn. "
                       + "The desktop keeps whatever it had.")
        }
    }
}
