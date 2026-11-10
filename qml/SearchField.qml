// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl

/// The one search field of this window, wherever there is something to
/// search: a pill with a magnifier at the start and a cross at the end once
/// something is typed. The library's and the settings' used to be drawn two
/// different ways (the user's word of 10 November).
TextField {
    id: field

    /// After the cross has emptied the field. `textChanged` says so as
    /// well; this is for whoever listens to `textEdited`, which the cross
    /// does not send.
    signal cleared()

    leftPadding: 36
    rightPadding: 34
    // Nothing anybody searches for here is longer, and a field that takes
    // a paragraph invites one.
    maximumLength: 50

    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 34
        radius: height / 2
        color: field.palette.base
        border.width: field.activeFocus ? 2 : 1
        border.color: field.activeFocus ? field.palette.highlight : field.palette.mid
        Behavior on border.color { ColorAnimation { duration: 120 } }
    }

    // A picture and not a button: a click on it lands on the field, which
    // is where somebody clicking a magnifier expects to end up.
    IconImage {
        x: 12
        anchors.verticalCenter: parent.verticalCenter
        width: 16
        height: 16
        name: "system-search"
        color: field.palette.text
        opacity: 0.7
    }

    RoundedTool {
        anchors.right: parent.right
        anchors.rightMargin: 3
        anchors.verticalCenter: parent.verticalCenter
        padding: 5
        icon.name: "edit-clear"
        icon.width: 16
        icon.height: 16
        // Fusion leaves a ToolButton's icon colour unset, and the icon is
        // then drawn in nothing at all.
        icon.color: field.palette.text
        flat: true
        focusPolicy: Qt.NoFocus
        // Nothing to clear until something is typed.
        visible: field.text.length > 0
        ToolTip.visible: hovered
        ToolTip.text: qsTr("Clear the search")
        onClicked: {
            field.clear();
            field.cleared();
        }
    }
}
