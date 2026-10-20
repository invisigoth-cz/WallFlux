// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// The Steam Workshop: three and a quarter million wallpapers, two ways in.
///
/// **Discover** is rows picked for you -- Trending, New, and one per genre
/// -- a few cards each, with more rows built as the page is scrolled rather
/// than all twenty-seven asked for at once. "See more" on a row is not a
/// third view: it switches to Workshop with that row's tag already ticked,
/// which is the same thing with paging and the filters showing.
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
    readonly property int cardWidth: 240
    readonly property int cardHeight: Math.round((workshop.cardWidth - 16) * 9 / 16) + 46

    /// Which of Wallpaper Engine's catalogues to call the tags by.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    /// 0 while Discover is showing, 1 while the Workshop grid is.
    property int mode: 0

    /// How many of the Discover rows have been built. More are added as the
    /// page is scrolled, so coming here costs two questions rather than
    /// twenty-seven.
    property int shown: 4

    /// What Steam gives in one answer. Not ours to choose.
    readonly property int perPage: 50

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
        Math.max(1, Math.ceil(workshop.total / workshop.perPage))

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
        if (Steamworks.available)
            Steamworks.browse("browse", workshop.order, workshop.page,
                              filters.picked, workshop.search);
    }

    /// Any change to what is being asked starts again at the first page --
    /// page eleven of the old question means nothing under the new one.
    function askAfresh() {
        workshop.page = 1;
        workshop.ask();
    }

    /// Switches to the grid showing one row's tag, which is what "See more"
    /// means. Replaces whatever was ticked: the row was the whole request.
    function seeMore(tag) {
        filters.picked = tag ? [tag] : [];
        workshop.search = "";
        workshop.mode = 1;
        workshop.askAfresh();
    }

    onModeChanged: if (workshop.mode === 1 && workshop.grid.length === 0) workshop.ask()

    // Steam may have been started since WallFlux was. Coming to this tab
    // asks again, so the usual way out of an empty page is to do the
    // obvious thing and come back.
    Component.onCompleted: if (!Steamworks.available) Steamworks.retry()

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
    /// uses, with something to do about not owning it.
    component WorkshopCard: PreviewTile {
        id: offered

        required property var item

        preview: offered.item.preview
        label: offered.item.title
        telling: offered.hovered

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 2

            Label {
                text: offered.item.tags.length > 0
                      ? Tags.name(offered.item.tags[0], workshop.catalogue)
                      : qsTr("Unspecified")
                font.weight: Font.DemiBold
            }
            Label {
                // What the workshop answers with. There is no resolution in
                // it -- Steam does not carry one -- so the size is what
                // there is to say about how big a thing this is.
                text: offered.item.bytes > 0
                      ? qsTr("%1 MB").arg(Math.round(offered.item.bytes / 1048576))
                      : ""
                opacity: 0.75
            }

            Item { Layout.fillHeight: true }

            Button {
                id: fetch

                /// Whether this one is already on the disk. Reads the
                /// library's own list, so a download that finishes turns
                /// the button over by itself the moment the folder watcher
                /// notices -- no asking Steam a second time.
                readonly property bool owned:
                    Wallpapers.installedIds.indexOf(offered.item.id) >= 0

                /// What `Steamworks.downloads` says about this one:
                /// undefined until it has been asked for, then 0 to 100,
                /// or -1 where Steam would not do it.
                readonly property var progress: Steamworks.downloads[offered.item.id]
                readonly property bool untouched: fetch.progress === undefined

                Layout.fillWidth: true
                text: fetch.owned ? qsTr("Installed")
                     : fetch.untouched ? qsTr("Download")
                     : fetch.progress < 0 ? qsTr("Steam would not")
                     : fetch.progress >= 100 ? qsTr("Downloaded")
                     : qsTr("%1 %").arg(fetch.progress)
                icon.name: fetch.owned ? "dialog-ok-apply"
                         : fetch.progress >= 100 ? "download-later" : "download"
                // Nothing to fetch for something already here. Asking again
                // while it is on its way would have Steam start over; a
                // refusal is worth a second try.
                enabled: Steamworks.available && !fetch.owned
                         && (fetch.untouched || fetch.progress < 0)
                onClicked: Steamworks.download(offered.item.id)
            }
        }
    }

    /// One Discover row: a heading, a sideways strip of cards, and a way
    /// into the rest of it.
    component DiscoverRow: ColumnLayout {
        id: row

        required property var section

        readonly property var items: Steamworks.found[row.section.key] || []
        readonly property bool answered:
            Steamworks.found[row.section.key] !== undefined

        Layout.fillWidth: true
        spacing: 6

        /// Asked when the row is built, which is when it is first scrolled
        /// to, and again if Steam becomes reachable after that -- a row
        /// built while Steam was not running would otherwise stay empty
        /// for good, which is what made "Try again" useless the first time
        /// it was tried. `answered` is what stops it being asked twice.
        function ask() {
            if (Steamworks.available && !row.answered) {
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

            Button {
                text: qsTr("See more")
                icon.name: "go-next"
                flat: true
                enabled: row.items.length > 0
                onClicked: workshop.seeMore(row.section.tag)
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
            Layout.preferredHeight: workshop.cardHeight
            visible: row.items.length > 0
            orientation: ListView.Horizontal
            spacing: 12
            clip: true
            model: row.items

            delegate: WorkshopCard {
                required property var modelData

                width: workshop.cardWidth
                height: ListView.view.height
                item: modelData
            }

            ScrollBar.horizontal: ScrollBar {}
        }
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

            ToolButton {
                icon.name: "compass"
                icon.color: workshop.palette.windowText
                checkable: true
                checked: workshop.mode === 0
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Discover")
                onClicked: workshop.mode = 0
            }
            ToolButton {
                icon.name: "view-list-icons"
                icon.color: workshop.palette.windowText
                checkable: true
                checked: workshop.mode === 1
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Workshop")
                onClicked: workshop.mode = 1
            }

            TextField {
                id: searchField

                Layout.preferredWidth: 300
                placeholderText: qsTr("Search workshop…")
                leftPadding: 34

                // Typing is not a question. Steam is asked once the typing
                // stops, or at once if somebody presses return.
                Timer {
                    id: settle
                    interval: 400
                    onTriggered: {
                        workshop.mode = 1;
                        workshop.search = searchField.text;
                        workshop.askAfresh();
                    }
                }

                onTextEdited: settle.restart()
                onAccepted: settle.triggered()

                ToolButton {
                    x: 4
                    anchors.verticalCenter: parent.verticalCenter
                    icon.name: "system-search"
                    icon.color: parent.palette.text
                    flat: true
                    focusPolicy: Qt.NoFocus
                }
            }

            Button {
                text: filters.chosen > 0 ? qsTr("Filters (%1)").arg(filters.chosen)
                                         : qsTr("Filters")
                icon.name: "view-filter"
                onClicked: {
                    workshop.mode = 1;
                    filters.open();
                }

                WorkshopFilters {
                    id: filters

                    y: parent.height + 4
                    // What is left of the window below the button, less a
                    // margin so it does not sit on the bottom edge.
                    room: workshop.height - 90
                    onChanged: workshop.askAfresh()
                }
            }

            Button {
                id: sortButton

                text: workshop.orderName(workshop.order)
                icon.name: "view-sort-descending"
                onClicked: sorting.open()

                // Plain items rather than checkable ones: a checkable
                // Controls item told what to be and also clicked breaks the
                // binding it was reading. The button's own text is where
                // the choice shows.
                Menu {
                    id: sorting

                    y: sortButton.height

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

            Button {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 6
                text: qsTr("Try again")
                icon.name: "view-refresh"
                onClicked: Steamworks.retry()
            }

            Item { Layout.fillHeight: true }
        }

        StackLayout {
            visible: Steamworks.available
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            currentIndex: workshop.mode

            // ---- Discover ----
            Flickable {
                id: discover

                contentWidth: width
                contentHeight: rows.implicitHeight
                clip: true
                ScrollBar.vertical: ScrollBar {}

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

                    width: discover.width
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
            ColumnLayout {
                spacing: 10

                GridView {
                    id: everything

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true

                    // The same sum the library's grid does: as many columns
                    // of about the card width as fit, then the width shared
                    // out between them.
                    readonly property int columns:
                        Math.max(1, Math.floor(everything.width / workshop.cardWidth))

                    cellWidth: Math.floor(everything.width / everything.columns)
                    cellHeight: Math.round((cellWidth - 16) * 9 / 16) + 46
                    model: workshop.grid

                    ScrollBar.vertical: ScrollBar {}

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

                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.bottomMargin: 8
                    spacing: 12
                    visible: workshop.grid.length > 0

                    Button {
                        text: qsTr("Previous")
                        icon.name: "go-previous"
                        enabled: workshop.page > 1 && !Steamworks.busy
                        onClicked: { workshop.page -= 1; workshop.ask(); }
                    }

                    Label {
                        text: qsTr("Page %1 of %2").arg(workshop.page).arg(workshop.pages)
                        font.pixelSize: 12
                        opacity: 0.7
                    }

                    Button {
                        text: qsTr("Next")
                        icon.name: "go-next"
                        enabled: workshop.page < workshop.pages && !Steamworks.busy
                        onClicked: { workshop.page += 1; workshop.ask(); }
                    }
                }
            }
        }
    }
}
