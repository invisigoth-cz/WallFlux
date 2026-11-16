// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// What to leave out of the grid, in sections of chips.
///
/// A panel beside the grid rather than a popup over it, which is what the
/// workshop's filter is -- the same button in the same window should not
/// behave two different ways. It holds the choice itself and the picker
/// reads the four lists and does the leaving out.
///
/// Every value in here comes out of project.json beside the wallpaper --
/// tags, content rating and type are all written there by Wallpaper Engine,
/// so nothing is asked of Steam or of the workshop over the network.
Rectangle {
    id: panel

    /// What is ticked in each section. Values rather than labels, since one
    /// of them is "no resolution at all" and another four are enum members.
    property var tags: []
    property var ratings: []
    property var resolutions: []
    property var kinds: []
    property var grades: []
    /// ["only"] while the favourites alone are wanted. A list like the rest
    /// so that it is a chip like the rest.
    property var favorites: []

    /// How many chips are on, for the button that opens this.
    readonly property int chosen: panel.tags.length + panel.ratings.length
                                 + panel.resolutions.length + panel.kinds.length
                                 + panel.grades.length + panel.favorites.length

    /// The same list with that value taken out or put in.
    function toggled(list, value) {
        return list.indexOf(value) >= 0 ? list.filter((kept) => kept !== value)
                                        : list.concat([value]);
    }

    function clear() {
        panel.tags = [];
        panel.ratings = [];
        panel.resolutions = [];
        panel.kinds = [];
        panel.grades = [];
        panel.favorites = [];
    }

    /// Which of Wallpaper Engine's catalogues to call the tags by. Read as
    /// a property rather than worked out inside `labelled`, so the chips
    /// rebuild when the language is switched.
    ///
    /// Called what the workshop's filter calls it, which was named that
    /// because a Popup has a `locale` already and it is FINAL -- the name
    /// took the whole application down at load.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    /// What the workshop says the installed wallpapers are tagged with,
    /// keyed by id. Handed in by the window, which is what asks.
    property var knownTags: ({})

    /// The tags the installed wallpapers actually carry, rather than all 65
    /// Steam knows: a filter for something nobody has is a dead chip.
    ///
    /// Both lists together, the workshop's and the disk's. project.json
    /// carries the genres its author ticked and nothing else -- ten
    /// distinct tags across this machine's library, against twenty-nine
    /// the workshop knows of the same wallpapers -- but measured the other
    /// way round, one genre here is on the disk and not in the workshop's
    /// answer, because an author can change what is published without the
    /// downloaded copy hearing of it. Taking one instead of the other lost
    /// that genre a chip.
    readonly property var carriedTags: {
        const seen = [];
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            const known = panel.knownTags[all[i].id] || [];
            const carried = known.concat(all[i].tags);
            for (let t = 0; t < carried.length; ++t) {
                if (seen.indexOf(carried[t]) < 0)
                    seen.push(carried[t]);
            }
        }
        return seen;
    }

    readonly property var genreValues:
        Tags.labelled(panel.carriedTags.filter((tag) => Tags.genres.indexOf(tag) >= 0),
                       panel.catalogue)

    /// Everything that is not a genre: audio responsive, customizable,
    /// approved and the rest, which describe a wallpaper rather than say
    /// what it is of.
    ///
    /// What is on the disk, and `Audio responsive` whether anything here
    /// carries it or not. Measured: of the 55 installed, every one carries
    /// genres and nothing else, so this section would not appear at all --
    /// and this is the one somebody goes looking for before they have the
    /// wallpaper that answers it. The chip is there, and the day one is
    /// downloaded it already means something.
    readonly property var tagValues: {
        // What the sections below ask about in their own words, so that a
        // kind, a rating or a picture size does not turn up here as well.
        // `Wallpaper` goes too: Steam tags nearly everything with it and a
        // chip that matches everything narrows nothing.
        const elsewhere = Tags.genres.concat(Tags.types, Tags.ratings, Tags.resolutions,
                                             ["Wallpaper"]);
        const carried = panel.carriedTags.filter((tag) => elsewhere.indexOf(tag) < 0);
        if (carried.indexOf("Audio responsive") < 0)
            carried.push("Audio responsive");
        return Tags.labelled(carried, panel.catalogue);
    }

    /// Every picture size among the videos, and one for everything that has
    /// none -- a scene or web wallpaper has no single answer, and a video
    /// that has not been measured yet has not answered.
    readonly property var resolutionValues: {
        const seen = [];
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            const shape = all[i].resolution;
            if (shape && seen.indexOf(shape) < 0)
                seen.push(shape);
        }
        seen.sort();
        return [{ v: "", l: qsTr("Unknown") }].concat(seen.map((s) => ({ v: s, l: s })));
    }

    implicitWidth: 300
    // Shown by the Filters button, which is the only way in.
    visible: false
    // Every section folded each time the panel is shown.
    onVisibleChanged: {
        if (!panel.visible)
            return;
        for (let i = 0; i < body.children.length; ++i) {
            if (body.children[i].chosenHere !== undefined)
                body.children[i].open = false;
        }
    }

    color: panel.palette.base
    border.color: panel.palette.mid
    border.width: 1
    radius: 8

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: body.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller {}

            ColumnLayout {
                id: body

                // Short of the scrollbar, so the bar is not sitting on the
                // chips it scrolls.
                width: parent.width - 18
                spacing: 16

                // The five the workshop's filter has too, in its order, so a
                // section is in the same place in both tabs; then the two
                // only the library has.
                ChipSection {
                    title: qsTr("TYPE")
                    values: [{ v: WallpaperKind.Scene, l: qsTr("Scene") },
                             { v: WallpaperKind.Video, l: qsTr("Video") },
                             { v: WallpaperKind.Web, l: qsTr("Web") },
                             { v: WallpaperKind.Application, l: qsTr("Application") }]
                    picked: panel.kinds
                    onAsked: (value) => panel.kinds = panel.toggled(panel.kinds, value)
                }

                ChipSection {
                    title: qsTr("AGE RATING")
                    // Steam's three words, which is what a wallpaper's rating is
                    // kept as whether it came off the disk or out of a query -- so
                    // the same chip means the same thing in both filters.
                    values: Tags.labelled(Tags.ratings, panel.catalogue)
                    picked: panel.ratings
                    onAsked: (value) => panel.ratings = panel.toggled(panel.ratings, value)
                }

                ChipSection {
                    title: qsTr("RESOLUTION")
                    values: panel.resolutionValues
                    picked: panel.resolutions
                    onAsked: (value) => panel.resolutions = panel.toggled(panel.resolutions, value)
                }

                ChipSection {
                    title: qsTr("GENRE")
                    values: panel.genreValues
                    picked: panel.tags
                    onAsked: (value) => panel.tags = panel.toggled(panel.tags, value)
                }

                ChipSection {
                    title: qsTr("TAGS")
                    values: panel.tagValues
                    picked: panel.tags
                    onAsked: (value) => panel.tags = panel.toggled(panel.tags, value)
                }

                ChipSection {
                    title: qsTr("FAVOURITES")
                    // The account's hearts on Steam, so only while it can
                    // be asked.
                    visible: Steamworks.available
                    values: [{ v: "only", l: qsTr("Only favourites") }]
                    picked: panel.favorites
                    onAsked: (value) => panel.favorites = panel.toggled(panel.favorites, value)
                }

                ChipSection {
                    title: qsTr("COMPATIBILITY")
                    // How a wallpaper fared when it was played: what the
                    // renderer left out, or whether it played at all. No
                    // "major issues": nothing can tell those from minor ones
                    // without somebody looking.
                    values: [{ v: "perfect", l: qsTr("Works") },
                             { v: "minor", l: qsTr("Minor issues") },
                             { v: "broken", l: qsTr("Broken") },
                             { v: "unknown", l: qsTr("Not tried yet") }]
                    picked: panel.grades
                    onAsked: (value) => panel.grades = panel.toggled(panel.grades, value)
                }
            }
        }

        // Outside the Flickable on purpose: a Clear button that has to be
        // scrolled to is a Clear button nobody finds.
        RowLayout {
            Layout.fillWidth: true

            Item { Layout.fillWidth: true }

            RoundedButton {
                text: qsTr("Clear selection")
                enabled: panel.chosen > 0
                onClicked: panel.clear()
            }
        }
    }
}
