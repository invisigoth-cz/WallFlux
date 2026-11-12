pragma ComponentBehavior: Bound

// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import WallFlux

/// A wallpaper's own settings, as its author declared them in project.json:
/// switches, sliders, colours, drop downs, a line of text, a file. What is
/// set here is laid over what the file says, and only that is remembered.
///
/// A setting may say when it is shown -- `condition`, a line of JavaScript
/// such as `showDate.value == true` -- and is worked out against the values
/// as they stand, so ticking one box brings out the settings under it, as
/// Wallpaper Engine does.
///
/// Every row holds one of each control and shows the one its kind wants.
/// Loaders would make only that one, but a component inside a Loader reads
/// the row through a context qmllint cannot follow, and a few dozen hidden
/// controls cost nothing.
ColumnLayout {
    id: panel

    /// The wallpaper, or null.
    property var wallpaper: null
    /// What has been set for it, by name.
    property var values: ({})
    /// The language Wallpaper Engine's own words are read in.
    property string catalogue: "en-us"

    /// `set` is a word QML keeps for itself, hence this one.
    signal chosen(string name, var value)
    signal restored()

    /// What project.json declares, preset values included.
    readonly property var declared: {
        try {
            return panel.wallpaper && panel.wallpaper.properties
                ? JSON.parse(panel.wallpaper.properties) : {};
        } catch (e) {
            return {};
        }
    }

    /// The settings in the author's order.
    readonly property var entries: Object.keys(panel.declared)
        .map(name => Object.assign({ name: name }, panel.declared[name]))
        .sort((a, b) => (a.order || 0) - (b.order || 0))

    function now(name) {
        const set = panel.values[name];
        return set !== undefined ? set : (panel.declared[name] || {}).value;
    }

    /// Whether a setting's condition holds. Read in C++, never run: the
    /// condition comes off the workshop, and run here it could do anything
    /// this window can. One that cannot be read is shown rather than lost.
    function shown(entry) {
        const values = {};
        for (const name of Object.keys(panel.declared))
            values[name] = panel.now(name);
        return Wallpapers.holds(String(entry.condition || ""), values);
    }

    /// Authors write labels as HTML with line breaks around them; the
    /// breaks at either end only push the words away from the control.
    function label(text) {
        return Tags.phrase(String(text || ""), panel.catalogue)
            .replace(/<img\b[^>]*>/gi, "")
            .replace(/^(\s|<br\s*\/?>)+|(\s|<br\s*\/?>)+$/gi, "");
    }

    /// Wallpaper Engine's colours are three numbers from nought to one.
    function colour(text) {
        const p = String(text || "1 1 1").split(/\s+/).map(Number);
        return Qt.rgba(p[0] || 0, p[1] || 0, p[2] || 0, 1);
    }
    function asText(c) {
        return [c.r, c.g, c.b].map(x => x.toFixed(5)).join(" ");
    }

    /// A path as Wallpaper Engine hands a page one, with no leading slash:
    /// pages put `file:///` in front of it.
    function asPath(url) {
        return decodeURIComponent(url.toString().replace(/^file:\/\/\//, ""));
    }

    spacing: 10
    visible: panel.entries.length > 0

    RowLayout {
        Layout.fillWidth: true

        Label {
            Layout.fillWidth: true
            text: qsTr("Properties")
            font.weight: Font.Bold
            font.capitalization: Font.AllUppercase
            opacity: 0.6
        }

        RoundedButton {
            text: qsTr("Defaults")
            flat: true
            visible: Object.keys(panel.values).length > 0
            onClicked: panel.restored()
        }
    }

    Repeater {
        model: panel.entries

        delegate: ColumnLayout {
            id: row

            required property var modelData
            readonly property string kind: row.modelData.type || ""
            readonly property var held: panel.now(row.modelData.name)

            Layout.fillWidth: true
            spacing: 4
            visible: panel.shown(row.modelData)

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    Layout.fillWidth: true
                    text: panel.label(row.modelData.text)
                    textFormat: Text.StyledText
                    wrapMode: Text.Wrap
                    opacity: row.kind === "text" ? 0.8 : 1
                }

                // Those that fit beside their name.
                Toggle {
                    visible: row.kind === "bool"
                    value: row.held === true
                    onChanged: (to) => panel.chosen(row.modelData.name, to)
                }

                Rectangle {
                    visible: row.kind === "color"
                    implicitWidth: 44
                    implicitHeight: 24
                    radius: 6
                    color: row.kind === "color" ? panel.colour(row.held) : "transparent"
                    border.width: 1
                    border.color: row.palette.mid

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            picking.name = row.modelData.name;
                            picking.selectedColor = panel.colour(row.held);
                            picking.open();
                        }
                    }
                }

                RoundedButton {
                    visible: row.kind === "file" || row.kind === "directory"
                    text: qsTr("Choose…")
                    onClicked: {
                        if (row.kind === "file") {
                            choosingFile.name = row.modelData.name;
                            choosingFile.open();
                        } else {
                            choosingFolder.name = row.modelData.name;
                            choosingFolder.open();
                        }
                    }
                }
            }

            // Those that need the width.
            RowLayout {
                Layout.fillWidth: true
                visible: row.kind === "slider"
                spacing: 8

                RoundSlider {
                    id: slider

                    readonly property real at: Number(row.held) || 0

                    function keep() {
                        panel.chosen(row.modelData.name, slider.value);
                        slider.value = Qt.binding(() => slider.at);
                    }

                    Layout.fillWidth: true
                    from: row.modelData.min !== undefined ? row.modelData.min : 0
                    to: row.modelData.max !== undefined ? row.modelData.max : 100
                    stepSize: row.modelData.step || (row.modelData.fraction ? 0.01 : 1)
                    snapMode: Slider.SnapAlways
                    value: slider.at
                    // A scene is loaded again for each value, so one is sent
                    // when the handle is let go, not on the way.
                    onPressedChanged: if (!slider.pressed) slider.keep()
                    onMoved: if (!slider.pressed) slider.keep()
                }

                Label {
                    Layout.minimumWidth: 36
                    horizontalAlignment: Text.AlignRight
                    text: slider.value.toFixed(row.modelData.fraction
                              ? (row.modelData.precision !== undefined ? row.modelData.precision : 2) : 0)
                    font.features: { "tnum": 1 }
                }
            }

            RoundedCombo {
                id: combo

                readonly property var options: row.kind === "combo" ? (row.modelData.options || []) : []
                readonly property int at: combo.options.findIndex(o => String(o.value) === String(row.held))

                Layout.fillWidth: true
                visible: row.kind === "combo"
                model: combo.options.map(o => panel.label(o.label))
                currentIndex: Math.max(0, combo.at)
                onActivated: (index) => {
                    panel.chosen(row.modelData.name, combo.options[index].value);
                    combo.currentIndex = Qt.binding(() => Math.max(0, combo.at));
                }
            }

            TextField {
                id: field

                Layout.fillWidth: true
                visible: row.kind === "textinput"
                text: row.kind === "textinput" ? String(row.held || "") : ""
                onEditingFinished: {
                    panel.chosen(row.modelData.name, field.text);
                    field.text = Qt.binding(() => String(row.held || ""));
                }
            }
        }
    }

    ColorDialog {
        id: picking

        property string name

        onAccepted: panel.chosen(picking.name, panel.asText(picking.selectedColor))
    }

    FileDialog {
        id: choosingFile

        property string name

        onAccepted: panel.chosen(choosingFile.name, panel.asPath(choosingFile.selectedFile))
    }

    FolderDialog {
        id: choosingFolder

        property string name

        onAccepted: panel.chosen(choosingFolder.name, panel.asPath(choosingFolder.selectedFolder))
    }
}
