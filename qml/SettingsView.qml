// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
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
        { name: qsTr("Appearance"), icon: "preferences-desktop-theme" }
    ]

    /// Wallpaper Engine's presets, by the pictures a second each draws.
    readonly property var rates: [
        { value: 10, name: qsTr("Low") + " · 10" },
        { value: 15, name: qsTr("Medium") + " · 15" },
        { value: 25, name: qsTr("High") + " · 25" },
        { value: 30, name: qsTr("Ultra") + " · 30" }
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
        p.fps = 15;
        p.music = true;
        p.mouse = true;
        p.parallax = true;
        p.particles = true;
        p.usage = false;
        p.scaling = "default";
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
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                    color: setting.palette.highlight
                }

                // Filling, or a row with no line under its title would leave
                // the space to be shared out round the control instead.
                Label {
                    Layout.fillWidth: true
                    text: setting.title
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
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
            visible: under.children.length > 0
            implicitHeight: under.childrenRect.height
        }
        }
    }

    /// A switch that shows a value and sets it. Clicking a Switch sets its
    /// own `checked`, which breaks a binding on it, so the binding is put
    /// back after every click: a reset to the defaults has to show.
    component Toggle: Switch {
        id: toggle

        property bool value
        signal changed(bool to)

        checked: toggle.value
        onToggled: {
            toggle.changed(toggle.checked);
            toggle.checked = Qt.binding(() => toggle.value);
        }

        // A pill in the accent with a round knob that slides, as switches
        // are drawn now, rather than Fusion's square one. No words beside
        // it, so no room kept for them either.
        padding: 0
        spacing: 0
        // Said outright: with the padding gone Fusion counts the indicator
        // nowhere, and a switch nought wide is drawn but cannot be clicked.
        implicitWidth: 44
        implicitHeight: 24

        indicator: Rectangle {
            implicitWidth: 44
            implicitHeight: 24
            x: 0
            y: (toggle.height - height) / 2
            radius: height / 2
            color: toggle.checked ? toggle.palette.highlight : Qt.alpha(toggle.palette.windowText, 0.16)
            border.width: toggle.checked ? 0 : 1
            border.color: toggle.palette.mid

            Behavior on color { ColorAnimation { duration: 160 } }

            Rectangle {
                width: 18
                height: 18
                radius: 9
                y: 3
                x: toggle.checked ? parent.width - width - 3 : 3
                scale: toggle.pressed ? 0.9 : 1
                color: toggle.checked ? view.ink(toggle.palette.highlight) : toggle.palette.base
                border.width: toggle.checked ? 0 : 1
                border.color: toggle.palette.mid

                Behavior on x { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
                Behavior on scale { NumberAnimation { duration: 100 } }
            }
        }
        contentItem: Item {}
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

            TextField {
                id: search

                Layout.fillWidth: true
                Layout.bottomMargin: 10
                placeholderText: qsTr("Search settings")
                leftPadding: 12
                rightPadding: 12
                onTextChanged: view.query = search.text

                background: Rectangle {
                    implicitHeight: 36
                    radius: 18
                    color: search.palette.base
                    border.width: search.activeFocus ? 2 : 1
                    border.color: search.activeFocus ? search.palette.highlight : search.palette.mid
                }
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
                    font.weight: Font.DemiBold
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

                        Slider {
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

                            // A track in the accent up to the handle, and a
                            // round handle that grows a little when held.
                            background: Rectangle {
                                x: volume.leftPadding
                                y: volume.topPadding + volume.availableHeight / 2 - height / 2
                                width: volume.availableWidth
                                height: 6
                                radius: 3
                                color: Qt.alpha(volume.palette.windowText, 0.15)

                                Rectangle {
                                    width: volume.visualPosition * parent.width
                                    height: parent.height
                                    radius: 3
                                    color: volume.palette.highlight
                                }
                            }

                            handle: Rectangle {
                                x: volume.leftPadding + volume.visualPosition * (volume.availableWidth - width)
                                y: volume.topPadding + volume.availableHeight / 2 - height / 2
                                implicitWidth: 20
                                implicitHeight: 20
                                radius: width / 2
                                scale: volume.pressed ? 1.15 : volume.hovered ? 1.07 : 1
                                color: volume.palette.base
                                border.width: 3
                                border.color: volume.palette.highlight

                                Behavior on scale { NumberAnimation { duration: 120 } }
                            }
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
                    title: qsTr("Pause sound while another program plays")
                    about: qsTr("A video or a game with sound of its own quietens the wallpaper until it stops.")

                    Toggle {
                        value: view.picker.quietForOthers
                        onChanged: (to) => view.picker.quietForOthers = to
                    }
                }

                Setting {
                    home: 1
                    title: qsTr("Pause sound under a maximised window")
                    about: qsTr("A window over the whole screen quietens and stops the wallpaper. "
                                + "Off, it goes on playing behind the window, sound and all.")

                    Toggle {
                        value: view.picker.quietUnderWindows
                        onChanged: (to) => view.picker.quietUnderWindows = to
                    }
                }

                // Performance / Display.

                Setting {
                    home: 2
                    title: qsTr("Frame rate")
                    about: qsTr("Pictures a second, for scenes and web wallpapers, as Wallpaper Engine's "
                                + "presets. More is smoother and costs more; a heavy scene that cannot "
                                + "keep up simply draws fewer.")

                    Choice {
                        options: view.rates
                        current: view.picker.fps
                        onChosen: (value) => view.picker.fps = value
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
                    title: qsTr("Follow the music")
                    about: qsTr("Wallpapers that answer to what the speakers play. Off, nothing "
                                + "listens, and Plasma shows no microphone icon for it.")

                    Toggle {
                        value: view.picker.music
                        onChanged: (to) => view.picker.music = to
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
            }
        }
    }

    /// The themes as tiles: a small picture of each one's window, its name
    /// under it, the chosen one ringed in the accent.
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
