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

    /// Paused from the tray, every monitor at once: stood still and quiet
    /// as under a window, and a playlist held unless it was told to change
    /// while paused -- what Wallpaper Engine's own Pause does. A monitor
    /// paused on its own is its `Display`'s, and resuming everything leaves
    /// it standing. Forgotten at quit, like the rest.
    property bool paused: false

    /// Every connector name seen in this run, in the order they came. A
    /// monitor unplugged keeps its `Display` -- its wallpaper, playlist and
    /// pause -- for when it comes back, until the program quits.
    ///
    /// A ListModel that is only ever appended to, never a JS array: an
    /// Instantiator over an array builds every delegate again when the
    /// array is replaced, so plugging in a new monitor threw away what all
    /// the others were showing.
    ListModel { id: seen }

    function seenNames() {
        const names = [];
        for (let i = 0; i < seen.count; ++i)
            names.push(seen.get(i).name);
        return names;
    }

    /// Adds the monitors not seen yet in this run.
    function noteSeen(names) {
        const known = root.seenNames();
        for (const name of names) {
            if (known.indexOf(name) < 0) {
                seen.append({ name: name });
                known.push(name);
            }
        }
    }

    readonly property var connectedNames: Application.screens.map(s => s.name)
    // The monitors there at the start count as seen; a change handler is
    // not told the first value.
    Component.onCompleted: {
        root.noteSeen(root.connectedNames);
        if (!picker.startMinimized)
            root.showPicker();
    }
    onConnectedNamesChanged: root.noteSeen(root.connectedNames)

    readonly property bool perDisplay: picker.layout !== "clone"

    /// Whether any screen showing a wallpaper has a window in front of it:
    /// what "pause all" and "stop all" answer to.
    readonly property bool anyCovered: root.displays.some(d => d.connected && d.showing.current.id !== ""
        && d.screens.some(s => ScreenCover.coveredScreens.indexOf(s.name) >= 0))

    /// The displays in play, in monitor order: left to right, then top to
    /// bottom, one whose monitor is away last. One, the clone, when cloning.
    readonly property var displays: {
        if (!root.perDisplay)
            return [clone];
        const all = [];
        for (let i = 0; i < perMonitor.count; ++i) {
            const one = perMonitor.objectAt(i);
            if (one)
                all.push(one);
        }
        const place = d => d.connected ? d.screens[0].virtualX * 100000 + d.screens[0].virtualY
                                       : Number.MAX_SAFE_INTEGER;
        return all.sort((a, b) => place(a) - place(b));
    }

    /// The display a key names -- a connector name, or "clone" -- or null.
    function display(key) {
        if (key === "clone")
            return clone;
        for (let i = 0; i < perMonitor.count; ++i) {
            const one = perMonitor.objectAt(i);
            if (one && one.key === key)
                return one;
        }
        return null;
    }

    /// The first display in play: what the window and the tray drive until
    /// a monitor can be chosen.
    readonly property var primary: root.displays.length > 0 ? root.displays[0] : clone

    /// The displays muted on their own, by key. Handed to `isMuted` so
    /// that a binding calling it is told when the list changes.
    readonly property var mutedKeys: picker.mutedMonitors

    /// Whether a display's sound is off on its own, kept by key.
    function isMuted(key, keys) {
        return (keys || root.mutedKeys).indexOf(key) >= 0;
    }

    /// The display drawing on the monitor of this connector name: its own,
    /// or the clone unless the monitor is left out of it. `displays` is
    /// read so that a binding asking is told when they change.
    function displayOn(name, displays) {
        if (!root.perDisplay)
            return clone.screens.some(s => s.name === name) ? clone : null;
        return (displays || root.displays).find(d => d.key === name) || null;
    }

    /// A monitor's number, counted left to right and then top to bottom,
    /// as the tray and the window call it. `names` is read for the same
    /// reason as above.
    function monitorNumber(name, names) {
        const placed = Application.screens.slice()
            .sort((a, b) => a.virtualX - b.virtualX || a.virtualY - b.virtualY)
            .map(s => s.name);
        void names;
        const at = placed.indexOf(name);
        return at < 0 ? placed.length + 1 : at + 1;
    }

    /// "Monitor 2 · ENC CS2420": its number and what it is, or its
    /// connector where Qt does not know the make.
    function monitorLabel(name, names) {
        const screen = Application.screens.find(s => s.name === name);
        const made = screen ? (screen.manufacturer + " " + screen.model).trim() : "";
        return qsTr("Monitor %1 · %2").arg(root.monitorNumber(name, names)).arg(made !== "" ? made : name);
    }

    function setMuted(key, on) {
        const rest = picker.mutedMonitors.filter(k => k !== key);
        picker.mutedMonitors = on ? rest.concat([key]) : rest;
    }

    /// A transition left running would fire into a display that has just
    /// been changed under it; finished at once instead.
    function settleTransitions() {
        clone.swapNow();
        for (let i = 0; i < perMonitor.count; ++i) {
            const one = perMonitor.objectAt(i);
            if (one && one.arriving !== "")
                one.swapNow();
        }
    }

    /// Changes between a wallpaper per monitor and one cloned.
    ///
    /// To clone, the clone takes the chosen display's wallpaper and running
    /// playlist and every monitor's own is let go of. Back to one per
    /// monitor, every monitor starts with what was cloned, its playlist on
    /// the first.
    function setLayout(layout) {
        if (layout === picker.layout)
            return;
        root.settleTransitions();
        if (layout === "clone") {
            const from = picker.selected;
            if (from.showing.current.id !== "") {
                picker.copyProperties(from.showing.current.id, from.key, "clone");
                clone.select(from.showing.current.id);
            }
            else
                clone.clear();
            if (from.run.playing !== "")
                clone.run.start(Playlists.named(from.run.playing));
            root.letGoOf("perDisplay");
            picker.layout = "clone";
            return;
        }
        const id = clone.showing.current.id;
        const playlist = clone.run.playing;
        root.letGoOf("clone");
        picker.layout = "perDisplay";
        const all = root.displays;
        for (let i = 0; i < all.length; ++i) {
            all[i].run.stop();
            if (id !== "") {
                picker.copyProperties(id, "clone", all[i].key);
                all[i].select(id);
            }
            else
                all[i].clear();
        }
        if (playlist !== "" && all.length > 0)
            all[0].run.start(Playlists.named(playlist));
    }

    /// Lets go of every display of a layout: the clone, or every monitor's
    /// own -- what they show and the playlists running on them.
    function letGoOf(layout) {
        if (layout === "clone") {
            clone.clear();
            return;
        }
        for (let i = 0; i < perMonitor.count; ++i) {
            const one = perMonitor.objectAt(i);
            if (one)
                one.clear();
        }
    }

    /// Puts one display's wallpaper on another.
    function copyTo(fromKey, toKey) {
        const from = root.display(fromKey);
        const to = root.display(toKey);
        if (!from || !to || from === to)
            return;
        to.run.stop();
        const id = from.showing.current.id;
        if (id !== "") {
            picker.copyProperties(id, from.key, to.key);
            to.select(id);
        } else {
            to.clear();
        }
    }

    /// Swaps two displays' wallpapers and playlists, and their mute.
    function swap(aKey, bKey) {
        const a = root.display(aKey);
        const b = root.display(bKey);
        if (!a || !b || a === b)
            return;
        root.settleTransitions();
        const was = { id: a.showing.current.id, playlist: a.run.playing, muted: root.isMuted(aKey),
                      properties: picker.ownProperties(a.showing.current.id, aKey) };
        const other = { id: b.showing.current.id, playlist: b.run.playing, muted: root.isMuted(bKey),
                        properties: picker.ownProperties(b.showing.current.id, bKey) };
        const put = (d, key, what) => {
            d.run.stop();
            // The playlist first and its first turn finished at once, then
            // the wallpaper swapped in, or the playlist's first wallpaper
            // would take its place straight away.
            if (what.playlist !== "") {
                d.run.start(Playlists.named(what.playlist));
                d.swapNow();
            }
            if (what.id !== "") {
                picker.setOwnProperties(what.id, key, what.properties);
                d.select(what.id);
            } else {
                d.clear();
            }
            root.setMuted(key, what.muted);
        };
        put(a, aKey, other);
        put(b, bKey, was);
    }

    /// Takes the wallpaper off a monitor. Cloned, a monitor is left out of
    /// the clone -- unless it is the one the clone is worked out on, which
    /// turns the clone off.
    function removeFrom(key) {
        if (root.perDisplay) {
            const one = root.display(key);
            if (one)
                one.clear();
            return;
        }
        if (key === "clone" || (clone.screens.length > 0 && clone.screens[0].name === key)) {
            clone.clear();
            return;
        }
        if (picker.excludedFromClone.indexOf(key) < 0)
            picker.excludedFromClone = picker.excludedFromClone.concat([key]);
    }

    /// The monitor the clone is worked out on, or empty while it is on none.
    function cloneSourceName() {
        return clone.screens.length > 0 ? clone.screens[0].name : "";
    }

    /// Puts a monitor back into the clone, if it was left out.
    function includeInClone(key) {
        picker.excludedFromClone = picker.excludedFromClone.filter(k => k !== key);
    }

    /// The whole arrangement, for a profile: the layout and, for every
    /// display, what it shows, its playlist and its mute.
    /// `keys`, where given, are the monitors to keep and no others: a
    /// profile of one monitor touches that one alone when it is loaded --
    /// the user's word of 25 November.
    function arrangement(keys) {
        const displays = {};
        const put = (key, d) => {
            displays[key] = { wallpaper: d.showing.current.id, playlist: d.run.playing,
                              muted: root.isMuted(key),
                              properties: picker.ownProperties(d.showing.current.id, key) };
        };
        if (root.perDisplay) {
            for (let i = 0; i < perMonitor.count; ++i) {
                const one = perMonitor.objectAt(i);
                if (one && (!keys || keys.indexOf(one.key) >= 0))
                    put(one.key, one);
            }
        } else {
            put("clone", clone);
        }
        return { layout: picker.layout, displays: displays,
                 excluded: picker.excludedFromClone, cloneSource: picker.cloneSource };
    }

    /// A profile's arrangement put in place. Monitors it names that are not
    /// here are kept for when they come; monitors it says nothing of stay
    /// as they are.
    function applyProfile(profile) {
        if (!profile || !profile.displays)
            return;
        root.settleTransitions();
        const layout = profile.layout === "clone" ? "clone" : "perDisplay";
        // The side being left is let go of, as `setLayout` does, or its
        // playlists would run on held and unseen and come back later.
        if (layout !== picker.layout)
            root.letGoOf(picker.layout);
        picker.layout = layout;
        if (profile.layout === "clone") {
            picker.excludedFromClone = profile.excluded || [];
            picker.cloneSource = profile.cloneSource || "";
        }
        const names = Object.keys(profile.displays);
        root.noteSeen(names.filter(n => n !== "clone"));
        for (const key of names) {
            const what = profile.displays[key];
            const d = root.display(key);
            if (!d)
                continue;
            d.run.stop();
            if (what.wallpaper) {
                picker.setOwnProperties(what.wallpaper, key, what.properties || {});
                d.select(what.wallpaper);
            }
            else
                d.clear();
            if (what.playlist)
                d.run.start(Playlists.named(what.playlist));
            root.setMuted(key, !!what.muted);
        }
    }

    /// Every monitor that is not left out, showing one wallpaper worked out
    /// once -- Wallpaper Engine's "Clone single wallpaper". The source, the
    /// one that works it out, first.
    Display {
        id: clone

        key: "clone"
        options: picker
        pausedAll: root.paused
        mutedAll: picker.muted
        mutedHere: root.isMuted("clone", root.mutedKeys)
        anyCovered: root.anyCovered
        order: picker.shown
        screens: {
            if (root.perDisplay)
                return [];
            const kept = Application.screens.filter(s => picker.excludedFromClone.indexOf(s.name) < 0);
            const source = kept.findIndex(s => s.name === picker.cloneSource);
            if (source > 0)
                kept.unshift(kept.splice(source, 1)[0]);
            return kept;
        }
    }

    /// A wallpaper per monitor, as Wallpaper Engine's default: one display
    /// for every monitor seen in this run.
    Instantiator {
        id: perMonitor

        model: seen

        delegate: Display {
            required property string name

            key: name
            options: picker
            pausedAll: root.paused
            mutedAll: picker.muted
            mutedHere: root.isMuted(name, root.mutedKeys)
            anyCovered: root.anyCovered
            order: picker.shown
            screens: root.perDisplay ? Application.screens.filter(s => s.name === name) : []
        }
    }

    // The mouse is asked after only while a scene is on show and in view:
    // nothing else here follows it, and every move is a call over the bus.
    Binding {
        target: Pointer
        property: "watching"
        value: picker.mouse && root.displays.some(d => d.connected && d.showingScene && !d.covered)
    }

    // How strongly the music is heard, for scenes and pages alike.
    Binding {
        target: SceneSounds
        property: "gain"
        value: picker.musicGain / 100
    }

    // The window at the start only when the Settings tab says so; otherwise
    // the program comes up as the tray icon alone.

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
        stage: root
        onChose: (id) => picker.selected.select(id)

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
        tooltip: {
            if (picker.manyMonitors)
                return [qsTr("WallFlux")].concat(root.displays.filter(d => d.connected).map(d => root.trayLine(d))).join("\n");
            const d = picker.selected;
            return !d.showing.current.playable ? qsTr("WallFlux")
                 : root.paused ? qsTr("WallFlux — %1 (paused)").arg(d.showing.current.title)
                 : qsTr("WallFlux — %1").arg(d.showing.current.title);
        }

        // Not one checkable item here: the words say the state. Recent is
        // the last five chosen in this run, the one on show among them and
        // greyed out; nothing of it is kept for the next start. With a
        // wallpaper on each of several monitors, each monitor has a menu of
        // its own with the same things in it, and pausing and muting
        // everything stay at the top -- two things apart from each
        // monitor's own, so that Resume all leaves a monitor paused on its
        // own standing still.
        items: picker.manyMonitors ? root.manyItems() : root.oneItems(picker.selected)

        onClicked: root.showPicker()
        onTriggered: (id) => root.trayChose(id)
    }

    /// A tray item chosen, by its id: "<what>@<monitor>",
    /// "<what>@<monitor>:<which>", or one of the program's own.
    function trayChose(id) {
        // A profile first: its name is anybody's and may hold an "@".
        if (id.startsWith("profile:")) {
            root.applyProfile(Profiles.named(id.slice(8)));
            return;
        }
        // "<what>@<monitor>", or "<what>@<monitor>:<which>".
        const at = id.indexOf("@");
        if (at >= 0) {
            const what = id.slice(0, at);
            const rest = id.slice(at + 1);
            const colon = rest.indexOf(":");
            const key = colon >= 0 ? rest.slice(0, colon) : rest;
            const which = colon >= 0 ? rest.slice(colon + 1) : "";
            root.onDisplay(root.display(key), what, which);
            return;
        }
        switch (id) {
        case "choose": root.showPicker(); break;
        case "pauseall": root.paused = !root.paused; break;
        case "muteall": picker.muted = !picker.muted; break;
        case "quit": Qt.quit(); break;
        }
    }

    /// One monitor's line: its label and what it shows.
    function trayLine(d) {
        const title = d.showing.current.id !== "" ? d.showing.current.title : qsTr("nothing");
        return qsTr("%1 — %2").arg(root.monitorLabel(d.key, root.connectedNames)).arg(title);
    }

    /// The last five wallpapers chosen, for a display's menu: always five
    /// places, the unused ones hidden, so that the menu keeps its shape as
    /// the list fills -- a menu that changes shape is built again, and the
    /// panel is not to have one pulled apart under it.
    function recentItems(key, onShow) {
        const ids = Wallpapers.recent.slice(0, 5);
        const items = [];
        for (let i = 0; i < 5; ++i) {
            const id = ids[i];
            items.push(id === undefined ? { id: "", text: "", visible: false }
                                        : { id: "recent@" + key + ":" + id, text: picker.titleOf(id),
                                            enabled: id !== onShow });
        }
        return items;
    }

    /// What a display's own menu holds. `key` is put into every id, so the
    /// one menu works for the single display of today and for each of
    /// several.
    function displayItems(d) {
        const key = d.key;
        const own = root.perDisplay;
        return [
            { id: "pause@" + key, text: d.pausedHere ? qsTr("Resume") : qsTr("Pause"),
              icon: d.pausedHere ? "media-playback-start" : "media-playback-pause",
              enabled: d.playing },
            { id: "previous@" + key, text: qsTr("Previous wallpaper"), icon: "media-skip-backward" },
            { id: "next@" + key, text: qsTr("Next wallpaper"), icon: "media-skip-forward" },
            { id: "mute@" + key, text: root.isMuted(key, root.mutedKeys) ? qsTr("Unmute") : qsTr("Mute"),
              icon: root.isMuted(key, root.mutedKeys) ? "audio-volume-high" : "audio-volume-muted" },
            { text: qsTr("Recent wallpapers"), icon: "document-open-recent",
              enabled: Wallpapers.recent.length > 0,
              submenu: root.recentItems(key, d.showing.current.id) },
            { text: qsTr("Playlists"), icon: "view-media-playlist",
              enabled: Playlists.all.length > 0,
              submenu: Playlists.all.map(one => ({
                  id: "playlist@" + key + ":" + one.name,
                  text: (d.run.playing === one.name ? "▶  " : "") + one.name })) },
            { id: "remove@" + key, text: qsTr("Remove wallpaper"), icon: "edit-clear",
              enabled: d.showing.current.id !== "", visible: own }
        ];
    }

    /// The menu with one display: today's, Pause and Mute being the whole
    /// program's.
    function oneItems(d) {
        return [
            { id: "choose", text: qsTr("Choose wallpaper…") },
            { separator: true },
            { id: "pauseall", text: root.paused ? qsTr("Resume") : qsTr("Pause"),
              icon: root.paused ? "media-playback-start" : "media-playback-pause",
              enabled: d.playing },
            { id: "previous@" + d.key, text: qsTr("Previous wallpaper"), icon: "media-skip-backward" },
            { id: "next@" + d.key, text: qsTr("Next wallpaper"), icon: "media-skip-forward" },
            { id: "muteall", text: picker.muted ? qsTr("Unmute") : qsTr("Mute"),
              icon: picker.muted ? "audio-volume-high" : "audio-volume-muted" },
            { text: qsTr("Recent wallpapers"), icon: "document-open-recent",
              enabled: Wallpapers.recent.length > 0,
              submenu: root.recentItems(d.key, d.showing.current.id) },
            { separator: true },
            { id: "quit", text: qsTr("Quit") }
        ];
    }

    /// The menu with a wallpaper on each of several monitors.
    function manyItems() {
        const monitors = root.displays.filter(d => d.connected).map(d => ({
            text: root.trayLine(d), icon: "video-display", submenu: root.displayItems(d) }));
        return [
            { id: "choose", text: qsTr("Choose wallpaper…") },
            { separator: true }
        ].concat(monitors).concat([
            { separator: true },
            { id: "pauseall", text: root.paused ? qsTr("Resume all") : qsTr("Pause all"),
              icon: root.paused ? "media-playback-start" : "media-playback-pause" },
            { id: "muteall", text: picker.muted ? qsTr("Unmute all") : qsTr("Mute all"),
              icon: picker.muted ? "audio-volume-high" : "audio-volume-muted" },
            { text: qsTr("Profiles"), icon: "document-open",
              enabled: Profiles.all.length > 0,
              submenu: Profiles.all.map(one => ({ id: "profile:" + one.name, text: one.name })) },
            { separator: true },
            { id: "quit", text: qsTr("Quit") }
        ]);
    }

    /// One display's menu item chosen.
    function onDisplay(d, what, which) {
        if (!d)
            return;
        switch (what) {
        case "pause": d.pausedHere = !d.pausedHere; break;
        case "previous": d.step(-1); break;
        case "next": d.step(1); break;
        case "mute": root.setMuted(d.key, !root.isMuted(d.key)); break;
        case "recent": d.select(which); break;
        case "playlist":
            if (d.run.playing === which)
                d.run.stop();
            else
                d.run.start(Playlists.named(which));
            break;
        case "remove": root.removeFrom(d.key); break;
        }
    }
}
