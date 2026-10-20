// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// Browsing the Steam Workshop: rows of wallpapers nobody here owns yet.
///
/// The layout only. Nothing is fetched: the rows are filled with the
/// previews of what is already installed, so the arrangement can be judged
/// against real pictures rather than grey rectangles, and the window says so
/// rather than letting anyone think these are the workshop's.
///
/// What it will be filled from is settled: Wallpaper Engine's own install
/// carries the whole workshop vocabulary in locale/ui_en-us.json -- 103 tags
/// across genre, type, age rating and resolution -- so the sections and the
/// filters need no API key and no network. Fetching the items themselves is
/// the open question, and the one thing a mock cannot answer.
Item {
    id: workshop

    /// The same width a card has in the library, so a wallpaper is the same
    /// size whether it is installed or not.
    readonly property int cardWidth: 240
    readonly property int cardHeight: Math.round((workshop.cardWidth - 16) * 9 / 16) + 46

    /// What the workshop said, where it could be asked; otherwise the
    /// previews of what is already installed, standing in so the layout can
    /// be judged against real pictures.
    readonly property bool real: Steamworks.trending.length > 0
    readonly property var trending: workshop.real ? Steamworks.trending
                                                  : Wallpapers.wallpapers.slice(0, 12)
    readonly property var fresh: Steamworks.newest.length > 0
                                 ? Steamworks.newest
                                 : (workshop.real ? [] : Wallpapers.wallpapers.slice(12))

    // Asked once, when somebody first comes here, and both rows at once --
    // they are asked one after another, which the backend sees to.
    Component.onCompleted: {
        if (Steamworks.available) {
            Steamworks.browse(Steamworks.Trending);
            Steamworks.browse(Steamworks.Newest);
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
                text: offered.item.tags.length > 0 ? offered.item.tags[0]
                                                   : qsTr("Unspecified")
                font.weight: Font.DemiBold
            }
            Label {
                text: offered.item.resolution ? offered.item.resolution
                                              : qsTr("Native")
                opacity: 0.75
            }

            Item { Layout.fillHeight: true }

            Button {
                id: fetch

                /// What `Steamworks.downloads` says about this one:
                /// undefined until it has been asked for, then 0 to 100,
                /// or -1 where Steam would not do it.
                readonly property var progress: Steamworks.downloads[offered.item.id]
                readonly property bool untouched: fetch.progress === undefined

                Layout.fillWidth: true
                text: fetch.untouched ? qsTr("Download")
                     : fetch.progress < 0 ? qsTr("Steam would not")
                     : fetch.progress >= 100 ? qsTr("Downloaded")
                     : qsTr("%1 %").arg(fetch.progress)
                icon.name: fetch.progress >= 100 ? "download-later" : "download"
                // Asking again while it is on its way would have Steam
                // start over; a refusal is worth a second try.
                enabled: Steamworks.available
                         && (fetch.untouched || fetch.progress < 0)
                onClicked: Steamworks.download(offered.item.id)
            }
        }
    }

    /// A heading, a way to see the rest of it, and somewhere to mark it.
    component Heading: RowLayout {
        id: heading

        required property string title

        Layout.fillWidth: true
        spacing: 8

        Label {
            text: heading.title
            font.pixelSize: 17
            font.weight: Font.DemiBold
        }

        Button {
            text: qsTr("See more")
            icon.name: "go-down"
            flat: true
            enabled: false
        }

        Item { Layout.fillWidth: true }

        ToolButton {
            checkable: true
            icon.name: checked ? "starred-symbolic" : "non-starred-symbolic"
            // Fusion leaves a ToolButton's icon colour unset, which draws it
            // in nothing at all. Measured.
            icon.color: checked ? "#3daee9" : workshop.palette.windowText
            ToolTip.visible: hovered
            ToolTip.text: qsTr("Keep this row at the top")
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

            ButtonGroup {
                id: layouts
                exclusive: true
            }

            ToolButton {
                icon.name: "compass"
                icon.color: workshop.palette.windowText
                checkable: true
                checked: true
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Rows picked for you")
            }
            ToolButton {
                icon.name: "view-list-icons"
                icon.color: workshop.palette.windowText
                checkable: true
                ButtonGroup.group: layouts
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Everything, in one grid")
            }

            TextField {
                Layout.preferredWidth: 300
                placeholderText: qsTr("Search workshop…")
                leftPadding: 34

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
                text: qsTr("Filters")
                icon.name: "view-filter"
            }

            Button {
                text: qsTr("Trending")
                icon.name: "view-sort-descending"
            }
        }

        // Said plainly rather than left to be worked out from the fact that
        // the rows look familiar. Two different truths: whether Steam can be
        // asked at all, and whether anything has asked it yet.
        Label {
            Layout.alignment: Qt.AlignHCenter
            Layout.maximumWidth: parent.width - 48
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            visible: !workshop.real
            text: Steamworks.available
                  ? qsTr("Asking Steam…")
                  : qsTr("%1 Until then the rows below are the wallpapers already "
                         + "installed, standing in so the layout can be judged.")
                    .arg(Steamworks.trouble)
            opacity: 0.5
            font.pixelSize: 11
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24

            contentWidth: width
            contentHeight: rows.implicitHeight
            clip: true
            ScrollBar.vertical: ScrollBar {}

            ColumnLayout {
                id: rows

                width: parent.width
                spacing: 10

                Heading { title: qsTr("Trending") }

                // Sideways, because a row that runs off the edge says there
                // is more of it without taking the height to show it.
                ListView {
                    Layout.fillWidth: true
                    Layout.preferredHeight: workshop.cardHeight
                    orientation: ListView.Horizontal
                    spacing: 12
                    clip: true
                    model: workshop.trending

                    delegate: WorkshopCard {
                        required property var modelData

                        width: workshop.cardWidth
                        height: ListView.view.height
                        item: modelData
                    }

                    ScrollBar.horizontal: ScrollBar {}
                }

                Heading { title: qsTr("New") }

                GridView {
                    id: latest

                    Layout.fillWidth: true
                    // As tall as it needs: the page below scrolls, so this
                    // one must not try to as well.
                    Layout.preferredHeight: latest.contentHeight
                    interactive: false

                    // The same sum the library's grid does: as many columns
                    // of about the card width as fit, then the width shared
                    // out between them.
                    readonly property int columns:
                        Math.max(1, Math.floor(latest.width / workshop.cardWidth))

                    cellWidth: Math.floor(latest.width / latest.columns)
                    cellHeight: Math.round((cellWidth - 16) * 9 / 16) + 46
                    model: workshop.fresh

                    delegate: Item {
                        id: cell

                        required property var modelData

                        width: latest.cellWidth
                        height: latest.cellHeight

                        WorkshopCard {
                            anchors.fill: parent
                            anchors.margins: 8
                            item: cell.modelData
                        }
                    }
                }
            }
        }
    }
}
