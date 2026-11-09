// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Controls.impl
import QtQuick.Layouts

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
    /// Whatever goes under the author, scrolled with the list: the
    /// wallpaper's own settings, in the library -- what can be changed
    /// before what is merely so.
    property alias below: under.data

    /// Asked for when somebody wants the rest of this author's work.
    signal authorAsked()

    /// Whether the panel is out.
    ///
    /// It slides rather than appearing: the width is what moves, and the
    /// grid beside it takes the room back as it goes. There is no cross
    /// on the picture any more -- clicking the card the panel is about
    /// is what puts it away, which is one fewer thing to aim at.
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

        PreviewTile {
            // Narrower than the panel, and centred.
            //
            // Sixteen by nine, where the grid's cards are squares: this
            // is the one place a whole preview is looked at, and a square
            // here would crop a third of it off for nothing. A tile takes
            // the shape it is given, so both of these are read.
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(column.width, 280)
            Layout.preferredHeight: Math.round(Math.min(column.width, 280) * 9 / 16)
            preview: panel.preview
            // The name goes under this in its own type, where it can wrap.
            label: ""
            clickable: false
        }

        Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: panel.title
            font.pixelSize: 15
            font.weight: Font.Bold
            wrapMode: Text.Wrap
            maximumLineCount: 3
            elide: Text.ElideRight
        }

        // The picture and the name stay put; everything under them scrolls.
        //
        // The picture used to scroll with the rest, which in a window of
        // any height meant a scrollbar: a hundred and sixty pixels of
        // photograph is most of what the panel has, so the content was
        // always taller than the room. What is worth scrolling is the list,
        // and the author sitting over it.
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            // A clear step down the page from the name to the small print,
            // on top of the column's own spacing.
            Layout.topMargin: 16

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

                ColumnLayout {
                    id: under

                    Layout.fillWidth: true
                    spacing: 0
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
                                text: fact.modelData.label
                                opacity: 0.7
                            }

                            Item { Layout.fillWidth: true }

                            Label {
                                Layout.maximumWidth: told.width * 0.55
                                horizontalAlignment: Text.AlignRight
                                text: fact.modelData.value
                                wrapMode: Text.Wrap
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
                            radius: 5
                            color: Qt.alpha(pill.palette.mid, 0.5)

                            Label {
                                id: name

                                anchors.centerIn: parent
                                text: pill.modelData
                                font.pixelSize: 12
                            }
                        }
                    }
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
}
