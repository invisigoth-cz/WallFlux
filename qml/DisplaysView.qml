// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Window
import WallFlux

/// The Monitors tab, as Wallpaper Engine's Displays: the layout -- a
/// wallpaper per monitor, or one cloned onto all -- the monitors drawn as
/// they are arranged, each with a menu on a right click, and profiles.
///
/// The screens are Qt's own -- `Application.screens` -- so there is no
/// `xrandr`, no `kscreen-doctor` and nobody's output to parse. Qt names a
/// screen after its connector, which is the same name KDE's display
/// settings show and the same one under /sys/class/drm.
Item {
    id: view

    /// Main.qml's root: the displays and what can be done with them.
    required property var stage

    /// The picker: the layout and what is remembered per monitor.
    required property var options

    /// "Change wallpaper" on a monitor: the library, with it chosen.
    signal changeAsked(string name)

    readonly property bool cloning: view.options.layout === "clone"

    /// What a monitor shows, in words, for its row.
    function showsOn(name, displays, excluded) {
        void excluded;
        const d = view.stage.displayOn(name, displays);
        if (!d)
            return view.cloning ? qsTr("Left out of the clone") : qsTr("Nothing");
        return d.showing.current.id !== "" ? d.showing.current.title : qsTr("Nothing");
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 24
        anchors.rightMargin: 24
        anchors.bottomMargin: 12
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            // A layout is a choice between monitors: with one there is
            // nothing to choose, and it goes the moment the other is
            // unplugged.
            Label {
                text: qsTr("Layout")
                visible: layoutBox.visible
            }

            // The same buttons as Settings' choices, lit the same way and
            // with the same colours from whichever theme is on.
            Row {
                id: layoutBox

                /// What each is called in the settings, never translated.
                readonly property var words: ["perDisplay", "clone"]

                visible: Application.screens.length > 1
                spacing: 6

                Repeater {
                    model: layoutBox.words

                    delegate: RoundedButton {
                        id: layoutOption

                        required property string modelData
                        required property int index
                        readonly property bool lit: view.options.layout === layoutOption.modelData

                        text: view.layoutName(layoutOption.index, Language.czech)
                        accent: layoutOption.lit ? view.palette.highlight : "transparent"
                        palette.buttonText: layoutOption.lit ? view.ink(view.palette.highlight)
                                                             : view.palette.buttonText
                        onClicked: view.stage.setLayout(layoutOption.modelData)
                    }
                }
            }

            Item { Layout.fillWidth: true }

            RoundedButton {
                text: qsTr("Load profile")
                icon.name: "document-open"
                enabled: Profiles.all.length > 0
                onClicked: loadingProfile.open()
            }

            RoundedButton {
                text: qsTr("Save profile")
                icon.name: "document-save"
                onClicked: savingProfile.open()
            }
        }

        DisplayMap {
            id: map

            Layout.fillWidth: true
            Layout.preferredHeight: 220
            stage: view.stage
            // The monitor the library acts on, which is the chosen one only
            // while it is plugged in.
            selectedName: view.options.selected ? view.options.selected.key : ""
            onChosen: (name) => view.options.selectedKey = name
            onContextAsked: (name, x, y) => {
                monitorMenu.name = name;
                monitorMenu.popup(map, x, y);
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font.pixelSize: 11
            opacity: 0.8
            text: qsTr("Click a monitor to choose it; right-click it for more.")
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: rows.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: rows

                width: parent.width - 18
                spacing: 8

                Repeater {
                    model: Application.screens

                    delegate: Rectangle {
                        id: row

                        required property var modelData

                        readonly property bool chosen: map.selectedName === row.modelData.name

                        Layout.fillWidth: true
                        implicitHeight: 62
                        radius: 8
                        color: row.palette.base
                        border.width: row.chosen ? 2 : 1
                        border.color: row.chosen ? row.palette.highlight : row.palette.mid

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.PointingHandCursor
                            onClicked: (mouse) => {
                                if (mouse.button === Qt.RightButton) {
                                    monitorMenu.name = row.modelData.name;
                                    monitorMenu.popup();
                                } else {
                                    view.options.selectedKey = row.modelData.name;
                                }
                            }
                        }

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 2

                            Label {
                                // Numbered only where there is more than one,
                                // as the tray and the library do.
                                // Nothing while the row goes with an unplugged
                                // monitor, whose screen is already gone.
                                text: !row.modelData || !row.modelData.name ? ""
                                      : Application.screens.length > 1
                                      ? view.stage.monitorLabel(row.modelData.name, view.stage.connectedNames)
                                      : row.modelData.name
                                font.pixelSize: 15
                                font.weight: Font.Bold
                            }

                            Label {
                                font.pixelSize: 12
                                opacity: 0.75
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                // What is on it, how big it is and where.
                                text: !row.modelData || !row.modelData.name ? "" : view.showsOn(row.modelData.name, view.stage.displays,
                                                   view.options.excludedFromClone)
                                      + "  ·  " + row.modelData.width + " × " + row.modelData.height
                                      + "  ·  " + row.modelData.name
                            }
                        }
                    }
                }
            }
        }
    }

    /// Black or white, whichever reads on this colour -- as Settings' choices.
    function ink(c) {
        return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) > 0.7 ? "#000000" : "#ffffff";
    }

    function layoutName(index, language) {
        void language;
        return index === 1 ? qsTr("Clone one wallpaper") : qsTr("Wallpaper per monitor");
    }

    // ---- One monitor's menu ------------------------------------------------

    Menu {
        id: monitorMenu

        /// The monitor it is about, by connector name.
        property string name: ""

        readonly property var shown: view.stage.displayOn(monitorMenu.name, view.stage.displays)
        readonly property bool left: view.cloning && view.options.excludedFromClone.indexOf(monitorMenu.name) >= 0
        readonly property bool isSource: view.cloning && view.stage.cloneSourceName() === monitorMenu.name
        /// Which display's sound this monitor's menu turns off: its own, or
        /// the clone's -- which only its source offers.
        readonly property string soundKey: view.cloning ? "clone" : monitorMenu.name
        readonly property bool muted: view.stage.isMuted(monitorMenu.soundKey, view.stage.mutedKeys)
        readonly property var others: Application.screens.map(s => s.name).filter(n => n !== monitorMenu.name)

        // Said out loud, because a Menu takes its width from its background
        // and a plain Rectangle has none (see CLAUDE.md).
        implicitWidth: 260

        MenuItem {
            text: qsTr("Change wallpaper")
            onTriggered: {
                if (monitorMenu.left)
                    view.stage.includeInClone(monitorMenu.name);
                view.changeAsked(monitorMenu.name);
            }
        }

        MenuItem {
            text: qsTr("Remove wallpaper")
            enabled: !monitorMenu.left && !!monitorMenu.shown
            onTriggered: view.stage.removeFrom(monitorMenu.name)
        }

        Menu {
            title: qsTr("Copy to")
            enabled: !view.cloning && monitorMenu.others.length > 0
            implicitWidth: 260

            Repeater {
                model: monitorMenu.others

                delegate: MenuItem {
                    required property string modelData
                    text: view.stage.monitorLabel(modelData, view.stage.connectedNames)
                    onTriggered: view.stage.copyTo(monitorMenu.name, modelData)
                }
            }
        }

        Menu {
            title: qsTr("Swap with")
            enabled: !view.cloning && monitorMenu.others.length > 0
            implicitWidth: 260

            Repeater {
                model: monitorMenu.others

                delegate: MenuItem {
                    required property string modelData
                    text: view.stage.monitorLabel(modelData, view.stage.connectedNames)
                    onTriggered: view.stage.swap(monitorMenu.name, modelData)
                }
            }
        }

        MenuItem {
            text: monitorMenu.muted ? qsTr("Unmute") : qsTr("Mute")
            // Cloned, there is one sound, and it is the source's to turn off.
            enabled: !view.cloning || monitorMenu.isSource
            onTriggered: view.stage.setMuted(monitorMenu.soundKey, !monitorMenu.muted)
        }

        MenuSeparator { visible: view.cloning }

        MenuItem {
            visible: view.cloning
            height: visible ? implicitHeight : 0
            text: qsTr("Make it the clone's source")
            enabled: !monitorMenu.isSource && !monitorMenu.left
            onTriggered: view.options.cloneSource = monitorMenu.name
        }

        MenuItem {
            readonly property bool flipped: view.options.flippedMonitors.indexOf(monitorMenu.name) >= 0

            visible: view.cloning
            height: visible ? implicitHeight : 0
            text: flipped ? qsTr("Unflip") : qsTr("Flip")
            // The source is what the others are copies of, and a copy is
            // what turns over -- as in Wallpaper Engine.
            enabled: !monitorMenu.isSource && !monitorMenu.left
            onTriggered: {
                const rest = view.options.flippedMonitors.filter(n => n !== monitorMenu.name);
                view.options.flippedMonitors = flipped ? rest : rest.concat([monitorMenu.name]);
            }
        }
    }

    // ---- Profiles ------------------------------------------------------------

    Popup {
        id: savingProfile

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: 18
        implicitWidth: 460
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        /// The monitors this profile keeps, with a wallpaper on each: the
        /// chosen one ticked to begin with, the others for the asking. Only
        /// these change when it is loaded. Cloned, there is one picture and
        /// nothing to pick.
        property var keys: []
        readonly property bool picking: view.stage.perDisplay && Application.screens.length > 1

        onOpened: {
            profileName.text = "";
            const names = Application.screens.map(s => s.name);
            savingProfile.keys = [names.indexOf(view.options.selectedKey) >= 0 ? view.options.selectedKey
                                                                                : names[0]];
            profileName.forceActiveFocus();
        }

        function keep() {
            if (savingProfile.picking && savingProfile.keys.length === 0)
                return;
            const name = profileName.text.trim() !== "" ? profileName.text.trim()
                                                         : profileName.placeholderText;
            Profiles.save(name, view.stage.arrangement(savingProfile.picking ? savingProfile.keys : undefined));
            savingProfile.close();
        }

        background: Rectangle {
            color: view.palette.base
            border.color: view.palette.mid
            border.width: 1
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16

            Label {
                text: qsTr("Save a profile")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                font.pixelSize: 12
                opacity: 0.85
                text: savingProfile.picking
                      ? qsTr("What the ticked monitors show, their playlists, their sound and the "
                             + "wallpapers' settings, kept under a name to load again from here or the "
                             + "tray. Loading it changes those monitors and leaves the others as they are.")
                      : qsTr("What every monitor shows, its playlist, its sound and the wallpaper's "
                             + "settings, kept under a name to load again from here or the tray.")
            }

            ColumnLayout {
                visible: savingProfile.picking
                spacing: 0

                Repeater {
                    model: Application.screens

                    CheckBox {
                        id: keeps

                        required property var modelData

                        text: view.stage.monitorLabel(modelData.name, view.stage.connectedNames)
                        checked: savingProfile.keys.indexOf(modelData.name) >= 0
                        onToggled: {
                            savingProfile.keys = keeps.checked
                                ? savingProfile.keys.concat([keeps.modelData.name])
                                : savingProfile.keys.filter(k => k !== keeps.modelData.name);
                            // A click breaks the binding it was reading (see
                            // the trap); the keys say again what is ticked,
                            // the next opening included.
                            keeps.checked = Qt.binding(() => savingProfile.keys.indexOf(keeps.modelData.name) >= 0);
                        }
                    }
                }
            }

            TextField {
                id: profileName

                Layout.fillWidth: true
                placeholderText: qsTr("Profile %1").arg(Profiles.all.length + 1)
                maximumLength: 50
                onAccepted: savingProfile.keep()
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                font.pixelSize: 11
                opacity: 0.8
                text: qsTr("If a profile of this name exists already, it will be replaced.")
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Confirm")
                    accent: Look.positive
                    enabled: !savingProfile.picking || savingProfile.keys.length > 0
                    onClicked: savingProfile.keep()
                }

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: savingProfile.close()
                }
            }
        }
    }

    Popup {
        id: loadingProfile

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: 18
        // As wide as the playlists' own, and held there: at 460 the row of
        // buttons ran out past the edge once Delete asked "Really delete?".
        width: 560
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        // Escape reaches a popup only where it has the keyboard.
        focus: true

        /// The name picked in the list, or empty while none is.
        property string picked: ""
        property bool renaming: false

        onPickedChanged: loadingProfile.renaming = false
        onClosed: loadingProfile.renaming = false

        function load(name) {
            view.stage.applyProfile(Profiles.named(name));
            loadingProfile.close();
        }

        background: Rectangle {
            color: view.palette.base
            border.color: view.palette.mid
            border.width: 1
            radius: 14
        }

        contentItem: ColumnLayout {
            spacing: 16

            Label {
                text: qsTr("Load a profile")
                font.pixelSize: 17
                font.weight: Font.Bold
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 200
                color: "transparent"
                border.color: view.palette.mid
                border.width: 1
                radius: 4

                ListView {
                    anchors.fill: parent
                    anchors.margins: 1
                    clip: true
                    model: Profiles.all
                    ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

                    delegate: Rectangle {
                        id: profileRow

                        required property var modelData

                        readonly property bool picked: loadingProfile.picked === profileRow.modelData.name

                        width: ListView.view.width
                        height: 34
                        color: profileRow.picked ? view.palette.highlight
                             : rowPointer.hovered ? Qt.alpha(view.palette.highlight, 0.18) : "transparent"

                        HoverHandler { id: rowPointer }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: loadingProfile.picked = profileRow.modelData.name
                            onDoubleClicked: loadingProfile.load(profileRow.modelData.name)
                        }

                        Label {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: 10
                            visible: !newProfileName.visible
                            text: profileRow.modelData.name
                            color: profileRow.picked ? view.palette.highlightedText : view.palette.windowText
                        }

                        TextField {
                            id: newProfileName

                            property bool refused: false

                            anchors.fill: parent
                            anchors.margins: 2
                            visible: loadingProfile.renaming && profileRow.picked
                            onVisibleChanged: {
                                if (!visible)
                                    return;
                                text = profileRow.modelData.name;
                                refused = false;
                                forceActiveFocus();
                                selectAll();
                            }
                            onTextEdited: refused = false
                            onAccepted: {
                                if (Profiles.rename(profileRow.modelData.name, text)) {
                                    loadingProfile.picked = text.trim();
                                    loadingProfile.renaming = false;
                                } else {
                                    refused = true;
                                }
                            }
                            Keys.onEscapePressed: event => {
                                loadingProfile.renaming = false;
                                loadingProfile.forceActiveFocus();
                            }
                            background: Rectangle {
                                radius: 4
                                color: view.palette.base
                                border.width: newProfileName.refused ? 2 : 1
                                border.color: newProfileName.refused ? Look.negative : view.palette.mid
                            }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                // Asks on the button itself, as a playlist's Delete does:
                // a profile thrown away cannot be had back.
                RoundedButton {
                    id: binProfile

                    property bool asking: false

                    text: binProfile.asking ? qsTr("Really delete?") : qsTr("Delete")
                    icon.name: "edit-delete"
                    accent: Look.negative
                    enabled: Profiles.named(loadingProfile.picked).name !== undefined
                    onClicked: {
                        if (!binProfile.asking) {
                            binProfile.asking = true;
                            return;
                        }
                        Profiles.forget(loadingProfile.picked);
                        loadingProfile.picked = "";
                        binProfile.asking = false;
                    }

                    Connections {
                        target: loadingProfile
                        function onPickedChanged() { binProfile.asking = false; }
                        function onClosed() { binProfile.asking = false; }
                    }
                }

                RoundedButton {
                    text: qsTr("Rename")
                    icon.name: "edit-rename"
                    enabled: !loadingProfile.renaming
                             && Profiles.named(loadingProfile.picked).name !== undefined
                    onClicked: loadingProfile.renaming = true
                }

                Item { Layout.fillWidth: true }

                RoundedButton {
                    text: qsTr("Load")
                    accent: Look.positive
                    enabled: Profiles.named(loadingProfile.picked).name !== undefined
                    onClicked: loadingProfile.load(loadingProfile.picked)
                }

                RoundedButton {
                    text: qsTr("Cancel")
                    onClicked: loadingProfile.close()
                }
            }
        }
    }
}
