// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion

/// A scrollbar that can be seen.
///
/// Fusion's own fades away whenever it is not being touched, which on a page
/// of pictures means it is never there when somebody looks for it -- the
/// grid ran over the bottom edge and nothing on screen said so. This one
/// stays, is wide enough to aim at, and darkens under the pointer so it
/// reads as something to take hold of.
///
/// Both ways round, so the sideways strips in Discover get the same one:
/// which of the two implicit sizes is used is the orientation's business.
ScrollBar {
    id: bar

    policy: ScrollBar.AlwaysOn
    // A handle for one page in sixty-five thousand is a dot nobody can
    // hit. This is the least of the bar it is ever drawn as.
    minimumSize: 0.08

    contentItem: Rectangle {
        implicitWidth: 9
        implicitHeight: 9
        radius: 4.5
        color: bar.pressed ? bar.palette.highlight
             : bar.hovered ? Qt.alpha(bar.palette.windowText, 0.8)
                           : Qt.alpha(bar.palette.windowText, 0.5)
    }

    background: Rectangle {
        implicitWidth: 9
        implicitHeight: 9
        radius: 4.5
        color: Qt.alpha(bar.palette.windowText, 0.12)
    }
}
