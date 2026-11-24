// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import WallFlux

/// The monitors as they are arranged, drawn to scale, each with the
/// wallpaper it shows, its number and its speaker -- Wallpaper Engine's
/// display picker. Used by the library's monitor chooser and by the
/// Monitors tab.
///
/// Read off the virtual desktop the screens share: a screen's `virtualX`
/// and `virtualY` are where the compositor has put it, so laying the
/// rectangles out by those is the arrangement somebody set up in their
/// display settings, without asking anybody about it.
Item {
    id: map

    /// Main.qml's root: the displays and what they show.
    required property var stage

    /// The monitor that is chosen, by connector name, ringed.
    property string selectedName: ""

    /// A monitor was clicked, by connector name.
    signal chosen(string name)

    /// A monitor was right-clicked, at this point of the map.
    signal contextAsked(string name, real x, real y)

    readonly property var screens: Application.screens

    readonly property rect bounds: {
        if (map.screens.length === 0)
            return Qt.rect(0, 0, 1, 1);
        let left = map.screens[0].virtualX;
        let top = map.screens[0].virtualY;
        let right = left + map.screens[0].width;
        let bottom = top + map.screens[0].height;
        for (let i = 1; i < map.screens.length; ++i) {
            const s = map.screens[i];
            left = Math.min(left, s.virtualX);
            top = Math.min(top, s.virtualY);
            right = Math.max(right, s.virtualX + s.width);
            bottom = Math.max(bottom, s.virtualY + s.height);
        }
        return Qt.rect(left, top, Math.max(1, right - left), Math.max(1, bottom - top));
    }

    /// What one pixel of desktop is worth here.
    readonly property real scale: Math.min(map.width / map.bounds.width,
                                           map.height / map.bounds.height)

    // As tall as the arrangement is at this width, and no taller than 220.
    implicitHeight: Math.min(220, map.width * map.bounds.height / map.bounds.width)

    Repeater {
        model: map.screens

        delegate: Rectangle {
            id: shape

            required property var modelData

            /// What draws on this monitor: its own display, or the clone
            /// unless it is left out of it.
            readonly property var shown: map.stage.displayOn(shape.modelData.name, map.stage.displays)
            readonly property string preview: shape.shown ? String(shape.shown.showing.current.preview) : ""
            readonly property bool chosen: map.selectedName === shape.modelData.name
            /// The speaker belongs to whatever plays the sound: each
            /// monitor's own, or the clone's source.
            readonly property bool hasSpeaker: !!shape.shown
                && (shape.shown.key !== "clone" || shape.shown.screens[0] === shape.modelData)

            x: (map.width - map.bounds.width * map.scale) / 2
               + (shape.modelData.virtualX - map.bounds.x) * map.scale
            y: (map.height - map.bounds.height * map.scale) / 2
               + (shape.modelData.virtualY - map.bounds.y) * map.scale
            width: Math.max(2, shape.modelData.width * map.scale - 4)
            height: Math.max(2, shape.modelData.height * map.scale - 4)

            radius: 6
            clip: true
            color: Qt.alpha(map.palette.windowText, 0.08)
            border.width: shape.chosen ? 3 : 1
            border.color: shape.chosen ? map.palette.highlight : map.palette.mid

            Image {
                anchors.fill: parent
                anchors.margins: shape.border.width
                source: shape.preview
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                opacity: shape.shown && shape.shown.covered ? 0.5 : 1
            }

            // The number, which is what the tray and the window call it.
            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.margins: 6
                width: 26
                height: 26
                radius: 13
                color: shape.chosen ? map.palette.highlight : Qt.rgba(0, 0, 0, 0.6)

                Label {
                    anchors.centerIn: parent
                    text: map.stage.monitorNumber(shape.modelData.name, map.stage.connectedNames)
                    color: "white"
                    font.weight: Font.Bold
                }
            }

            Label {
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottomMargin: 6
                width: parent.width - 12
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                text: shape.modelData.name
                color: "white"
                style: Text.Outline
                styleColor: Qt.rgba(0, 0, 0, 0.7)
                font.weight: Font.DemiBold
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: Qt.PointingHandCursor
                onClicked: (mouse) => {
                    if (mouse.button === Qt.RightButton)
                        map.contextAsked(shape.modelData.name, shape.x + mouse.x, shape.y + mouse.y);
                    else
                        map.chosen(shape.modelData.name);
                }
            }

            // Its sound, on or off, where it can be heard from.
            RoundedTool {
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 4
                visible: shape.hasSpeaker
                readonly property bool muted: shape.shown ? map.stage.isMuted(shape.shown.key, map.stage.mutedKeys) : false
                icon.name: muted ? "audio-volume-muted" : "audio-volume-high"
                icon.color: muted ? Look.negative : "white"
                ToolTip.visible: hovered
                ToolTip.text: muted ? qsTr("Unmute") : qsTr("Mute")
                onClicked: map.stage.setMuted(shape.shown.key, !muted)
            }
        }
    }
}
