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

    /// The names of the screens the wallpaper is wanted on. Read by
    /// `Main.qml`, which builds one surface per name.
    readonly property var screensWanted: displays.wanted

    /// Which way the colours go. Dark to start with, because a wallpaper
    /// picker is a window full of pictures.
    property bool dark: true

    /// The one the details panel is about, or null while it is shut.
    property var inspected: null

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
        readonly property color mark: picker.dark ? "#3daee9" : "#1f6f9e"

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
        told.push({ label: qsTr("Audio responsive"), icon: "audio-volume-high",
                    value: picker.rawTagsOf(one).indexOf("Audio responsive") >= 0
                               ? qsTr("Yes") : qsTr("Not stated") });
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
        if (!picker.inspected)
            return null;
        return picker.authors[picker.inspected.id] || { name: "…", avatar: "" };
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
            for (let i = 0; i < ids.length; ++i) {
                const said = Steam.items[ids[i]];
                if (said && said.tags.length > 0)
                    seen[ids[i]] = said.tags;
            }
            picker.workshopTags = seen;
        });
    }

    function lookUpAuthor(id) {
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

    /// Shuts the panel in both tabs, and forgets any click that was
    /// still being counted -- which would otherwise open a panel in the
    /// tab that had just been left, a few tenths of a second after
    /// leaving it.
    function shutThePanels() {
        secondClick.stop();
        details.open = false;
        if (steam.item)
            steam.item.shut();
    }

    /// Puts one on the desktop, which is what a second click means. Never
    /// for a scene wallpaper: there is nothing here that can draw one.
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

    /// Whether the workshop has been asked about the library yet.
    property bool askedAboutThemAll: false

    onVisibleChanged: {
        if (!picker.visible || picker.askedAboutThemAll)
            return;
        picker.askedAboutThemAll = true;
        picker.lookUpEverything();
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
                ComboBox {
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

                ComboBox {
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

                    Slider {
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

                        // Fusion draws a slider's handle as a rounded
                        // square. A dot is what was asked for and what
                        // everything else round here has its corners
                        // taken off to look like.
                        handle: Rectangle {
                            x: transitionTime.leftPadding
                               + transitionTime.visualPosition
                                 * (transitionTime.availableWidth - width)
                            y: transitionTime.topPadding
                               + (transitionTime.availableHeight - height) / 2
                            implicitWidth: 18
                            implicitHeight: 18
                            radius: width / 2
                            color: transitionTime.pressed
                                       ? Qt.darker(transitionTime.palette.highlight, 1.2)
                                       : transitionTime.palette.highlight
                            border.width: 1
                            border.color: Qt.darker(transitionTime.palette.highlight, 1.4)
                        }
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
                    accent: "#2e9e5b"
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

        onOpened: loading.picked = picker.playlistName

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
                                elide: Text.ElideRight
                                text: (Playlists.playing === row.modelData.name ? "▶  " : "")
                                      + row.modelData.name
                                color: loading.picked === row.modelData.name
                                           ? picker.palette.highlightedText
                                           : picker.palette.windowText
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
                    accent: "#c0392b"
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

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: "#2e9e5b"
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
                    accent: "#2e9e5b"
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
    readonly property url ambient: {
        if (tabs.currentIndex === 0)
            return details.open && picker.inspected ? picker.inspected.preview : "";
        if (tabs.currentIndex === 1 && steam.item)
            return steam.item.ambient;
        return "";
    }

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
                //
                // And leaving a tab shuts its panel: the panel is a
                // question about one wallpaper in one grid, and walking
                // away from the grid is the end of the question. Coming
                // back to a tab that had one open and finding it still
                // open is the window remembering something nobody asked
                // it to.
                onCurrentIndexChanged: {
                    if (tabs.currentIndex === 1)
                        steam.active = true;
                    picker.shutThePanels();
                }

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
                        // Blue while the panel it opens is open, so the
                        // button says which of its two states it is in.
                        accent: filters.visible ? "#3daee9" : "transparent"
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

                                WallpaperCard {
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
                                    showTick: picker.picking
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
                        tags: picker.tagsOf(picker.inspected, Language.czech)
                        author: picker.inspectedAuthor
                        onAuthorAsked: picker.everythingByTheAuthor()

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
                                details.open = false;
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

            DisplaysView {
                id: displays

                // Every screen to begin with. Done here rather than as a
                // default on the view, because the list of screens is not
                // there yet when the view is built.
                Component.onCompleted: {
                    const names = [];
                    const all = Qt.application.screens;
                    for (let i = 0; i < all.length; ++i)
                        names.push(all[i].name);
                    displays.wanted = names;
                }
            }

            // The one that is not built yet.
            Label {
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                opacity: 0.5
                font.pixelSize: 18
                text: qsTr("Coming soon")
            }
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
