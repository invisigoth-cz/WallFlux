// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux
import "steamprofile.js" as Steam

/// The Steam Workshop: three and a quarter million wallpapers, two ways in.
///
/// **Discover** is rows picked for you -- Trending, New, and one per genre
/// -- a few cards each, with more rows built as the page is scrolled rather
/// than all twenty-seven asked for at once. Nothing here is filtered or
/// sorted by hand: that is what the Workshop is for. "See more" on a row
/// opens that row whole, as a page of its own with the section's name at
/// the top and the way back beside it.
///
/// **Workshop** is one grid: search, filters and fifty at a time.
///
/// Every section and every filter is the same mechanism underneath. Type,
/// age rating, picture size and genre are all just tags on a workshop item,
/// so a genre row and a ticked chip both come out as a required tag on the
/// one query.
Item {
    id: workshop

    /// The same width a card has in the library, so a wallpaper is the same
    /// size whether it is installed or not.
    readonly property int cardWidth: 300
    readonly property int cardHeight: Math.round((workshop.cardWidth - 16) * 9 / 16) + 46

    /// Which of Wallpaper Engine's catalogues to call the tags by.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    /// Previews are taken exactly as Steam gives them, and **asking the
    /// CDN to resize one is a trap** -- measured twice, in both directions.
    ///
    /// It looks like the obvious saving: a preview is a few hundred
    /// kilobytes and a card draws it 300 across, and Steam's own pages put
    /// `?imw=...&imh=...&ima=fit&impolicy=Letterbox` on the same URLs. For
    /// a photograph it is a real saving -- 296 kB becomes 11 kB. But most
    /// of the workshop's previews are animated, and for those:
    ///
    /// - the picture is **re-encoded when it is asked for**, and a size
    ///   nobody has asked for before is made on the spot. Measured over six
    ///   previews: 0.7 to 7.9 seconds each, where the file as stored came
    ///   back in 0.03 to 0.09. That is where "a page takes tens of seconds"
    ///   came from, with fifty of them on a page.
    /// - it is not even smaller. One came back as 1 099 kB asked for at 268
    ///   across and 924 kB asked for not at all.
    ///
    /// Sixteen previews as stored, eight at a time: 8 MB in 0.19 s. So the
    /// bytes were never the problem on a connection like this one. On a
    /// slow one the sums would come out differently for the photographs --
    /// if that ever matters, it is this comment and a URL, not a design.

    /// 0 while Discover is showing, 1 while the Workshop grid is, 2 while
    /// one of Discover's sections is open whole.
    property int mode: 0

    /// The one the details panel is about, or null while it is shut.
    property var inspected: null

    /// Shows one wallpaper of the workshop in the panel. There is nothing
    /// else a click on a card out here can mean: it is not ours to play.
    function inspect(item) {
        workshop.inspected = item;
        shown.visible = true;
        if (item)
            workshop.lookUpAuthor(item.owner);
    }

    /// Who an account is, keyed by steamID64: `{ name, avatar }`. Filled
    /// in as the answers arrive; the asking itself is steamprofile.js,
    /// which the library shares.
    property var authors: ({})

    /// The one the panel is about: the name and the face once they have
    /// arrived, and a row of dots in the meantime.
    ///
    /// Not null until the answer lands. The button appeared a second after
    /// the panel did, which is how it came to be missed altogether -- it
    /// is there as soon as there is somebody to ask about.
    readonly property var inspectedAuthor: {
        const id = workshop.inspected ? workshop.inspected.owner : "";
        if (!id || id === "0")
            return null;
        return workshop.authors[id] || { name: "…", avatar: "" };
    }

    function lookUpAuthor(id) {
        Steam.profileOf(id, function (who) {
            // A new object, not the one already there: assigning the same
            // reference back changes nothing anybody is bound to.
            const known = Object.assign({}, workshop.authors);
            known[id] = who;
            workshop.authors = known;
            if (workshop.creator === id)
                workshop.creatorName = who.name;
        });
    }

    /// Whose work the grid is narrowed to, as a steamID64, and what to
    /// call them. Empty while it is the whole workshop.
    property string creator: ""
    property string creatorName: ""

    /// Everything else by whoever made the wallpaper in the panel. Asked
    /// for here, or from the library's panel, which has no grid of its own
    /// to answer with.
    function everythingBy(id) {
        workshop.creator = id;
        // The id stands in until the name arrives -- which for anybody
        // already asked about, in either tab, is the same instant.
        workshop.creatorName = id;
        workshop.lookUpAuthor(id);
        workshop.section = null;
        workshop.mode = 1;
        searchField.text = "";
        workshop.search = "";
        workshop.askAfresh();
    }

    function wholeWorkshopAgain() {
        workshop.creator = "";
        workshop.creatorName = "";
        workshop.askAfresh();
    }

    /// Whether this is one of the workshop's building blocks rather than
    /// a wallpaper of its own.
    ///
    /// An asset and a preset carry no `type` in their project.json -- an
    /// asset is a part for somebody to build a scene out of, a preset is
    /// somebody else's wallpaper plus a file of settings -- so the library
    /// has nothing to list them as and does not. Found the expensive way:
    /// one was downloaded, the button said Downloaded, and it turned up
    /// nowhere.
    function notAWallpaper(item) {
        if (!item)
            return false;
        return item.tags.indexOf("Asset") >= 0 || item.tags.indexOf("Preset") >= 0;
    }

    /// The first of `tags` that is one of `list`, in words -- which is how
    /// the type, the age rating and the picture size are read, since Steam
    /// keeps all three as tags like any other.
    ///
    /// `language` is read and not used; see `wordsFor` below for why.
    function firstOf(tags, list, language) {
        for (let i = 0; i < tags.length; ++i) {
            if (list.indexOf(tags[i]) >= 0)
                return Tags.name(tags[i], workshop.catalogue);
        }
        return "";
    }

    /// What the workshop says about one of its own, in the order it reads.
    function factsFor(item, language) {
        if (!item)
            return [];

        const told = [];
        const kind = workshop.firstOf(item.tags, Tags.types, language);
        if (kind)
            told.push({ label: qsTr("Type"), icon: "layer-visible-on", value: kind });

        const size = workshop.firstOf(item.tags, Tags.resolutions, language);
        if (size)
            told.push({ label: qsTr("Resolution"), icon: "monitor", value: size });

        if (item.bytes > 0) {
            told.push({ label: qsTr("Size"), icon: "drive-harddisk",
                        value: qsTr("%1 MB").arg(Math.round(item.bytes / 1048576)) });
        }

        const rating = workshop.firstOf(item.tags, Tags.ratings, language);
        if (rating)
            told.push({ label: qsTr("Age rating"), icon: "flag", value: rating });

        told.push({ label: qsTr("Audio responsive"), icon: "audio-volume-high",
                    value: item.tags.indexOf("Audio responsive") >= 0 ? qsTr("Yes")
                                                                      : qsTr("Not stated") });

        if (item.votesUp > 0)
            told.push({ label: qsTr("Likes"), icon: "draw-star", value: String(item.votesUp) });

        told.push({ label: qsTr("Workshop id"), icon: "documentinfo", value: item.id });
        return told;
    }

    /// Which section is open, while one is. The same entry out of
    /// `sections`, so it carries its own name, order and tag.
    property var section: null

    /// Which mode the answer now in the grid was asked for, since both the
    /// Workshop and a section are shown in the same grid. Coming back to
    /// the Workshop from a section has to ask again, or it would go on
    /// showing the section's answer under the Workshop's controls. -1 until
    /// anything has been asked at all.
    property int answering: -1

    /// How many of the Discover rows have been built. More are added as the
    /// page is scrolled, so coming here costs a handful of questions rather
    /// than twenty-seven.
    property int shown: 4

    /// Tops the rows up to what the page has room for, with two over the
    /// edge so there is something to scroll towards.
    ///
    /// Four of them filled a window of the usual size and left a maximised
    /// one half empty -- and with nothing hanging over the bottom edge
    /// there was nothing to scroll, so the rest were never asked for at
    /// all. Called on every resize, which is what maximising is.
    function topUp() {
        // A row is a card, the heading over it and the gap after it.
        const each = workshop.cardHeight + 60;
        const need = Math.ceil(workshop.height / each) + 2;
        if (need > workshop.shown)
            workshop.shown = Math.min(need, workshop.sections.length);
    }

    onHeightChanged: workshop.topUp()

    /// What Steam gives in one answer. Not ours to choose.
    readonly property int perPage: 50

    /// How many of that answer a Discover row shows.
    ///
    /// A row is a taste of a section rather than the section: fifty cards
    /// in a strip is a lot of sideways scrolling for something somebody is
    /// only glancing along, and Wallpaper Engine's own rows stop at twenty.
    /// "See more" is where the rest of them are.
    readonly property int perRow: 20

    /// How deep the paging goes, whatever the total says.
    ///
    /// Steam does not answer for ever: measured against Trending, page 1001
    /// came back with its fifty and page 1024 was refused outright -- result
    /// 25, the limit -- as were 2000 and 64929. And offering 64 929 pages
    /// of three and a quarter million wallpapers is not an offer anyway; it
    /// is a number nobody can do anything with. Wallpaper Engine stops at a
    /// thousand pages and so does this.
    readonly property int deepest: 1000

    // ---- Discover -----------------------------------------------------

    /// Trending and New, then a row per genre. Each row is its own question
    /// under its own name, so one row's answer never lands in another.
    readonly property var sections: {
        const rows = [
            { key: "trending", title: qsTr("Trending"),
              order: Steamworks.Trending, tag: "" },
            { key: "new", title: qsTr("New"), order: Steamworks.Newest, tag: "" }
        ];
        const genres = Tags.genres;
        for (let i = 0; i < genres.length; ++i) {
            rows.push({ key: "genre:" + genres[i],
                        title: Tags.name(genres[i], workshop.catalogue),
                        order: Steamworks.Trending,
                        tag: genres[i] });
        }
        return rows;
    }

    // ---- Workshop -----------------------------------------------------

    property int order: Steamworks.Trending
    property int page: 1
    property string search: ""

    readonly property var grid: Steamworks.found["browse"] || []
    readonly property int total: Steamworks.totals["browse"] || 0
    readonly property int pages:
        Math.max(1, Math.min(workshop.deepest,
                             Math.ceil(workshop.total / workshop.perPage)))

    /// The row of pages to offer: the first, the last, the five around the
    /// one being looked at, and a gap -- written as 0 -- wherever numbers
    /// were left out. A thousand pages cannot all be on the screen, and
    /// the ones worth a click are the near ones and the two ends.
    readonly property var pageRow: {
        const last = workshop.pages;
        const near = [];
        for (let i = workshop.page - 2; i <= workshop.page + 2; ++i) {
            if (i >= 1 && i <= last)
                near.push(i);
        }

        const row = [];
        if (near[0] > 1) {
            row.push(1);
            if (near[0] > 2)
                row.push(0);
        }
        for (let i = 0; i < near.length; ++i)
            row.push(near[i]);
        const end = near[near.length - 1];
        if (end < last) {
            if (end < last - 1)
                row.push(0);
            row.push(last);
        }
        return row;
    }

    /// The reason the workshop cannot be asked, in words.
    ///
    /// `language` is read but not used: it is there so this is asked again
    /// when the language is switched. `retranslate()` refreshes a binding
    /// that translates, and a call into a function is not one -- without
    /// the argument the heading changes language and this does not.
    function wordsFor(why, language) {
        switch (why) {
        case Steamworks.NoLibrary:
            return qsTr("The Steamworks library is not on this machine.");
        case Steamworks.UnknownLibrary:
            return qsTr("The Steamworks library on this machine is not one this "
                        + "program knows how to talk to.");
        case Steamworks.SteamSilent:
            return qsTr("Steam did not answer. It has to be running, and signed in "
                        + "to the account that owns Wallpaper Engine.");
        case Steamworks.NoWorkshop:
            return qsTr("Steam answered but offered no workshop to ask about.");
        }
        return "";
    }

    function orderName(which) {
        switch (which) {
        case Steamworks.Newest: return qsTr("New");
        case Steamworks.MostSubscribed: return qsTr("Most subscribed");
        }
        return qsTr("Trending");
    }

    /// Asks for the grid as it is currently set up. Called rather than
    /// bound: a binding would fire once per property on the way to a new
    /// set of filters, and each firing is a question to Steam.
    function ask() {
        if (!Steamworks.available)
            return;

        // A section is its own question -- the row's tag and the row's
        // order -- and has nothing to do with what the Workshop's filter
        // and sort happen to say.
        const open = workshop.mode === 2 && workshop.section;
        workshop.answering = workshop.mode;

        // Urgent: this is the question somebody is sitting in front of, and
        // Discover's rows are not. Without it a search typed on arriving
        // here waited out every row still queued.
        Steamworks.browse("browse",
                          open ? workshop.section.order : workshop.order,
                          workshop.page,
                          open ? (workshop.section.tag ? [workshop.section.tag] : [])
                               : filters.picked,
                          open ? "" : workshop.search,
                          true,
                          // A section is somebody else's row, so it is
                          // never one author's.
                          open ? "" : workshop.creator);
    }

    /// Goes to a page, which is the same thing whichever arrow or number
    /// was clicked.
    function turnTo(page) {
        if (page < 1 || page > workshop.pages || page === workshop.page)
            return;
        workshop.page = page;
        workshop.ask();
    }

    /// Any change to what is being asked starts again at the first page --
    /// page eleven of the old question means nothing under the new one.
    function askAfresh() {
        workshop.page = 1;
        workshop.ask();
    }

    /// Opens one of Discover's rows whole, which is what "See more" means.
    ///
    /// Not a trip to the Workshop with the row's tag ticked, which is what
    /// it used to be: this is still Discover, so there is nothing to filter
    /// or sort by hand -- only the section, its wallpapers, and the way
    /// back.
    function seeMore(section) {
        workshop.section = section;
        filters.visible = false;
        searchField.text = "";
        workshop.search = "";
        workshop.mode = 2;
        workshop.askAfresh();
    }

    onModeChanged: if (workshop.mode === 1 && workshop.answering !== 1) workshop.askAfresh()

    // Steam may have been started since WallFlux was. Coming to this tab
    // asks again, so the usual way out of an empty page is to do the
    // obvious thing and come back.
    Component.onCompleted: {
        workshop.topUp();
        if (!Steamworks.available)
            Steamworks.retry();
    }

    // The grid has the same problem the rows do: switched to while Steam
    // was not running, it asked nothing and would never ask again.
    Connections {
        target: Steamworks
        function onAvailableChanged() {
            if (Steamworks.available && workshop.mode === 1 && workshop.grid.length === 0)
                workshop.ask();
        }
    }

    /// One wallpaper as the workshop shows it: the same tile the library
    /// uses, and nothing else yet.
    ///
    /// What was here -- the genre, the size on disk and a Download button,
    /// all drawn over the preview on hover -- has been taken out: it is to
    /// be said somewhere else, and the tile is to carry something else.
    /// Downloading goes with it for the moment; `Steamworks.download()` is
    /// still there and nothing calls it.
    component WorkshopCard: PreviewTile {
        id: offered

        required property var item

        preview: offered.item.preview
        label: offered.item.title
        onClicked: workshop.inspect(offered.item)
    }

    /// One Discover row: a heading, a sideways strip of cards, and a way
    /// into the rest of it.
    component DiscoverRow: ColumnLayout {
        id: row

        required property var section

        readonly property var items: Steamworks.found[row.section.key] || []
        readonly property bool answered:
            Steamworks.found[row.section.key] !== undefined

        /// Whether this row's question is already with Steam. `answered`
        /// alone was not enough: a row built before the workshop was known
        /// to be reachable asks on being told it is, and measured, that is
        /// a second helping of the same question while the first is still
        /// in the air.
        property bool asked: false

        Layout.fillWidth: true
        spacing: 6

        /// Asked when the row is built, which is when it is first scrolled
        /// to, and again if Steam becomes reachable after that -- a row
        /// built while Steam was not running would otherwise stay empty
        /// for good, which is what made "Try again" useless the first time
        /// it was tried. `answered` is what stops it being asked twice.
        function ask() {
            if (Steamworks.available && !row.answered && !row.asked) {
                row.asked = true;
                Steamworks.browse(row.section.key, row.section.order, 1,
                                  row.section.tag ? [row.section.tag] : []);
            }
        }

        Component.onCompleted: row.ask()

        Connections {
            target: Steamworks
            function onAvailableChanged() { row.ask(); }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                text: row.section.title
                font.pixelSize: 17
                font.weight: Font.DemiBold
            }

            RoundedButton {
                text: qsTr("See more")
                icon.name: "go-next"
                flat: true
                enabled: row.items.length > 0
                onClicked: workshop.seeMore(row.section)
            }

            Item { Layout.fillWidth: true }

            Label {
                text: row.answered ? qsTr("nothing here") : qsTr("asking…")
                visible: row.items.length === 0
                opacity: 0.45
                font.pixelSize: 11
            }
        }

        // Sideways, because a row that runs off the edge says there is more
        // of it without taking the height to show it.
        ListView {
            Layout.fillWidth: true
            // The cards, and a strip under them for the scrollbar: a bar
            // drawn along the bottom edge of the view sat across the
            // bottom of every preview in the row.
            Layout.preferredHeight: workshop.cardHeight + 20
            visible: row.items.length > 0
            orientation: ListView.Horizontal
            spacing: 12
            clip: true
            model: row.items.slice(0, workshop.perRow)

            delegate: WorkshopCard {
                required property var modelData

                width: workshop.cardWidth
                height: workshop.cardHeight
                item: modelData
            }

            ScrollBar.horizontal: Scroller {}
        }
    }

    // Steam is being asked something. Measured at a tenth of a second to
    // three quarters of one for a question it answers at once, and longer
    // for one that waits its turn -- long enough that without this the
    // window looked like it had not heard the click.
    BusyIndicator {
        anchors.centerIn: parent
        implicitWidth: 56
        implicitHeight: 56
        z: 2
        running: Steamworks.busy && workshop.mode !== 0 && Steamworks.available
        visible: running
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        // Centred, as the picture has it: the controls belong to the whole
        // page rather than to the left edge of it.
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 4
            spacing: 10
            visible: Steamworks.available

            ButtonGroup {
                id: layouts
                exclusive: true
            }

            RoundedTool {
                icon.name: "compass"
                icon.color: workshop.palette.windowText
                checkable: true
                // A section opened out of Discover is still Discover, so
                // this stays lit while one is showing -- and clicking it is
                // the way back out.
                checked: workshop.mode !== 1
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Discover")
                onClicked: {
                    workshop.mode = 0;
                    workshop.creator = "";
                    workshop.creatorName = "";
                }
            }
            RoundedTool {
                icon.name: "view-list-icons"
                icon.color: workshop.palette.windowText
                checkable: true
                checked: workshop.mode === 1
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Workshop")
                // The whole of it, whoever was being looked at. Clearing
                // the author is not enough on its own: coming here from a
                // grid that was already in mode 1 changes nothing, so
                // `onModeChanged` never fires and the last author's work
                // sits there under a button that says Workshop.
                onClicked: {
                    const narrowed = workshop.creator !== "";
                    workshop.creator = "";
                    workshop.creatorName = "";
                    workshop.mode = 1;
                    if (narrowed)
                        workshop.askAfresh();
                }
            }

            TextField {
                id: searchField

                Layout.preferredWidth: 300
                placeholderText: qsTr("Search workshop…")
                leftPadding: 34
                // Room for the cross on the other side.
                rightPadding: 34
                // Nothing Steam is asked is longer than this, and a field
                // that takes a paragraph invites one.
                maximumLength: 50

                // Typing is not a question. Steam is asked once the typing
                // stops, or at once if somebody presses return.
                Timer {
                    id: settle
                    interval: 400
                    onTriggered: {
                        workshop.mode = 1;
                        workshop.search = searchField.text;
                        // Searching is a question about the workshop, not
                        // about whoever was being looked at.
                        workshop.creator = "";
                        workshop.creatorName = "";
                        workshop.askAfresh();
                    }
                }

                onTextEdited: settle.restart()
                onAccepted: settle.triggered()

                RoundedTool {
                    x: 4
                    anchors.verticalCenter: parent.verticalCenter
                    icon.name: "system-search"
                    icon.color: parent.palette.text
                    flat: true
                    focusPolicy: Qt.NoFocus
                }

                RoundedTool {
                    anchors.right: parent.right
                    anchors.rightMargin: 2
                    anchors.verticalCenter: parent.verticalCenter
                    icon.name: "edit-clear"
                    icon.color: searchField.palette.text
                    flat: true
                    focusPolicy: Qt.NoFocus
                    visible: searchField.text.length > 0
                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Clear the search")
                    // Emptying the field is only half of it: what is on the
                    // screen is still the answer to the word that was
                    // there, so the question is asked again without it.
                    onClicked: {
                        searchField.clear();
                        settle.triggered();
                    }
                }
            }

            // Whose work is being shown, while it is one person's. A chip
            // rather than a line of type: it is a filter, and it comes off
            // the way the filters do.
            RoundedButton {
                text: qsTr("By %1").arg(workshop.creatorName)
                icon.name: "user-identity"
                visible: workshop.mode === 1 && workshop.creator !== ""
                accent: "#3daee9"
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Back to the whole workshop")
                onClicked: workshop.wholeWorkshopAgain()
            }

            RoundedButton {
                text: filters.chosen > 0 ? qsTr("Filters (%1)").arg(filters.chosen)
                                         : qsTr("Filters")
                icon.name: "view-filter"
                // Only the Workshop has filters. Discover is a page of
                // rows somebody else picked, and a section out of it is
                // that row whole -- neither is something to narrow by
                // hand, which is what the Workshop is there for.
                visible: workshop.mode === 1
                // The panel stands in the layout beside the grid rather
                // than popping up over it, so it is in the same place
                // whatever size the window is -- see WorkshopFilters.qml.
                // Blue while the panel it opens is open, so the
                // button says which of its two states it is in.
                accent: filters.visible ? "#3daee9" : "transparent"
                onClicked: filters.visible = !filters.visible
            }

            RoundedButton {
                id: sortButton

                text: workshop.orderName(workshop.order)
                icon.name: "view-sort-descending"
                // As with the filters: a section is already in the order
                // its row is in.
                visible: workshop.mode === 1
                onClicked: sorting.opened ? sorting.close() : sorting.open()

                // Plain items rather than checkable ones: a checkable
                // Controls item told what to be and also clicked breaks the
                // binding it was reading. The button's own text is where
                // the choice shows.
                Menu {
                    id: sorting

                    y: sortButton.height
                    // Without this the menu closes on the press and the
                    // click that follows opens it again, which looks like
                    // a flicker and leaves it showing. Pressing the button
                    // that owns it is not pressing outside it.
                    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

                    Repeater {
                        model: [Steamworks.Trending, Steamworks.Newest,
                                Steamworks.MostSubscribed]

                        delegate: MenuItem {
                            required property var modelData

                            text: workshop.orderName(modelData)
                            onTriggered: {
                                workshop.order = modelData;
                                workshop.askAfresh();
                            }
                        }
                    }
                }
            }
        }

        // Nothing at all rather than rows that sit there saying "asking..."
        // about a question nobody asked. Which of the two reasons it is --
        // no library on the machine, or Steam not running -- is the one
        // thing that matters here, so it is said in full and in the middle
        // of the page rather than in small grey type under the controls.
        ColumnLayout {
            // Centred as a block rather than stretched with its children
            // aligned inside it: filling the width and aligning each child
            // left the whole thing sitting against the left edge. Looked at.
            Layout.alignment: Qt.AlignHCenter
            Layout.fillHeight: true
            visible: !Steamworks.available
            spacing: 10

            Item { Layout.fillHeight: true }

            Label {
                Layout.alignment: Qt.AlignHCenter
                text: qsTr("The workshop is not available")
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }

            Label {
                Layout.alignment: Qt.AlignHCenter
                Layout.maximumWidth: Math.min(460, workshop.width - 48)
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: workshop.wordsFor(Steamworks.trouble, Language.czech)
                opacity: 0.7
            }

            RoundedButton {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 6
                text: qsTr("Try again")
                icon.name: "view-refresh"
                onClicked: Steamworks.retry()
            }

            Item { Layout.fillHeight: true }
        }

        RowLayout {
            visible: Steamworks.available
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 24
            // Short, because the scrollbar lives in this edge and the whole
            // point of it is to be out at the side rather than among the
            // pictures.
            Layout.rightMargin: 8
            spacing: 14

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: workshop.mode === 0 ? 0 : 1

                // ---- Discover ----
                Flickable {
                    id: discover

                    contentWidth: width
                    contentHeight: rows.implicitHeight
                    clip: true
                    ScrollBar.vertical: Scroller {}

                    // One more handful of rows once the bottom is in sight.
                    // Each row asks Steam for itself, so building them all at
                    // once would be twenty-seven questions nobody has looked at.
                    onContentYChanged: {
                        if (workshop.shown < workshop.sections.length
                            && contentY + height > contentHeight - 400) {
                            workshop.shown += 2;
                        }
                    }

                    ColumnLayout {
                        id: rows

                        // Short of the scrollbar, as the grids are.
                        width: discover.width - 30
                        spacing: 16

                        Repeater {
                            model: workshop.sections.slice(0, workshop.shown)

                            delegate: DiscoverRow {
                                required property var modelData

                                section: modelData
                            }
                        }

                        Label {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.topMargin: 8
                            Layout.bottomMargin: 8
                            visible: workshop.shown < workshop.sections.length
                            text: qsTr("More rows as you scroll")
                            font.pixelSize: 11
                            opacity: 0.45
                        }
                    }
                }

                // ---- Workshop ----
                RowLayout {
                    spacing: 14

                    WorkshopFilters {
                        id: filters

                        Layout.fillHeight: true
                        Layout.bottomMargin: 8
                        onChanged: workshop.askAfresh()
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 10

                        // Where you are, and the way out of it. Only a section
                        // has either: the Workshop is where the controls above
                        // say it is.
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 2
                            spacing: 8
                            visible: workshop.mode === 2

                            RoundedTool {
                                icon.name: "go-previous"
                                icon.color: workshop.palette.windowText
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Back to Discover")
                                onClicked: {
                    workshop.mode = 0;
                    workshop.creator = "";
                    workshop.creatorName = "";
                }
                            }

                            Label {
                                // The section's own name and nothing else. The
                                // arrow beside it says where it goes back to.
                                text: workshop.section ? workshop.section.title : ""
                                font.pixelSize: 17
                                font.weight: Font.DemiBold
                            }

                            Item { Layout.fillWidth: true }
                        }

                        GridView {
                            id: everything

                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true

                            // The same sum the library's grid does: as many columns
                            // of about the card width as fit, then the width shared
                            // out between them -- less a strip down the right for
                            // the scrollbar to live in, since a bar drawn over the
                            // last column of previews reads as part of the picture.
                            readonly property int room: everything.width - 30

                            readonly property int columns:
                                Math.max(1, Math.floor(everything.room / workshop.cardWidth))

                            cellWidth: Math.floor(everything.room / everything.columns)
                            cellHeight: Math.round((cellWidth - 16) * 9 / 16) + 46
                            model: workshop.grid

                            // A new answer starts at its own beginning. Page
                            // two opened where page one had been scrolled to,
                            // and so did a section opened out of Discover.
                            onModelChanged: everything.positionViewAtBeginning()

                            // Out of the way while a new answer is on its way:
                            // what is on the screen is the answer to the last
                            // question, and leaving it at full strength under a
                            // new one is what made searching look like nothing
                            // happening at all.
                            opacity: Steamworks.busy ? 0.35 : 1
                            Behavior on opacity {
                                NumberAnimation { duration: 120 }
                            }

                            ScrollBar.vertical: Scroller {}

                            delegate: Item {
                                required property var modelData

                                width: everything.cellWidth
                                height: everything.cellHeight

                                WorkshopCard {
                                    anchors.fill: parent
                                    anchors.margins: 6
                                    item: parent.modelData
                                }
                            }
                        }

                        Label {
                            Layout.alignment: Qt.AlignHCenter
                            visible: workshop.grid.length === 0
                            text: Steamworks.busy ? qsTr("Asking Steam…")
                                                  : qsTr("Nothing matches all of that")
                            opacity: 0.5
                            font.pixelSize: 12
                        }

                        // The pages themselves rather than Previous and Next:
                        // a number somebody can aim at, the two ends, and
                        // arrows that are not there when there is nothing that
                        // way.
                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.bottomMargin: 8
                            spacing: 4
                            visible: workshop.grid.length > 0

                            RoundedTool {
                                icon.name: "go-first"
                                icon.color: workshop.palette.windowText
                                visible: workshop.page > 1
                                enabled: !Steamworks.busy
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("First page")
                                onClicked: workshop.turnTo(1)
                            }

                            RoundedTool {
                                icon.name: "go-previous"
                                icon.color: workshop.palette.windowText
                                visible: workshop.page > 1
                                enabled: !Steamworks.busy
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Previous page")
                                onClicked: workshop.turnTo(workshop.page - 1)
                            }

                            Repeater {
                                model: workshop.pageRow

                                delegate: RowLayout {
                                    id: entry

                                    required property int modelData

                                    spacing: 0

                                    Label {
                                        visible: entry.modelData === 0
                                        text: "…"
                                        leftPadding: 4
                                        rightPadding: 4
                                        opacity: 0.5
                                    }

                                    Chip {
                                        visible: entry.modelData !== 0
                                        enabled: !Steamworks.busy
                                        label: entry.modelData
                                        on: entry.modelData === workshop.page
                                        onAsked: workshop.turnTo(entry.modelData)
                                    }
                                }
                            }

                            RoundedTool {
                                icon.name: "go-next"
                                icon.color: workshop.palette.windowText
                                visible: workshop.page < workshop.pages
                                enabled: !Steamworks.busy
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Next page")
                                onClicked: workshop.turnTo(workshop.page + 1)
                            }

                            RoundedTool {
                                icon.name: "go-last"
                                icon.color: workshop.palette.windowText
                                visible: workshop.page < workshop.pages
                                enabled: !Steamworks.busy
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Last page")
                                onClicked: workshop.turnTo(workshop.pages)
                            }
                        }
                    }
                }
            }

            Details {
                id: shown

                Layout.fillHeight: true
                Layout.bottomMargin: 8
                preview: workshop.inspected ? workshop.inspected.preview : ""
                title: workshop.inspected ? workshop.inspected.title : ""
                facts: workshop.factsFor(workshop.inspected, Language.czech)
                tags: workshop.inspected
                      ? Tags.labelled(workshop.inspected.tags, workshop.catalogue)
                            .map((one) => one.l)
                      : []
                author: workshop.inspectedAuthor
                onAuthorAsked: workshop.everythingBy(workshop.inspected.owner)
                onClosed: {
                    shown.visible = false;
                    workshop.inspected = null;
                }

                RoundedButton {
                    id: fetch

                    /// Whether this one is already on the disk. Reads the
                    /// library's own list, so a download that finishes
                    /// turns the button over by itself the moment the
                    /// folder watcher notices -- no asking Steam again.
                    readonly property string id: workshop.inspected ? workshop.inspected.id : ""
                    readonly property bool owned: Wallpapers.installedIds.indexOf(fetch.id) >= 0

                    /// What `Steamworks.downloads` says about this one:
                    /// undefined until it has been asked for, then 0 to
                    /// 100, or -1 where Steam would not do it.
                    readonly property var progress: Steamworks.downloads[fetch.id]
                    readonly property bool untouched: fetch.progress === undefined

                    Layout.fillWidth: true
                    // The one button in this window that is the thing to do
                    // on the page it is on.
                    accent: fetch.owned ? "transparent" : "#2e9e5b"
                    text: fetch.owned ? qsTr("Installed")
                         : fetch.untouched ? qsTr("Download")
                         : fetch.progress < 0 ? qsTr("Steam would not")
                         : fetch.progress >= 100 ? qsTr("Downloaded")
                         : qsTr("%1 %").arg(fetch.progress)
                    icon.name: fetch.owned ? "dialog-ok-apply"
                             : fetch.progress >= 100 ? "download-later" : "download"
                    // Nothing to fetch for something already here. Asking
                    // again while it is on its way would have Steam start
                    // over; a refusal is worth a second try.
                    enabled: Steamworks.available && !fetch.owned
                             && (fetch.untouched || fetch.progress < 0)
                    onClicked: Steamworks.download(fetch.id)
                }

                Label {
                    Layout.fillWidth: true
                    visible: workshop.notAWallpaper(workshop.inspected)
                    wrapMode: Text.Wrap
                    font.pixelSize: 11
                    opacity: 0.8
                    text: qsTr("A part for Wallpaper Engine rather than a "
                               + "wallpaper of its own. Downloading it will not "
                               + "put anything among the installed ones.")
                }
            }
        }
    }
}
