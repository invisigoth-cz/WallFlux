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
/// Unlike the library's filter, which offers only what is on the disk, this
/// offers the workshop's whole vocabulary -- there are three and a quarter
/// million wallpapers out there and no way to know what they carry without
/// asking. Every section is tags as far as Steam is concerned: type, age
/// rating, picture size and genre are all just tags on a workshop item, so
/// they go out as one list of required tags.
Popup {
    id: panel

    /// Everything ticked, in the one list Steam is given.
    property var picked: []

    /// How many chips are on, for the button that opens this.
    readonly property int chosen: panel.picked.length

    /// Which of Wallpaper Engine's catalogues to call the tags by. Not
    /// named `locale`: a Popup has one already and it is FINAL.
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

    implicitWidth: 420
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
            title: qsTr("TAGS")
            values: Tags.labelled(Tags.genres, panel.catalogue)
            picked: panel.picked
            onAsked: (value) => panel.pick(value)
        }

        RowLayout {
            Layout.fillWidth: true

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

            Button {
                text: qsTr("Clear")
                enabled: panel.chosen > 0
                onClicked: panel.clear()
            }
        }
    }
}
