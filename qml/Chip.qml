// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

import QtQuick
import QtQuick.Controls.Fusion

/// One value, on or off.
///
/// A Control with a rectangle behind it rather than a CheckBox: a checkable
/// Controls item that is told what to be and also clicked ends up fighting
/// its own binding, and a chip is what was asked for anyway.
Control {
    id: chip

    required property string label
    required property bool on

    signal asked()

    implicitHeight: 28
    leftPadding: 12
    rightPadding: 12
    // Inherited, so a section with nothing behind it can switch this off
    // and the mouse area inside goes with it.
    opacity: chip.enabled ? 1 : 0.4

    contentItem: Label {
        text: chip.label
        font.pixelSize: 12
        verticalAlignment: Text.AlignVCenter
        color: chip.on ? chip.palette.highlightedText : chip.palette.windowText
    }

    background: Rectangle {
        radius: height / 2
        color: chip.on ? chip.palette.highlight : "transparent"
        border.width: 1
        border.color: chip.on ? chip.palette.highlight : chip.palette.mid
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: chip.asked()
    }
}
