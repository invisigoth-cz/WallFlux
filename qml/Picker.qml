// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtCore
import QtQuick
import QtQuick.Window
// The Fusion style by name rather than through QQuickStyle: it is the one
// that takes a palette, and asking for it here keeps the choice beside the
// colours it is set with.
import QtQuick.Controls.Fusion
import QtQuick.Effects
import QtQuick.Layouts
import WallFlux
import "steamprofile.js" as Steam

/// The window for choosing a wallpaper: everything installed, in a grid of
/// previews, and what the one that is playing sounds like.
///
/// Says what was clicked and nothing more -- it neither plays a wallpaper nor
/// remembers one. Which of the two is on screen is the wallpaper window's
/// business, and remembering is nobody's, by design.
ApplicationWindow {
    id: picker

    /// What was clicked. Never emitted for a wallpaper that cannot be played.
    signal chose(string id)

    /// Whether the sound is off. Read by the players in Main.qml, together
    /// with `volume`: the controls in this window are the setting itself, so
    /// there is one place it lives and nothing to keep in step.
    property bool muted: false

    /// How loud a video wallpaper plays, 0 to 1.
    ///
    /// One volume rather than one per wallpaper: a volume per wallpaper would
    /// have to be remembered between runs, and this program deliberately
    /// remembers nothing.
    property real volume: 1

    /// The names of the screens the wallpaper is wanted on. Read by
    /// `Main.qml`, which builds one surface per name.
    readonly property var screensWanted: displays.wanted

    /// Which set of colours the window wears: `light`, `dark`, `system`
    /// (whichever the desktop is), `steam`, `hardLight` or `hardDark` -- the
    /// last two high contrast, pure white or pure black with hard edges.
    /// Dark to start with, because a wallpaper picker is a window full of
    /// pictures.
    property string theme: "dark"
    /// The colour buttons, switches and the ring are lit in, or empty for
    /// the theme's own.
    property string accent: ""
    /// The typeface the window is written in, or empty for the desktop's
    /// own -- Noto Sans on Plasma unless somebody changed it there.
    property string typeface: ""
    font.family: picker.typeface !== "" ? picker.typeface : Qt.application.font.family
    /// Whether the colours are the dark kind, whichever theme gave them.
    readonly property bool dark: shades.dark
    /// The desktop's own colours, for the System theme's tile.
    readonly property var systemColours: shades.systemSet

    /// Each theme's colours. Light and dark are Breeze's own two, so the
    /// window belongs on this desktop rather than looking like a visitor;
    /// Steam's are its client's.
    readonly property var themes: ({
        light: { dark: false, window: "#eff0f1", base: "#fcfcfc", text: "#232629",
                 button: "#fcfcfc", mid: "#c4c8cc", highlight: "#3daee9" },
        dark: { dark: true, window: "#1b1e20", base: "#232629", text: "#fcfcfc",
                button: "#31363b", mid: "#3a4045", highlight: "#3daee9" },
        steam: { dark: true, window: "#171a21", base: "#1b2838", text: "#c7d5e0",
                 button: "#2a475e", mid: "#2f4157", highlight: "#66c0f4" },
        hardLight: { dark: false, window: "#ffffff", base: "#ffffff", text: "#000000",
                     button: "#ffffff", mid: "#000000", highlight: "#0040c0" },
        hardDark: { dark: true, window: "#000000", base: "#000000", text: "#ffffff",
                    button: "#000000", mid: "#ffffff", highlight: "#ffd400" }
    })

    /// The colours in use, eased from one theme to the next rather than
    /// switched: everything in the window takes them through its palette.
    Item {
        id: shades

        visible: false
        // System: the desktop's own colour scheme, as Plasma hands it to
        // every program -- not Breeze's dark or light, which are only what
        // Plasma ships with.
        SystemPalette { id: desktop; colorGroup: SystemPalette.Active }
        readonly property var systemSet: ({
            dark: Qt.styleHints.colorScheme === Qt.ColorScheme.Dark
                  || (0.2126 * desktop.window.r + 0.7152 * desktop.window.g + 0.0722 * desktop.window.b) < 0.5,
            window: desktop.window, base: desktop.base, text: desktop.windowText,
            button: desktop.button, mid: desktop.mid, highlight: desktop.highlight
        })
        readonly property var set: picker.theme === "system"
                                   ? shades.systemSet
                                   : (picker.themes[picker.theme] || picker.themes.dark)
        readonly property bool dark: shades.set.dark

        property color window: shades.set.window
        property color base: shades.set.base
        property color text: shades.set.text
        property color button: shades.set.button
        property color mid: shades.set.mid
        property color highlight: picker.accent !== "" ? picker.accent : shades.set.highlight
        /// White on a dark accent, black on a light one (Hard Dark's yellow).
        readonly property color onHighlight:
            (0.2126 * shades.highlight.r + 0.7152 * shades.highlight.g + 0.0722 * shades.highlight.b) > 0.7
            ? "#000000" : "#ffffff"

        Behavior on window { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
        Behavior on base { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
        Behavior on text { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
        Behavior on button { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
        Behavior on mid { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
        Behavior on highlight { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }
    }

    /// What the Settings tab sets, read by Main.qml for every screen.
    /// Pictures a second: Wallpaper Engine's presets give 10, 15, 25 and 30,
    /// and its welcome dialog picks 15.
    property int fps: 15
    /// Whether a wallpaper may follow the music the speakers are playing.
    property bool music: true
    /// Whether a scene may follow the mouse: lean its layers, push its particles.
    property bool mouse: true
    property bool parallax: true
    property bool particles: true
    /// Whether the Steam tab shows Anime; see `showAnime` in WorkshopView.
    property bool showAnime: true
    /// Whether the status bar says what WallFlux costs: processor, graphics
    /// card, memory.
    property bool usage: false
    /// Whether the window is tinted by the wallpaper it is showing.
    property bool tint: true
    /// Whether the strip along the bottom is there: the version, Turn off,
    /// the sound and what is playing.
    property bool statusBar: true

    /// General: the tray icon, and what the window does at the start and on
    /// being closed. Without the icon the window is reached by starting
    /// WallFlux again, which shows the copy already running.
    property bool tray: true
    property bool startMinimized: true
    property bool closeToTray: true

    /// Audio: whether the wallpaper goes quiet by itself while another
    /// program plays, and while a window covers the screen -- a covered
    /// wallpaper is also stopped, so with this off it goes on playing.
    property bool quietForOthers: true
    property bool quietUnderWindows: true
    /// And stopped while another program has the keyboard -- Wallpaper
    /// Engine's "other application focused". Off unless asked for.
    property bool pauseOnFocus: false

    /// Whether GitHub is asked about a newer WallFlux when the window first
    /// opens -- not at startup: a program nobody has looked at yet should not
    /// be talking to anybody.
    property bool checkUpdates: true

    /// On a battery: drawn on the Low preset, and stopped below so many per
    /// cent of it (nought for never).
    property bool batteryLow: true
    property int batteryFloor: 20

    /// How a wallpaper meets a screen of another shape: `default` (as
    /// Wallpaper Engine draws it, which is cropped to fill), `fill`, `fit`
    /// with bars, or `stretch`.
    property string scaling: "default"

    /// Quality, as Wallpaper Engine's: samples a pixel for smoothing edges
    /// (0, 2, 4, 8), whether a scene may glow -- its "post-processing",
    /// which reaches the bloom and nothing else -- and pictures at half
    /// their size. With `fps`, the first two are what a preset sets.
    property int msaa: 0
    property bool bloom: true
    property bool halfTextures: false
    /// A wallpaper's own preset, by its id, over the settings.
    property var qualityOf: ({})
    /// A wallpaper's own settings as set in the panel, by its id and then by
    /// the setting's name: only what was changed, laid over project.json.
    property var propertiesOf: ({})

    /// How each wallpaper fared the first time it was played, and every
    /// time after, by id: `{ grade, why }`, the grade "perfect", "minor" or
    /// "broken" and `why` what the renderer left out or the error. Measured
    /// rather than guessed: a scene's renderer says what it could not build,
    /// a video or a page whether it would play at all.
    property var tried: ({})

    /// Writes down how a wallpaper fared. Only a change is written, since
    /// the grid is built out of this while the filter asks about it.
    function noteCompatibility(id, grade, why) {
        const was = picker.tried[id];
        if (was && was.grade === grade && JSON.stringify(was.why) === JSON.stringify(why))
            return;
        const all = Object.assign({}, picker.tried);
        all[id] = { grade: grade, why: why };
        picker.tried = all;
    }

    /// A wallpaper's name by its id, or the id where nothing has it.
    function titleOf(id) {
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            if (all[i].id === id)
                return all[i].title;
        }
        return id;
    }

    /// Every wallpaper that can be played, tried out by Compatibility --
    /// unseen and unheard, with its own settings -- and graded as it goes.
    function testLibrary() {
        const kinds = {};
        kinds[WallpaperKind.Scene] = "scene";
        kinds[WallpaperKind.Video] = "video";
        kinds[WallpaperKind.Web] = "web";
        const all = Wallpapers.wallpapers;
        const list = [];
        for (let i = 0; i < all.length; ++i) {
            const one = all[i];
            if (one.playable && kinds[one.kind])
                list.push({ id: one.id, kind: kinds[one.kind], source: one.source.toString(),
                            values: picker.valuesFor(one) });
        }
        Compatibility.test(list);
    }

    Connections {
        target: Compatibility
        function onGraded(id, grade, why) { picker.noteCompatibility(id, grade, why); }
    }

    /// How a wallpaper fares: what it was found to do when it was played,
    /// "broken" where it cannot be played at all, and "unknown" until then.
    function compatibility(one) {
        if (!one.playable)
            return "broken";
        const was = picker.tried[one.id];
        return was ? was.grade : "unknown";
    }

    function compatibilityName(grade, language) {
        switch (grade) {
        case "perfect": return qsTr("Works");
        case "minor": return qsTr("Minor issues");
        case "broken": return qsTr("Broken");
        }
        return qsTr("Not tried yet");
    }

    function setProperty(id, name, value) {
        const all = Object.assign({}, picker.propertiesOf);
        all[id] = Object.assign({}, all[id] || {});
        all[id][name] = value;
        picker.propertiesOf = all;
    }
    function resetProperties(id) {
        const all = Object.assign({}, picker.propertiesOf);
        delete all[id];
        picker.propertiesOf = all;
    }
    /// A wallpaper's settings as project.json declares them, with what was
    /// set laid over: the JSON a page is handed by `applyUserProperties`.
    function propertiesFor(one) {
        if (!one || !one.properties)
            return "";
        const declared = JSON.parse(one.properties);
        const set = picker.propertiesOf[one.id] || {};
        for (const name in set) {
            if (declared[name])
                declared[name].value = set[name];
        }
        return JSON.stringify(declared);
    }
    /// The same as name to value, which is what the renderer takes.
    function valuesFor(one) {
        const all = picker.propertiesFor(one);
        if (!all)
            return "";
        const declared = JSON.parse(all);
        const values = {};
        for (const name in declared) {
            if (declared[name].value !== undefined)
                values[name] = declared[name].value;
        }
        return JSON.stringify(values);
    }

    /// Wallpaper Engine's four, out of `getQualityPreset` in its
    /// `ui/dist/scripts/scripts.js`. Ultra's own difference there is HDR
    /// bloom, which no scene here asks for, so it is High at thirty.
    readonly property var presets: ({
        low: { fps: 10, msaa: 0, bloom: false },
        medium: { fps: 15, msaa: 0, bloom: true },
        high: { fps: 25, msaa: 2, bloom: true },
        ultra: { fps: 30, msaa: 2, bloom: true }
    })
    /// Which preset the settings are, or empty when they are somebody's own.
    readonly property string quality: {
        for (const name in picker.presets) {
            const p = picker.presets[name];
            if (p.fps === picker.fps && p.msaa === picker.msaa && p.bloom === picker.bloom)
                return name;
        }
        return "";
    }
    function usePreset(name) {
        const p = picker.presets[name];
        picker.fps = p.fps;
        picker.msaa = p.msaa;
        picker.bloom = p.bloom;
    }
    /// What the wallpaper on show is drawn with: its own preset where it
    /// has one, the settings where it has not.
    readonly property var drawnWith: {
        if (picker.batteryLow && Power.onBattery)
            return picker.presets.low;
        const own = picker.presets[picker.qualityOf[Wallpapers.current.id]];
        return own ? own : { fps: picker.fps, msaa: picker.msaa, bloom: picker.bloom };
    }
    /// A new map, not the old one changed: see the trap about putting an
    /// answer back.
    function setQualityOf(id, name) {
        const all = Object.assign({}, picker.qualityOf);
        if (name)
            all[id] = name;
        else
            delete all[id];
        picker.qualityOf = all;
    }
    /// What a preset is called; `language` is never read, see the trap.
    function qualityName(name, language) {
        switch (name) {
        case "low": return qsTr("Low");
        case "medium": return qsTr("Medium");
        case "high": return qsTr("High");
        case "ultra": return qsTr("Ultra");
        default: return qsTr("As in Settings");
        }
    }

    // The one place anything here is remembered between runs, besides the
    // playlists: ~/.config/WallFlux/wallflux.conf. The wallpaper itself is
    // still not -- nothing plays at startup, by design.
    Settings {
        id: remembered

        category: "settings"
        property alias theme: picker.theme
        property alias accent: picker.accent
        property alias typeface: picker.typeface
        property alias statusBar: picker.statusBar
        property alias tray: picker.tray
        property alias startMinimized: picker.startMinimized
        property alias closeToTray: picker.closeToTray
        property alias quietForOthers: picker.quietForOthers
        property alias quietUnderWindows: picker.quietUnderWindows
        property alias pauseOnFocus: picker.pauseOnFocus
        property alias checkUpdates: picker.checkUpdates
        property alias batteryLow: picker.batteryLow
        property alias batteryFloor: picker.batteryFloor
        property alias scaling: picker.scaling
        property alias msaa: picker.msaa
        property alias bloom: picker.bloom
        property alias halfTextures: picker.halfTextures
        property alias qualityOf: picker.qualityOf
        property alias propertiesOf: picker.propertiesOf
        property alias tried: picker.tried
        property alias muted: picker.muted
        property alias volume: picker.volume
        property alias fps: picker.fps
        property alias music: picker.music
        property alias mouse: picker.mouse
        property alias parallax: picker.parallax
        property alias particles: picker.particles
        property alias showAnime: picker.showAnime
        property alias usage: picker.usage
        property alias tint: picker.tint
        property bool czech: false
        property var screens: []
    }

    Component.onCompleted: {
        Language.czech = remembered.czech;
        // Kept as dark or not before there were themes.
        if (String(remembered.value("theme", "")) === "" && String(remembered.value("dark", "true")) === "false")
            picker.theme = "light";
    }

    // Closed with the tray icon off or told not to hide, closing is quitting.
    onClosing: if (!picker.closeToTray) Qt.quit()
    Connections {
        target: Language
        function onCzechChanged() { remembered.czech = Language.czech; }
    }

    /// The one the details panel is about, or null while it is shut.
    property var inspected: null

    // Its Remove needs Steam, and Steam usually comes up after WallFlux at a
    // login: asked once at startup it said no, and nothing asked again, so
    // the button stayed grey with Steam running. Looking at one asks again.
    onInspectedChanged: if (picker.inspected && !Steamworks.available) Steamworks.retry()

    /// Whether a wallpaper came from the workshop. Wallpaper Engine's own --
    /// Beach, Deep Space and the rest -- are named rather than numbered, have
    /// no author and no workshop item, and are not Steam's to take back.
    function fromWorkshop(one) {
        return !!one && /^\d+$/.test(one.id);
    }

    /// Whether a wallpaper can be given back from here. Steam owns a
    /// workshop item's folder, so Steam is what takes it away: without it
    /// running there is nothing to do but delete somebody's files behind
    /// Steam's back, and they would come back the next time it looked. One
    /// of Wallpaper Engine's own needs no Steam: it is hidden.
    function canRemove(one) {
        return !!one && (one.official || (picker.fromWorkshop(one) && Steamworks.available));
    }

    /// Gives a wallpaper back -- the panel's Remove and the Delete key both
    /// come here, each after asking.
    function remove(one) {
        if (!picker.canRemove(one))
            return;
        // Taking the file out from under a player that is drawing it
        // leaves a black screen with no reason given.
        if (Wallpapers.current.id === one.id)
            Wallpapers.clear();
        // Two halves of the same thing: Steam lets go of it, and the
        // folder goes. Steam leaves the folder exactly where it is --
        // measured -- so the second half is ours.
        if (!one.official)
            Steamworks.remove(one.id);
        Wallpapers.discard(one.id);
        if (picker.inspected && picker.inspected.id === one.id) {
            details.open = false;
            picker.inspected = null;
        }
    }

    /// Which card the ring is on, as an id rather than as a row.
    ///
    /// A GridView puts its own `currentIndex` back to nought every time
    /// its model changes, and this grid's model changes whenever a video
    /// answers with its size -- a dozen times in the first seconds after
    /// a scan, and again whenever Steam finishes a download. A row number
    /// is therefore no answer to "which card is chosen": it was being
    /// handed back to the first card a moment after every click, so only
    /// the first card could be picked at all.
    ///
    /// So nothing here reads that row. The id is the ring, the grid's own
    /// key navigation is switched off, and the arrows move the id. Putting
    /// the row back after each rebuild was tried first and is a race: the
    /// grid resets itself whenever it gets round to it, which is sometimes
    /// after the repair has run -- measured, with the ring landing on a
    /// card nobody had pressed an arrow towards.
    property string ringOn: ""

    /// The wallpapers ticked for a playlist, as ids in the order they were
    /// ticked. Nothing is remembered about this between runs: a playlist
    /// is remembered once it is saved and not before.
    property var inPlaylist: []

    /// What the playlist being put together is called, until it is saved.
    ///
    /// Nothing else about how it runs is here: the spin boxes and the drop
    /// downs in the settings popup hold their own values and the window
    /// reads them off when it saves. Binding a control's value to a
    /// property and setting that property back from the control is the trap
    /// this project has been caught by twice -- a control told what to be
    /// and also asked what it is ends up fighting its own binding.
    /// Measured: the box read 1 and the property was still 15, so a
    /// playlist set to one minute was saved as a quarter of an hour and sat
    /// there not changing anything.
    property string playlistName: ""

    /// Whether every card is showing its tick box rather than the one under
    /// the pointer. Add wallpaper in the playlist strip switches it.
    property bool picking: false

    /// The ticked wallpapers themselves, in the order they were ticked --
    /// which is what the strip along the bottom draws.
    readonly property var playlistItems: {
        const all = Wallpapers.wallpapers;
        const held = [];
        for (let i = 0; i < picker.inPlaylist.length; ++i) {
            for (let w = 0; w < all.length; ++w) {
                if (all[w].id === picker.inPlaylist[i]) {
                    held.push(all[w]);
                    break;
                }
            }
        }
        return held;
    }

    /// Puts one in or takes it out.
    function tick(id) {
        picker.inPlaylist = picker.inPlaylist.indexOf(id) >= 0
            ? picker.inPlaylist.filter((kept) => kept !== id)
            : picker.inPlaylist.concat([id]);
    }

    /// Writes the playlist down under whatever it is called, giving it a
    /// name if nobody has. Every setting is read off the control holding
    /// it, for the reason spelled out on `playlistName`.
    function savePlaylist() {
        if (picker.playlistName === "")
            picker.playlistName = qsTr("Playlist %1").arg(Playlists.all.length + 1);
        Playlists.save(picker.playlistName, picker.inPlaylist, {
            hours: everyHours.value,
            minutes: everyMinutes.value,
            seconds: everySeconds.value,
            shuffle: orderBox.currentIndex === 1,
            videoEnds: afterVideoBox.checked,
            whilePaused: whilePausedBox.checked,
            alwaysFirst: firstBox.checked,
            transition: transitionBox.words[transitionBox.currentIndex],
            transitionMs: transitionTime.value
        });
    }

    /// Opens a saved playlist and sets it going: the strip fills, the
    /// settings come back, and its first wallpaper goes on the desktop.
    /// Picking one out of a list and then having to find the Playlists
    /// button to hear from it was a step nobody wanted.
    function playPlaylist(name) {
        picker.loadPlaylist(name);
        Playlists.start(name);
    }

    /// Puts a saved playlist back on the strip, settings and all, so it can
    /// be looked at and changed rather than only started.
    function loadPlaylist(name) {
        const one = Playlists.named(name);
        if (!one.name)
            return;
        picker.playlistName = one.name;
        picker.inPlaylist = one.ids;
        everyHours.value = one.hours;
        everyMinutes.value = one.minutes;
        everySeconds.value = one.seconds;
        orderBox.currentIndex = one.shuffle ? 1 : 2;
        afterVideoBox.checked = one.videoEnds;
        whilePausedBox.checked = one.whilePaused;
        firstBox.checked = one.alwaysFirst;
        transitionBox.currentIndex = Math.max(0, transitionBox.words.indexOf(one.transition));
        transitionTime.value = one.transitionMs;
    }

    /// One of the three numbers of a playlist's interval: a word, and a
    /// box with the number typed into it.
    ///
    /// The box holds its own value and the window reads it off when it
    /// saves -- `value` here is an alias onto the box itself, which is the
    /// same object read and written rather than a property bound to a
    /// control and set back from it. That is the trap this project has
    /// been caught by twice.
    /// One of the three numbers of a playlist's interval: a word, and a
    /// box with the number typed into it.
    ///
    /// The box holds its own value and the window reads it off when it
    /// saves -- `value` here is an alias onto the box itself, which is the
    /// same object read and written rather than a property bound to a
    /// control and set back from it. That is the trap this project has
    /// been caught by twice.
    component PlaylistNumber: ColumnLayout {
        id: number

        property alias value: box.value
        required property string label

        /// The most this box will take while it is being typed into. Not
        /// the ceiling of the unit: sixty seconds typed into the seconds
        /// is a minute, not an hour of arguing with a field that will not
        /// accept it, so the two-digit boxes take up to 99 and
        /// `settleInterval()` turns whatever was typed into a time.
        property int most: 99

        spacing: 4

        Label {
            // Both of these fill the column, so the word and the box it
            // is over are the same width and share a middle. Centre only
            // the word and it centres on whatever the column happens to
            // be, which is not where the box is.
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: number.label
            font.pixelSize: 12
        }

        SpinBox {
            id: box

            Layout.fillWidth: true
            Layout.preferredWidth: 96
            from: 0
            to: number.most
            editable: true
            // Typed, and only typed. Clicking an arrow fourteen times to
            // say "one minute" is not a way to ask for anything, and with
            // seconds in the row it would be sixty.
            up.indicator: null
            down.indicator: null

            onValueModified: picker.settleInterval()

            // Fusion's own, with a length on it: the validator refuses a
            // number out of range but lets the digits be typed first, so
            // without this a field two characters wide takes as many as
            // somebody cares to hold down.
            // Picked out the moment the box is reached, so that typing
            // replaces the number rather than being refused. Two
            // characters is the whole field, so a box reading 15 is full:
            // without this the first keystroke did nothing at all and the
            // number had to be selected by hand before it could be
            // changed. Through `callLater` because a click sets the
            // cursor *after* the focus moves, and would undo a selection
            // made on the spot. A second click in a box that already has
            // the keyboard puts the cursor where it was asked for, which
            // is how a field is edited rather than replaced.
            onActiveFocusChanged: if (box.activeFocus) Qt.callLater(box.pickItAll)

            function pickItAll() {
                if (box.activeFocus)
                    box.contentItem.selectAll();
            }

            contentItem: TextInput {
                text: box.displayText
                maximumLength: 2
                font: box.font
                color: box.palette.text
                selectionColor: box.palette.highlight
                selectedTextColor: box.palette.highlightedText
                selectByMouse: true
                horizontalAlignment: Qt.AlignHCenter
                verticalAlignment: Qt.AlignVCenter
                readOnly: !box.editable
                validator: box.validator
                inputMethodHints: Qt.ImhFormattedNumbersOnly
            }

            // Fusion paints the strip the arrows sat in whether or not
            // there are any, and lights it up when the box has the
            // keyboard -- a grey block hanging off the end of the number
            // with nothing to click. Ours is a plain field.
            background: Rectangle {
                implicitHeight: 30
                radius: 3
                color: box.palette.base
                border.width: 1
                border.color: box.activeFocus ? box.palette.highlight : box.palette.mid
            }
        }
    }

    /// Turns whatever was typed into the three boxes into a time.
    ///
    /// The rule itself is `Playlists.asClock()`, where a test can reach
    /// it: added up as seconds and handed back as hours, minutes and
    /// seconds, so sixty seconds becomes a minute and ninety-five minutes
    /// becomes an hour and thirty-five.
    function settleInterval() {
        const clock = Playlists.asClock(everyHours.value * 3600
                                        + everyMinutes.value * 60 + everySeconds.value);
        everyHours.value = clock.hours;
        everyMinutes.value = clock.minutes;
        everySeconds.value = clock.seconds;
    }

    /// A tick box of the playlist settings: out at the right of the panel,
    /// big enough to aim at, and ringed in blue so it reads as something
    /// to click rather than a hairline square.
    component PlaylistCheck: CheckBox {
        id: check

        Layout.alignment: Qt.AlignRight
        // No room around it: a CheckBox with no words keeps the padding
        // and the gap it would have left for them, which puts the box
        // itself past the edge it was aligned to.
        padding: 0
        spacing: 0

        /// The blue it fills with. Lighter on the dark theme, darker on
        /// the light one: white on #3daee9 is a little over two to one,
        /// which is fine against a dark window and thin against a white
        /// page.
        readonly property color mark: picker.dark ? picker.palette.highlight
                                                  : Qt.darker(picker.palette.highlight, 1.6)

        indicator: Rectangle {
            implicitWidth: 26
            implicitHeight: 26
            radius: 5
            color: check.checked ? check.mark : "transparent"
            border.width: 3
            border.color: check.enabled ? check.mark : check.palette.mid

            Label {
                anchors.centerIn: parent
                text: "✓"
                font.pixelSize: 16
                font.weight: Font.Bold
                visible: check.checked
                color: "#ffffff"
            }
        }
    }

    /// What each row of the two playlist lists is called.
    ///
    /// `language` is read and not used, the same trick the rest of this
    /// window uses: a binding that gets its words out of a function is
    /// not rebuilt when the language changes unless the language is an
    /// argument it takes.
    function orderName(row, language) {
        switch (row) {
        case 1: return qsTr("Random");
        case 2: return qsTr("Ordered");
        }
        return "";
    }

    function transitionName(row, language) {
        switch (row) {
        case 1: return qsTr("None (reduce flicker)");
        case 2: return qsTr("None");
        case 3: return qsTr("Fade");
        case 4: return qsTr("Fade to black");
        }
        return "";
    }

    /// Which of Wallpaper Engine's catalogues to call the tags by.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    /// What a wallpaper is, in a word.
    ///
    /// `language` is read and not used. A binding that gets its words out
    /// of a function is not rebuilt when the language changes unless the
    /// language is an argument it takes -- it looks like a pointless
    /// parameter and it is not.
    function kindName(kind, language) {
        switch (kind) {
        case WallpaperKind.Web: return qsTr("Web page");
        case WallpaperKind.Video: return qsTr("Video");
        case WallpaperKind.Scene: return qsTr("Scene");
        case WallpaperKind.Application: return qsTr("Application");
        }
        return qsTr("Unknown");
    }

    /// What this window knows about one wallpaper, in the order it reads.
    /// What it cannot answer is left out rather than shown as a dash.
    function factsFor(one, language) {
        if (!one)
            return [];

        const told = [{ label: qsTr("Type"), icon: "layer-visible-on",
                        value: picker.kindName(one.kind, language) }];
        if (one.resolution)
            told.push({ label: qsTr("Resolution"), icon: "monitor", value: one.resolution });
        if (one.sizeOnDisk)
            told.push({ label: qsTr("Size on disk"), icon: "drive-harddisk",
                        value: one.sizeOnDisk });
        if (one.rating)
            told.push({ label: qsTr("Age rating"), icon: "flag",
                        value: Tags.name(one.rating, picker.catalogue) });

        // Out of the tags, which is the only place that could say so.
        // The workshop's list, not project.json's -- the tag is never on
        // the disk, so reading it there said "not stated" about a
        // wallpaper whose own chips underneath said otherwise.
        //
        // That is following the music somebody else plays, and it was read
        // as having a sound at all -- "not stated" over a wallpaper with its
        // own soundtrack. So the two are two rows, the second measured off
        // the wallpaper rather than taken from its author, and left out
        // where nothing can say: a web page plays what it likes.
        told.push({ label: qsTr("Follows the music"), icon: "view-media-visualization",
                    value: picker.rawTagsOf(one).indexOf("Audio responsive") >= 0
                               ? qsTr("Yes") : qsTr("Not stated") });
        if (one.soundKnown)
            told.push({ label: qsTr("Sound of its own"), icon: "audio-volume-high",
                        value: one.hasSound ? qsTr("Yes") : qsTr("No") });
        const grade = picker.compatibility(one);
        told.push({ label: qsTr("Compatibility"),
                    icon: grade === "perfect" ? "emblem-success"
                        : grade === "minor" ? "emblem-warning"
                        : grade === "broken" ? "emblem-error" : "emblem-question",
                    value: picker.compatibilityName(grade, language) });
        if (picker.fromWorkshop(one))
            told.push({ label: qsTr("Workshop id"), icon: "documentinfo", value: one.id });
        return told;
    }

    /// The tags of one wallpaper, in the language of the window. Their own
    /// list rather than a row in the one above: the panel draws them as
    /// chips, and a chip is not a value.
    ///
    /// The workshop's list where it has arrived, and what is on the disk
    /// until it does -- so an installed wallpaper reads the same here as
    /// it does in the Steam tab rather than showing a third of the tags.
    function tagsOf(one, language) {
        return picker.rawTagsOf(one).map((tag) => Tags.name(tag, picker.catalogue));
    }

    /// The same list, as Valve's own words rather than the window's --
    /// which is what anything asking "does it carry this tag" wants.
    ///
    /// Both lists together, not one instead of the other. The workshop
    /// knows of tags project.json does not -- the kind, the picture size,
    /// the rating, whether it answers to sound -- and measured the other
    /// way round, one genre on this machine is on the disk and not in the
    /// workshop's answer, because an author can change what is published
    /// without the downloaded copy hearing of it. Replacing one list with
    /// the other lost that genre out of the filter.
    function rawTagsOf(one) {
        if (!one)
            return [];
        const said = picker.workshopTags[one.id];
        if (!said)
            return one.tags;

        const all = said.slice();
        for (let i = 0; i < one.tags.length; ++i) {
            if (all.indexOf(one.tags[i]) < 0)
                all.push(one.tags[i]);
        }
        return all;
    }

    /// Who made each installed wallpaper, keyed by its workshop id:
    /// `{ owner, name, avatar }`.
    ///
    /// project.json does not say -- checked across every one of them -- so
    /// it takes two questions, both keyless and both in steamprofile.js:
    /// the workshop is asked who made this id, and then who that account
    /// is. The workshop tab shares the script, so an author looked up in
    /// one tab is free in the other.
    property var authors: ({})

    /// The one the panel is about: the name and the face once they have
    /// arrived, and a row of dots in the meantime, so the button is there
    /// from the moment the panel opens rather than a second later.
    readonly property var inspectedAuthor: {
        const one = picker.inspected;
        if (!picker.fromWorkshop(one) && !(one && one.author))
            return null;
        return picker.authors[one.id] || { name: "…", avatar: "" };
    }

    /// What the workshop says the tags of an installed wallpaper are,
    /// keyed by its id.
    ///
    /// project.json carries only the genres its author ticked -- measured:
    /// one wallpaper reads `["Sci-Fi"]` on the disk and six in the
    /// workshop, which adds Wallpaper, the type, the resolution, the age
    /// rating and whether it answers to sound. The same question that
    /// finds the author brings these back with it, so it costs nothing.
    property var workshopTags: ({})

    /// When the workshop says each installed wallpaper was last published,
    /// by id: the same answer as the tags.
    property var workshopUpdated: ({})

    /// The installed wallpapers whose author has published a newer version
    /// than the one on the disk: newer than Steam's own record says it
    /// fetched, by what Steam has heard or the workshop says now. Steam
    /// fetches these by itself while it runs; this is for when it did not.
    readonly property var outdated: {
        const found = [];
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            const one = all[i];
            const newest = Math.max(one.latestAt, picker.workshopUpdated[one.id] || 0);
            if (one.installedAt > 0 && newest > one.installedAt)
                found.push(one.id);
        }
        return found;
    }
    /// Asks Steam for the newer versions; the folder watcher sees them come.
    function update(ids) {
        for (let i = 0; i < ids.length; ++i)
            Steamworks.download(ids[i]);
    }

    // Steam writes down that it fetched a version a moment after the files,
    // and the folder watcher answers to the files: looked at again once the
    // record has caught up, so the Update badge goes when it should.
    Connections {
        target: Steamworks
        function onDownloadsChanged() {
            for (let i = 0; i < picker.outdated.length; ++i) {
                if (Steamworks.downloads[picker.outdated[i]] === 100) {
                    caughtUp.restart();
                    return;
                }
            }
        }
    }
    Timer {
        id: caughtUp
        interval: 5000
        onTriggered: Wallpapers.rescan()
    }

    /// Asks the workshop about every installed wallpaper at once, so that
    /// the filter beside the grid has the tags they really carry rather
    /// than the handful project.json keeps.
    ///
    /// Measured on this machine: ten distinct tags on the disk against
    /// twenty-nine in the workshop's answer -- the age ratings among them,
    /// which project.json gives as "Everyone" for every single one.
    ///
    /// One question for the lot, asked when the window is first opened
    /// rather than at startup: nothing in the tray needs it, and a program
    /// that has not been looked at yet should not be talking to Steam.
    function lookUpEverything() {
        const all = Wallpapers.wallpapers;
        const ids = [];
        for (let i = 0; i < all.length; ++i)
            ids.push(all[i].id);

        Steam.detailsOfMany(ids, function () {
            const seen = Object.assign({}, picker.workshopTags);
            const updated = Object.assign({}, picker.workshopUpdated);
            for (let i = 0; i < ids.length; ++i) {
                const said = Steam.items[ids[i]];
                if (said && said.tags.length > 0)
                    seen[ids[i]] = said.tags;
                if (said && said.updated)
                    updated[ids[i]] = said.updated;
            }
            picker.workshopTags = seen;
            picker.workshopUpdated = updated;
        });
    }

    function lookUpAuthor(id) {
        // One of Wallpaper Engine's own says who made it, and the workshop
        // has never heard of it.
        const one = picker.inspected && picker.inspected.id === id ? picker.inspected : null;
        if (one && one.official) {
            if (one.author && !picker.authors[id])
                Steam.profileOf(one.author, function (person) {
                    const known = Object.assign({}, picker.authors);
                    known[id] = { owner: one.author, name: person.name, avatar: person.avatar };
                    picker.authors = known;
                });
            return;
        }
        Steam.detailsOf(id, function (said) {
            // Only what is new. An answer that is already in the map is
            // put back as a new object every time otherwise -- and since
            // the grid's model is built out of these tags, that rebuilds
            // the whole grid: every delegate destroyed, and the view
            // scrolled back to the top. Measured: one click on a card
            // somewhere down the list put the grid back to its first row.
            if (said.tags.length > 0 && !picker.workshopTags[id]) {
                const seen = Object.assign({}, picker.workshopTags);
                seen[id] = said.tags;
                picker.workshopTags = seen;
            }
            if (!said.maker || picker.authors[id])
                return;
            Steam.profileOf(said.maker, function (person) {
                const known = Object.assign({}, picker.authors);
                known[id] = { owner: said.maker, name: person.name,
                              avatar: person.avatar };
                picker.authors = known;
            });
        });
    }

    /// Everything else by whoever made the wallpaper in the panel, which
    /// is a question only the workshop can answer -- so the window goes
    /// there to ask it.
    function everythingByTheAuthor() {
        const known = picker.inspected ? picker.authors[picker.inspected.id] : null;
        if (!known || !known.owner)
            return;
        steam.active = true;
        tabs.currentIndex = 1;
        steam.item.everythingBy(known.owner);
    }

    /// Which card was clicked last, and the wait that tells one click from
    /// two.
    ///
    /// The double click is counted here rather than left to the card,
    /// because the card may not live long enough to count it: the grid's
    /// model is rebuilt whenever a wallpaper answers with its size, and a
    /// rebuild destroys every delegate and builds new ones. The second
    /// press then lands on a different object, which sees a first click.
    ///
    /// And the panel waits out that gap before it opens, because opening
    /// it takes the grid from three columns to two -- every card but the
    /// first moves, and a card that moves out from under the pointer
    /// cannot be clicked a second time. The ring moves at once, so the
    /// first click is still answered immediately.
    property string lastChosen: ""

    /// What the click being counted will do once it is clear that no
    /// second one is coming: open the panel, or put it away again.
    property bool opening: true

    Timer {
        id: secondClick

        interval: 400
        onTriggered: details.open = picker.opening
    }

    /// What a click on a card means: the first picks it out, a second one
    /// on the same card puts it on the desktop -- and a click on the one
    /// the panel is already about shuts the panel again.
    ///
    /// Closing waits out the same 400 ms as opening, and for the same
    /// reason: it moves every card too, and a card that moves out from
    /// under the pointer cannot be clicked a second time.
    function chooseOrApply(id) {
        if (id === picker.lastChosen && secondClick.running) {
            secondClick.stop();
            picker.apply(id);
            return;
        }

        picker.lastChosen = id;
        picker.opening = !(details.open && picker.inspected
                           && picker.inspected.id === id);
        // The ring goes where the click landed either way: it says which
        // card the keyboard is on, which is still true with the panel
        // shut. What is in the panel is left alone while it slides away,
        // so it does not empty itself out on the way.
        picker.ringOn = id;
        if (picker.opening) {
            picker.inspect(id);
            picker.lookUpAuthor(id);
        }
        secondClick.restart();
    }

    /// Shows one wallpaper in the panel. One click says which wallpaper
    /// is being talked about and nothing more -- putting it on the desktop
    /// takes two, since otherwise reading about one changed what was on
    /// the screen.
    function inspect(id) {
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            if (all[i].id === id) {
                picker.inspected = all[i];
                break;
            }
        }
        picker.ringOn = id;
        // Opened by `secondClick`, once it is clear that no second click
        // is coming -- see the note on `lastChosen`. Already open stays
        // open, and simply shows the new one.
        if (details.open)
            picker.lookUpAuthor(id);
    }

    /// Puts one on the desktop, which is what a second click means. Never
    /// for one that cannot be played -- a scene whose package is missing.
    ///
    /// The panel is left exactly as it was. A double click is not two
    /// clicks as far as the counting above is concerned: Qt delivers the
    /// second press as `doubleClicked`, which comes straight here and
    /// never passes `chooseOrApply`, so the click being counted was still
    /// running -- and four tenths of a second after the wallpaper
    /// started, the panel did what that first click had decided, which
    /// was to shut. Stopping it is what makes a double click mean one
    /// thing only.
    function apply(id) {
        secondClick.stop();

        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            if (all[i].id === id && all[i].playable) {
                picker.chose(id);
                return;
            }
        }
    }

    /// What the grid is ordered by, and which way round. Both are held by
    /// the menu items themselves rather than set from them: a checkable
    /// Controls item that is told what to be and also says what it is ends
    /// up fighting its own binding.
    readonly property int sortBy: byAdded.checked ? 1
                                : bySize.checked ? 2
                                : byRecent.checked ? 3
                                : 0
    readonly property bool ascending: upwards.checked

    /// What each of those is called. Read twice over -- by the menu and by
    /// the button in front of it -- so it is written once.
    readonly property var sortLabels: [qsTr("Name"), qsTr("Date added"),
                                       qsTr("Size"), qsTr("Recent")]

    /// What each section across the top is called, and what it is for.
    ///
    /// Here rather than on the tabs themselves because the heading has to
    /// read them too, and a tab is a QQuickItem as far as the tooling is
    /// concerned -- it cannot see a property added to one.
    readonly property var sections: [
        { name: qsTr("Installed"),
          blurb: qsTr("Wallpapers downloaded to your system") },
        { name: qsTr("Steam"),
          blurb: qsTr("Browse wallpapers from Steam Workshop") },
        { name: qsTr("Displays"),
          blurb: qsTr("Which wallpaper goes on which screen") },
        { name: qsTr("Settings"),
          blurb: qsTr("How WallFlux behaves") }
    ]

    /// One section across the top: an icon, what it is called, and the
    /// accent underneath while it is the one showing.
    /// One section of the bar across the top: an equal share of the
    /// window's width, its icon in the middle, its name when pointed at.
    ///
    /// Laid out as the user chose on 10 November: segments rather than a
    /// cluster of icons in the middle, and no line under the one that is
    /// open -- the tile behind it, sliding from one to the next, says that.
    component Section: TabButton {
        id: section

        required property var entry
        /// Whether the icon takes the window's colours. Steam's logo is a
        /// picture rather than a glyph, and tinted it comes out as a plain
        /// disc -- measured both ways round.
        property bool tinted: true

        display: AbstractButton.IconOnly
        implicitHeight: 52
        icon.width: 24
        icon.height: 24
        icon.color: !section.tinted ? "transparent"
                  : section.checked ? picker.palette.highlight : picker.palette.buttonText
        opacity: section.checked || section.hovered ? 1 : 0.7
        Behavior on opacity { NumberAnimation { duration: 150 } }
        ToolTip.visible: hovered
        ToolTip.delay: 500
        ToolTip.text: entry.name

        // Under the pointer the segment lifts a little; the open one has
        // the sliding tile in the bar's background instead.
        background: Rectangle {
            anchors.fill: parent
            anchors.margins: 6
            radius: 9
            color: Qt.alpha(picker.palette.windowText, 0.07)
            opacity: section.hovered && !section.checked ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 150 } }
        }
    }

    /// The wallpapers the grid shows: what was typed, in the order asked for.
    readonly property var shown: {
        const needle = search.text.trim().toLowerCase();
        const all = Wallpapers.wallpapers;
        const kept = [];
        for (let i = 0; i < all.length; ++i) {
            const one = all[i];
            if (needle && !one.title.toLowerCase().includes(needle))
                continue;
            // Each section narrows, and inside a section any one of the
            // chosen will do: three tags means "any of these three", not
            // "all three at once", which is what nobody means by it.
            // The workshop's tags where they have arrived, which is what
            // the chips are built from -- matching on project.json's
            // shorter list would leave half the chips finding nothing.
            const carried = picker.rawTagsOf(one);
            if (filters.tags.length && !carried.some((tag) => filters.tags.indexOf(tag) >= 0))
                continue;
            // Age rating the same way round: project.json says "Everyone"
            // for all but a handful, and the workshop knows better.
            if (filters.ratings.length
                && !filters.ratings.some((rating) => carried.indexOf(rating) >= 0
                                                     || rating === one.rating))
                continue;
            if (filters.resolutions.length && filters.resolutions.indexOf(one.resolution) < 0)
                continue;
            if (filters.kinds.length && filters.kinds.indexOf(one.kind) < 0)
                continue;
            // Read only while it is asked about: everything a binding reads
            // rebuilds the grid when it changes, and a rebuild puts the view
            // back to its first row.
            if (filters.grades.length && filters.grades.indexOf(picker.compatibility(one)) < 0)
                continue;
            if (filters.favorites.length && Steamworks.favorites.indexOf(one.id) < 0)
                continue;
            kept.push(one);
        }

        // The backend hands them over in name order, so that one is already
        // done; the other three are worked out here because what they need
        // -- a date, a size, what was played when -- is all on the wallpaper
        // or on the singleton already.
        //
        // What was played when is read inside its own case and nowhere
        // else. A binding depends on what it reads, so reading it up here
        // made this whole list -- and with it every card in the grid --
        // rebuild each time a wallpaper was put on the desktop, whatever
        // the order was. Which is a double click: the wallpaper started
        // and the grid jumped back to its first row.
        switch (picker.sortBy) {
        case 1:
            kept.sort((a, b) => a.added < b.added ? -1 : a.added > b.added ? 1 : 0);
            break;
        case 2:
            kept.sort((a, b) => a.bytes - b.bytes);
            break;
        case 3: {
            // Rank 0 is the one played last. Anything never played has no
            // rank at all and goes behind everything that has one, which is
            // every wallpaper until something is chosen.
            const played = Wallpapers.recent;
            kept.sort((a, b) => {
                const ra = played.indexOf(a.id);
                const rb = played.indexOf(b.id);
                return (ra < 0 ? Infinity : ra) - (rb < 0 ? Infinity : rb);
            });
            break;
        }
        default:
            break;
        }
        return picker.ascending ? kept : kept.reverse();
    }

    // Five cards across and two rows of them, which is as much of the
    // library as a window can show without taking the whole screen. It
    // opened at 1100 x 720 while a card was sixteen by nine, where the
    // grid itself came out 419 pixels tall -- measured -- and showed six
    // wallpapers out of fifty-three.
    width: 1400
    height: 900
    // Wide enough for the whole of the header. Narrower than this and a
    // layout starts squeezing what is in it, which for a Controls item means
    // squeezing it down to nothing at all rather than clipping it.
    minimumWidth: 900
    minimumHeight: 420
    title: qsTr("WallFlux — choose a wallpaper")

    // Breeze's own two sets, so the window belongs on this desktop rather
    // than looking like a visitor. Set on the window because a palette
    // passes down to everything inside it -- which is the whole of the theme
    // switch: nothing below names a colour of its own.
    palette {
        window: shades.window
        windowText: shades.text
        base: shades.base
        text: shades.text
        button: shades.button
        buttonText: shades.text
        mid: shades.mid
        placeholderText: Qt.alpha(shades.text, 0.5)
        highlight: shades.highlight
        highlightedText: shades.onHighlight
    }
    color: picker.palette.window

    // What the theme is besides its palette, for everything in the window.
    Binding { target: Look; property: "theme"; value: picker.theme }
    Binding { target: Look; property: "highlight"; value: shades.highlight }

    /// Whether the workshop has been asked about the library yet.
    property bool askedAboutThemAll: false

    onVisibleChanged: {
        if (!picker.visible || picker.askedAboutThemAll)
            return;
        picker.askedAboutThemAll = true;
        picker.lookUpEverything();
        if (picker.checkUpdates)
            Updater.check();
    }

    // A wallpaper that turns up later -- Steam finishing a download, or a
    // Refresh -- is asked about too. Everything already answered for is
    // skipped, so this costs nothing when nothing is new.
    Connections {
        target: Wallpapers
        function onWallpapersChanged() {
            if (picker.askedAboutThemAll)
                picker.lookUpEverything();
        }
    }

    // Escape is not wired to anything here, and that is the point. It used
    // to hide the window -- which meant that with a popup open it hid the
    // whole picker instead of closing the popup, since a window-wide
    // Shortcut gets the key before the popup does. Every popup closes on
    // Escape by its own policy; the window is put away with the buttons in
    // its corner, like every other window.

    /// How the playlist being put together runs.
    ///
    /// Four of these choose between things that are not written yet --
    /// when the wallpaper changes, and what it does while it changes.
    /// They are drop downs rather than nothing because the list of what
    /// WallFlux is going to offer is the thing worth deciding first; each
    /// one keeps the value it is set to and nothing reads it yet.
    ///
    /// Every control here holds its own value and the window reads it off
    /// when it saves. Translating a drop down's model is a binding, so
    /// switching language rebuilds the list and puts it back on its first
    /// item -- including Order. Worth knowing; not worth the code to stop
    /// it while there are three lists behind it that do nothing.
    Popup {
        id: settings

        anchors.centerIn: parent
        modal: true
        padding: 18
        // Wide enough for the longest line of words and the row of three
        // numbers beside it, which together are what decides it: narrower
        // and they run out past the edge, and a Popup does not clip what
        // overflows it -- it draws it over the window.
        implicitWidth: 640
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        // A Popup only closes on Escape when it has the keyboard, and it
        // does not take it by asking to be opened. Measured: the Save
        // popup closed on Escape and these two did not, because Save puts
        // the cursor in its name field on the way up and these have
        // nothing to put it in -- the key went to the grid behind them.
        focus: true

        background: Rectangle {
            color: picker.palette.base
            border.color: picker.palette.mid
            border.width: 1
            // Rounder than the panels beside the grid: those sit in the
            // layout and this floats over the window, and the corners are
            // what says which is which.
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16

            Label {
                text: qsTr("Playlist settings")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 14
                rowSpacing: 12

                Label { text: qsTr("Change every") }

                RowLayout {
                    spacing: 10

                    // Pushed over rather than left where they fall: the
                    // drop downs and the tick boxes all meet the right
                    // edge, and three boxes against the left of the same
                    // column read as a different panel.
                    Item { Layout.fillWidth: true }

                    PlaylistNumber {
                        id: everyHours

                        label: qsTr("Hours")
                        // Nothing above the hours to carry into, so this
                        // one really is a ceiling: 24 is simply refused.
                        most: 23
                        value: 0
                    }

                    PlaylistNumber {
                        id: everyMinutes

                        label: qsTr("Minutes")
                        value: 15
                    }

                    PlaylistNumber {
                        id: everySeconds

                        label: qsTr("Seconds")
                        value: 0
                    }
                }

                Label { text: qsTr("Order") }

                // Neither of these lists is translated. A translated
                // model is a binding, so `retranslate()` rebuilds it and
                // the box drops back to its first row -- measured: Random
                // read as nothing after a language switch, and a playlist
                // saved then was saved in order without anybody being
                // told. The model is the words as they go into the file,
                // which never change, and only what is *shown* is
                // translated.
                RoundedCombo {
                    id: orderBox

                    /// What each row is called in the file. Row nought is
                    /// nothing chosen, which saves as in order -- the
                    /// quieter of the two to be given by default.
                    readonly property var words: ["", "random", "ordered"]

                    Layout.fillWidth: true
                    model: orderBox.words
                    displayText: picker.orderName(orderBox.currentIndex, Language.czech)

                    delegate: ItemDelegate {
                        required property int index

                        width: orderBox.width
                        text: picker.orderName(index, Language.czech)
                        highlighted: orderBox.highlightedIndex === index
                    }
                }

                Label { text: qsTr("Wallpaper transition") }

                RoundedCombo {
                    id: transitionBox

                    /// What each row is called in the file. A word rather
                    /// than the number of a row: these have been
                    /// reordered twice already, and a number written down
                    /// would have meant something else each time. Random
                    /// was here and is not any more -- a playlist saved
                    /// while it was still says `random` and still plays.
                    readonly property var words: ["", "soft", "none", "fade", "black"]

                    /// Whether what is chosen takes any time to happen.
                    /// Neither of the two ways of not fading does, and
                    /// nor does choosing nothing at all, so the row that
                    /// asks how long is drawn for the two fades and for
                    /// nothing else.
                    readonly property bool takesTime: transitionBox.currentIndex >= 3

                    Layout.fillWidth: true
                    model: transitionBox.words
                    displayText: picker.transitionName(transitionBox.currentIndex,
                                                       Language.czech)

                    delegate: ItemDelegate {
                        required property int index

                        width: transitionBox.width
                        text: picker.transitionName(index, Language.czech)
                        highlighted: transitionBox.highlightedIndex === index
                    }
                }

                Label {
                    text: qsTr("Transition time")
                    visible: transitionBox.takesTime
                    Layout.topMargin: 8
                }

                RowLayout {
                    spacing: 10
                    visible: transitionBox.takesTime
                    Layout.topMargin: 8

                    RoundSlider {
                        id: transitionTime

                        Layout.fillWidth: true
                        from: 0
                        to: 3000
                        stepSize: 50
                        // Holds the number itself, like the three boxes
                        // above it: a control told what to be and also
                        // asked what it is ends up fighting its own
                        // binding.
                        value: 1500
                    }

                    Label {
                        // As wide as the longest it can read, so the
                        // slider does not shuffle sideways as it is
                        // dragged.
                        Layout.preferredWidth: 64
                        horizontalAlignment: Text.AlignRight
                        text: qsTr("%1 ms").arg(transitionTime.value)
                    }
                }

                // A step between what a change looks like and what sets
                // one off. Its own row rather than a margin on the one
                // below, because the row above it comes and goes with the
                // transition and the gap should not go with it.
                Item { Layout.preferredHeight: 12 }

                Item { Layout.preferredHeight: 12 }

                Label { text: qsTr("Change wallpaper when the video ends") }

                PlaylistCheck { id: afterVideoBox }

                Label { text: qsTr("Allow changing while paused") }

                PlaylistCheck { id: whilePausedBox }

                Label { text: qsTr("Always start with the first wallpaper") }

                PlaylistCheck { id: firstBox }
            }

            RowLayout {
                Layout.fillWidth: true
                // Clear of the last thing that can be set, so the two
                // buttons read as what to do about the panel rather than
                // as another row of it. The same gap the switches have
                // above them, so the panel reads in three pieces of equal
                // weight rather than two and a bit.
                Layout.topMargin: 20

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: settings.close()
                }

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: Look.positive
                    onClicked: settings.close()
                }
            }
        }
    }

    /// Which saved playlist to put back on the strip.
    ///
    /// Saving under a name that is taken replaces it, so this is also
    /// where a playlist is changed: open it, change it, save it. And it is
    /// the only place one can be thrown away.
    ///
    /// Wallpaper Engine's own has a keyboard shortcut field under the
    /// list. There is nothing here that could listen for one, so the field
    /// is left out rather than drawn dead.
    Popup {
        id: loading

        anchors.centerIn: parent
        modal: true
        padding: 18
        implicitWidth: 560
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        // A Popup only closes on Escape when it has the keyboard, and it
        // does not take it by asking to be opened. Measured: the Save
        // popup closed on Escape and these two did not, because Save puts
        // the cursor in its name field on the way up and these have
        // nothing to put it in -- the key went to the grid behind them.
        focus: true

        /// The name picked in the list, or empty while none is.
        property string picked: ""

        /// Whether the picked row is a field to type its new name into.
        property bool renaming: false

        onOpened: loading.picked = picker.playlistName
        onPickedChanged: loading.renaming = false
        onClosed: loading.renaming = false

        /// Gives the picked one the name typed, and says whether it took.
        /// The strip's own name follows, or the next Save would make the
        /// old name again beside the new one.
        function renameTo(name) {
            const from = loading.picked;
            if (!Playlists.rename(from, name))
                return false;
            if (picker.playlistName === from)
                picker.playlistName = name.trim();
            loading.picked = name.trim();
            return true;
        }

        background: Rectangle {
            color: picker.palette.base
            border.color: picker.palette.mid
            border.width: 1
            // Rounder than the panels beside the grid: those sit in the
            // layout and this floats over the window, and the corners are
            // what says which is which.
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16

            Label {
                text: qsTr("Open a playlist")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            Rectangle {
                Layout.fillWidth: true
                // A box of a fixed height whether or not there is anything
                // in it, so the dialog does not change size as playlists
                // come and go.
                Layout.preferredHeight: 200
                color: "transparent"
                border.color: picker.palette.mid
                border.width: 1
                radius: 4

                Label {
                    anchors.fill: parent
                    anchors.margins: 10
                    wrapMode: Text.Wrap
                    opacity: 0.8
                    visible: Playlists.all.length === 0
                    text: qsTr("You have not made a playlist yet.")
                }

                ListView {
                    anchors.fill: parent
                    anchors.margins: 1
                    clip: true
                    model: Playlists.all
                    // The box is a fixed height whether or not it is full,
                    // so most of the time there is nothing to scroll and a
                    // bar down its edge is a line saying otherwise.
                    ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

                    delegate: Rectangle {
                        id: row

                        required property var modelData

                        width: ListView.view.width
                        height: 34
                        color: loading.picked === row.modelData.name
                                   ? picker.palette.highlight
                                   : rowPointer.hovered
                                       ? Qt.alpha(picker.palette.highlight, 0.18)
                                       : "transparent"

                        HoverHandler { id: rowPointer }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: loading.picked = row.modelData.name
                            // Twice is the shortcut for picking it and
                            // pressing Confirm, which is what a list of
                            // names everywhere else does.
                            onDoubleClicked: {
                                picker.playPlaylist(row.modelData.name);
                                loading.close();
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            // Clear of the scrollbar, which keeps a strip
                            // of its own rather than fading out over what
                            // is under it.
                            anchors.rightMargin: 18
                            spacing: 8

                            Label {
                                Layout.fillWidth: true
                                visible: !newName.visible
                                elide: Text.ElideRight
                                text: (Playlists.playing === row.modelData.name ? "▶  " : "")
                                      + row.modelData.name
                                color: loading.picked === row.modelData.name
                                           ? picker.palette.highlightedText
                                           : picker.palette.windowText
                            }

                            TextField {
                                id: newName

                                /// Set when the name typed was refused --
                                /// empty, or another playlist's -- and
                                /// cleared by typing.
                                property bool refused: false

                                Layout.fillWidth: true
                                visible: loading.renaming && loading.picked === row.modelData.name
                                onVisibleChanged: {
                                    if (!visible)
                                        return;
                                    text = row.modelData.name;
                                    refused = false;
                                    forceActiveFocus();
                                    selectAll();
                                }
                                onTextEdited: refused = false
                                onAccepted: {
                                    if (loading.renameTo(text))
                                        loading.renaming = false;
                                    else
                                        refused = true;
                                }
                                // Escape takes the field back and leaves
                                // the dialog open; a second one closes it.
                                Keys.onEscapePressed: event => {
                                    loading.renaming = false;
                                    loading.forceActiveFocus();
                                }
                                background: Rectangle {
                                    radius: 4
                                    color: picker.palette.base
                                    border.width: newName.refused ? 2 : 1
                                    border.color: newName.refused ? Look.negative : picker.palette.mid
                                }
                            }

                            Label {
                                font.pixelSize: 11
                                opacity: 0.8
                                text: qsTr("Wallpapers: %1").arg(row.modelData.ids.length)
                                      + " · " + row.modelData.hours + " h "
                                      + row.modelData.minutes + " min"
                                      + (row.modelData.seconds
                                             ? " " + row.modelData.seconds + " s" : "")
                                color: loading.picked === row.modelData.name
                                           ? picker.palette.highlightedText
                                           : picker.palette.windowText
                            }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                // Asks before it does it, on the button itself rather than
                // in a second popup over the first. Throwing a playlist
                // away cannot be undone and there is nowhere to undo it
                // from, and one click beside a list of names is one click
                // too few for that.
                RoundedButton {
                    id: binIt

                    property bool asking: false

                    text: binIt.asking ? qsTr("Really delete?") : qsTr("Delete")
                    icon.name: "edit-delete"
                    accent: Look.negative
                    enabled: Playlists.named(loading.picked).name !== undefined
                    onClicked: {
                        if (!binIt.asking) {
                            binIt.asking = true;
                            return;
                        }
                        Playlists.forget(loading.picked);
                        loading.picked = "";
                        binIt.asking = false;
                    }

                    // Picking another name, or leaving, takes the question
                    // back: a button left saying "Really delete?" about
                    // something else is how the wrong thing gets deleted.
                    Connections {
                        target: loading
                        function onPickedChanged() { binIt.asking = false; }
                        function onClosed() { binIt.asking = false; }
                    }
                }

                RoundedButton {
                    text: qsTr("Rename")
                    icon.name: "edit-rename"
                    enabled: !loading.renaming
                             && Playlists.named(loading.picked).name !== undefined
                    onClicked: loading.renaming = true
                }

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: Look.positive
                    enabled: Playlists.named(loading.picked).name !== undefined
                    onClicked: {
                        picker.playPlaylist(loading.picked);
                        loading.close();
                    }
                }

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: loading.close()
                }
            }
        }
    }

    /// Delete's question: whether this wallpaper is really to go.
    Popup {
        id: removing

        property var one: null

        anchors.centerIn: parent
        modal: true
        // See the trap about CloseOnEscape: a popup with no field to put
        // the cursor in has to be told to take the keyboard.
        focus: true
        padding: 18
        implicitWidth: 420
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onClosed: grid.forceActiveFocus()

        function confirm() {
            const one = removing.one;
            removing.close();
            picker.remove(one);
        }

        background: Rectangle {
            color: picker.palette.base
            border.color: picker.palette.mid
            border.width: 1
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16
            focus: true
            Keys.onReturnPressed: removing.confirm()
            Keys.onEnterPressed: removing.confirm()

            Label {
                text: qsTr("Remove the wallpaper?")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: !removing.one ? ""
                    : removing.one.official
                      ? qsTr("“%1” will be hidden, as Wallpaper Engine hides its own.")
                            .arg(removing.one.title)
                      : qsTr("“%1” will be unsubscribed in Steam and deleted from the disk.")
                            .arg(removing.one.title)
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Remove")
                    icon.name: "edit-delete"
                    accent: Look.negative
                    onClicked: removing.confirm()
                }

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: removing.close()
                }
            }
        }
    }

    /// What to call the playlist being saved.
    Popup {
        id: saving

        anchors.centerIn: parent
        modal: true
        padding: 18
        implicitWidth: 460
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        onOpened: {
            savingName.text = picker.playlistName;
            savingName.forceActiveFocus();
            savingName.selectAll();
        }

        function keep() {
            picker.playlistName = savingName.text;
            picker.savePlaylist();
            saving.close();
        }

        background: Rectangle {
            color: picker.palette.base
            border.color: picker.palette.mid
            border.width: 1
            // Rounder than the panels beside the grid: those sit in the
            // layout and this floats over the window, and the corners are
            // what says which is which.
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16

            Label {
                text: qsTr("Save the playlist")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 14
                rowSpacing: 6

                Label { text: qsTr("Name") }

                TextField {
                    id: savingName

                    Layout.fillWidth: true
                    placeholderText: qsTr("Playlist %1").arg(Playlists.all.length + 1)
                    maximumLength: 50
                    onAccepted: saving.keep()
                }

                Item {}

                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    font.pixelSize: 11
                    opacity: 0.8
                    // Said whether or not the name is taken: replacing is
                    // how a playlist is changed, so it is the rule rather
                    // than a warning about this particular name.
                    text: qsTr("If a playlist of this name exists already, "
                               + "it will be replaced.")
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: Look.positive
                    onClicked: saving.keep()
                }

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: saving.close()
                }
            }
        }
    }

    /// The wallpaper the window is tinted by: whichever panel is open in
    /// whichever tab is being looked at, and nothing at all while both
    /// are shut.
    readonly property url ambient: picker.tint ? picker.lastPicked : ""

    /// What each grid has picked out, and whichever of the two was picked
    /// last -- the one that tints every tab, until it is put back or
    /// another is picked. Put back, the other's comes through, if any.
    readonly property url pickedInstalled: details.open && picker.inspected ? picker.inspected.preview : ""
    readonly property url pickedSteam: steam.item ? steam.item.ambient : ""
    property url lastPicked: ""
    onPickedInstalledChanged: picker.lastPicked = picker.pickedInstalled != "" ? picker.pickedInstalled : picker.pickedSteam
    onPickedSteamChanged: picker.lastPicked = picker.pickedSteam != "" ? picker.pickedSteam : picker.pickedInstalled

    // The wallpaper being looked at, blurred out of all recognition
    // behind the whole window. Declared before everything else, which is
    // what puts it behind: the cards, the bars and the panel are all
    // drawn over it, and what shows through is the room between them.
    //
    // The picture hangs over every edge, because a blur reaches past what
    // it is given and would otherwise go soft and see-through around the
    // outside of the window. The box crops it back.
    Item {
        anchors.fill: parent
        clip: true

        Image {
            id: backdrop

            anchors.fill: parent
            anchors.margins: -60
            source: picker.ambient
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            // Drawn by the effect below instead.
            visible: false
        }

        MultiEffect {
            anchors.fill: backdrop
            source: backdrop
            blurEnabled: true
            blur: 1
            blurMax: 48
            // Half the colour out of it, so the window is tinted by the
            // wallpaper rather than painted with it.
            saturation: -0.5
            // Fades in and out rather than appearing, over twice as long
            // as the panel takes to slide: this is the room changing
            // colour, not a thing arriving.
            opacity: picker.ambient != "" ? 0.4 : 0
            Behavior on opacity {
                NumberAnimation { duration: 600; easing.type: Easing.InOutQuad }
            }
        }

        // And the window's own colour laid back over it, three tenths of
        // it. Without this a bright preview lifts the whole window: the
        // picture is only four tenths there, but four tenths of something
        // bright over something dark is a pale grey room with grey
        // writing in it. Measured by drawing the three strengths side by
        // side and reading the words on each.
        Rectangle {
            anchors.fill: parent
            color: picker.palette.window
            opacity: 0.3 * (picker.ambient != "" ? 1 : 0)
            Behavior on opacity {
                NumberAnimation { duration: 600; easing.type: Easing.InOutQuad }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The sections, across the top, each an equal share of the width
        // -- the user's word of 10 November -- so the bar fills any window
        // on any screen the same way.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: tabs.implicitHeight
            color: picker.palette.base

            // The same hairline the footer has, along the edge the header
            // starts at.
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: picker.palette.mid
            }

            TabBar {
                id: tabs

                // What builds the Steam tab, the once. Here rather than on
                // the Loader, which cannot both follow the tab and stay
                // behind when it changes.
                //
                // A wallpaper picked out in the library stays picked out
                // across the tabs, the user's word of 9 November: its panel
                // is still open on coming back, and its colours tint every
                // tab meanwhile. The Steam tab's panel is shut on leaving
                // it, the user's word of 10 November -- what was looked at
                // there has usually just been downloaded, and coming back
                // to its panel is coming back to something finished. The
                // click still being counted is forgotten either way, or it
                // would open a panel in the tab just left.
                onCurrentIndexChanged: {
                    if (tabs.currentIndex === 1)
                        steam.active = true;
                    else if (steam.item)
                        steam.item.shut();
                    secondClick.stop();
                }

                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                anchors.bottomMargin: 1
                spacing: 4

                // The tile behind the open section, which slides to the
                // next one rather than jumping. Worked out from the index,
                // since every section is the same width: `currentItem` is
                // handed over a moment after the index changes, and a tile
                // bound to it was left behind on the section before.
                background: Item {
                    Rectangle {
                        readonly property real share:
                            (tabs.availableWidth - tabs.spacing * (tabs.count - 1)) / Math.max(1, tabs.count)

                        x: tabs.leftPadding + tabs.currentIndex * (share + tabs.spacing) + 6
                        y: 6
                        width: share - 12
                        height: parent.height - 12
                        radius: 9
                        color: Qt.alpha(picker.palette.highlight, 0.18)
                        border.width: 1
                        border.color: Qt.alpha(picker.palette.highlight, 0.35)

                        Behavior on x { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }
                        Behavior on width { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }
                    }
                }

                Section {
                    entry: picker.sections[0]
                    icon.name: "download"
                }
                Section {
                    entry: picker.sections[1]
                    icon.name: "steam"
                    tinted: false
                }
                Section {
                    entry: picker.sections[2]
                    icon.name: "preferences-desktop-display"
                }
                Section {
                    entry: picker.sections[3]
                    icon.name: "settings-configure"
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 24
            Layout.bottomMargin: 12
            spacing: 12

            ColumnLayout {
                spacing: 0

                Label {
                    // The section that is showing, rather than a word written
                    // in here: the heading is this window saying where you
                    // are, and five tabs under one title would be a lie.
                    text: picker.sections[tabs.currentIndex].name
                    font.pixelSize: 30
                    font.weight: Font.Bold
                }
                Label {
                    text: picker.sections[tabs.currentIndex].blurb
                    opacity: 0.65
                }
            }

        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            // Installed: the only one there is anything behind.
            ColumnLayout {
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    spacing: 10

                    SearchField {
                        id: search

                        Layout.fillWidth: true
                        Layout.maximumWidth: 420
                        placeholderText: qsTr("Search wallpapers…")
                    }

                    RoundedButton {
                        text: filters.chosen > 0 ? qsTr("Filters (%1)").arg(filters.chosen)
                                                 : qsTr("Filters")
                        icon.name: "view-filter"
                        // One click shows the panel beside the grid, the
                        // next puts it away -- the same button in the same
                        // window as the workshop's, behaving the same way.
                        // Blue while the panel it opens is open, so the
                        // button says which of its two states it is in.
                        accent: filters.visible ? Look.highlight : "transparent"
                        onClicked: filters.visible = !filters.visible
                    }

                    RoundedButton {
                        id: lists

                        // The button says what it opens, always -- the
                        // name of what is playing belongs in the list,
                        // beside the tick, the same as the sort menu.
                        text: qsTr("Playlists")
                        icon.name: "view-media-playlist"
                        // Nothing to offer until one has been saved.
                        enabled: Playlists.all.length > 0
                        onClicked: saved.opened ? saved.close() : saved.open()

                        Menu {
                            id: saved

                            y: lists.height + 4
                            closePolicy: Popup.CloseOnEscape
                                         | Popup.CloseOnPressOutsideParent
                            // Said out loud, because a Menu takes its
                            // width from the background it is given and
                            // the one below has none: without this the
                            // menu came up twelve pixels wide -- the
                            // padding and nothing else -- with its items
                            // sized to nothing and no text in sight.
                            implicitWidth: 260
                            padding: 6

                            background: Rectangle {
                                color: picker.palette.base
                                border.color: picker.palette.mid
                                border.width: 1
                                radius: 14
                            }

                            Repeater {
                                model: Playlists.all

                                delegate: MenuItem {
                                    id: oneList

                                    required property var modelData

                                    leftPadding: 10
                                    rightPadding: 10
                                    // The mark goes on the right, where
                                    // the eye lands after reading the
                                    // name, so the box on the left is
                                    // taken away.
                                    indicator: null

                                    // Not a checkable item with `checked`
                                    // bound to what is playing: clicking
                                    // one of those breaks the very binding
                                    // it was reading. Which playlist is on
                                    // is Playlists' own business and this
                                    // only reads it.
                                    contentItem: RowLayout {
                                        spacing: 8

                                        Label {
                                            Layout.fillWidth: true
                                            text: oneList.modelData.name
                                            elide: Text.ElideRight
                                            color: oneList.palette.windowText
                                        }
                                        Label {
                                            text: "✓"
                                            visible: Playlists.playing
                                                     === oneList.modelData.name
                                            color: oneList.palette.windowText
                                        }
                                    }

                                    background: Rectangle {
                                        radius: 4
                                        color: oneList.highlighted
                                               ? Qt.alpha(oneList.palette.highlight, 0.28)
                                               : "transparent"
                                    }

                                    onTriggered: {
                                        if (Playlists.playing === oneList.modelData.name)
                                            Playlists.stop();
                                        else
                                            Playlists.start(oneList.modelData.name);
                                    }
                                }
                            }
                        }
                    }

                    RoundedButton {
                        id: order

                        text: picker.sortLabels[picker.sortBy]
                        icon.name: picker.ascending ? "view-sort-ascending"
                                                    : "view-sort-descending"
                        onClicked: orderMenu.opened ? orderMenu.close() : orderMenu.open()

                        Menu {
                            id: orderMenu

                            y: order.height + 4
                            // Without this the menu closes on the press
                            // and the click that follows opens it again,
                            // which looks like a flicker and leaves it
                            // showing. Pressing the button that owns it is
                            // not pressing outside it.
                            closePolicy: Popup.CloseOnEscape
                                         | Popup.CloseOnPressOutsideParent
                            implicitWidth: 240
                            padding: 6

                            background: Rectangle {
                                color: picker.palette.base
                                border.color: picker.palette.mid
                                border.width: 1
                                radius: 14
                            }

                            // A heading rather than an item: it says what
                            // the list is, and there is nothing to pick.
                            Label {
                                leftPadding: 10
                                topPadding: 6
                                bottomPadding: 6
                                text: qsTr("SORT BY")
                                font.pixelSize: 11
                                font.letterSpacing: 1.2
                                font.weight: Font.Bold
                                opacity: 0.55
                            }

                            // One of the four at a time. A ButtonGroup
                            // rather than autoExclusive, which would take
                            // the direction below into the same group --
                            // they share a parent and nothing else.
                            ButtonGroup {
                                id: keys
                                exclusive: true
                            }

                            component Key: MenuItem {
                                id: key

                                /// A glyph in front of the name, for the one
                                /// item that is not one of the four keys.
                                property string leading: ""

                                checkable: true
                                leftPadding: 10
                                rightPadding: 10

                                // The mark goes on the right, where the eye
                                // lands after reading the name, so the tick
                                // box on the left is taken away.
                                indicator: null

                                contentItem: RowLayout {
                                    spacing: 8

                                    Label {
                                        text: key.leading
                                        visible: key.leading !== ""
                                        color: key.palette.windowText
                                        opacity: 0.8
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: key.text
                                        color: key.palette.windowText
                                    }
                                    Label {
                                        text: "✓"
                                        visible: key.checked
                                        color: key.palette.windowText
                                    }
                                }

                                background: Rectangle {
                                    radius: 4
                                    color: key.highlighted
                                           ? Qt.alpha(key.palette.highlight, 0.28)
                                           : "transparent"
                                }
                            }

                            Key {
                                id: byName
                                text: picker.sortLabels[0]
                                ButtonGroup.group: keys
                                checked: true
                            }
                            Key {
                                id: byAdded
                                text: picker.sortLabels[1]
                                ButtonGroup.group: keys
                            }
                            Key {
                                id: bySize
                                text: picker.sortLabels[2]
                                ButtonGroup.group: keys
                            }
                            Key {
                                id: byRecent
                                text: picker.sortLabels[3]
                                ButtonGroup.group: keys
                            }

                            MenuSeparator {
                                contentItem: Rectangle {
                                    implicitHeight: 1
                                    color: picker.palette.mid
                                }
                            }

                            Key {
                                id: upwards

                                text: qsTr("Ascending")
                                leading: "⇅"
                                // Ticked to begin with: the grid comes up in
                                // name order from A down, and an unticked
                                // Ascending would quietly mean the other way
                                // round. Lost once in a restyle, which turned
                                // the whole grid back to front.
                                checked: true
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Every wallpaper with a newer version, in one go. Only
                    // while there are some and Steam is there to fetch them.
                    RoundedButton {
                        visible: picker.outdated.length > 0 && Steamworks.available
                        text: qsTr("Update all (%1)").arg(picker.outdated.length)
                        icon.name: "update-none"
                        accent: Look.highlight
                        onClicked: picker.update(picker.outdated)
                    }

                    // Looks through Steam's folders again. Down here with the
                    // other things done to the grid, rather than up beside
                    // the heading of a page it is only one control of.
                    RoundedButton {
                        text: qsTr("Refresh")
                        icon.name: "view-refresh"
                        onClicked: Wallpapers.rescan()
                    }
                }

                // The filter stands beside the grid, which reflows into
                // what is left of the window.
                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.leftMargin: 24
                    // Short on this side, because the scrollbar lives in
                    // this edge and the whole point of it is to be out at
                    // the side rather than among the pictures.
                    Layout.rightMargin: 8
                    spacing: 14

                    FilterPanel {
                        id: filters

                        knownTags: picker.workshopTags
                        Layout.fillHeight: true
                        Layout.bottomMargin: 8
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 14

                        GridView {
                            id: grid

                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true

                            // As many columns of about 240 as fit, then share the
                            // width out between them: a fixed cell width leaves a
                            // ragged empty strip down the side of every window that
                            // is not an exact multiple of it.
                            //
                            // Not the whole width: a strip down the right is left
                            // for the scrollbar, which otherwise draws over the last
                            // column of previews and reads as part of the picture.
                            readonly property int room: grid.width - 30

                            // Two hundred and sixty, which puts five
                            // across the window this opens at. It was three
                            // hundred while a card was sixteen by nine and
                            // carried its name in a row underneath: a square
                            // of three hundred is half again as tall, so the
                            // same number would have left one row and a half
                            // showing.
                            readonly property int columns: Math.max(1, Math.floor(grid.room / 260))

                            cellWidth: Math.floor(grid.room / grid.columns)
                            // A square, with the name inside the picture.
                            // The eight of margin a card keeps on each side
                            // is what leaves a gap between them, across and
                            // down alike.
                            cellHeight: cellWidth

                            model: picker.shown

                            // The arrows are answered below rather than by
                            // the grid itself, which walks `currentIndex`
                            // -- the one thing a model rebuild throws
                            // away. See `ringOn`.
                            keyNavigationEnabled: false

                            // The grid is what the window hands the keyboard
                            // to when it opens: arrows walk the cards, Enter
                            // puts one on the desktop, Space ticks it into
                            // the playlist. The search field takes the
                            // keyboard back the moment it is clicked.
                            focus: true

                            /// Which row the ring is on, or -1 when the one
                            /// it was on is not in the grid any more.
                            function at() {
                                for (let i = 0; i < picker.shown.length; ++i) {
                                    if (picker.shown[i].id === picker.ringOn)
                                        return i;
                                }
                                return -1;
                            }

                            /// The card the arrows are on, or null where the
                            /// grid is empty or has not been walked yet.
                            function here() {
                                const row = grid.at();
                                return row < 0 ? null : picker.shown[row];
                            }

                            /// Brings the card the ring is on back into
                            /// view, after something has moved it: a
                            /// rebuilt model, the panel opening, the
                            /// window being resized, an arrow key.
                            function keepTheRingInView() {
                                const row = grid.at();
                                if (row < 0)
                                    return;

                                // Rewound first, and only then sent where
                                // it belongs. A view that was scrolled
                                // when the window changed size is left
                                // past the end of a grid that is now
                                // shorter -- 2 588 down a grid 1 491 tall
                                // -- and in that state it draws no cards
                                // at all and will not be moved:
                                // `positionViewAtIndex`, `returnToBounds`,
                                // `forceLayout` and writing `contentY` by
                                // hand were each measured doing nothing.
                                // Going back to the beginning gives it
                                // somewhere to count from again, and from
                                // there it goes where it is asked. Both
                                // calls are in the one function, so
                                // nothing is drawn in between.
                                //
                                // Only when it is actually stranded,
                                // though. Rewinding destroys every
                                // delegate on screen and the previews
                                // come back as new Images loading
                                // asynchronously -- the whole grid
                                // visibly blinking on every resize, for a
                                // repair that one case in many needs. In
                                // bounds, `Contain` scrolls the least it
                                // can and does nothing at all when the
                                // card is on screen already.
                                if (grid.contentY > Math.max(0, grid.contentHeight - grid.height))
                                    grid.positionViewAtBeginning();
                                grid.positionViewAtIndex(row, GridView.Contain);
                            }

                            /// Moves the ring by so many cards and brings it
                            /// into view.
                            ///
                            /// The panel follows it only once it is open.
                            /// Opening one because somebody pressed Right
                            /// would be the window deciding what the
                            /// question was.
                            function walk(by) {
                                if (picker.shown.length === 0)
                                    return;
                                const was = grid.at();
                                const to = was < 0 ? 0
                                    : Math.max(0, Math.min(picker.shown.length - 1,
                                                           was + by));
                                picker.ringOn = picker.shown[to].id;
                                grid.keepTheRingInView();
                                if (details.open)
                                    picker.inspect(picker.ringOn);
                            }

                            // Any rebuild of the model below -- a video
                            // answering with its size, the workshop
                            // answering about the whole library, a
                            // download arriving -- puts the view back to
                            // its first row. The card the ring is on is
                            // where somebody is working, so it is brought
                            // back. Nothing moves while that card is on
                            // screen already, so this is quiet except
                            // when the view has actually been thrown.
                            onModelChanged: settle.restart()

                            // And the panel opening takes the grid from
                            // three columns to two, which moves every card
                            // down -- measured, a click near the bottom
                            // put the card that was clicked off the
                            // screen. Answered here rather than where the
                            // panel is opened, because this is the moment
                            // the cards actually move, and without the
                            // wait that a `Qt.callLater` from there would
                            // have needed.
                            onWidthChanged: settle.restart()

                            // The height as well: a window being
                            // maximised changes the two one after the
                            // other, and where the cards belong is not
                            // known while only one of them has arrived.
                            onHeightChanged: settle.restart()

                            // Both of those are answered a beat later,
                            // which is also what puts several of them
                            // together: the model and the width often
                            // change in the same breath, and the view is
                            // only worth moving once the cards have their
                            // new places.
                            Timer {
                                id: settle

                                interval: 150
                                onTriggered: grid.keepTheRingInView()
                            }

                            // The arrows start on the first card, as they
                            // did while the grid kept the ring itself.
                            // Nothing is drawn until the grid has the
                            // keyboard, so this is not a choice anybody
                            // sees being made for them.
                            Component.onCompleted: {
                                if (picker.ringOn === "" && picker.shown.length > 0)
                                    picker.ringOn = picker.shown[0].id;
                            }

                            Keys.onLeftPressed: grid.walk(-1)
                            Keys.onRightPressed: grid.walk(1)
                            Keys.onUpPressed: grid.walk(-grid.columns)
                            Keys.onDownPressed: grid.walk(grid.columns)

                            Keys.onReturnPressed: {
                                const one = grid.here();
                                if (one)
                                    picker.apply(one.id);
                            }
                            Keys.onEnterPressed: {
                                const one = grid.here();
                                if (one)
                                    picker.apply(one.id);
                            }
                            // Delete asks first, in a window of its own: a
                            // key is easier to press by mistake than a
                            // button that has to be clicked twice.
                            Keys.onDeletePressed: {
                                const one = grid.here();
                                if (picker.canRemove(one)) {
                                    removing.one = one;
                                    removing.open();
                                }
                            }
                            Keys.onSpacePressed: {
                                const one = grid.here();
                                if (one && one.playable)
                                    picker.tick(one.id);
                            }

                            delegate: Item {
                                id: cell

                                required property var modelData

                                width: grid.cellWidth
                                height: grid.cellHeight
                                // Over its neighbours, or they would cover
                                // the glow on two sides.
                                z: shownCard.current ? 1 : 0

                                WallpaperCard {
                                    id: shownCard

                                    anchors.fill: parent
                                    anchors.margins: 8
                                    wallpaper: cell.modelData
                                    current: grid.activeFocus
                                             && cell.modelData.id === picker.ringOn
                                    // A click moves the arrows here as well,
                                    // so the two ways of walking the grid do
                                    // not each keep a place of their own.
                                    onChose: (id) => {
                                        grid.forceActiveFocus();
                                        picker.chooseOrApply(id);
                                    }
                                    onApplied: (id) => picker.apply(id)
                                    ticked: picker.inPlaylist.indexOf(cell.modelData.id) >= 0
                                    outdated: picker.outdated.indexOf(cell.modelData.id) >= 0
                                    showTick: picker.picking
                                    onTickAsked: picker.tick(cell.modelData.id)
                                    canFavorite: Steamworks.available && picker.fromWorkshop(cell.modelData)
                                    favorite: Steamworks.favorites.indexOf(cell.modelData.id) >= 0
                                    onFavoriteAsked: Steamworks.favorite(cell.modelData.id, !favorite)
                                }
                            }

                            ScrollBar.vertical: Scroller {}
                        }

                        Label {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.margins: 24
                            visible: picker.shown.length === 0
                            horizontalAlignment: Text.AlignHCenter
                            opacity: 0.65
                            text: Wallpapers.wallpapers.length === 0
                                  ? qsTr("Nothing installed.\nWallpaper Engine's workshop folder is empty or missing.")
                                  : qsTr("Nothing here by that name.")
                        }
                    }

                    Details {
                        id: details

                        Layout.fillHeight: true
                        Layout.bottomMargin: 8
                        preview: picker.inspected ? picker.inspected.preview : ""
                        title: picker.inspected ? picker.inspected.title : ""
                        canFavorite: Steamworks.available && !!picker.inspected && picker.fromWorkshop(picker.inspected)
                        favorite: !!picker.inspected && Steamworks.favorites.indexOf(picker.inspected.id) >= 0
                        onFavoriteAsked: Steamworks.favorite(picker.inspected.id, !favorite)
                        facts: picker.factsFor(picker.inspected, Language.czech)
                        tags: picker.tagsOf(picker.inspected, Language.czech)
                        author: picker.inspectedAuthor
                        onAuthorAsked: picker.everythingByTheAuthor()
                        onCloseAsked: {
                            secondClick.stop();
                            details.open = false;
                        }

                        below: PropertiesView {
                            Layout.fillWidth: true
                            Layout.topMargin: 20
                            Layout.bottomMargin: 10
                            wallpaper: picker.inspected
                            values: picker.inspected ? (picker.propertiesOf[picker.inspected.id] || {}) : {}
                            catalogue: Language.czech ? "cs-cz" : "en-us"
                            onChosen: (name, value) => picker.setProperty(picker.inspected.id, name, value)
                            onRestored: picker.resetProperties(picker.inspected.id)
                        }

                        // This wallpaper's own quality, over the settings:
                        // a heavy scene on Low while the rest stay High.
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.bottomMargin: 6
                            visible: !!picker.inspected && picker.inspected.playable
                            spacing: 10

                            Label {
                                text: qsTr("Quality")
                                Layout.fillWidth: true
                            }

                            RoundedCombo {
                                id: ownQuality

                                readonly property string held: picker.inspected
                                    ? (picker.qualityOf[picker.inspected.id] || "") : ""

                                Layout.preferredWidth: 170
                                // The words as they go into the file, which
                                // never change; see the trap about
                                // translating a drop down's model.
                                model: ["", "low", "medium", "high", "ultra"]
                                currentIndex: Math.max(0, ownQuality.model.indexOf(ownQuality.held))
                                displayText: picker.qualityName(ownQuality.model[ownQuality.currentIndex],
                                                                Language.czech)
                                onActivated: (index) => {
                                    picker.setQualityOf(picker.inspected.id, ownQuality.model[index]);
                                    // Picking a row breaks the binding; the
                                    // next wallpaper in the panel needs it.
                                    ownQuality.currentIndex = Qt.binding(
                                        () => Math.max(0, ownQuality.model.indexOf(ownQuality.held)));
                                }

                                delegate: ItemDelegate {
                                    required property string modelData
                                    required property int index

                                    width: ownQuality.width
                                    text: picker.qualityName(modelData, Language.czech)
                                    highlighted: ownQuality.highlightedIndex === index
                                }
                            }
                        }

                        // A newer version, fetched through Steam, with how far
                        // it has got while it comes.
                        RoundedButton {
                            id: updating

                            readonly property var progress: picker.inspected
                                ? Steamworks.downloads[picker.inspected.id] : undefined

                            Layout.fillWidth: true
                            visible: !!picker.inspected
                                     && picker.outdated.indexOf(picker.inspected.id) >= 0
                            enabled: Steamworks.available && updating.progress === undefined
                            text: updating.progress === undefined ? qsTr("Update")
                                : updating.progress < 0 ? qsTr("Steam would not update it")
                                : qsTr("Updating… %1 %").arg(Math.round(updating.progress))
                            icon.name: "update-none"
                            accent: Look.highlight
                            onClicked: picker.update([picker.inspected.id])
                        }

                        RoundedButton {
                            id: giveBack

                            /// Asked once on the button itself, as a
                            /// playlist's Delete does: Wallpaper Engine asks
                            /// too, and one click is one too few for this.
                            property bool asking: false

                            readonly property bool official:
                                !!picker.inspected && picker.inspected.official

                            Layout.fillWidth: true
                            text: giveBack.asking ? qsTr("Really remove?") : qsTr("Remove")
                            icon.name: "edit-delete"
                            // Red, because it is the one thing in this
                            // window that takes something away.
                            accent: Look.negative
                            // See `canRemove`: shown for what could be given
                            // back, and live only while it can be now.
                            visible: giveBack.official || picker.fromWorkshop(picker.inspected)
                            enabled: picker.canRemove(picker.inspected)
                            onClicked: {
                                if (!giveBack.asking) {
                                    giveBack.asking = true;
                                    return;
                                }
                                giveBack.asking = false;
                                picker.remove(picker.inspected);
                            }

                            // Another wallpaper in the panel takes the
                            // question back: a button left saying "Really
                            // remove?" about something else is how the
                            // wrong one goes.
                            Connections {
                                target: picker
                                function onInspectedChanged() { giveBack.asking = false; }
                            }
                        }

                        // Under the button rather than in a bubble that
                        // has to be hovered for: what a button is about to
                        // do to somebody's disk should be readable without
                        // asking for it.
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                            font.pixelSize: 11
                            opacity: 0.8
                            text: giveBack.official
                                ? qsTr("It comes with Wallpaper Engine, which keeps its files "
                                       + "and hides it, as Wallpaper Engine does itself. "
                                       + "Hidden here, it is hidden in Wallpaper Engine too.")
                                : Steamworks.available
                                ? qsTr("Steam unsubscribes it and takes the folder away. "
                                       + "Subscribing to it again is all it takes to have it back.")
                                : qsTr("Steam has to be running to give a wallpaper back.")
                        }
                    }
                }

                // The playlist being put together. It is here and not in a
                // tab of its own because a playlist is made out of the
                // grid above it -- the tab that used to say "Coming soon"
                // is gone.
                PlaylistBar {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 8
                    Layout.bottomMargin: 8
                    items: picker.playlistItems
                    adding: picker.picking
                    anySaved: Playlists.all.length > 0
                    onDropped: (id) => picker.tick(id)
                    onLoadAsked: loading.open()
                    onSaveAsked: saving.open()
                    onConfigureAsked: settings.open()
                    onAddAsked: picker.picking = !picker.picking
                }
            }

            // Steam Workshop. Built the first time it is looked at and not
            // before -- it fills rows of previews, which is work nobody
            // asked for while they are in the library -- and kept from then
            // on. Built and thrown away with the tab, every switch meant
            // rebuilding the rows and fetching every preview over the
            // network again, which is what made both tabs slow to reach.
            Loader {
                id: steam

                active: false
                source: "WorkshopView.qml"
            }

            // The switch lives in Settings, which is there whether or not
            // this tab has been built.
            Binding {
                target: steam.item
                property: "showAnime"
                value: picker.showAnime
                when: steam.item !== null
            }

            DisplaysView {
                id: displays

                // Every screen to begin with. Done here rather than as a
                // default on the view, because the list of screens is not
                // there yet when the view is built.
                // Or the ones ticked last time, those of them still plugged in.
                Component.onCompleted: {
                    const names = [];
                    const all = Qt.application.screens;
                    for (let i = 0; i < all.length; ++i)
                        names.push(all[i].name);
                    const kept = (remembered.screens || []).filter((name) => names.indexOf(name) >= 0);
                    displays.wanted = kept.length > 0 ? kept : names;
                }
                onWantedChanged: remembered.screens = displays.wanted
            }

            SettingsView {
                id: settingsPage

                picker: picker
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 38
            visible: picker.statusBar
            color: picker.palette.base

            // A hairline rather than a border, so the line is only along the
            // top where the grid ends.
            Rectangle {
                width: parent.width
                height: 1
                color: picker.palette.mid
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 12
                spacing: 8

                Label {
                    // The screen's own name, which is what the compositor
                    // calls the output this window is on.
                    text: qsTr("%1  |  %2").arg(Screen.name).arg(
                              Wallpapers.current.playable ? Wallpapers.current.title
                                                          : qsTr("No active wallpaper"))
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    opacity: 0.8
                }

                // What the program costs, read only while it is on show:
                // reading /proc every second for a window nobody sees would
                // be a cost of its own.
                Binding {
                    target: Usage
                    property: "watching"
                    value: picker.usage && picker.statusBar && picker.visible
                }
                Label {
                    visible: picker.usage
                    text: qsTr("CPU %1 %  ·  GPU %2  ·  RAM %3 MB")
                              .arg(Usage.cpu.toFixed(1))
                              .arg(Usage.gpu < 0 ? qsTr("n/a") : Usage.gpu.toFixed(0) + " %")
                              .arg(Usage.ram.toFixed(0))
                    font.features: { "tnum": 1 }
                    opacity: 0.8
                }

                RoundedTool {
                    icon.name: "help-about"
                    // See the magnifier above: without this it is invisible.
                    icon.color: picker.palette.windowText
                    ToolTip.visible: hovered
                    // What it has to say is said by hovering it. A dialog
                    // with one line in it is a dialog nobody wants, so
                    // there is nothing here for a click to do.
                    ToolTip.text: qsTr("WallFlux %1").arg(Application.version)
                }

                // A newer WallFlux, said where it is seen without looking for
                // it; the click goes to where it is installed from.
                RoundedButton {
                    visible: Updater.state === Updater.Available
                    text: qsTr("WallFlux %1 is out").arg(Updater.latest)
                    accent: Look.highlight
                    flat: true
                    onClicked: {
                        tabs.currentIndex = 3;
                        settingsPage.section = 0;
                    }
                }

                RoundedButton {
                    text: qsTr("Turn off")
                    flat: true
                    // Nothing to turn off until something plays, which is the
                    // state the program starts in.
                    enabled: Wallpapers.current.playable
                    onClicked: Wallpapers.clear()
                }

                RoundedTool {
                    icon.name: picker.muted ? "audio-volume-muted" : "audio-volume-high"
                    // See the magnifier above: without this it is invisible.
                    icon.color: picker.palette.windowText
                    ToolTip.visible: hovered
                    ToolTip.text: picker.muted ? qsTr("Turn the sound on")
                                               : qsTr("Turn the sound off")
                    onClicked: picker.muted = !picker.muted
                }
            }
        }
    }}
