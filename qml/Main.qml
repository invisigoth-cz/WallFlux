// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

import QtQuick
import QtQuick.Window
import QtWebEngine
import WallFlux

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
        visible: Wallpapers.current.kind === WallpaperKind.Web
        url: visible ? Wallpapers.current.source : "about:blank"
    }

    // Stands in for the interface still to be built: without it there is no
    // way to see that the scan found anything.
    Text {
        anchors.centerIn: parent
        visible: !wallpaper.visible
        horizontalAlignment: Text.AlignHCenter
        color: "#808080"
        font.pixelSize: 20
        text: Wallpapers.wallpapers.length === 0
              ? qsTr("No Wallpaper Engine wallpapers found.")
              : qsTr("%1 wallpapers found, none playing.").arg(Wallpapers.wallpapers.length)
    }
}
