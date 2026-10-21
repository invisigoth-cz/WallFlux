// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
// The Fusion style by name rather than through QQuickStyle: it is the one
// that takes a palette, and asking for it here keeps the choice beside the
// colours it is set with.
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// The window for choosing a wallpaper: everything installed, in a grid of
/// previews, and what the one that is playing sounds like.
///
/// Says what was clicked and nothing more -- it neither plays a wallpaper nor
/// remembers one. Which of the two is on screen is the wallpaper window's
/// business, and remembering is nobody's, by design.
Window {
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

    /// Which way the colours go. Dark to start with, because a wallpaper
    /// picker is a window full of pictures.
    property bool dark: true

    /// The one the details panel is about, or null while it is shut.
    property var inspected: null

    /// The wallpapers ticked for a playlist, as ids in the order they were
    /// ticked. Nothing is remembered about this between runs: a playlist
    /// is remembered once it is saved and not before.
    property var inPlaylist: []

    /// What the playlist being put together is called and whether it is
    /// shuffled, until it is saved.
    ///
    /// How often it turns is not here: the two spin boxes hold that
    /// themselves and the window reads it off them when it saves. Binding
    /// their `value` to a property and setting that property back from the
    /// control is the trap this project was already caught by once -- a
    /// control told what to be and also asked what it is ends up fighting
    /// its own binding. Measured again here: the box read 1 and the
    /// property was still 15, so a playlist set to one minute was saved as
    /// a quarter of an hour and sat there not changing anything.
    property string playlistName: ""
    property bool playlistShuffle: false

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
    /// name if nobody has.
    function savePlaylist() {
        if (picker.playlistName === "")
            picker.playlistName = qsTr("Playlist %1").arg(Playlists.all.length + 1);
        Playlists.save(picker.playlistName, picker.inPlaylist, everyHours.value,
                       everyMinutes.value, picker.playlistShuffle);
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

        const told = [{ label: qsTr("Type"), value: picker.kindName(one.kind, language) }];
        if (one.resolution)
            told.push({ label: qsTr("Resolution"), value: one.resolution });
        if (one.sizeOnDisk)
            told.push({ label: qsTr("Size on disk"), value: one.sizeOnDisk });
        if (one.rating)
            told.push({ label: qsTr("Age rating"),
                        value: Tags.name(one.rating, picker.catalogue) });

        // Out of the tags, which is the only place that could say so --
        // and on this machine no installed wallpaper carries the tag at
        // all, so "not stated" is the honest answer rather than "no".
        told.push({ label: qsTr("Audio responsive"),
                    value: one.tags.indexOf("Audio responsive") >= 0 ? qsTr("Yes")
                                                                     : qsTr("Not stated") });
        if (one.tags.length > 0) {
            told.push({ label: qsTr("Tags"),
                        value: one.tags.map((tag) => Tags.name(tag, picker.catalogue))
                                       .join(", ") });
        }
        told.push({ label: qsTr("Workshop id"), value: one.id });
        return told;
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
        details.visible = true;
    }

    /// Puts one on the desktop, which is what a second click means. Never
    /// for a scene wallpaper: there is nothing here that can draw one.
    function apply(id) {
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
    component Section: TabButton {
        required property var entry

        display: AbstractButton.IconOnly
        // Room to aim at, and an icon big enough to tell from the next one.
        // Left to itself a tab with nothing but a 16-pixel icon in it comes
        // out two dozen pixels wide.
        implicitWidth: 54
        implicitHeight: 40
        icon.width: 22
        icon.height: 22
        ToolTip.visible: hovered
        ToolTip.text: entry.name

        // Fusion raises the current tab a little, which is not enough to
        // find at a glance in a row of five icons.
        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 2
            color: picker.palette.highlight
            visible: parent.checked
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
            if (filters.tags.length && !one.tags.some((tag) => filters.tags.indexOf(tag) >= 0))
                continue;
            if (filters.ratings.length && filters.ratings.indexOf(one.rating) < 0)
                continue;
            if (filters.resolutions.length && filters.resolutions.indexOf(one.resolution) < 0)
                continue;
            if (filters.kinds.length && filters.kinds.indexOf(one.kind) < 0)
                continue;
            kept.push(one);
        }

        // The backend hands them over in name order, so that one is already
        // done; the other three are worked out here because what they need
        // -- a date, a size, what was played when -- is all on the wallpaper
        // or on the singleton already.
        const played = Wallpapers.recent;
        switch (picker.sortBy) {
        case 1:
            kept.sort((a, b) => a.added < b.added ? -1 : a.added > b.added ? 1 : 0);
            break;
        case 2:
            kept.sort((a, b) => a.bytes - b.bytes);
            break;
        case 3:
            // Rank 0 is the one played last. Anything never played has no
            // rank at all and goes behind everything that has one, which is
            // every wallpaper until something is chosen.
            kept.sort((a, b) => {
                const ra = played.indexOf(a.id);
                const rb = played.indexOf(b.id);
                return (ra < 0 ? Infinity : ra) - (rb < 0 ? Infinity : rb);
            });
            break;
        default:
            break;
        }
        return picker.ascending ? kept : kept.reverse();
    }

    width: 1100
    height: 720
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
        window: picker.dark ? "#1b1e20" : "#eff0f1"
        windowText: picker.dark ? "#fcfcfc" : "#232629"
        base: picker.dark ? "#232629" : "#fcfcfc"
        text: picker.dark ? "#fcfcfc" : "#232629"
        button: picker.dark ? "#31363b" : "#fcfcfc"
        buttonText: picker.dark ? "#fcfcfc" : "#232629"
        mid: picker.dark ? "#3a4045" : "#c4c8cc"
        placeholderText: "#7f8c8d"
        highlight: "#3daee9"
        highlightedText: "#ffffff"
    }
    color: picker.palette.window

    // What every other window on this desktop does, and the one key a person
    // reaches for without being told.
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: picker.hide()
    }

    /// How the playlist being put together runs.
    ///
    /// Wallpaper Engine's own has more in it -- a fade between wallpapers
    /// and how long it takes, changing when a video ends, whether to
    /// always begin with the first one. None of those are here, because
    /// none of them can be done yet and a switch that does nothing is
    /// worse than no switch.
    Popup {
        id: settings

        anchors.centerIn: parent
        modal: true
        padding: 18
        // Wide enough for the row of two numbers and their words, which
        // is what decides it: narrower and they run out past the edge.
        implicitWidth: 560
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: picker.palette.base
            border.color: picker.palette.mid
            border.width: 1
            radius: 8
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

                Label { text: qsTr("Name") }

                TextField {
                    Layout.fillWidth: true
                    text: picker.playlistName
                    placeholderText: qsTr("Playlist %1").arg(Playlists.all.length + 1)
                    maximumLength: 50
                    onTextEdited: picker.playlistName = text
                }

                Label { text: qsTr("Change every") }

                RowLayout {
                    spacing: 8

                    SpinBox {
                        id: everyHours

                        Layout.preferredWidth: 96
                        from: 0
                        to: 23
                        // Holds the number itself; see the note on
                        // `playlistName` for why it is bound to nothing.
                        // Typed as well as clicked, because clicking an
                        // arrow fourteen times to say "one minute" is not
                        // a way to ask for anything.
                        editable: true
                        value: 0
                    }
                    Label { text: qsTr("hours") }

                    SpinBox {
                        id: everyMinutes

                        Layout.preferredWidth: 96
                        from: 0
                        to: 59
                        editable: true
                        value: 15
                    }
                    Label { text: qsTr("minutes") }

                    Item { Layout.fillWidth: true }
                }

                Label { text: qsTr("Order") }

                RowLayout {
                    spacing: 6

                    Chip {
                        label: qsTr("In order")
                        on: !picker.playlistShuffle
                        onAsked: picker.playlistShuffle = false
                    }
                    Chip {
                        label: qsTr("Shuffled")
                        on: picker.playlistShuffle
                        onAsked: picker.playlistShuffle = true
                    }
                    Item { Layout.fillWidth: true }
                }
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                font.pixelSize: 11
                opacity: 0.8
                // Said plainly, because nought would otherwise look like a
                // setting rather than a mistake.
                text: qsTr("A minute at the least, however short this is set.")
            }

            RowLayout {
                Layout.fillWidth: true

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: "#2e9e5b"
                    onClicked: settings.close()
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The sections, across the top. Only the first of them exists; the
        // rest are named here because naming them is what decides where the
        // next piece of work goes, and because a person can then see what
        // this program is going to be.
        //
        // The strip runs the width of the window while the tabs in it keep
        // the width they need: a TabBar asked to fill the window shares the
        // room out between its tabs, which turns five icons into five wide
        // slabs.
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
                onCurrentIndexChanged: if (tabs.currentIndex === 1) steam.active = true

                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.bottom: parent.bottom

                Section {
                    entry: picker.sections[0]
                    icon.name: "download"
                    icon.color: picker.palette.buttonText
                }
                Section {
                    entry: picker.sections[1]
                    // The one icon in this window with no colour set on
                    // it. It is Steam's own logo and a picture rather than
                    // a glyph, and tinting does not know the difference:
                    // asked to follow the palette it comes out as a plain
                    // white disc. Measured both ways round.
                    icon.name: "steam"
                }
                Section {
                    entry: picker.sections[2]
                    icon.name: "preferences-desktop-display"
                    icon.color: picker.palette.buttonText
                }
                Section {
                    entry: picker.sections[3]
                    icon.name: "settings-configure"
                    icon.color: picker.palette.buttonText
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
                    font.weight: Font.DemiBold
                }
                Label {
                    text: picker.sections[tabs.currentIndex].blurb
                    opacity: 0.65
                }
            }

            Item { Layout.fillWidth: true }

            RoundedButton {
                text: qsTr("Refresh")
                icon.name: "view-refresh"
                // There is nothing to look for again from any other section.
                visible: tabs.currentIndex === 0
                onClicked: Wallpapers.rescan()
            }

            RoundedButton {
                // The language it is in now, rather than the one the click
                // would give you: a language is what somebody looks for to
                // check, and the tooltip says what happens.
                //
                // Not translated, deliberately. "EN" and "CZ" are the same
                // in both, and a language switch that renames itself is a
                // language switch nobody can find their way back from.
                text: Language.czech ? "CZ" : "EN"
                ToolTip.visible: hovered
                ToolTip.text: Language.czech ? qsTr("Switch to English")
                                             : qsTr("Switch to Czech")
                onClicked: Language.czech = !Language.czech
            }

            RoundedButton {
                // The icon is what the click will do, not what is on
                // screen now: turn the lights up, or down.
                //
                // These two rather than a sun and a moon out of the
                // weather icons: those are drawn for a 48-pixel applet
                // and all that survives at 16 is a white disc. Looked
                // at, not assumed. A Button rather than a ToolButton for
                // the same reason the one beside it is: an icon alone on
                // the window background says nothing about being a
                // control at all.
                icon.name: picker.dark ? "brightness-high" : "brightness-low"
                ToolTip.visible: hovered
                ToolTip.text: picker.dark ? qsTr("Light theme") : qsTr("Dark theme")
                onClicked: picker.dark = !picker.dark
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

                    TextField {
                        id: search

                        Layout.fillWidth: true
                        Layout.maximumWidth: 420
                        placeholderText: qsTr("Search wallpapers…")
                        // Room for the magnifier that sits in it, and for
                        // the cross on the other side.
                        leftPadding: 34
                        rightPadding: 34
                        // No wallpaper is named at greater length than
                        // this, and a field that takes a paragraph invites
                        // one.
                        maximumLength: 50

                        RoundedTool {
                            x: 4
                            anchors.verticalCenter: parent.verticalCenter
                            icon.name: "system-search"
                            // Fusion leaves a ToolButton's icon colour unset
                            // and the icon is then drawn in nothing at all.
                            // Measured against a Button, which sets it and
                            // shows up.
                            icon.color: search.palette.text
                            flat: true
                            focusPolicy: Qt.NoFocus
                            // Not decoration for its own sake: it is where
                            // somebody who clicks the magnifier expects to
                            // end up.
                            onClicked: search.forceActiveFocus()
                        }

                        RoundedTool {
                            anchors.right: parent.right
                            anchors.rightMargin: 2
                            anchors.verticalCenter: parent.verticalCenter
                            icon.name: "edit-clear"
                            icon.color: search.palette.text
                            flat: true
                            focusPolicy: Qt.NoFocus
                            // Nothing to clear until something is typed.
                            visible: search.text.length > 0
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Clear the search")
                            onClicked: search.clear()
                        }
                    }

                    RoundedButton {
                        text: filters.chosen > 0 ? qsTr("Filters (%1)").arg(filters.chosen)
                                                 : qsTr("Filters")
                        icon.name: "view-filter"
                        // One click shows the panel beside the grid, the
                        // next puts it away -- the same button in the same
                        // window as the workshop's, behaving the same way.
                        onClicked: filters.visible = !filters.visible
                    }

                    RoundedButton {
                        id: lists

                        text: Playlists.playing !== "" ? Playlists.playing
                                                       : qsTr("Playlists")
                        icon.name: "view-media-playlist"
                        // Nothing to offer until one has been saved.
                        enabled: Playlists.all.length > 0
                        onClicked: saved.opened ? saved.close() : saved.open()

                        Menu {
                            id: saved

                            y: lists.height + 4
                            closePolicy: Popup.CloseOnEscape
                                         | Popup.CloseOnPressOutsideParent

                            Repeater {
                                model: Playlists.all

                                delegate: MenuItem {
                                    required property var modelData

                                    text: (Playlists.playing === modelData.name ? "▶  " : "")
                                          + modelData.name
                                          + " (" + modelData.ids.length + ")"
                                    onTriggered: {
                                        if (Playlists.playing === modelData.name)
                                            Playlists.stop();
                                        else
                                            Playlists.start(modelData.name);
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
                                radius: 8
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
                                font.weight: Font.DemiBold
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

                            readonly property int columns: Math.max(1, Math.floor(grid.room / 240))

                            cellWidth: Math.floor(grid.room / grid.columns)
                            // The preview is sixteen by nine, and the name sits
                            // under it.
                            cellHeight: Math.round((cellWidth - 16) * 9 / 16) + 46

                            model: picker.shown

                            delegate: Item {
                                id: cell

                                required property var modelData

                                width: grid.cellWidth
                                height: grid.cellHeight

                                WallpaperCard {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    wallpaper: cell.modelData
                                    onChose: (id) => picker.inspect(id)
                                    onApplied: (id) => picker.apply(id)
                                    ticked: picker.inPlaylist.indexOf(cell.modelData.id) >= 0
                                    onTickAsked: picker.tick(cell.modelData.id)
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
                        facts: picker.factsFor(picker.inspected, Language.czech)
                        onClosed: {
                            details.visible = false;
                            picker.inspected = null;
                        }

                        RoundedButton {
                            id: giveBack

                            Layout.fillWidth: true
                            text: qsTr("Remove")
                            icon.name: "edit-delete"
                            // Red, because it is the one thing in this
                            // window that takes something away.
                            accent: "#c0392b"
                            // Steam owns the folder, so Steam is what takes
                            // it away. Without it running there is nothing
                            // this could do but delete somebody's files
                            // behind Steam's back, which would come back
                            // the next time it looked.
                            enabled: Steamworks.available
                            onClicked: {
                                const going = picker.inspected;
                                if (!going)
                                    return;
                                // Taking the file out from under a player
                                // that is drawing it leaves a black screen
                                // with no reason given.
                                if (Wallpapers.current.id === going.id)
                                    Wallpapers.clear();
                                // Two halves of the same thing: Steam
                                // lets go of it, and the folder goes.
                                // Steam leaves the folder exactly where it
                                // is -- measured -- so the second half is
                                // ours.
                                Steamworks.remove(going.id);
                                Wallpapers.discard(going.id);
                                details.visible = false;
                                picker.inspected = null;
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
                            text: Steamworks.available
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
                    visible: picker.inPlaylist.length > 0
                    items: picker.playlistItems
                    onDropped: (id) => picker.tick(id)
                    onSaveAsked: picker.savePlaylist()
                    onConfigureAsked: settings.open()
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

            // The three that are not built yet. One component, three times,
            // because three copies of the same label is three places to
            // change when one of them stops being a placeholder.
            component Later: Label {
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                opacity: 0.5
                font.pixelSize: 18
                text: qsTr("Coming soon")
            }

            Later {}
            Later {}
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 38
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
    }
}
