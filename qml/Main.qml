// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

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

    /// Whether the running playlist's clock should be standing still.
    ///
    /// Two reasons, and this window is the only place that knows either.
    /// A playlist told to turn when a video ends is waiting on the video
    /// and not on the clock -- and only here is it known that what is on
    /// the screen is a video at all. And a playlist not told to carry on
    /// while the wallpaper is stopped behind a window should not count the
    /// minutes nobody is watching; that is what Wallpaper Engine means by
    /// paused, and it is why its own playlists look stuck to people who
    /// leave a window maximised.
    readonly property bool holdingThePlaylist:
        (Playlists.changesOnVideoEnd && root.showingVideo)
        || (root.covered && !Playlists.changesWhilePaused)

    onHoldingThePlaylistChanged: Playlists.hold(root.holdingThePlaylist)

    onCoveredChanged: {
        if (root.covered) {
            player.pause();
            return;
        }
        // Asked for only where there is a video to resume: play() with
        // nothing loaded is how this player ends up stopped for good.
        if (root.showingVideo)
            player.play();
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

    /// The wallpaper a transition is on its way to, or empty while none
    /// is running. Also what says a transition *is* running.
    property string arriving: ""

    /// Which transition is actually running. The same as the playlist's,
    /// except that "random" is rolled here -- once per change, and held
    /// for the length of that change so the end of it matches the start.
    property string fading: ""

    /// Puts a wallpaper on the desktop the way the running playlist asked
    /// for.
    ///
    /// Four ways, and three of them need the old wallpaper to stay on the
    /// screen after it has stopped being the wallpaper. There is no second
    /// player to hold it in -- one video and one web view is all there is
    /// -- so what is held is a picture of it, grabbed the instant before
    /// the switch. `leaving` is that picture.
    function changeTo(id) {
        const asked = Playlists.transition;
        root.fading = asked === "random" ? (Math.random() < 0.5 ? "fade" : "black") : asked;

        // Nothing to cross from on the first wallpaper of a playlist, and
        // nothing asked for in the other two cases.
        if (root.fading === "" || root.fading === "none" || !root.playing) {
            Wallpapers.select(id);
            return;
        }

        if (root.fading === "black") {
            root.arriving = id;
            throughBlack.start();
            return;
        }

        root.arriving = id;
        // Grabbing is asynchronous and the callback never comes at all
        // when the window is not being drawn -- which is exactly what a
        // covered screen means. The rescue is why a covered wallpaper
        // still changes.
        if (!root.contentItem.grabToImage((result) => root.holdAndSwap(result)))
            root.swapNow();
        else
            rescue.restart();
    }

    /// The picture came back: hold it over the screen, change underneath
    /// it, and take it away in the manner asked for.
    function holdAndSwap(result) {
        rescue.stop();
        if (root.arriving === "")
            return;

        leaving.source = result.url;
        leaving.opacity = 1;
        root.swapNow();

        // "None (reduce flicker)" is not a fade. It holds the last frame
        // over the gap while the next wallpaper finds its first one, which
        // is the black blink it is named for.
        if (root.fading === "soft")
            holdOn.restart();
        else
            crossOver.start();
    }

    /// Changes the wallpaper and forgets that a transition was running.
    function swapNow() {
        const id = root.arriving;
        root.arriving = "";
        if (id !== "")
            Wallpapers.select(id);
    }

    /// Out through black and back. The one transition that needs no
    /// picture of anything: a black rectangle over the lot, and the
    /// wallpaper changed behind it while nothing can be seen.
    SequentialAnimation {
        id: throughBlack

        NumberAnimation {
            target: veil
            property: "opacity"
            to: 1
            duration: Math.max(1, Playlists.transitionMs / 2)
        }
        ScriptAction { script: root.swapNow() }
        NumberAnimation {
            target: veil
            property: "opacity"
            to: 0
            duration: Math.max(1, Playlists.transitionMs / 2)
        }
        ScriptAction { script: leaving.source = "" }
    }

    /// The old wallpaper fading off the new one.
    NumberAnimation {
        id: crossOver

        target: leaving
        property: "opacity"
        to: 0
        duration: Math.max(1, Playlists.transitionMs)
        onFinished: leaving.source = ""
    }

    /// Long enough for the next wallpaper to have a frame to show, and no
    /// longer: this is the transition that is meant to look like none.
    Timer {
        id: holdOn

        interval: 300
        onTriggered: {
            leaving.opacity = 0;
            leaving.source = "";
        }
    }

    /// The grab that never came back. Without this a wallpaper behind a
    /// maximised window would simply never change.
    Timer {
        id: rescue

        interval: 400
        onTriggered: root.swapNow()
    }

    /// A playlist puts its wallpapers on the desktop the same way a click
    /// does -- it says which one and nothing more, so there is one place
    /// where a wallpaper starts playing rather than two.
    Connections {
        target: Playlists
        function onPlays(id) {
            root.changeTo(id);
        }

        // A playlist started while a window is already covering the
        // wallpaper has to be told so: the binding above has not changed,
        // so nothing else would say it.
        function onPlayingChanged() {
            Playlists.hold(root.holdingThePlaylist);
        }
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
    //
    // A player and a surface rather than the `Video` convenience item,
    // which wraps exactly these two and hides the one property this needs:
    // `mediaStatus`. Its `stopped` signal is not a stand-in -- it fires
    // when a source is swapped as well as when a clip ends, so a playlist
    // turning from one video to another would have turned twice.
    VideoOutput {
        id: video

        anchors.fill: parent
        visible: root.showingVideo
        // A wallpaper fills the screen; the clip's own shape gives way rather
        // than leaving bars down the sides.
        fillMode: VideoOutput.PreserveAspectCrop
    }

    MediaPlayer {
        id: player

        videoOutput: video
        source: root.showingVideo ? Wallpapers.current.source : ""
        // Round and round, unless a playlist is waiting for the end of it:
        // a video told to loop for ever never reports an end, so there
        // would be nothing to turn on.
        loops: Playlists.changesOnVideoEnd ? 1 : MediaPlayer.Infinite

        audioOutput: AudioOutput {
            muted: root.silent
            // Set from the slider on the wallpaper's own card in the picker.
            volume: picker.volume
        }

        // autoPlay starts the first file and no other: hand it a second one
        // and it loads it, says nothing, reports no error and sits there
        // stopped. Measured, not guessed -- twenty-four switches gave one
        // start with autoPlay and twenty-five with this. Switching wallpaper
        // is the whole point of the picker, so it cannot be left to that.
        onSourceChanged: {
            if (root.showingVideo && !root.covered)
                player.play();
        }

        onMediaStatusChanged: {
            if (player.mediaStatus !== MediaPlayer.EndOfMedia)
                return;

            // Told to turn when the video ends, so that is what its end
            // means. The playlist picks the next one and `onPlays` puts it
            // on, the same way a click does.
            if (Playlists.changesOnVideoEnd)
                Playlists.next();

            // Then play whatever is on the screen now. Usually the next
            // wallpaper, which `onSourceChanged` has already started and
            // which this leaves alone -- but a playlist of one wallpaper
            // turns to itself, and `select()` stays silent when the choice
            // has not changed, so without this the video would sit there
            // on its last frame for good.
            if (root.showingVideo && !root.covered)
                player.play();
        }

        onErrorOccurred: (error, errorString) =>
            console.warn("wallflux: cannot play", Wallpapers.current.title, "--", errorString)
    }

    // The two things a transition is made of, over everything else and in
    // this order: the picture of the wallpaper that is leaving, and the
    // black that one of the transitions goes out through.
    //
    // Neither takes any input -- the whole window is a background surface
    // that takes none -- and both are nothing at all until a transition
    // runs.
    Image {
        id: leaving

        anchors.fill: parent
        z: 1
        opacity: 0
        visible: leaving.opacity > 0 && leaving.source !== ""
        fillMode: Image.PreserveAspectCrop
        // The grab is the size of the window, so this is drawing it back
        // at the size it was taken at; smoothing it would only cost.
        smooth: false
        cache: false
    }

    Rectangle {
        id: veil

        anchors.fill: parent
        z: 2
        color: "black"
        opacity: 0
        visible: veil.opacity > 0
    }

}
