// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

import QtQuick
import QtQuick.Window
import QtMultimedia
import QtWebEngine
import WallFlux

Window {
    id: root

    /// Which drawer the chosen wallpaper belongs to. Both want a file on
    /// disk, so a kind alone is not enough to start on.
    readonly property bool showingWeb: Wallpapers.current.kind === WallpaperKind.Web
                                       && Wallpapers.current.hasFile
    readonly property bool showingVideo: Wallpapers.current.kind === WallpaperKind.Video
                                         && Wallpapers.current.hasFile

    /// Whether what is chosen is of a kind this program can draw today.
    readonly property bool playing: showingWeb || showingVideo

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

    // Plays the first wallpaper we can draw. Choosing between them is the
    // interface's job, once there is an interface; going through select()
    // rather than reaching for the url directly means that interface will
    // not have to undo anything here.
    Component.onCompleted: {
        const found = Wallpapers.wallpapers;
        for (let i = 0; i < found.length; ++i) {
            if (found[i].hasFile
                && (found[i].kind === WallpaperKind.Web
                    || found[i].kind === WallpaperKind.Video)) {
                Wallpapers.select(found[i].id);
                return;
            }
        }
    }

    WebEngineView {
        anchors.fill: parent
        visible: root.showingWeb
        // Already a file:// URL. The path was turned into one in C++, where
        // the spaces, brackets and Chinese characters these file names carry
        // get escaped properly; pasting "file://" onto the front of a plain
        // path here would break exactly those.
        url: root.showingWeb ? Wallpapers.current.source : "about:blank"
    }

    // Video wallpapers are short clips meant to run on end. Their sound --
    // where they have any -- plays, which is what Wallpaper Engine does too;
    // the switch for it belongs to the interface, once there is one.
    Video {
        anchors.fill: parent
        visible: root.showingVideo
        source: root.showingVideo ? Wallpapers.current.source : ""
        // A wallpaper fills the screen; the clip's own shape gives way rather
        // than leaving bars down the sides.
        fillMode: VideoOutput.PreserveAspectCrop
        loops: MediaPlayer.Infinite
        autoPlay: true

        // A codec the system has no decoder for is the likely failure here,
        // and it fails quietly otherwise: a black screen and no reason given.
        onErrorOccurred: (error, errorString) =>
            console.warn("wallflux: cannot play", Wallpapers.current.title, "--", errorString)
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
                  : qsTr("%1 installed, none of them a web or video wallpaper.")
                        .arg(Wallpapers.wallpapers.length)
        }
    }
}
