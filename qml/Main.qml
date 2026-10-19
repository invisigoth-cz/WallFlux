// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

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

    /// Whether the wallpaper should be making no sound.
    ///
    /// Sound is on to begin with and goes off on its own for anything that
    /// has better claim to the speakers or to the screen: another program
    /// playing something, or a window pulled up to cover the wallpaper
    /// nobody can then see anyway. Neither is a setting -- what is left to
    /// choose is whether the wallpaper has sound at all.
    readonly property bool silent: picker.muted || OtherAudio.playing || ScreenCover.covered

    /// Whether a window is filling the screen in front of the wallpaper.
    ///
    /// Nothing of the wallpaper is on screen then, so it stops rather than
    /// going on decoding frames and running JavaScript behind a game. Named
    /// here rather than reached for twice below, because what follows reads
    /// as what it is for and not as where the answer came from.
    readonly property bool covered: ScreenCover.covered

    onCoveredChanged: {
        if (root.covered) {
            video.pause();
            return;
        }
        // Asked for only where there is a video to resume: play() with
        // nothing loaded is how this player ends up stopped for good.
        if (root.showingVideo)
            video.play();
    }

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
    // Transparent while nothing is playing, so the desktop's own wallpaper
    // shows through rather than being covered by a black rectangle. Black
    // under a wallpaper that is playing, which is what a video's letterbox
    // edges sit on in the moment before the first frame arrives.
    color: root.playing ? "black" : "transparent"
    title: qsTr("WallFlux")

    function showPicker() {
        picker.show();
        picker.raise();
        picker.requestActivate();
    }

    // Nothing is chosen at startup, by design: the program comes up as an
    // icon in the panel and the desktop keeps the wallpaper it already had
    // until somebody picks one from the grid.

    Picker {
        id: picker

        // Not transient for the wallpaper. That one is a layer-shell surface
        // the compositor keeps in the background and takes no input, and a
        // window belonging to it would be dragged down there with it.
        transientParent: null
        onChose: (id) => Wallpapers.select(id)
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
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: qsTr("Quit")
                onTriggered: Qt.quit()
            }
        }
    }

    WebEngineView {
        id: web

        anchors.fill: parent
        visible: root.showingWeb && !root.covered
        // A web wallpaper is a page and can make noise of its own, so the
        // switch has to reach this one too, not just the video.
        audioMuted: root.silent
        // Already a file:// URL. The path was turned into one in C++, where
        // the spaces, brackets and Chinese characters these file names carry
        // get escaped properly; pasting "file://" onto the front of a plain
        // path here would break exactly those.
        url: root.showingWeb ? Wallpapers.current.source : "about:blank"

        // Frozen, not merely hidden: a hidden page still runs its timers,
        // and these wallpapers are animation loops that would go on burning
        // a core behind a fullscreen window. Chromium will not freeze a page
        // that is still on screen, which is why this follows the visibility
        // rather than the cover itself -- by the time the view is hidden it
        // can be frozen, and it is woken again before it is shown.
        onVisibleChanged: web.lifecycleState = web.visible
                              ? WebEngineView.LifecycleState.Active
                              : WebEngineView.LifecycleState.Frozen
    }

    // Video wallpapers are short clips meant to run on end. Their sound --
    // where they have any -- plays, which is what Wallpaper Engine does too;
    // the switch for it belongs to the interface, once there is one.
    Video {
        id: video

        anchors.fill: parent
        visible: root.showingVideo
        source: root.showingVideo ? Wallpapers.current.source : ""
        muted: root.silent
        // Set from the slider on the wallpaper's own card in the picker.
        volume: picker.volume
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
            if (root.showingVideo && !root.covered)
                video.play();
        }

        // A codec the system has no decoder for is the likely failure here,
        // and it fails quietly otherwise: a black screen and no reason given.
        onErrorOccurred: (error, errorString) =>
            console.warn("wallflux: cannot play", Wallpapers.current.title, "--", errorString)
    }

}
