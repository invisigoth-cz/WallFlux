// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import QtMultimedia
import QtWebEngine
import WallFlux

/// One screen's worth of wallpaper.
///
/// A layer-shell surface anchored to the whole of one screen, with the
/// drawers for the three kinds WallFlux can play and the two things a
/// transition is made of. There is one of these per screen the wallpaper
/// is wanted on; they come and go as monitors do.
///
/// What is on it is not its own business. `Main.qml` holds the one chosen
/// wallpaper and the one running playlist and hands both down, so that
/// three screens show the same thing rather than three programs' worth of
/// state.
Window {
    id: surface

    /// Which screen this one belongs to. Set before it is shown, because
    /// a layer-shell surface is given to a screen once and does not move.
    required property var onScreen

    /// What is on it, handed down rather than read here.
    required property bool showingWeb
    required property bool showingVideo
    required property bool showingScene
    required property bool playing
    required property bool silent
    required property bool covered

    /// How loud a video plays, 0 to 1.
    required property real volume
    /// What the Settings tab allows, for every scene and page alike.
    property int fps: 15
    property bool music: true
    property bool parallax: true
    property bool particles: true
    /// `default`, `fill`, `fit` or `stretch`; see the picker.
    property string scaling: "default"
    /// Quality, for scenes; see the picker.
    property int msaa: 0
    property bool bloom: true
    property bool halfTextures: false

    /// Whether this screen works the wallpaper out, or shows the picture the
    /// first one worked out -- Wallpaper Engine's "Clone single wallpaper".
    /// One simulation, one video, one page, however many screens, and all of
    /// them in step.
    required property bool primary

    /// The first screen's picture, which every other screen shows.
    property Item mirrorOf: null

    /// Whether other screens show this one's picture. That is what makes it
    /// a texture, which costs a copy and is not made for nobody.
    property bool cloned: false

    /// What this screen draws, for the others to show.
    readonly property Item picture: content

    screen: surface.onScreen

    onCoveredChanged: {
        if (surface.covered) {
            player.pause();
            return;
        }
        // Asked for only where there is a video to resume: play() with
        // nothing loaded is how this player ends up stopped for good.
        if (surface.showingVideo)
            player.play();
    }

    // Layer-shell takes the size from its anchors, so no fullscreen state is
    // asked for. These two are what the window falls back to where there is
    // no layer-shell, such as under X11.
    width: surface.onScreen ? surface.onScreen.width : Screen.width
    height: surface.onScreen ? surface.onScreen.height : Screen.height

    // Transparent for input is what lets a click fall through to the desktop
    // underneath; without it the wallpaper would swallow every one.
    flags: Qt.FramelessWindowHint | Qt.WindowTransparentForInput

    // Built hidden and shown by hand, because the layer-shell surface has
    // to be asked for before the window reaches the compositor. Asking
    // afterwards does nothing at all: by then it is an ordinary surface.
    visible: false

    Component.onCompleted: {
        Desktop.asWallpaper(surface);
        surface.visible = true;
    }
    // Transparent while nothing is playing, so the desktop's own wallpaper
    // shows through rather than being covered by a black rectangle. Black
    // under a wallpaper that is playing, which is what a video's letterbox
    // edges sit on in the moment before the first frame arrives.
    color: surface.playing ? "black" : "transparent"
    title: qsTr("WallFlux")

    /// Holds what is on this screen still, so that it can be changed
    /// underneath, and says `held()` once it is holding.
    ///
    /// There is no second player to keep the old wallpaper in -- one video
    /// and one web view per screen is all there is -- so what is held is a
    /// picture of it, grabbed the instant before the switch. Going out
    /// through black needs no picture: the veil is what is held behind.
    ///
    /// `how` and `ms` are handed in rather than read off `Playlists`,
    /// because "random" has to be rolled once for all the screens at a
    /// time and not once per screen.
    function holdStill(how, ms) {
        if (how === "black") {
            veilUp.duration = Math.max(1, ms / 2);
            veilUp.start();
            return;
        }

        // The grab never comes back at all when the window is not being
        // drawn, which is what a covered screen means -- Main.qml keeps a
        // rescue for that, and this says so at once where it can.
        if (!surface.contentItem.grabToImage((result) => surface.holdThis(result)))
            surface.held();
    }

    function holdThis(result) {
        leaving.source = result.url;
        leaving.opacity = 1;
        surface.held();
    }

    /// Lets go again, in the manner asked for, once the wallpaper
    /// underneath has been changed.
    function release(how, ms) {
        if (how === "black") {
            veilDown.duration = Math.max(1, ms / 2);
            veilDown.start();
            return;
        }

        // "None (reduce flicker)" is not a fade. It holds the last frame
        // over the gap while the next wallpaper finds its first one, which
        // is the black blink it is named for.
        if (how === "soft") {
            holdOn.restart();
            return;
        }

        crossOver.duration = Math.max(1, ms);
        crossOver.start();
    }

    /// Said when this screen is holding still and can be changed under.
    signal held()

    NumberAnimation {
        id: veilUp

        target: veil
        property: "opacity"
        to: 1
        onFinished: surface.held()
    }

    NumberAnimation {
        id: veilDown

        target: veil
        property: "opacity"
        to: 0
        onFinished: leaving.source = ""
    }

    /// The old wallpaper fading off the new one.
    NumberAnimation {
        id: crossOver

        target: leaving
        property: "opacity"
        to: 0
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

    // Everything the screen shows, gathered so that it can be one texture
    // for the other screens -- the transitions included, so that a fade is
    // worked out once too.
    Item {
        id: content

        anchors.fill: parent
        visible: surface.primary
        layer.enabled: surface.cloned

        WebEngineView {
            id: web

            anchors.fill: parent
            visible: surface.showingWeb && !surface.covered
            // A web wallpaper is a page and can make noise of its own, so the
            // switch has to reach this one too, not just the video.
            audioMuted: surface.silent
            // Already a file:// URL. The path was turned into one in C++, where
            // the spaces, brackets and Chinese characters these file names carry
            // get escaped properly; pasting "file://" onto the front of a plain
            // path here would break exactly those.
            url: surface.showingWeb ? Wallpapers.current.source : "about:blank"

            /// Whether the page asked to hear the speakers.
            property bool listens: false

            settings.playbackRequiresUserGesture: false

            // What Wallpaper Engine gives every page. It has to be there before
            // the page's own scripts run, which call it while they load.
            userScripts.collection: [{
                name: "wallpaper-engine",
                injectionPoint: WebEngineScript.DocumentCreation,
                worldId: WebEngineScript.MainWorld,
                sourceCode: "window.wallpaperRegisterAudioListener = function (listener) { window.wallfluxHear = listener; };"
            }, {
                // A page draws as often as it asks for a frame, which Chromium
                // grants at the screen's 144 a second. As often as a scene, by the
                // Settings tab: frames that come sooner are passed over. A change
                // reaches the next page loaded, not the one on show.
                // Not a timer -- Chromium runs a frame's callbacks well after the
                // time it hands them, and a timer counted from that time ran at
                // sixty. CSS animations and video are not held.
                name: "frame-rate",
                injectionPoint: WebEngineScript.DocumentCreation,
                worldId: WebEngineScript.MainWorld,
                sourceCode: `(function () {
                    const frame = window.requestAnimationFrame.bind(window);
                    let waiting = new Map(), next = 1, asked = false, last = -Infinity;
                    function tick(now) {
                        if (now - last < 1000 / ${surface.fps} - 2) {
                            frame(tick);
                            return;
                        }
                        asked = false;
                        last = now;
                        const due = waiting;
                        waiting = new Map();
                        due.forEach(callback => callback(now));
                    }
                    window.requestAnimationFrame = function (callback) {
                        const id = next++;
                        waiting.set(id, callback);
                        if (!asked) {
                            asked = true;
                            frame(tick);
                        }
                        return id;
                    };
                    window.cancelAnimationFrame = id => { waiting.delete(id); };
                })();`
            }]

            // Unasked, a page draws its own defaults: Perfect Wallpaper shows its
            // first picture whatever its preset says, and neither clock nor ring.
            onLoadingChanged: request => {
                web.listens = false;
                if (request.status !== WebEngineView.LoadSucceededStatus || !surface.showingWeb)
                    return;
                web.runJavaScript("window.wallpaperPropertyListener && window.wallpaperPropertyListener.applyUserProperties && window.wallpaperPropertyListener.applyUserProperties("
                                  + (Wallpapers.current.properties || "{}") + "); !!window.wallfluxHear",
                                  heard => web.listens = !!heard);
            }

            // Read only while a page wants it and is on show: listening puts the
            // microphone icon in the tray, see the renderer's audio.
            Timer {
                interval: 33
                repeat: true
                running: web.listens && web.visible && surface.music
                onTriggered: web.runJavaScript("window.wallfluxHear(" + JSON.stringify(SceneSounds.speakers()) + ")")
            }

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
            visible: surface.showingVideo
            // A wallpaper fills the screen; the clip's own shape gives way rather
            // than leaving bars down the sides.
            fillMode: surface.scaling === "fit" ? VideoOutput.PreserveAspectFit
                      : surface.scaling === "stretch" ? VideoOutput.Stretch
                      : VideoOutput.PreserveAspectCrop
        }

        MediaPlayer {
            id: player

            videoOutput: video
            source: surface.showingVideo ? Wallpapers.current.source : ""
            // Round and round, unless a playlist is waiting for the end of it:
            // a video told to loop for ever never reports an end, so there
            // would be nothing to turn on.
            loops: Playlists.changesOnVideoEnd ? 1 : MediaPlayer.Infinite

            audioOutput: AudioOutput {
                muted: surface.silent
                // Set from the slider on the wallpaper's own card in the picker.
                volume: surface.volume
            }

            // autoPlay starts the first file and no other: hand it a second one
            // and it loads it, says nothing, reports no error and sits there
            // stopped. Measured, not guessed -- twenty-four switches gave one
            // start with autoPlay and twenty-five with this. Switching wallpaper
            // is the whole point of the picker, so it cannot be left to that.
            onSourceChanged: {
                if (surface.showingVideo && !surface.covered)
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
                if (surface.showingVideo && !surface.covered)
                    player.play();
            }

            onErrorOccurred: (error, errorString) =>
                console.warn("wallflux: cannot play", Wallpapers.current.title, "--", errorString)
        }

        // A scene is drawn by the renderer under crates/, into this item, on
        // the scene graph's own thread. Covered, its clock stops and nothing is
        // drawn, which is what a video's pause is for the player above.
        SceneView {
            id: scene

            anchors.fill: parent
            visible: surface.showingScene
            source: surface.showingScene ? Wallpapers.current.source : ""
            running: surface.showingScene && !surface.covered
            fps: surface.fps
            music: surface.music
            parallax: surface.parallax
            particles: surface.particles
            scaling: surface.scaling === "fit" ? 1 : surface.scaling === "stretch" ? 2 : 0
            msaa: surface.msaa
            bloom: surface.bloom
            halfTextures: surface.halfTextures
            // The desktop's coordinates less where this screen starts on it,
            // which is this item's own: it fills a surface that fills the screen.
            pointer: Pointer.known && surface.onScreen
                     ? Qt.point(Pointer.position.x - surface.onScreen.virtualX,
                                Pointer.position.y - surface.onScreen.virtualY)
                     : undefined

            onErrorChanged: {
                if (scene.error !== "")
                    console.warn("wallflux: cannot draw", Wallpapers.current.title, "--", scene.error)
            }
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

    Mirror {
        anchors.fill: parent
        visible: !surface.primary
        source: surface.mirrorOf
    }
}
