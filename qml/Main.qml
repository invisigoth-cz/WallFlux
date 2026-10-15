// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

import QtQuick
import QtQuick.Window
import QtWebEngine

Window {
    id: root

    visible: true
    visibility: Window.FullScreen
    // Frameless and bottom-most is as far as a plain window goes. Putting it
    // in the compositor's background layer needs the layer-shell protocol,
    // which Qt does not speak on its own; that comes later.
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnBottomHint
    color: "black"
    title: qsTr("WallFlux")

    WebEngineView {
        id: wallpaper

        anchors.fill: parent
        // Nothing to show until a wallpaper is picked: finding those in the
        // Steam workshop folder is the next step.
        url: "about:blank"
    }
}
