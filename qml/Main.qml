// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import Qt.labs.platform as Platform
import WallFlux

/// The program: a tray icon, the picker, and a wallpaper on every screen
/// it is wanted on.
///
/// Not a window itself any more. A wallpaper is a layer-shell surface and
/// a layer-shell surface belongs to one screen, so there is one window per
/// screen -- `WallpaperSurface.qml` -- and this holds what they all show.
/// One chosen wallpaper and one running playlist, handed down, rather than
/// three screens each keeping their own idea of what is playing.
Item {
    id: root

    /// Which drawer the chosen wallpaper belongs to. All three want a file
    /// on disk, so a kind alone is not enough to start on.
    readonly property bool showingWeb: Wallpapers.current.kind === WallpaperKind.Web
                                       && Wallpapers.current.hasFile
    readonly property bool showingVideo: Wallpapers.current.kind === WallpaperKind.Video
                                         && Wallpapers.current.hasFile
    readonly property bool showingScene: Wallpapers.current.kind === WallpaperKind.Scene
                                         && Wallpapers.current.hasFile

    /// Whether what is chosen is of a kind this program can draw today.
    readonly property bool playing: root.showingWeb || root.showingVideo || root.showingScene

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
    /// going on decoding frames and running JavaScript behind a game.
    ///
    /// One answer for every screen, because that is all `ScreenCover` can
    /// say: KWin is asked whether *a* window covers the wallpaper, not
    /// which screen it is on. A second monitor nobody has covered is
    /// stopped along with the first; worth knowing before it is called a
    /// bug.
    readonly property bool covered: ScreenCover.covered

    /// Whether the running playlist's clock should be standing still.
    ///
    /// Two reasons, and this program is the only place that knows either.
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

    // The mouse is asked after only while a scene is on show and in view:
    // nothing else here follows it, and every move is a call over the bus.
    Binding {
        target: Pointer
        property: "watching"
        value: root.showingScene && !root.covered
    }

    function showPicker() {
        picker.show();
        picker.raise();
        picker.requestActivate();
    }

    // One wallpaper per screen. The model is Qt's own list of screens, so
    // plugging a monitor in builds another surface and unplugging one
    // takes its surface with it -- there is nothing here to keep in step
    // with the compositor.
    //
    // Qt names a screen after its connector, which is the same name KDE's
    // display settings and /sys/class/drm use. No `xrandr`, no
    // `kscreen-doctor`, no parsing anybody's output.
    Instantiator {
        id: surfaces

        // The screens the Displays tab has ticked, as screen objects.
        model: {
            const kept = [];
            const all = Qt.application.screens;
            for (let i = 0; i < all.length; ++i) {
                if (picker.screensWanted.indexOf(all[i].name) >= 0)
                    kept.push(all[i]);
            }
            return kept;
        }

        delegate: WallpaperSurface {
            required property var modelData

            onScreen: modelData
            showingWeb: root.showingWeb
            showingVideo: root.showingVideo
            showingScene: root.showingScene
            playing: root.playing
            silent: root.silent
            covered: root.covered
            volume: picker.volume
            onHeld: root.oneIsHolding()
        }
    }

    // A scene's own music, once for the program rather than once per screen:
    // a player on every surface would play it as many times over as there
    // are monitors.
    Repeater {
        model: root.showingScene ? SceneSounds.of(Wallpapers.current.source) : []

        delegate: SceneSound {
            required property var modelData

            files: modelData.files
            mode: modelData.mode
            volume: modelData.volume
            mintime: modelData.mintime
            maxtime: modelData.maxtime
            startsilent: modelData.startsilent
            silent: root.silent
            covered: root.covered
            loudness: picker.volume
        }
    }

    // Nothing is chosen at startup, by design: the program comes up as an
    // icon in the panel and the desktop keeps the wallpaper it already had
    // until somebody picks one from the grid.

    Picker {
        id: picker

        // Not transient for a wallpaper. Those are layer-shell surfaces the
        // compositor keeps in the background and which take no input, and a
        // window belonging to one would be dragged down there with it.
        transientParent: null
        onChose: (id) => Wallpapers.select(id)
    }

    // ---- Transitions ---------------------------------------------------

    /// The wallpaper a transition is on its way to, or empty while none is
    /// running. Also what says a transition *is* running.
    property string arriving: ""

    /// Which transition is running. The playlist's, except that "random"
    /// is rolled here -- once per change, and held for the length of it,
    /// so every screen fades the same way and the end of a transition
    /// matches its start.
    property string fading: ""

    /// How many screens have yet to say they are holding still.
    property int waiting: 0

    /// Puts a wallpaper on the desktop the way the running playlist asked
    /// for.
    ///
    /// Every screen is asked to hold what it has, and the wallpaper is
    /// changed only once they all have. Changing it as each one answers
    /// would leave the slowest screen holding a picture of the *new*
    /// wallpaper -- a fade from a thing to itself, which is a cut.
    function changeTo(id) {
        const asked = Playlists.transition;
        root.fading = asked === "random" ? (Math.random() < 0.5 ? "fade" : "black") : asked;

        // Nothing to cross from on the first wallpaper of a playlist, and
        // nothing asked for in the other two cases.
        if (root.fading === "" || root.fading === "none" || !root.playing
            || surfaces.count === 0) {
            Wallpapers.select(id);
            return;
        }

        root.arriving = id;
        root.waiting = surfaces.count;

        // Only the ways that hold a *picture* need rescuing. Going out
        // through black holds nothing: its own animation says when it is
        // ready, and that is half the transition away -- the rescue fired
        // first and the veil never got off the ground, so the screen never
        // went black at all. Measured: rgb 0.46 halfway through what was
        // meant to be the darkest moment of it.
        if (root.fading !== "black")
            rescue.restart();
        for (let i = 0; i < surfaces.count; ++i)
            surfaces.objectAt(i).holdStill(root.fading, Playlists.transitionMs);
    }

    /// One screen is holding. The last one to say so is what lets the
    /// change happen.
    function oneIsHolding() {
        if (root.arriving === "")
            return;
        root.waiting -= 1;
        if (root.waiting > 0)
            return;
        rescue.stop();
        root.swapNow();
    }

    /// Changes the wallpaper and lets every screen go again.
    function swapNow() {
        const id = root.arriving;
        const how = root.fading;
        root.arriving = "";
        root.waiting = 0;

        if (id !== "")
            Wallpapers.select(id);
        for (let i = 0; i < surfaces.count; ++i)
            surfaces.objectAt(i).release(how, Playlists.transitionMs);
    }

    /// The screen that never answered. A grab comes back only while the
    /// window is being drawn, and a covered screen is not -- without this
    /// a playlist behind a maximised window would simply never change.
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

    // The only thing a person can click: the wallpapers are in the
    // background layer and transparent to input, so without this there
    // would be no way to reach the picker at all.
    Platform.SystemTrayIcon {
        visible: true
        // From the icon theme rather than shipped with us; Breeze and every
        // other freedesktop theme has this one.
        icon.name: "preferences-desktop-wallpaper"
        tooltip: Wallpapers.current.playable
                 ? qsTr("WallFlux — %1").arg(Wallpapers.current.title)
                 : qsTr("WallFlux")

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
}
