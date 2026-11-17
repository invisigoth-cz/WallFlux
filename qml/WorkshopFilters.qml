// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// What to ask the workshop for, in sections of chips.
///
/// A panel beside the grid rather than a popup over it. Fifty-nine chips is
/// taller than most windows, and a popup that does not fit does not scroll
/// -- it runs off the bottom edge taking Clear with it, and how much room
/// there is is something only whoever placed it can know. Standing in the
/// layout, it is given the height of the page and the grid reflows into
/// what is left, so the same panel fits a small window and a maximised one.
///
/// Unlike the library's filter, which offers only what is on the disk, this
/// offers the workshop's whole vocabulary -- there are three and a quarter
/// million wallpapers out there and no way to know what they carry without
/// asking. Every section is tags as far as Steam is concerned: type, age
/// rating, picture size and genre are all just tags on a workshop item, so
/// they go out as one list of required tags.
Rectangle {
    id: panel

    /// Everything ticked, in the one list Steam is given.
    property var picked: []

    /// Whether Anime is offered at all. Off in Settings, its chip is not
    /// there to tick (the user's choice of 10 November): the switch in
    /// Settings is the one way to have Anime back.
    property bool anime: true

    /// How many chips are on, for the button that opens this.
    readonly property int chosen: panel.picked.length

    /// Which of Wallpaper Engine's catalogues to call the tags by.
    readonly property string catalogue: Language.czech ? "cs-cz" : "en-us"

    signal changed()

    function toggled(value) {
        return panel.picked.indexOf(value) >= 0
            ? panel.picked.filter((kept) => kept !== value)
            : panel.picked.concat([value]);
    }

    function pick(value) {
        panel.picked = panel.toggled(value);
        panel.changed();
    }

    function clear() {
        panel.picked = [];
        panel.changed();
    }

    implicitWidth: 300

    /// Whether it is out. Shown by the Filters button, which is the only
    /// way in.
    property bool open: false

    /// How far out it is, nought to one. It slides as the library's filter
    /// does: the width moves, and the grid beside it shrinks as a picture
    /// while it does (see `gridArea` in WorkshopView.qml).
    property real slide: panel.open ? 1 : 0

    Behavior on slide {
        NumberAnimation {
            duration: 220
            easing.type: Easing.OutCubic
        }
    }

    Layout.preferredWidth: Math.round(panel.implicitWidth * panel.slide)
    // Away means away: a layout leaves no spacing for something invisible.
    visible: panel.slide > 0
    // The chips keep their full width and the moving edge cuts them off.
    clip: true

    // Every section folded each time the panel is shown.
    onOpenChanged: {
        if (!panel.open)
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
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.margins: 14
        width: panel.implicitWidth - 28
        spacing: 12

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: body.implicitHeight
            clip: true

            // Only while there is more than fits: with the sections folded
            // it usually all does, and a bar beside nothing is a line down
            // the edge.
            ScrollBar.vertical: Scroller {
                id: bar
                policy: ScrollBar.AsNeeded
            }

            ColumnLayout {
                id: body

                // Short of the scrollbar, while there is one, so the bar is
                // not sitting on the chips it scrolls.
                width: parent.width - (bar.visible ? 18 : 0)
                spacing: 16

                ChipSection {
                    title: qsTr("TYPE")
                    values: Tags.labelled(Tags.types, panel.catalogue)
                    picked: panel.picked
                    onAsked: (value) => panel.pick(value)
                }

                ChipSection {
                    title: qsTr("AGE RATING")
                    values: Tags.labelled(Tags.ratings, panel.catalogue)
                    picked: panel.picked
                    onAsked: (value) => panel.pick(value)
                }

                ChipSection {
                    title: qsTr("RESOLUTION")
                    values: Tags.labelled(Tags.resolutions, panel.catalogue)
                    picked: panel.picked
                    onAsked: (value) => panel.pick(value)
                }

                ChipSection {
                    title: qsTr("GENRE")
                    values: Tags.labelled(panel.anime ? Tags.genres
                                                      : Tags.genres.filter((g) => g !== "Anime"),
                                          panel.catalogue)
                    picked: panel.picked
                    onAsked: (value) => panel.pick(value)
                }

                ChipSection {
                    title: qsTr("TAGS")
                    values: Tags.labelled(Tags.properties, panel.catalogue)
                    picked: panel.picked
                    onAsked: (value) => panel.pick(value)
                }
            }
        }

        // Outside the Flickable on purpose: a Clear button that has to be
        // scrolled to is a Clear button nobody finds.
        RowLayout {
            Layout.fillWidth: true

            // On the left, under the chips it clears.
            RoundedButton {
                text: qsTr("Clear selection")
                enabled: panel.chosen > 0
                onClicked: panel.clear()
            }

            Label {
                Layout.fillWidth: true
                // Worth saying, because it is the opposite of what a row of
                // chips usually means: Steam requires every tag given, so
                // two picture sizes at once match nothing at all.
                text: qsTr("Everything ticked has to match")
                font.pixelSize: 11
                opacity: 0.55
                wrapMode: Text.WordWrap
            }
        }
    }
}
