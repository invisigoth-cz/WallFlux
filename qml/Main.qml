// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

import QtQuick
import QtQuick.Window
import QtWebEngine
import WallFlux

Window {
    id: root

    /// Whether what is chosen is of a kind this program can draw today.
    readonly property bool playing: Wallpapers.current.kind === WallpaperKind.Web

    // Layer-shell takes the size from its anchors, so no fullscreen state is
    // asked for. These two are what the window falls back to where there is
    // no layer-shell, such as under X11.
    width: Screen.width
    height: Screen.height

    // Transparent for input is what lets a click fall through to the desktop
    // underneath; without it the wallpaper would swallow every one.
    flags: Qt.FramelessWindowHint | Qt.WindowTransparentForInput

    // Shown by main(), not here: the layer-shell surface has to be asked for
    // before the window reaches the compositor, and showing it from QML would
    // be too early for that.
    visible: false
    color: "black"
    title: qsTr("WallFlux")

    // Plays the first web wallpaper there is. Choosing between them is the
    // interface's job, once there is an interface; going through select()
    // rather than reaching for the url directly means that interface will
    // not have to undo anything here.
    Component.onCompleted: {
        const found = Wallpapers.wallpapers;
        for (let i = 0; i < found.length; ++i) {
            if (found[i].kind === WallpaperKind.Web) {
                Wallpapers.select(found[i].id);
                return;
            }
        }
    }

    WebEngineView {
        anchors.fill: parent
        visible: root.playing
        // Already a file:// URL. The path was turned into one in C++, where
        // the spaces, brackets and Chinese characters these file names carry
        // get escaped properly; pasting "file://" onto the front of a plain
        // path here would break exactly those.
        url: root.playing ? Wallpapers.current.source : "about:blank"
    }

    // Shown whenever nothing is playing, which is most of the time so far.
    Column {
        anchors.centerIn: parent
        spacing: 10
        visible: !root.playing

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#d8d8d8"
            font.pixelSize: 30
            font.weight: Font.Light
            text: Wallpapers.wallpapers.length === 0
                  ? qsTr("No wallpapers found")
                  : qsTr("Nothing to play")
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#787878"
            font.pixelSize: 15
            text: Wallpapers.wallpapers.length === 0
                  ? qsTr("Wallpaper Engine's workshop folder is empty or missing.")
                  : qsTr("%1 installed, none of them a web wallpaper.")
                        .arg(Wallpapers.wallpapers.length)
        }
    }
}
