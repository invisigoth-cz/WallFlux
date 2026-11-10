// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion

/// A slider as they are drawn now: a thin track in the accent up to a round
/// handle that grows a little when held. Fusion's is a rounded square on a
/// grooved bar, which next to everything else here looked broken. The one
/// slider of this window -- the volume, a playlist's transition and a
/// wallpaper's own settings.
Slider {
    id: slider

    background: Rectangle {
        x: slider.leftPadding
        y: slider.topPadding + slider.availableHeight / 2 - height / 2
        implicitWidth: 160
        implicitHeight: 6
        width: slider.availableWidth
        height: 6
        radius: 3
        color: Qt.alpha(slider.palette.windowText, 0.15)

        Rectangle {
            width: slider.visualPosition * parent.width
            height: parent.height
            radius: 3
            color: slider.palette.highlight
        }
    }

    handle: Rectangle {
        x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
        y: slider.topPadding + slider.availableHeight / 2 - height / 2
        implicitWidth: 20
        implicitHeight: 20
        radius: width / 2
        scale: slider.pressed ? 1.15 : slider.hovered ? 1.07 : 1
        color: slider.palette.base
        border.width: 3
        border.color: slider.palette.highlight

        Behavior on scale { NumberAnimation { duration: 120 } }
    }
}
