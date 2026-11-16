// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtMultimedia

/// One sound object of a scene wallpaper, played the way the scene asks.
///
/// Whatever the mode, the first file is one picked at random, as Wallpaper
/// Engine picks it, and it starts at once. `loop` then goes through the
/// files in order and round again -- most scenes hold one, which simply repeats. `random` waits between `mintime`
/// and `maxtime` seconds after each and picks again, which is how a scene
/// carrying a whole album shuffles it. `single` plays the one it picked once. A sound that starts silent waits for a script to start it, and
/// there are no scripts here yet, so it stays silent.
Item {
    id: sound

    required property var files
    required property string mode
    required property real volume
    required property real mintime
    required property real maxtime
    required property bool startsilent

    /// Handed down from Main.qml, as they are to the video player.
    required property bool silent
    required property bool covered
    required property real loudness

    /// Which of `files` is loaded.
    property int playing: 0

    function anyOne() {
        return Math.floor(Math.random() * sound.files.length);
    }

    function start() {
        if (!sound.startsilent && !sound.covered)
            player.play();
    }

    /// Moves on to `next`, and plays it even when it is the file already
    /// loaded -- the player says nothing when its source is set to what it
    /// already holds, so a shuffle that picks the same track twice would
    /// otherwise stop there.
    function turnTo(next) {
        if (next === sound.playing) {
            player.position = 0;
            sound.start();
        } else {
            sound.playing = next;
        }
    }

    onCoveredChanged: sound.covered ? player.pause() : sound.start()
    // Picked here rather than in a binding, which could be worked out before
    // the files were handed in.
    Component.onCompleted: {
        sound.playing = sound.anyOne();
        sound.start();
    }

    MediaPlayer {
        id: player

        source: sound.files.length > 0 ? sound.files[sound.playing] : ""
        // One file looping is the player's own business; anything else turns
        // over at the end of each file.
        loops: sound.mode === "loop" && sound.files.length === 1 ? MediaPlayer.Infinite : 1

        audioOutput: AudioOutput {
            muted: sound.silent
            // The file's number is how loud it sounds, not how far the samples
            // reach: Augmentation at 0.5 and a Space Marine at 1.9 play at 0.75
            // and 1.33 of their files in recordings of Wallpaper Engine.
            volume: Math.sqrt(sound.volume) * sound.loudness
        }

        // A new source is loaded and left stopped unless asked, the same as
        // for a video wallpaper: autoPlay starts the first file and no other.
        onSourceChanged: sound.start()

        onMediaStatusChanged: {
            if (player.mediaStatus !== MediaPlayer.EndOfMedia)
                return;
            if (sound.mode === "loop")
                sound.turnTo((sound.playing + 1) % sound.files.length);
            else if (sound.mode === "random")
                gap.restart();
        }

        onErrorOccurred: (error, errorString) =>
            console.warn("wallflux: cannot play", sound.files[sound.playing], "--", errorString)
    }

    /// The quiet between two tracks of a shuffle.
    Timer {
        id: gap

        interval: 1000 * (sound.mintime + Math.random() * (sound.maxtime - sound.mintime))
        onTriggered: sound.turnTo(sound.anyOne())
    }
}
