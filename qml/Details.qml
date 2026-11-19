// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import QtQuick.Layouts
import WallFlux

/// What is known about one wallpaper, in a panel beside the grid.
///
/// The same panel in the library and in the workshop: a picture, a name,
/// a list of what is known, and whatever can be done about it -- which is
/// handed in as children, since that is the one thing the two tabs do not
/// share. What is in the list is the caller's business too: the two know
/// different things about a wallpaper and neither has to pretend
/// otherwise.
Rectangle {
    id: panel

    required property url preview
    required property string title

    /// What is known, as `{ label, value, icon }` in the order it should
    /// read -- `icon` by the name the icon theme calls it, and left out
    /// where there is none. Whoever builds the list leaves out what it
    /// cannot answer rather than filling a row with a dash.
    required property var facts

    /// The tags, already in the language of the window. Drawn as chips
    /// under their own heading rather than as a line of commas, which is
    /// what the rest of this window does with a tag.
    property var tags: []

    /// Who made it, as `{ name, avatar }`, or null where nobody knows --
    /// which is every wallpaper in the library, since project.json does
    /// not say who wrote it.
    property var author: null

    /// Whatever can be done about this one. One button, so far.
    default property alias actions: doing.data
    /// Whatever goes under the tags, scrolled with the list: the
    /// wallpaper's own settings, in the library -- under what it is, the
    /// user's order of 12 November.
    property alias below: under.data

    /// Asked for when somebody wants the rest of this author's work.
    signal authorAsked()

    /// The heart on the picture, as on the cards: whether this one is a
    /// favourite on Steam, whether it can be, and the click asking for it.
    property bool favorite: false
    property bool canFavorite: false
    signal favoriteAsked()

    /// Asked for by the cross on the picture. Clicking the card the panel
    /// is about again puts it away too; the cross came back on 10 November,
    /// at the user's asking, as the way that can be seen.
    signal closeAsked()

    /// Whether the panel is out.
    ///
    /// It slides rather than appearing: the width is what moves, and the
    /// grid beside it takes the room back as it goes.
    property bool open: false

    /// How far out it is, nought to one. The width is read off this
    /// rather than animated straight, because this is a plain property of
    /// our own and `Layout.preferredWidth` is an attached one.
    property real slide: panel.open ? 1 : 0

    Behavior on slide {
        NumberAnimation {
            duration: 220
            easing.type: Easing.OutCubic
        }
    }

    /// The least the list is worth standing the picture still for: four or
    /// five of its rows.
    readonly property int roomForTheList: 200
    /// Whether the window is too short for the picture to stand still: it
    /// then scrolls with the list. Worked out from the column's height,
    /// which the window sets, and from what the picture, the name and the
    /// buttons ask for -- none of which this changes, so it cannot loop.
    readonly property bool short: column.height - head.implicitHeight - doing.implicitHeight
                                  - 2 * column.spacing - 16 < panel.roomForTheList

    implicitWidth: 360
    Layout.preferredWidth: Math.round(panel.implicitWidth * panel.slide)
    // Away means away: a layout leaves no spacing for something invisible,
    // so the grid ends where the window does.
    visible: panel.slide > 0
    // What makes this a drawer rather than a column being crushed: the
    // contents below keep their full width and the edge cuts them off.
    clip: true

    color: panel.palette.base
    border.color: panel.palette.mid
    border.width: 1
    radius: 8

    ColumnLayout {
        id: column

        // Held at its full width against the right-hand edge, which is
        // the edge that stays put while the panel slides. Anchored to
        // `parent` and sized off `panel`, so nothing in here reflows
        // while the width is moving.
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.margins: 18
        width: panel.implicitWidth - 36
        spacing: 14

        // Where the picture and the name stand while there is room for them
        // to stand still; see `short`.
        Item {
            id: atTop

            Layout.fillWidth: true
            Layout.preferredHeight: head.implicitHeight
            visible: !panel.short
        }

        // The picture and the name stay put; everything under them scrolls.
        //
        // The picture used to scroll with the rest, which in a window of
        // any height meant a scrollbar: a hundred and sixty pixels of
        // photograph is most of what the panel has, so the content was
        // always taller than the room. What is worth scrolling is the list,
        // and the author sitting over it.
        //
        // Unless the window is short. At 1366x768 the panel is about 380
        // high, and the picture, the name and the buttons took all of it:
        // the list was squeezed to ten pixels and the wallpaper seemed to
        // have no properties at all (19 November, on the laptop). So where
        // the list would get less than `roomForTheList`, the picture and the
        // name scroll with it and the buttons still stay put.
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            // A clear step down the page from the name to the small print,
            // on top of the column's own spacing.
            Layout.topMargin: panel.short ? 0 : 16

            contentWidth: width
            contentHeight: told.implicitHeight
            clip: true
            // The one scrollbar in this window that comes and goes. Every
            // other is on a page that always has more under it, where a
            // bar that fades is a bar nobody finds; a panel about one
            // wallpaper often fits, and a bar beside nothing is just a
            // line down the edge.
            ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: told

                // Short of the scrollbar, and then some: at eighteen the
                // bar was against the ends of the values.
                width: parent.width - 30
                spacing: 0

                // Where the picture and the name go in a short window, with
                // the same step down to the list as above.
                Item {
                    id: inList

                    Layout.fillWidth: true
                    Layout.preferredHeight: head.implicitHeight + 16
                    visible: panel.short
                }

                // Who made it, and the way to the rest of their work.
                // Nothing at all where nobody knows, rather than a row
                // saying so.
                //
                // The whole row is the button. A magnifier on its own
                // beside the name was there and was not seen: a ToolButton
                // draws nothing until it is pointed at, so there was no
                // button on the page to notice. This has an edge and a
                // word on it.
                Button {
                    id: more

                    Layout.fillWidth: true
                    visible: !!panel.author
                    padding: 8
                    ToolTip.visible: more.hovered
                    ToolTip.text: qsTr("Everything else by this author")
                    onClicked: panel.authorAsked()

                    background: Rectangle {
                        radius: 7
                        color: more.down ? Qt.alpha(panel.palette.highlight, 0.35)
                             : more.hovered ? Qt.alpha(panel.palette.highlight, 0.18)
                                            : "transparent"
                        border.width: 1
                        border.color: panel.palette.mid
                    }

                    contentItem: RowLayout {
                        spacing: 8

                        Image {
                            // The avatar Steam serves is 32 square; this
                            // draws it at 22 and lets the picture do the
                            // smoothing.
                            source: panel.author ? panel.author.avatar : ""
                            sourceSize.width: 32
                            sourceSize.height: 32
                            Layout.preferredWidth: 22
                            Layout.preferredHeight: 22
                            fillMode: Image.PreserveAspectCrop
                            visible: status === Image.Ready
                        }

                        Label {
                            Layout.fillWidth: true
                            text: panel.author ? panel.author.name : ""
                            textFormat: Text.PlainText   // off the workshop: never HTML
                            color: panel.palette.highlight
                            elide: Text.ElideRight
                        }

                        IconImage {
                            name: "edit-find"
                            color: panel.palette.windowText
                            sourceSize.width: 16
                            sourceSize.height: 16
                        }
                    }
                }

                Item {
                    // Only where there is a button over it to step down
                    // from.
                    visible: !!panel.author
                    Layout.preferredHeight: 12
                }

                Repeater {
                    model: panel.facts

                    delegate: ColumnLayout {
                        id: fact

                        required property var modelData

                        Layout.fillWidth: true
                        spacing: 0

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 11
                            Layout.bottomMargin: 11
                            spacing: 10

                            // The only public way to draw a theme icon in
                            // QML without a button round it. It is told a
                            // colour for the same reason the tab icons
                            // are: untinted it keeps whatever the icon
                            // theme drew it, and goes invisible in one of
                            // the two themes.
                            IconImage {
                                name: fact.modelData.icon ?? ""
                                color: panel.palette.windowText
                                sourceSize.width: 16
                                sourceSize.height: 16
                                visible: !!fact.modelData.icon
                                opacity: 0.7
                            }

                            Label {
                                // What the row is, quieter than what it says.
                                text: fact.modelData.label
                                opacity: 0.6
                            }

                            Item { Layout.fillWidth: true }

                            Label {
                                Layout.maximumWidth: told.width * 0.55
                                horizontalAlignment: Text.AlignRight
                                text: fact.modelData.value
                                textFormat: Text.PlainText   // off the workshop: never HTML
                                wrapMode: Text.Wrap
                                font.weight: Font.DemiBold
                            }
                        }

                        // A hairline under every row, which is what turns a
                        // list of pairs into something that reads across.
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 1
                            color: panel.palette.mid
                            opacity: 0.6
                        }
                    }
                }

                Label {
                    Layout.topMargin: 20
                    text: qsTr("Tags")
                    font.pixelSize: 11
                    font.letterSpacing: 1.1
                    font.weight: Font.Bold
                    font.capitalization: Font.AllUppercase
                    opacity: 0.6
                    visible: panel.tags.length > 0
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.topMargin: 8
                    spacing: 6

                    Repeater {
                        model: panel.tags

                        // A pill with a name in it, and nothing to click.
                        // Not the Chip from the filters: that one is a
                        // control with a hand cursor and a signal, and a
                        // tag here is a label.
                        delegate: Rectangle {
                            id: pill

                            required property string modelData

                            implicitWidth: name.implicitWidth + 20
                            implicitHeight: 24
                            radius: Look.tags === "outline" ? 12 : 5
                            // As the theme draws a tag: grey, tinted in the
                            // accent as Steam's own are, or an outline.
                            color: Look.tags === "tinted" ? Qt.alpha(Look.highlight, 0.18)
                                 : Look.tags === "outline" ? "transparent"
                                 : Qt.alpha(pill.palette.mid, 0.5)
                            border.width: Look.tags === "outline" ? 1 : 0
                            border.color: pill.palette.windowText

                            Label {
                                id: name

                                anchors.centerIn: parent
                                text: pill.modelData
                                font.pixelSize: 12
                                color: Look.tags === "tinted" && pill.palette.window.hslLightness < 0.5
                                       ? Qt.lighter(Look.highlight, 1.15)
                                       : Look.tags === "tinted" ? Qt.darker(Look.highlight, 1.6)
                                       : pill.palette.windowText
                            }
                        }
                    }
                }

                ColumnLayout {
                    id: under

                    Layout.fillWidth: true
                    spacing: 0
                }
            }
        }

        // Outside the Flickable, like Clear in the filters: what there is
        // to do about a wallpaper is not something to scroll for.
        ColumnLayout {
            id: doing

            Layout.fillWidth: true
            spacing: 6
        }
    }

    // The picture and the name, as one: they stand in `atTop`, or in
    // `inList` in a short window, where they scroll with the list.
    ColumnLayout {
        id: head

        parent: panel.short ? inList : atTop
        width: parent.width
        spacing: 14

        PreviewTile {
            // Narrower than the panel, and centred.
            //
            // Sixteen by nine, where the grid's cards are squares: this
            // is the one place a whole preview is looked at, and a square
            // here would crop a third of it off for nothing. A tile takes
            // the shape it is given, so both of these are read.
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(head.width, 280)
            Layout.preferredHeight: Math.round(Math.min(head.width, 280) * 9 / 16)
            preview: panel.preview
            // The name goes under this in its own type, where it can wrap.
            label: ""
            clickable: false

            Heart {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 8
                visible: panel.canFavorite
                on: panel.favorite
                onAsked: panel.favoriteAsked()
            }

            // The cross, drawn as the heart is: a dark disc that reads on
            // any picture.
            Rectangle {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                implicitWidth: 26
                implicitHeight: 26
                radius: width / 2
                color: shut.containsMouse ? "#cc000000" : "#99000000"
                Behavior on color { ColorAnimation { duration: 120 } }

                IconImage {
                    anchors.centerIn: parent
                    width: 14
                    height: 14
                    name: "window-close"
                    color: "#ffffff"
                    scale: shut.pressed ? 0.85 : 1
                    Behavior on scale { NumberAnimation { duration: 90 } }
                }

                MouseArea {
                    id: shut

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.closeAsked()
                }

                ToolTip.visible: shut.containsMouse
                ToolTip.delay: 600
                ToolTip.text: qsTr("Close")
            }
        }

        Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: panel.title
            textFormat: Text.PlainText   // off the workshop: never HTML
            font.pixelSize: 15
            font.weight: Font.Bold
            wrapMode: Text.Wrap
            maximumLineCount: 3
            elide: Text.ElideRight
        }
    }
}
