// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import QtQuick.Layouts
import WallFlux

/// How WallFlux behaves: the Settings tab.
///
/// Laid out as a desktop's own settings are these days: a search and the
/// sections down the left, the section's name large over its settings on
/// the right, one card to a setting. Searching shows what matches in every
/// section at once, each card saying which section it is from. Every card
/// does what it says now -- a switch that waits for its feature is a
/// promise, not a setting -- and every value lives on the picker, which
/// remembers it between runs.
///
/// Not here on purpose: a wallpaper's own properties, which belong beside
/// it in the details panel.
Item {
    id: view

    required property var picker

    /// Which section is open, by its place in `sections`.
    property int section: 0
    /// What is typed into the search, or nothing.
    property string query: ""
    readonly property bool searching: view.query.trim() !== ""

    readonly property var sections: [
        { name: qsTr("General"), icon: "preferences-system" },
        { name: qsTr("Audio"), icon: "audio-volume-high" },
        { name: qsTr("Performance / Display"), icon: "video-display" },
        { name: qsTr("Appearance"), icon: "preferences-desktop-theme" },
        { name: qsTr("Controls"), icon: "help-contents" }
    ]

    /// What the wallpaper can do when something else needs the screen or
    /// the speakers, Wallpaper Engine's four; the words never reach a file.
    readonly property var cases: [
        { value: "keep", name: qsTr("Keep playing") },
        { value: "mute", name: qsTr("Mute", "what the wallpaper does: one of four") },
        { value: "pause", name: qsTr("Pause") },
        { value: "stop", name: qsTr("Stop") }
    ]

    /// Wallpaper Engine's presets, by the pictures a second each draws.
    readonly property var rates: [
        { value: 10, name: "10" },
        { value: 15, name: "15" },
        { value: 25, name: "25" },
        { value: 30, name: "30" }
    ]

    readonly property var qualities: [
        { value: "low", name: qsTr("Low") },
        { value: "medium", name: qsTr("Medium") },
        { value: "high", name: qsTr("High") },
        { value: "ultra", name: qsTr("Ultra") }
    ]

    readonly property var smoothings: [
        { value: 0, name: qsTr("Off") },
        { value: 2, name: "2×" },
        { value: 4, name: "4×" },
        { value: 8, name: "8×" }
    ]

    readonly property var floors: [
        { value: 0, name: qsTr("Never") },
        { value: 10, name: "10 %" },
        { value: 20, name: "20 %" },
        { value: 30, name: "30 %" },
        { value: 50, name: "50 %" }
    ]

    readonly property var textureSizes: [
        { value: false, name: qsTr("Full") },
        { value: true, name: qsTr("Half") }
    ]

    readonly property var scalings: [
        { value: "default", name: qsTr("Default") },
        { value: "fill", name: qsTr("Fill") },
        { value: "fit", name: qsTr("Fit") },
        { value: "stretch", name: qsTr("Stretch") }
    ]

    /// The themes, in the order they are offered, with what each is called.
    readonly property var themeList: [
        { value: "system", name: qsTr("System") },
        { value: "light", name: qsTr("Light") },
        { value: "dark", name: qsTr("Dark") },
        { value: "steam", name: "Steam" },
        { value: "hardLight", name: qsTr("Hard Light") },
        { value: "hardDark", name: qsTr("Hard Dark") }
    ]

    /// Accents to choose from; the empty one is the theme's own.
    readonly property var accents: ["", "#3daee9", "#1d99f3", "#2e9e5b", "#1abc9c", "#9b59b6",
                                     "#e93d8f", "#da4453", "#f67400", "#fdbc4b"]

    /// What reads on a colour: black on a light one (Hard Dark's yellow),
    /// white on the rest.
    function ink(c) {
        return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) > 0.7 ? "#000000" : "#ffffff";
    }

    /// Folded for matching: lower case and without accents, so "zvuk"
    /// finds "Zvuk" and "cestina" finds "Čeština".
    function fold(text) {
        let folded = String(text).toLowerCase();
        try {
            folded = folded.normalize("NFD").replace(/[̀-ͯ]/g, "");
        } catch (e) {
            // Without normalize() the plain lower case still matches most.
        }
        return folded;
    }

    function matches(title, about) {
        const wanted = view.fold(view.query.trim());
        return view.fold(title).indexOf(wanted) >= 0 || view.fold(about).indexOf(wanted) >= 0;
    }

    /// Everything back as it comes, except the language and starting at
    /// login, which are the user's and the desktop's rather than WallFlux's.
    function resetToDefaults() {
        const p = view.picker;
        p.theme = "dark";
        p.accent = "";
        p.tint = true;
        p.statusBar = true;
        p.tray = true;
        p.startMinimized = true;
        p.closeToTray = true;
        p.muted = false;
        p.volume = 1;
        p.quietForOthers = true;
        p.quietUnderWindows = true;
        p.pauseOnFocus = false;
        p.whenCovered = "pause";
        p.whenOtherSound = "mute";
        p.whenFocused = "keep";
        p.whenAsleep = "pause";
        p.musicGain = 100;
        p.checkUpdates = true;
        p.batteryLow = true;
        p.batteryFloor = 20;
        p.fps = 15;
        p.music = true;
        p.mouse = true;
        p.parallax = true;
        p.particles = true;
        p.showAnime = true;
        p.usage = false;
        p.scaling = "default";
        p.msaa = 0;
        p.bloom = true;
        p.halfTextures = false;
        p.qualityOf = {};
    }

    // A section opened slides in and fades up, rather than appearing.
    onSectionChanged: arrive.restart()
    ParallelAnimation {
        id: arrive

        NumberAnimation { target: pages; property: "opacity"; from: 0; to: 1; duration: 220; easing.type: Easing.OutCubic }
        NumberAnimation { target: pages; property: "shift"; from: 14; to: 0; duration: 260; easing.type: Easing.OutCubic }
    }

    /// One setting: what it is called, a line saying what it does, and the
    /// control at the right. Not named `Row`, which is QtQuick's own.
    /// Shown while its section is open, or while a search finds it.
    component Setting: Rectangle {
        id: setting

        /// Which of `sections` it belongs to.
        required property int home
        property string title
        property string about
        // The control goes straight into the line, after the words, which
        // fill what is left and so push it to the right edge.
        default property alias control: line.data
        /// What goes under the words, the card's whole width: the theme tiles.
        property alias below: under.data
        /// Whether `below` is shown: the update's line, only while there is one.
        property bool belowShown: true

        visible: view.searching ? view.matches(setting.title, setting.about) : setting.home === view.section
        Layout.fillWidth: true
        implicitHeight: Math.max(64, body.implicitHeight + 24)
        radius: 10
        color: hover.hovered ? Qt.alpha(setting.palette.highlight, 0.07) : setting.palette.base
        border.width: 1
        border.color: setting.palette.mid

        Behavior on color { ColorAnimation { duration: 150 } }

        HoverHandler { id: hover }

        ColumnLayout {
            id: body

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            spacing: 14

        RowLayout {
            id: line

            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                id: words

                Layout.fillWidth: true
                spacing: 2

                // Which section it is from, while a search mixes them.
                Label {
                    visible: view.searching
                    text: view.sections[setting.home].name.toUpperCase()
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                    color: setting.palette.highlight
                }

                // Filling, or a row with no line under its title would leave
                // the space to be shared out round the control instead.
                Label {
                    Layout.fillWidth: true
                    text: setting.title
                    font.pixelSize: 15
                    font.weight: Font.Bold
                }

                Label {
                    Layout.fillWidth: true
                    visible: setting.about !== ""
                    text: setting.about
                    wrapMode: Text.Wrap
                    font.pixelSize: 12
                    opacity: 0.7
                }
            }
        }

        Item {
            id: under

            Layout.fillWidth: true
            visible: setting.belowShown && under.children.length > 0
            implicitHeight: under.childrenRect.height
        }
        }
    }

    /// One card of Controls: how one part of WallFlux is used, a line to a
    /// thing to do, with that part's icon where a setting has its control.
    /// The user's word of 10 November: a guide anybody can read, in the
    /// window rather than on a web page, and found by the same search.
    component Guide: Setting {
        id: guide

        property string glyph
        property bool tinted: true

        home: 4

        IconImage {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: 2
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            sourceSize.width: 28
            sourceSize.height: 28
            name: guide.glyph
            color: guide.tinted ? guide.palette.highlight : "transparent"
        }
    }

    /// One of a few choices, as a segmented control: the chosen one lit.
    component Choice: RowLayout {
        id: choice

        property var options: []
        property var current
        signal chosen(var value)

        spacing: 6

        Repeater {
            model: choice.options

            delegate: RoundedButton {
                id: option

                required property var modelData
                readonly property bool lit: option.modelData.value === choice.current

                text: option.modelData.name
                accent: option.lit ? choice.palette.highlight : "transparent"
                palette.buttonText: option.lit ? view.ink(choice.palette.highlight) : choice.palette.buttonText
                onClicked: choice.chosen(option.modelData.value)
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 24
        anchors.rightMargin: 24
        anchors.bottomMargin: 12
        spacing: 24

        // The search and the sections. More sections will come; each is one
        // entry in `sections` and the `home` of its cards.
        // Held to its width: the entries fill it, and a filling child would
        // otherwise make the whole column fill the row.
        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            Layout.fillWidth: false
            Layout.preferredWidth: 230
            Layout.maximumWidth: 230
            spacing: 4

            SearchField {
                id: search

                Layout.fillWidth: true
                Layout.bottomMargin: 10
                placeholderText: qsTr("Search settings")
                onTextChanged: view.query = search.text
            }

            Repeater {
                model: view.sections

                delegate: ItemDelegate {
                    id: entry

                    required property var modelData
                    required property int index

                    readonly property bool chosen: !view.searching && view.section === entry.index

                    Layout.fillWidth: true
                    implicitHeight: 40
                    leftPadding: 12
                    text: entry.modelData.name
                    icon.name: entry.modelData.icon
                    icon.color: entry.chosen ? entry.palette.highlight : entry.palette.windowText
                    font.weight: entry.chosen ? Font.DemiBold : Font.Normal
                    onClicked: {
                        search.clear();
                        view.section = entry.index;
                    }

                    background: Rectangle {
                        radius: 10
                        color: entry.chosen ? Qt.alpha(entry.palette.highlight, 0.18)
                               : entry.hovered ? Qt.alpha(entry.palette.windowText, 0.07)
                               : "transparent"

                        Behavior on color { ColorAnimation { duration: 150 } }

                        // The accent's bar at the start of the chosen one.
                        Rectangle {
                            anchors.left: parent.left
                            anchors.verticalCenter: parent.verticalCenter
                            width: 3
                            height: entry.chosen ? 18 : 0
                            radius: 2
                            color: entry.palette.highlight

                            Behavior on height { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
                        }
                    }
                }
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: pages.implicitHeight + pages.shift
            clip: true
            ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: pages

                /// How far down it starts while sliding in.
                property real shift: 0

                y: pages.shift
                width: parent.width - 18
                spacing: 8

                Label {
                    Layout.bottomMargin: 8
                    text: view.searching ? qsTr("Results") : view.sections[view.section].name
                    font.pixelSize: 26
                    font.weight: Font.Bold
                }

                Label {
                    visible: view.searching && !pages.anything
                    opacity: 0.7
                    text: qsTr("Nothing matches “%1”.").arg(view.query.trim())
                }

                /// Whether a search found any card at all.
                readonly property bool anything: {
                    for (let i = 0; i < pages.children.length; ++i) {
                        const child = pages.children[i];
                        if (child.home !== undefined && child.visible)
                            return true;
                    }
                    return false;
                }

                // General.

                Setting {
                    home: 0
                    title: qsTr("Language")

                    Choice {
                        options: [{ name: "English", value: false }, { name: "Čeština", value: true }]
                        current: Language.czech
                        onChosen: (value) => Language.czech = value
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Start when I log in")
                    about: qsTr("The desktop keeps its own wallpaper until one is chosen.")

                    Toggle {
                        value: Autostart.enabled
                        onChanged: (to) => Autostart.enabled = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Icon in the system tray")
                    about: qsTr("Without it, start WallFlux again from the menu to open this window.")

                    Toggle {
                        value: view.picker.tray
                        onChanged: (to) => view.picker.tray = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Start minimised")
                    about: qsTr("Start without opening this window.")

                    Toggle {
                        value: view.picker.startMinimized
                        onChanged: (to) => view.picker.startMinimized = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Minimise on close")
                    about: qsTr("Closing the window keeps WallFlux and the wallpaper running. "
                                + "Off, closing it quits, and the wallpaper goes with it.")

                    Toggle {
                        value: view.picker.closeToTray
                        onChanged: (to) => view.picker.closeToTray = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Updates")
                    about: {
                        switch (Updater.state) {
                        case Updater.Checking:
                            return qsTr("Asking GitHub for the newest release…");
                        case Updater.Current:
                            return qsTr("WallFlux %1 is the newest release.").arg(Application.version);
                        case Updater.Available:
                            // Updated by the same line it was installed
                            // with: install.sh builds the newest release.
                            if (updateButton.refused)
                                return qsTr("Konsole could not be opened. To update, paste this line "
                                            + "into a terminal.");
                            return qsTr("WallFlux %1 is out; this is %2. Update opens a terminal, asks "
                                        + "whether to go ahead, asks for your password, builds the new "
                                        + "version and starts it again. Or paste the line into a terminal "
                                        + "yourself.")
                                      .arg(Updater.latest).arg(Application.version);
                        case Updater.Failed:
                            return qsTr("That did not work: %1").arg(Updater.detail);
                        default:
                            return qsTr("This is WallFlux %1.").arg(Application.version);
                        }
                    }

                    // What the new version brings, before updating to it.
                    RoundedButton {
                        visible: Updater.state === Updater.Available
                        text: qsTr("What is new")
                        onClicked: Qt.openUrlExternally(Updater.page)
                    }
                    RoundedButton {
                        id: updateButton

                        /// Konsole would not start, so the line is the way.
                        property bool refused: false

                        visible: Updater.state === Updater.Available
                        accent: Look.positive
                        text: qsTr("Update")
                        onClicked: refused = !Updater.install()
                    }
                    RoundedButton {
                        visible: Updater.state !== Updater.Available
                        enabled: Updater.state !== Updater.Checking
                        text: qsTr("Check now")
                        onClicked: Updater.check()
                    }

                    belowShown: Updater.state === Updater.Available
                    below: RowLayout {
                        width: parent ? parent.width : 0
                        spacing: 8

                        TextField {
                            id: updateLine

                            Layout.fillWidth: true
                            readOnly: true
                            selectByMouse: true
                            font.family: "monospace"
                            text: Updater.command
                        }
                        RoundedButton {
                            id: copyLine

                            property bool copied: false

                            text: copied ? qsTr("Copied") : qsTr("Copy")
                            onClicked: {
                                updateLine.selectAll();
                                updateLine.copy();
                                updateLine.deselect();
                                copied = true;
                                copiedFor.restart();
                            }

                            Timer {
                                id: copiedFor
                                interval: 2000
                                onTriggered: copyLine.copied = false
                            }
                        }
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Check for updates")
                    about: qsTr("Asks GitHub once, the first time the window opens after WallFlux "
                                + "starts, and only says so. Nothing is installed until Update is pressed.")

                    Toggle {
                        value: view.picker.checkUpdates
                        onChanged: (to) => view.picker.checkUpdates = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Anime in the workshop")
                    about: qsTr("Off, the Steam tab leaves out wallpapers tagged Anime -- in "
                                + "Discover, where the Anime row goes too, and in the Workshop. "
                                + "Its chip leaves the Workshop's filter as well. Wallpaper "
                                + "Engine's Show Anime / Hide Anime; your own favourites still "
                                + "show them.")

                    Toggle {
                        value: view.picker.showAnime
                        onChanged: (to) => view.picker.showAnime = to
                    }
                }

                Setting {
                    home: 0
                    title: qsTr("Reset to defaults")
                    about: qsTr("Every setting in every section, except the language and starting at login.")

                    RoundedButton {
                        text: qsTr("Reset")
                        onClicked: view.resetToDefaults()
                    }
                }

                // Audio.

                Setting {
                    home: 1
                    title: qsTr("Volume")
                    about: qsTr("How loud the wallpaper's own sound plays.")

                    RowLayout {
                        spacing: 12

                        RoundSlider {
                            id: volume

                            implicitWidth: 240
                            from: 0
                            to: 1
                            value: view.picker.volume
                            // Turned with the mouse wheel as well, five per
                            // cent a notch.
                            stepSize: 0.05
                            wheelEnabled: true
                            enabled: !view.picker.muted
                            opacity: enabled ? 1 : 0.45
                            onMoved: view.picker.volume = volume.value
                        }

                        Label {
                            Layout.preferredWidth: 44
                            horizontalAlignment: Text.AlignRight
                            text: Math.round(view.picker.volume * 100) + " %"
                            font.features: { "tnum": 1 }
                            opacity: view.picker.muted ? 0.45 : 1
                        }
                    }
                }

                Setting {
                    home: 1
                    title: qsTr("Mute")
                    about: qsTr("The same as the sound button in the status bar.")

                    Toggle {
                        value: view.picker.muted
                        onChanged: (to) => view.picker.muted = to
                    }
                }

                Setting {
                    home: 1
                    title: qsTr("Follow the music")
                    about: qsTr("Wallpapers that answer to what the speakers play. Off, nothing "
                                + "listens, and Plasma shows no microphone icon for it.")

                    Toggle {
                        value: view.picker.music
                        onChanged: (to) => view.picker.music = to
                    }
                }

                Setting {
                    home: 1
                    title: qsTr("Sensitivity to music")
                    about: qsTr("How strongly the wallpapers that follow the music hear it. 100 % is "
                                + "Wallpaper Engine's own; quiet music wants more, loud music less.")

                    RowLayout {
                        spacing: 12
                        enabled: view.picker.music
                        opacity: enabled ? 1 : 0.45

                        RoundSlider {
                            id: gain

                            implicitWidth: 240
                            from: 0
                            to: 400
                            value: view.picker.musicGain
                            stepSize: 10
                            wheelEnabled: true
                            onMoved: view.picker.musicGain = Math.round(gain.value)
                        }

                        Label {
                            Layout.preferredWidth: 52
                            horizontalAlignment: Text.AlignRight
                            text: view.picker.musicGain + " %"
                            font.features: { "tnum": 1 }
                        }
                    }
                }

                Setting {
                    home: 1
                    title: qsTr("When another program plays sound")
                    about: qsTr("A video or a game with sound of its own. It comes back by itself "
                                + "when the other program stops.")

                    Choice {
                        options: view.cases.slice(0, 3)
                        current: view.picker.forOtherSound
                        onChosen: (value) => view.picker.whenOtherSound = value
                    }
                }

                // Performance / Display.

                Setting {
                    home: 2
                    title: qsTr("Compatibility test")
                    about: {
                        const done = Compatibility.done, total = Compatibility.total;
                        if (Compatibility.running)
                            return qsTr("Trying %1 of %2: %3").arg(Math.min(done + 1, total)).arg(total)
                                .arg(view.picker.titleOf(Compatibility.current));
                        if (total > 0 && done === total)
                            return qsTr("Tried %1 wallpapers. Each card in the library says how it fared.")
                                .arg(total);
                        return qsTr("Plays every wallpaper in the library for a moment, unseen and "
                                    + "unheard, and marks each one Works, Minor issues or Broken. "
                                    + "Run it again after downloading new ones.");
                    }

                    RoundedButton {
                        text: Compatibility.running ? qsTr("Stop") : qsTr("Test the library")
                        accent: Compatibility.running ? "transparent" : Look.highlight
                        onClicked: Compatibility.running ? Compatibility.stop() : view.picker.testLibrary()
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Quality")
                    about: qsTr("Wallpaper Engine's presets, setting the three below at once. Changed "
                                + "one by one they are your own, and no preset is lit. A wallpaper can "
                                + "have a preset of its own in the panel beside the grid.")

                    Choice {
                        options: view.qualities
                        current: view.picker.quality
                        onChosen: (value) => view.picker.usePreset(value)
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Frame rate")
                    about: qsTr("Pictures a second, for scenes and web wallpapers. More is smoother and "
                                + "costs more; a heavy scene that cannot keep up simply draws fewer.")

                    Choice {
                        options: view.rates
                        current: view.picker.fps
                        onChosen: (value) => view.picker.fps = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Smooth edges (MSAA)")
                    about: qsTr("Samples a pixel, for the edges of turned layers and particles in "
                                + "scenes. More is smoother and asks more of the graphics card.")

                    Choice {
                        options: view.smoothings
                        current: view.picker.msaa
                        onChosen: (value) => view.picker.msaa = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Glow (post-processing)")
                    about: qsTr("The glow round bright places that some scenes ask for. Wallpaper "
                                + "Engine's Low turns it off, and nothing else with it.")

                    Toggle {
                        value: view.picker.bloom
                        onChanged: (to) => view.picker.bloom = to
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Texture resolution")
                    about: qsTr("Half loads a scene's pictures at half their width and height: less "
                                + "memory on the graphics card, a little softer. The scene on show is "
                                + "loaded again.")

                    Choice {
                        options: view.textureSizes
                        current: view.picker.halfTextures
                        onChosen: (value) => view.picker.halfTextures = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Default scaling")
                    about: qsTr("How a wallpaper meets a screen of another shape: as Wallpaper Engine "
                                + "draws it, cropped to fill the screen, fitted inside it with bars, "
                                + "or stretched to it. A wallpaper the screen's own shape looks the "
                                + "same every way.")

                    Choice {
                        options: view.scalings
                        current: view.picker.scaling
                        onChosen: (value) => view.picker.scaling = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Follow the mouse")
                    about: qsTr("Particles that move away from the pointer, light that goes where it goes.")

                    Toggle {
                        value: view.picker.mouse
                        onChanged: (to) => view.picker.mouse = to
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Parallax")
                    about: qsTr("Layers of a scene that lean as the mouse moves, for depth.")

                    Toggle {
                        value: view.picker.parallax
                        onChanged: (to) => view.picker.parallax = to
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Particles")
                    about: qsTr("Smoke, sparks, snow and the like in scenes. They are most of what "
                                + "a scene costs; off, they are neither worked out nor drawn.")

                    Toggle {
                        value: view.picker.particles
                        onChanged: (to) => view.picker.particles = to
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("When a window covers the screen")
                    about: qsTr("A maximised or full-screen window. Pause stands the wallpaper still "
                                + "and quiet; Stop lets go of it altogether, freeing its memory, and "
                                + "loads it again when the window goes.")

                    Choice {
                        options: view.cases
                        current: view.picker.underWindow
                        onChosen: (value) => view.picker.whenCovered = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("When you work in another program")
                    about: qsTr("Whenever another program's window has the keyboard, however small "
                                + "it is; clicking the desktop brings the wallpaper back. This "
                                + "window does not count.")

                    Choice {
                        options: view.cases
                        current: view.picker.forFocus
                        onChosen: (value) => view.picker.whenFocused = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("When the screens are off or locked")
                    about: qsTr("Every screen switched off by power saving, or the session locked: "
                                + "nobody can see the wallpaper, and under the lock it would go on "
                                + "playing its sound.")

                    Choice {
                        options: view.cases
                        current: view.picker.whenAsleep
                        onChosen: (value) => view.picker.whenAsleep = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Low quality on battery")
                    about: (Power.hasBattery ? "" : qsTr("This computer has no battery. "))
                           + qsTr("On battery every wallpaper is drawn on the Low preset, its own "
                                  + "preset or not, so the battery lasts longer.")

                    Toggle {
                        value: view.picker.batteryLow
                        onChanged: (to) => view.picker.batteryLow = to
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Pause on a low battery")
                    about: (Power.hasBattery ? "" : qsTr("This computer has no battery. "))
                           + qsTr("Below this much battery the wallpaper stops altogether, until the "
                                  + "charger is plugged in.")

                    Choice {
                        options: view.floors
                        current: view.picker.batteryFloor
                        onChosen: (value) => view.picker.batteryFloor = value
                    }
                }

                Setting {
                    home: 2
                    title: qsTr("Show usage")
                    about: qsTr("The status bar says what WallFlux takes of the processor, the "
                                + "graphics card and the memory, its helpers included -- which is "
                                + "how a lower frame rate or particles turned off can be seen.")

                    Toggle {
                        value: view.picker.usage
                        onChanged: (to) => view.picker.usage = to
                    }
                }

                // Appearance.

                Setting {
                    home: 3
                    title: qsTr("Theme")
                    about: qsTr("System follows the desktop's light or dark. Hard Light and Hard Dark "
                                + "are high contrast.")

                    below: ThemeTiles {
                        width: parent ? parent.width : 0
                    }
                }

                Setting {
                    home: 3
                    title: qsTr("Typeface")
                    about: qsTr("The letters the whole window is written in. The desktop's own is "
                                + "the one your other programs use.")

                    below: FontTiles {
                        width: parent ? parent.width : 0
                    }
                }

                Setting {
                    home: 3
                    title: qsTr("Accent colour")
                    about: qsTr("What buttons, switches and the ring round a chosen card are lit in.")

                    Row {
                        spacing: 8

                        Repeater {
                            model: view.accents

                            delegate: Rectangle {
                                id: dot

                                required property string modelData
                                readonly property bool chosen: view.picker.accent === dot.modelData

                                width: 24
                                height: 24
                                radius: 12
                                // The theme's own is drawn as half its colour
                                // and half the window, with a ring.
                                color: dot.modelData !== "" ? dot.modelData : "transparent"
                                border.width: dot.chosen ? 3 : 1
                                border.color: dot.chosen ? dot.palette.windowText : dot.palette.mid
                                scale: dotHover.hovered ? 1.12 : 1

                                Behavior on scale { NumberAnimation { duration: 120 } }

                                Label {
                                    anchors.centerIn: parent
                                    visible: dot.modelData === ""
                                    text: "A"
                                    font.pixelSize: 11
                                    font.weight: Font.DemiBold
                                }

                                HoverHandler { id: dotHover; cursorShape: Qt.PointingHandCursor }
                                TapHandler { onTapped: view.picker.accent = dot.modelData }

                                ToolTip.visible: dotHover.hovered && dot.modelData === ""
                                ToolTip.text: qsTr("The theme's own")
                            }
                        }
                    }
                }

                Setting {
                    home: 3
                    title: qsTr("Tint by the wallpaper")
                    about: qsTr("The window takes on the colours of the wallpaper whose details are open.")

                    Toggle {
                        value: view.picker.tint
                        onChanged: (to) => view.picker.tint = to
                    }
                }

                Setting {
                    home: 3
                    title: qsTr("Show the status bar")
                    about: qsTr("The strip along the bottom: what is playing, the sound, Turn off and the version.")

                    Toggle {
                        value: view.picker.statusBar
                        onChanged: (to) => view.picker.statusBar = to
                    }
                }

                // Controls: how WallFlux is used. One card a part of it,
                // each line one thing to do.

                Guide {
                    glyph: "view-list-icons"
                    title: qsTr("The library")
                    about: qsTr("• One click on a wallpaper opens its panel on the right: the picture, "
                                + "what is known about it, its own settings and its quality. A second "
                                + "click on it, or the cross on the picture, closes the panel.\n"
                                + "• Two clicks put it on the desktop.\n"
                                + "• Search, Filters and the order button narrow and sort the grid; "
                                + "Refresh reads Steam's folders again.\n"
                                + "• The heart makes it a favourite on your Steam account. Update on a "
                                + "card means the workshop has a newer version of it; Update all, beside "
                                + "Refresh, fetches every one.\n"
                                + "• Remove in the panel gives the wallpaper back to Steam and deletes "
                                + "its folder.")
                }

                Guide {
                    glyph: "steam"
                    tinted: false
                    title: qsTr("The Steam workshop")
                    about: qsTr("• Steam has to be running, with Wallpaper Engine in your library.\n"
                                + "• Discover is rows picked for you; See more opens a whole row. "
                                + "Workshop is the whole catalogue, with search, filters, order and pages.\n"
                                + "• Search finds words in names and descriptions, not authors; a "
                                + "wallpaper's number or the link to its page finds that one.\n"
                                + "• Click a wallpaper for its panel, then Download. Steam fetches it, "
                                + "and a few seconds later it is in the library by itself.\n"
                                + "• The author's button in the panel shows everything else they made.\n"
                                + "• Your favourites and your friends' favourites are rows of Discover and "
                                + "orders of Workshop.")
                }

                Guide {
                    glyph: "view-media-playlist"
                    title: qsTr("Playlists")
                    about: qsTr("• Tick wallpapers with the box in a card's corner, or press Add "
                                + "wallpaper to show every box at once. They line up along the bottom.\n"
                                + "• Save gives the playlist a name; Load opens a saved one and starts "
                                + "it, renames it or throws it away; Configure sets how long each stays, "
                                + "the order and the transition, and Load shows what each saved one is set to.\n"
                                + "• The Playlists button beside the filters starts a saved playlist, "
                                + "and stops it when clicked again. Turn off in the footer stops it too.")
                }

                Guide {
                    glyph: "preferences-desktop-wallpaper"
                    title: qsTr("The tray icon")
                    about: qsTr("• A click opens this window.\n"
                                + "• A right click pauses and resumes the wallpaper, goes to the next or "
                                + "previous wallpaper -- of the playlist, if one is running, and "
                                + "otherwise in the library's order -- mutes, offers the last five wallpapers and quits.\n"
                                + "• With Minimise on close (General), closing this window leaves "
                                + "WallFlux running in the tray.")
                }

                Guide {
                    glyph: "input-keyboard"
                    title: qsTr("The keyboard")
                    about: qsTr("• Arrows walk the library's grid; Enter puts the wallpaper on the "
                                + "desktop; Space ticks it for a playlist; Delete removes it, after "
                                + "asking.\n"
                                + "• Escape closes whatever popup is open.")
                }

                Guide {
                    glyph: "preferences-desktop-display"
                    title: qsTr("Screens")
                    about: qsTr("• The Displays tab shows your monitors as they are arranged. Switch "
                                + "on each one the wallpaper should be on; the same wallpaper plays on "
                                + "all of them.")
                }

                Guide {
                    glyph: "audio-volume-high"
                    title: qsTr("Sound and pausing")
                    about: qsTr("• The wallpaper goes quiet while another program plays sound, and "
                                + "quiet and still under a maximised window, while you work in another "
                                + "program or while the screens are off or locked -- each as Audio "
                                + "and Performance say: keep playing, mute, pause or stop and free "
                                + "its memory. It comes back by itself.\n"
                                + "• On a battery it can drop to Low quality and stop when the battery "
                                + "runs low (Performance).\n"
                                + "• The speaker in the status bar mutes it.")
                }

                Guide {
                    glyph: "configure"
                    title: qsTr("A wallpaper's own settings")
                    about: qsTr("• In the library's panel, under the tags: the colours, sliders and "
                                + "switches its author offered. A change shows at once; Defaults puts "
                                + "them back.\n"
                                + "• Quality in the panel sets this one wallpaper apart from Performance "
                                + "in Settings -- a heavy scene on Low while the rest stay High.")
                }

                Guide {
                    glyph: "dialog-ok-apply"
                    title: qsTr("Compatibility")
                    about: qsTr("• The panel says how a wallpaper plays here: Works, Minor issues "
                                + "(something in it is not drawn yet) or Broken. It is filled in the "
                                + "first time the wallpaper plays.\n"
                                + "• Test the library (Performance) tries every wallpaper at once, "
                                + "unseen and unheard; run it again after downloading new ones.\n"
                                + "• Filters › Compatibility finds them by it.")
                }
            }
        }
    }

    /// The themes as tiles: a small picture of each one's window, its name
    /// under it, the chosen one ringed in the accent.
    /// One tile per typeface: a large Áá and its name, both in it.
    component FontTiles: Flow {
        spacing: 10

        Repeater {
            // The desktop's own first, as the default; then the ones
            // WallFlux carries, every one with all of Czech.
            model: ["", "Inter", "Geist", "Manrope", "Plus Jakarta Sans", "Figtree", "Onest", "DM Sans"]

            delegate: Item {
                id: face

                required property string modelData
                readonly property bool chosen: view.picker.typeface === face.modelData
                readonly property string family: face.modelData !== "" ? face.modelData
                                                                        : Application.font.family

                width: 116
                height: 108

                Rectangle {
                    id: sample

                    width: parent.width
                    height: 66
                    radius: 10
                    color: face.palette.base
                    border.width: face.chosen ? 3 : 1
                    border.color: face.chosen ? face.palette.highlight : face.palette.mid
                    scale: faceHover.hovered ? 1.04 : 1

                    Behavior on scale { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }

                    Label {
                        anchors.centerIn: parent
                        text: "Áá"
                        font.family: face.family
                        font.pixelSize: 28
                        font.weight: Font.Bold
                    }
                }

                Label {
                    anchors.top: sample.bottom
                    anchors.topMargin: 6
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    // The desktop's own says so, and which it is underneath.
                    text: face.modelData !== "" ? face.modelData
                                                : qsTr("Desktop's") + "\n" + face.family
                    font.family: face.family
                    font.pixelSize: 12
                    font.weight: face.chosen ? Font.DemiBold : Font.Normal
                }

                HoverHandler { id: faceHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: view.picker.typeface = face.modelData }
            }
        }
    }

    component ThemeTiles: Flow {
        spacing: 10

        Repeater {
            model: view.themeList

            delegate: Item {
                id: tile

                required property var modelData
                readonly property bool chosen: view.picker.theme === tile.modelData.value
                readonly property var colours: tile.modelData.value === "system"
                                               ? view.picker.systemColours : view.picker.themes[tile.modelData.value]

                width: 116
                height: 92

                Rectangle {
                    id: frame

                    width: parent.width
                    height: 66
                    radius: 10
                    clip: true
                    color: tile.colours ? tile.colours.window : "#1b1e20"
                    border.width: tile.chosen ? 3 : 1
                    border.color: tile.chosen ? tile.palette.highlight : tile.palette.mid
                    scale: tileHover.hovered ? 1.04 : 1

                    Behavior on scale { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }

                    // A card, a line of text and the accent, in the theme's own.
                    Rectangle {
                        x: 12
                        y: 12
                        width: parent.width - 24
                        height: 26
                        radius: 5
                        color: tile.colours ? tile.colours.base : "#3a4045"
                        border.width: 1
                        border.color: tile.colours ? tile.colours.mid : "#c4c8cc"

                        Rectangle {
                            x: 7
                            anchors.verticalCenter: parent.verticalCenter
                            width: 36
                            height: 4
                            radius: 2
                            color: tile.colours ? tile.colours.text : "#fcfcfc"
                        }

                        Rectangle {
                            anchors.right: parent.right
                            anchors.rightMargin: 7
                            anchors.verticalCenter: parent.verticalCenter
                            width: 18
                            height: 10
                            radius: 5
                            color: tile.colours ? tile.colours.highlight : "#3daee9"
                        }
                    }
                }

                Label {
                    anchors.top: frame.bottom
                    anchors.topMargin: 6
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: tile.modelData.name
                    font.pixelSize: 12
                    font.weight: tile.chosen ? Font.DemiBold : Font.Normal
                }

                HoverHandler { id: tileHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: view.picker.theme = tile.modelData.value }
            }
        }
    }
}
