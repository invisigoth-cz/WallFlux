// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import WallFlux
import "emblem.js" as Emblem

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

    /// Paused from the tray: stood still and quiet as under a window, and
    /// a playlist held unless it was told to change while paused -- what
    /// Wallpaper Engine's own Pause does. Forgotten at quit, like the rest.
    property bool paused: false

    /// Every monitor that is ticked, showing one wallpaper worked out once
    /// -- Wallpaper Engine's "Clone single wallpaper".
    Display {
        id: clone

        key: "clone"
        options: picker
        pausedAll: root.paused
        mutedAll: picker.muted
        order: picker.shown

        // The screens the Displays tab has ticked, as screen objects.
        screens: {
            const kept = [];
            const all = Application.screens;
            for (let i = 0; i < all.length; ++i) {
                if (picker.screensWanted.indexOf(all[i].name) >= 0)
                    kept.push(all[i]);
            }
            return kept;
        }
    }

    // The mouse is asked after only while a scene is on show and in view:
    // nothing else here follows it, and every move is a call over the bus.
    Binding {
        target: Pointer
        property: "watching"
        value: clone.showingScene && !clone.covered && picker.mouse
    }

    // How strongly the music is heard, for scenes and pages alike.
    Binding {
        target: SceneSounds
        property: "gain"
        value: picker.musicGain / 100
    }

    // The window at the start only when the Settings tab says so; otherwise
    // the program comes up as the tray icon alone.
    Component.onCompleted: if (!picker.startMinimized) root.showPicker()

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

        // Not transient for a wallpaper. Those are layer-shell surfaces the
        // compositor keeps in the background and which take no input, and a
        // window belonging to one would be dragged down there with it.
        transientParent: null
        run: clone.run
        showing: clone.showing
        onChose: (id) => clone.select(id)

        /// Whether peeking at the desktop put this window away, and so
        /// whether it comes back when the peeking stops. A window that was
        /// shut or minimised already stays as it was.
        property bool tucked: false

        // Out of the way while somebody peeks at the desktop, as every
        // other program's windows are -- KWin leaves this one, see
        // `ScreenCover.tuckAway`.
        Connections {
            target: ScreenCover
            function onShowingDesktopChanged() {
                if (ScreenCover.showingDesktop) {
                    picker.tucked = picker.visible && picker.visibility !== Window.Minimized;
                    if (picker.tucked)
                        ScreenCover.tuckAway(true);
                } else if (picker.tucked) {
                    picker.tucked = false;
                    ScreenCover.tuckAway(false);
                }
            }
        }
    }

    Tray {
        visible: picker.tray
        // Ours, in the window's colours: the theme's target colours rather
        // than the ones easing towards them, or a change of theme would
        // send the panel a new picture every frame of the fade.
        svg: {
            const set = picker.theme === "system" ? picker.systemColours
                                                  : (picker.themes[picker.theme] || picker.themes.dark);
            const colours = { dark: set.dark, window: Qt.color(set.window), base: Qt.color(set.base) };
            return Emblem.svg(colours, Qt.color(picker.accent !== "" ? picker.accent : set.highlight));
        }
        tooltip: !clone.showing.current.playable ? qsTr("WallFlux")
                 : root.paused ? qsTr("WallFlux — %1 (paused)").arg(clone.showing.current.title)
                 : qsTr("WallFlux — %1").arg(clone.showing.current.title)

        // Not one checkable item here: the words say the state. Recent is
        // the last five chosen in this run, the one on show among them and
        // greyed out; nothing of it is kept for the next start.
        items: [
            { id: "choose", text: qsTr("Choose wallpaper…") },
            { separator: true },
            { id: "pause", text: root.paused ? qsTr("Resume") : qsTr("Pause"),
              icon: root.paused ? "media-playback-start" : "media-playback-pause",
              enabled: clone.playing },
            { id: "previous", text: qsTr("Previous wallpaper"), icon: "media-skip-backward" },
            { id: "next", text: qsTr("Next wallpaper"), icon: "media-skip-forward" },
            { id: "mute", text: picker.muted ? qsTr("Unmute") : qsTr("Mute"),
              icon: picker.muted ? "audio-volume-high" : "audio-volume-muted" },
            { text: qsTr("Recent wallpapers"), icon: "document-open-recent",
              enabled: Wallpapers.recent.length > 0,
              submenu: Wallpapers.recent.slice(0, 5).map(id => ({
                  id: "recent:" + id, text: picker.titleOf(id), enabled: id !== clone.showing.current.id })) },
            { separator: true },
            { id: "quit", text: qsTr("Quit") }
        ]

        onClicked: root.showPicker()
        onTriggered: (id) => {
            switch (id) {
            case "choose": root.showPicker(); break;
            case "pause": root.paused = !root.paused; break;
            case "previous": clone.step(-1); break;
            case "next": clone.step(1); break;
            case "mute": picker.muted = !picker.muted; break;
            case "quit": Qt.quit(); break;
            default:
                if (id.startsWith("recent:"))
                    clone.select(id.slice(7));
            }
        }
    }
}
