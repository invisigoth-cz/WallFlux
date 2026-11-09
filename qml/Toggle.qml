// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

import QtQuick
import QtQuick.Controls

/// A switch as they are drawn now: a pill in the accent with a round knob.
///
/// Told its state with `value` and answering with `changed`, rather than
/// being bound and toggled: see the trap about a Controls item that is both
/// told what to be and asked what it is.
Switch {
    id: toggle

    property bool value
    signal changed(bool to)

    /// Black or white, whichever reads on `c`.
    function ink(c) {
        return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) > 0.7 ? "#000000" : "#ffffff";
    }

    checked: toggle.value
    onToggled: {
        toggle.changed(toggle.checked);
        toggle.checked = Qt.binding(() => toggle.value);
    }

    // A pill in the accent with a round knob that slides, as switches
    // are drawn now, rather than Fusion's square one. No words beside
    // it, so no room kept for them either.
    padding: 0
    spacing: 0
    // Said outright: with the padding gone Fusion counts the indicator
    // nowhere, and a switch nought wide is drawn but cannot be clicked.
    implicitWidth: 44
    implicitHeight: 24

    indicator: Rectangle {
        implicitWidth: 44
        implicitHeight: 24
        x: 0
        y: (toggle.height - height) / 2
        radius: height / 2
        color: toggle.checked ? toggle.palette.highlight : Qt.alpha(toggle.palette.windowText, 0.16)
        border.width: toggle.checked ? 0 : 1
        border.color: toggle.palette.mid

        Behavior on color { ColorAnimation { duration: 160 } }

        Rectangle {
            width: 18
            height: 18
            radius: 9
            y: 3
            x: toggle.checked ? parent.width - width - 3 : 3
            scale: toggle.pressed ? 0.9 : 1
            color: toggle.checked ? toggle.ink(toggle.palette.highlight) : toggle.palette.base
            border.width: toggle.checked ? 0 : 1
            border.color: toggle.palette.mid

            Behavior on x { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
            Behavior on scale { NumberAnimation { duration: 100 } }
        }
    }
    contentItem: Item {}
}
