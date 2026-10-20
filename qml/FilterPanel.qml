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
/// Holds the choice itself, because a popup stays while the grid is rebuilt
/// around it. The picker reads the four lists and does the leaving out.
///
/// Every value in here comes out of project.json beside the wallpaper --
/// tags, content rating and type are all written there by Wallpaper Engine,
/// so nothing is asked of Steam or of the workshop over the network.
Popup {
    id: panel

    /// What is ticked in each section. Values rather than labels, since one
    /// of them is "no resolution at all" and another four are enum members.
    property var tags: []
    property var ratings: []
    property var resolutions: []
    property var kinds: []

    /// How many chips are on, for the button that opens this.
    readonly property int chosen: panel.tags.length + panel.ratings.length
                                 + panel.resolutions.length + panel.kinds.length

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
    }

    /// Which of Wallpaper Engine's catalogues to call the tags by. Read as
    /// a property rather than worked out inside `labelled`, so the chips
    /// rebuild when the language is switched.
    ///
    /// Not called `locale`: a Popup has one of those already and it is
    /// FINAL, so the name takes the whole application down at load with
    /// "Cannot override FINAL property" -- not just this file.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    /// The tags the installed wallpapers actually carry, rather than all 65
    /// Steam knows: a filter for something nobody has is a dead chip.
    readonly property var carriedTags: {
        const seen = [];
        const all = Wallpapers.wallpapers;
        for (let i = 0; i < all.length; ++i) {
            const carried = all[i].tags;
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
    readonly property var tagValues:
        Tags.labelled(panel.carriedTags.filter((tag) => Tags.genres.indexOf(tag) < 0),
                       panel.catalogue)

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

    implicitWidth: 360
    padding: 14
    modal: false
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

    background: Rectangle {
        color: panel.palette.base
        border.color: panel.palette.mid
        border.width: 1
        radius: 8
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 16

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
            title: qsTr("AGE RATING")
            // The three Wallpaper Engine has, under the names shown here.
            // Everything installed on the machine this was written on is the
            // first of them.
            values: [{ v: "G", l: "G" }, { v: "PG13", l: "PG13" }, { v: "R", l: "R" }]
            picked: panel.ratings
            onAsked: (value) => panel.ratings = panel.toggled(panel.ratings, value)
        }

        ChipSection {
            title: qsTr("COMPATIBILITY")
            // Nothing answers this yet: it is for when a wallpaper fetched
            // from the workshop has been tried and found to work or not.
            // Shown and switched off, so the plan is visible and nothing
            // pretends to do anything.
            enabled: false
            values: [{ v: "perfect", l: qsTr("Perfect") },
                     { v: "minor", l: qsTr("Minor issues") },
                     { v: "major", l: qsTr("Major issues") },
                     { v: "broken", l: qsTr("Broken") },
                     { v: "unknown", l: qsTr("Unknown") }]
            picked: []
            onAsked: (value) => {}
        }

        ChipSection {
            title: qsTr("RESOLUTION")
            values: panel.resolutionValues
            picked: panel.resolutions
            onAsked: (value) => panel.resolutions = panel.toggled(panel.resolutions, value)
        }

        ChipSection {
            title: qsTr("TYPE")
            values: [{ v: WallpaperKind.Scene, l: qsTr("Scene") },
                     { v: WallpaperKind.Video, l: qsTr("Video") },
                     { v: WallpaperKind.Web, l: qsTr("Web") },
                     { v: WallpaperKind.Application, l: qsTr("Application") }]
            picked: panel.kinds
            onAsked: (value) => panel.kinds = panel.toggled(panel.kinds, value)
        }

        RowLayout {
            Layout.fillWidth: true

            Item { Layout.fillWidth: true }

            Button {
                text: qsTr("Clear")
                flat: true
                enabled: panel.chosen > 0
                onClicked: panel.clear()
            }
        }
    }
}
