// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

import QtQuick
import QtQuick.Controls.Fusion

/// The heart that makes a wallpaper a favourite on Steam, drawn on a
/// picture: a dark round backing so it reads on any preview, red and full
/// when it is on, an outline when it is not.
///
/// It says what it is and asks; whoever holds it decides what a click
/// does, so the state lives in one place -- `Steamworks.favorites`.
Rectangle {
    id: heart

    property bool on: false
    signal asked()

    implicitWidth: 26
    implicitHeight: 26
    radius: width / 2
    // Fixed colours: it sits on a picture, not on the window.
    color: area.containsMouse ? "#cc000000" : "#99000000"
    Behavior on color { ColorAnimation { duration: 120 } }

    Label {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: 1
        text: heart.on ? "♥" : "♡"
        font.pixelSize: 16
        color: heart.on ? "#ff4d6d" : "#ffffff"
        scale: area.pressed ? 0.85 : 1
        Behavior on scale { NumberAnimation { duration: 90 } }
    }

    MouseArea {
        id: area

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: heart.asked()
    }

    ToolTip.visible: area.containsMouse
    ToolTip.delay: 600
    ToolTip.text: heart.on ? qsTr("Remove from favourites") : qsTr("Add to favourites")
}
