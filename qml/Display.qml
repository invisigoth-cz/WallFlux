// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import WallFlux

/// What plays on one monitor -- or, cloned, on every monitor at once: the
/// wallpaper, the playlist running on it, its pause and its transitions,
/// the scene's own music, and the surface on each of its screens.
///
/// The first of `screens` works the wallpaper out and every other one shows
/// its picture, as Wallpaper Engine's "Clone single wallpaper". With a
/// wallpaper per monitor each `Display` has one screen, or none while its
/// monitor is unplugged -- it is kept then, so the monitor comes back to
/// what it had.
Item {
    id: display

    /// The screens it draws on, as screen objects, the first one working
    /// the wallpaper out. Empty while its monitor is away.
    required property var screens

    /// Which display this is: the monitor's connector name, or "clone".
    property string key: ""

    /// The picker, for the settings every display draws with.
    required property var options

    /// Paused from the tray: this monitor alone, and everything at once.
    /// Two things, so that resuming everything leaves a monitor paused on
    /// its own standing still.
    property bool pausedHere: false
    property bool pausedAll: false

    /// Muted: this monitor alone, and everything at once.
    property bool mutedHere: false
    property bool mutedAll: false

    /// Whether a window covers any screen that shows a wallpaper, this
    /// display's or another's: what "pause all" and "stop all" answer to.
    property bool anyCovered: false

    /// The order Next and Previous go through when no playlist runs: the
    /// library as the grid shows it.
    property var order: []

    /// What is on show, and the playlist running on it.
    readonly property alias showing: shown
    readonly property alias run: playlist

    readonly property bool connected: display.screens.length > 0

    /// The quality this display's wallpaper is drawn with: its own preset,
    /// or the settings.
    readonly property var drawn: display.options.drawnFor(shown.current.id, display.options.qualityOf,
        display.options.fps, display.options.msaa, display.options.bloom,
        Power.onBattery, display.options.batteryLow)

    /// Which drawer the chosen wallpaper belongs to. All three want a file
    /// on disk, so a kind alone is not enough to start on.
    readonly property bool showingWeb: shown.current.kind === WallpaperKind.Web
                                       && shown.current.hasFile
    readonly property bool showingVideo: shown.current.kind === WallpaperKind.Video
                                         && shown.current.hasFile
    readonly property bool showingScene: shown.current.kind === WallpaperKind.Scene
                                         && shown.current.hasFile

    /// Whether what is chosen is of a kind this program can draw today.
    readonly property bool playing: display.showingWeb || display.showingVideo || display.showingScene

    /// Whether a window is in front of every screen this display is on: one
    /// wallpaper is worked out for all of them, and one screen left in view
    /// is somebody looking at it.
    readonly property bool everyScreenCovered: display.connected
        && display.screens.every(s => ScreenCover.coveredScreens.indexOf(s.name) >= 0)

    /// What is happening that the wallpaper may answer, each with what
    /// Settings › Performance says to do about it: "keep", "mute", "pause"
    /// or "stop". A window answers on the screens it covers, or -- told
    /// "pause all" or "stop all" -- on every display once it covers any.
    readonly property var reasons: [
        // Cloned, there is one wallpaper and today's rule stands: still only
        // when every screen it is on is covered, whatever "all" says.
        { on: display.options.underWindow.endsWith("all") && display.key !== "clone"
              ? display.anyCovered : display.everyScreenCovered,
          mode: display.options.underWindow.replace("all", "") },
        // Another program with the keyboard answers on the monitor it is on,
        // or -- told "all" -- on every display; cloned, on the clone.
        { on: ScreenCover.focused && (display.options.forFocus.endsWith("all") || display.key === "clone"
              || display.screens.some(s => s.name === ScreenCover.focusedScreen)),
          mode: display.options.forFocus.replace("all", "") },
        { on: OtherAudio.playing, mode: display.options.forOtherSound },
        { on: Asleep.asleep, mode: display.options.whenAsleep }
    ]

    /// Let go of altogether while one of those holds and was told Stop:
    /// the first screen draws nothing, so the scene, the video or the page
    /// is freed, and it is loaded again from the start afterwards.
    readonly property bool stopped: display.reasons.some(r => r.on && r.mode === "stop")

    /// Whether the wallpaper is to stand still: paused from the tray, a
    /// window filling the screen in front of it, another program with the
    /// keyboard where that was asked for, or a battery below the floor set
    /// in Settings.
    ///
    /// Nothing of the wallpaper is on screen in the second case, so it
    /// stops rather than going on decoding frames and running JavaScript
    /// behind a game; the others are somebody's choice to save the machine.
    readonly property bool covered: display.pausedHere || display.pausedAll
        || display.reasons.some(r => r.on && (r.mode === "pause" || r.mode === "stop"))
        || (Power.onBattery && Power.hasBattery && Power.percent < display.options.batteryFloor)

    /// Whether the wallpaper should be making no sound.
    ///
    /// Sound is on to begin with and goes off on its own for anything that
    /// has better claim to the speakers or to the screen: another program
    /// playing something, or a window pulled up to cover the wallpaper
    /// nobody can then see anyway.
    readonly property bool silent: display.mutedHere || display.mutedAll || display.covered
        || display.reasons.some(r => r.on && r.mode !== "keep")

    /// Whether the running playlist's clock should be standing still.
    ///
    /// A playlist told to turn when a video ends is waiting on the video
    /// and not on the clock -- and only here is it known that what is on
    /// the screen is a video at all. A playlist not told to carry on while
    /// the wallpaper is stopped behind a window should not count the
    /// minutes nobody is watching; that is what Wallpaper Engine means by
    /// paused. And a monitor that is unplugged has nobody watching it.
    readonly property bool holdingThePlaylist:
        (playlist.changesOnVideoEnd && display.showingVideo)
        || (display.covered && !playlist.changesWhilePaused)
        || !display.connected

    onHoldingThePlaylistChanged: playlist.hold(display.holdingThePlaylist)

    // A monitor unplugged in the middle of a transition takes its surface
    // with it, and the screen that was to say it was holding never does:
    // the wallpaper on its way was skipped, and the old one came back with
    // the monitor (25 November, measured with kscreen-doctor). Finished at
    // once, there being nothing left to animate.
    onConnectedChanged: {
        if (!display.connected && display.arriving !== "")
            display.swapNow();
    }

    /// Puts this wallpaper on show at once, with no transition.
    ///
    /// In the middle of one, the transition is taken over and goes to this
    /// wallpaper instead: left to finish, it put the playlist's back over
    /// a wallpaper somebody had just chosen (25 November, seen by the user).
    function select(id) {
        if (display.arriving !== "") {
            display.arriving = id;
            display.swapNow();
            return true;
        }
        return shown.select(id);
    }

    /// Shows nothing, and stops the playlist, or its clock would put the
    /// next one on by itself a moment later. A transition running is let
    /// go of with it, or it would bring its wallpaper back after Off.
    function clear() {
        const midway = display.arriving !== "";
        display.arriving = "";
        playlist.stop();
        shown.clear();
        if (midway)
            display.swapNow();
    }

    /// What is on show.
    Showing {
        id: shown
        library: Wallpapers
    }

    /// A playlist puts its wallpapers on the desktop the same way a click
    /// does -- it says which one and nothing more, so there is one place
    /// where a wallpaper starts playing rather than two.
    PlaylistRun {
        id: playlist
    }

    // A playlist renamed or thrown away in the library reaches the run.
    Connections {
        target: Playlists
        function onRenamed(from, to) { playlist.renamed(from, to); }
        function onForgotten(name) { playlist.forgotten(name); }
    }

    Connections {
        target: playlist
        function onPlays(id) {
            display.changeTo(id);
        }

        // A playlist started while a window is already covering the
        // wallpaper has to be told so: the binding above has not changed,
        // so nothing else would say it.
        function onPlayingChanged() {
            playlist.hold(display.holdingThePlaylist);
        }
    }

    /// Next and Previous: through the running playlist if there is one, and
    /// otherwise through the library in the order the grid shows it, round
    /// from the end to the start. Counted from the one on show, or --
    /// switched off, which leaves nothing on show -- from the one played
    /// last, so that Next after Off is the next one and not the first.
    function step(by) {
        if (playlist.playing !== "") {
            if (by > 0)
                playlist.next();
            else
                playlist.previous();
            return;
        }
        const order = display.order.filter(w => w.playable);
        if (order.length === 0)
            return;
        const from = shown.current.id || Wallpapers.recent[0] || "";
        const at = order.findIndex(w => w.id === from);
        const to = at < 0 ? (by > 0 ? 0 : order.length - 1)
                          : (at + by + order.length) % order.length;
        display.select(order[to].id);
    }

    /// What the first screen draws, which every other one shows.
    readonly property Item firstPicture: surfaces.count > 0 && display.firstSurface()
                                         ? display.firstSurface().picture : null

    /// The surface that works the wallpaper out.
    function firstSurface(): WallpaperSurface {
        return surfaces.objectAt(0) as WallpaperSurface;
    }

    // One window per screen. Plugging a monitor in builds another surface
    // and unplugging one takes its surface with it -- there is nothing here
    // to keep in step with the compositor.
    Instantiator {
        id: surfaces

        model: display.screens

        delegate: WallpaperSurface {
            required property var modelData
            required property int index

            onScreen: modelData
            run: playlist
            showing: shown
            // The first screen works the wallpaper out and the others show
            // its picture, as Wallpaper Engine's "Clone single wallpaper".
            primary: index === 0
            mirrorOf: index === 0 ? null : display.firstPicture
            flipped: index > 0 && display.options.flippedMonitors.indexOf(modelData.name) >= 0
            cloned: index === 0 && surfaces.count > 1
            showingWeb: index === 0 && display.showingWeb && !display.stopped
            showingVideo: index === 0 && display.showingVideo && !display.stopped
            showingScene: index === 0 && display.showingScene && !display.stopped
            playing: display.playing
            silent: display.silent
            covered: display.covered
            volume: display.options.volume
            fps: display.drawn.fps
            msaa: display.drawn.msaa
            bloom: display.drawn.bloom
            halfTextures: display.options.halfTextures
            properties: display.options.propertiesFor(shown.current, display.key, display.options.propertiesOf)
            values: display.options.valuesFor(shown.current, display.key, display.options.propertiesOf)
            music: display.options.music
            parallax: display.options.parallax
            particles: display.options.particles
            scaling: display.options.scaling
            onHeld: display.oneIsHolding()
            onTried: (id, grade, why) => display.options.noteCompatibility(id, grade, why)
        }
    }

    // A scene's own music, once for the display rather than once per
    // screen: a player on every surface would play it as many times over as
    // there are monitors cloned.
    Repeater {
        model: display.connected && display.showingScene && !display.stopped
               ? SceneSounds.of(shown.current.source) : []

        delegate: SceneSound {
            required property var modelData

            files: modelData.files
            mode: modelData.mode
            volume: modelData.volume
            mintime: modelData.mintime
            maxtime: modelData.maxtime
            startsilent: modelData.startsilent
            silent: display.silent
            covered: display.covered
            loudness: display.options.volume
        }
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

    /// Puts a wallpaper on show the way the running playlist asked for.
    ///
    /// Every screen is asked to hold what it has, and the wallpaper is
    /// changed only once they all have. Changing it as each one answers
    /// would leave the slowest screen holding a picture of the *new*
    /// wallpaper -- a fade from a thing to itself, which is a cut.
    function changeTo(id) {
        const asked = playlist.transition;
        display.fading = asked === "random" ? (Math.random() < 0.5 ? "fade" : "black") : asked;

        // Nothing to cross from on the first wallpaper of a playlist, and
        // nothing asked for in the other two cases.
        if (display.fading === "" || display.fading === "none" || !display.playing
            || surfaces.count === 0) {
            shown.select(id);
            return;
        }

        display.arriving = id;
        // Only the first screen holds still: the others show its picture.
        display.waiting = 1;

        // Only the ways that hold a *picture* need rescuing. Going out
        // through black holds nothing: its own animation says when it is
        // ready, and that is half the transition away -- the rescue fired
        // first and the veil never got off the ground, so the screen never
        // went black at all. Measured: rgb 0.46 halfway through what was
        // meant to be the darkest moment of it.
        if (display.fading !== "black")
            rescue.restart();
        display.firstSurface().holdStill(display.fading, playlist.transitionMs);
    }

    /// One screen is holding. The last one to say so is what lets the
    /// change happen.
    function oneIsHolding() {
        if (display.arriving === "")
            return;
        display.waiting -= 1;
        if (display.waiting > 0)
            return;
        rescue.stop();
        display.swapNow();
    }

    /// Changes the wallpaper and lets every screen go again. Also what
    /// finishes a transition at once, before the display is changed or let
    /// go of in the middle of one.
    function swapNow() {
        const id = display.arriving;
        const how = display.fading;
        display.arriving = "";
        display.waiting = 0;
        rescue.stop();

        if (id !== "")
            shown.select(id);
        const first = surfaces.count > 0 ? display.firstSurface() : null;
        if (first)
            first.release(how, playlist.transitionMs);
    }

    /// The screen that never answered. A grab comes back only while the
    /// window is being drawn, and a covered screen is not -- without this
    /// a playlist behind a maximised window would simply never change.
    Timer {
        id: rescue

        interval: 400
        onTriggered: display.swapNow()
    }
}
