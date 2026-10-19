// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

import QtCore
import QtQuick
import QtQuick.Window
import QtMultimedia
import QtWebEngine
import Qt.labs.platform as Platform
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

    /// Plays this one and remembers it for next time. Everything that
    /// chooses a wallpaper goes through here, so there is one place where a
    /// choice is made and one place where it is written down.
    function choose(id: string) {
        if (Wallpapers.select(id))
            saved.wallpaperId = id;
    }

    function showPicker() {
        picker.show();
        picker.raise();
        picker.requestActivate();
    }

    // Written to ~/.config/WallFlux/wallflux.conf, under the names main()
    // gives the application.
    Settings {
        id: saved

        property string wallpaperId
    }

    // Last time's wallpaper, or -- on a first run, or if that one has since
    // been uninstalled -- the first one we can draw, so that the screen is
    // never left empty with wallpapers sitting right there.
    Component.onCompleted: {
        if (Wallpapers.select(saved.wallpaperId))
            return;

        const found = Wallpapers.wallpapers;
        for (let i = 0; i < found.length; ++i) {
            if (found[i].playable) {
                root.choose(found[i].id);
                return;
            }
        }
    }

    Picker {
        id: picker

        // Not transient for the wallpaper. That one is a layer-shell surface
        // the compositor keeps in the background and takes no input, and a
        // window belonging to it would be dragged down there with it.
        transientParent: null
        onChose: (id) => root.choose(id)
    }

    // The only thing a person can click: the wallpaper itself is in the
    // background layer and transparent to input, so without this there would
    // be no way to reach the picker at all.
    Platform.SystemTrayIcon {
        visible: true
        // From the icon theme rather than shipped with us; Breeze and every
        // other freedesktop theme has this one.
        icon.name: "preferences-desktop-wallpaper"
        tooltip: Wallpapers.current.playable
                 ? qsTr("WallFlux — %1").arg(Wallpapers.current.title)
                 : qsTr("WallFlux")

        // A plain left click is the shortest way to the grid; the menu is
        // for everything else.
        onActivated: (reason) => {
            if (reason === Platform.SystemTrayIcon.Trigger)
                root.showPicker();
        }

        menu: Platform.Menu {
            Platform.MenuItem {
                text: qsTr("Choose wallpaper…")
                onTriggered: root.showPicker()
            }
            Platform.MenuItem {
                text: qsTr("Look for new wallpapers")
                onTriggered: Wallpapers.rescan()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: qsTr("Quit")
                onTriggered: Qt.quit()
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
        id: video

        anchors.fill: parent
        visible: root.showingVideo
        source: root.showingVideo ? Wallpapers.current.source : ""
        // A wallpaper fills the screen; the clip's own shape gives way rather
        // than leaving bars down the sides.
        fillMode: VideoOutput.PreserveAspectCrop
        loops: MediaPlayer.Infinite

        // autoPlay starts the first file and no other: hand it a second one
        // and it loads it, says nothing, reports no error and sits there
        // stopped. Measured, not guessed -- twenty-four switches gave one
        // start with autoPlay and twenty-five with this. Switching wallpaper
        // is the whole point of the picker, so it cannot be left to that.
        onSourceChanged: {
            if (root.showingVideo)
                video.play();
        }

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
