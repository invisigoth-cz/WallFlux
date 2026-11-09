// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import WallFlux

/// How WallFlux behaves: the Settings tab.
///
/// Sections down the left, as linux-wallpaper-engine-ux has them, and one
/// row to a setting on the right. Every row does what it says now -- a
/// switch that waits for its feature is a promise, not a setting -- and
/// every value lives on the picker, which remembers it between runs.
///
/// Not here on purpose: a wallpaper's own properties (they belong beside
/// it, in the details panel), and pausing or quietening the wallpaper under
/// a window or another program's sound, which WallFlux does by itself.
Item {
    id: view

    required property var picker

    /// Which section is open, by its place in `sections`.
    property int section: 0

    readonly property var sections: [
        { name: qsTr("Appearance"), icon: "preferences-desktop-theme" },
        { name: qsTr("System"), icon: "preferences-system-performance" },
        { name: qsTr("General"), icon: "preferences-system" }
    ]

    /// Wallpaper Engine's presets, by the pictures a second each draws.
    readonly property var rates: [
        { fps: 10, name: qsTr("Low") },
        { fps: 15, name: qsTr("Medium") },
        { fps: 25, name: qsTr("High") },
        { fps: 30, name: qsTr("Ultra") }
    ]

    function resetToDefaults() {
        view.picker.dark = true;
        view.picker.tint = true;
        view.picker.fps = 15;
        view.picker.music = true;
        view.picker.mouse = true;
        view.picker.parallax = true;
        view.picker.particles = true;
        view.picker.muted = false;
        view.picker.volume = 1;
    }

    /// One setting: what it is called, a line saying what it does, and the
    /// control at the right. Not named `Row`, which is QtQuick's own.
    component Setting: Rectangle {
        id: setting

        property string title
        property string about
        // The control goes straight into the line, after the words, which
        // fill what is left and so push it to the right edge.
        default property alias control: line.data

        Layout.fillWidth: true
        implicitHeight: Math.max(64, words.implicitHeight + 24)
        radius: 8
        color: setting.palette.base
        border.width: 1
        border.color: setting.palette.mid

        RowLayout {
            id: line

            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            spacing: 16

            ColumnLayout {
                id: words

                Layout.fillWidth: true
                spacing: 2

                // Filling, or a row with no line under its title would leave
                // the space to be shared out round the control instead.
                Label {
                    Layout.fillWidth: true
                    text: setting.title
                    font.pixelSize: 15
                    font.weight: Font.Bold
                }

                Label {
                    Layout.fillWidth: true
                    visible: setting.about !== ""
                    text: setting.about
                    wrapMode: Text.Wrap
                    font.pixelSize: 12
                    opacity: 0.75
                }
            }

        }
    }

    /// A switch that shows a value and sets it. Clicking a Switch sets its
    /// own `checked`, which breaks a binding on it, so the binding is put
    /// back after every click: a reset to the defaults has to show.
    component Toggle: Switch {
        id: toggle

        property bool value
        signal changed(bool to)

        checked: toggle.value
        onToggled: {
            toggle.changed(toggle.checked);
            toggle.checked = Qt.binding(() => toggle.value);
        }
    }

    /// One of a few choices, as buttons side by side; the chosen one lit.
    component Choice: RowLayout {
        id: choice

        property var options: []
        property var current
        signal chosen(var value)

        spacing: 6

        Repeater {
            model: choice.options

            delegate: RoundedButton {
                required property var modelData

                text: modelData.name
                accent: modelData.value === choice.current ? choice.palette.highlight : "transparent"
                onClicked: choice.chosen(modelData.value)
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 24
        anchors.rightMargin: 24
        anchors.bottomMargin: 12
        spacing: 20

        // The sections. More will come; each is one entry in `sections` and
        // one page in the stack beside it.
        // Held to its width: the entries fill it, and a filling child would
        // otherwise make the whole column fill the row.
        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            Layout.fillWidth: false
            Layout.preferredWidth: 190
            Layout.maximumWidth: 190
            spacing: 4

            Repeater {
                model: view.sections

                delegate: ItemDelegate {
                    id: entry

                    required property var modelData
                    required property int index

                    Layout.fillWidth: true
                    text: entry.modelData.name
                    icon.name: entry.modelData.icon
                    icon.color: entry.palette.windowText
                    highlighted: view.section === entry.index
                    onClicked: view.section = entry.index

                    background: Rectangle {
                        radius: 8
                        color: entry.highlighted ? Qt.alpha(entry.palette.highlight, 0.28)
                               : entry.hovered ? Qt.alpha(entry.palette.windowText, 0.08)
                               : "transparent"
                    }
                }
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true

            contentWidth: width
            contentHeight: pages.implicitHeight
            clip: true
            ScrollBar.vertical: Scroller { policy: ScrollBar.AsNeeded }

            StackLayout {
                id: pages

                width: parent.width - 18
                currentIndex: view.section

                // Appearance.
                ColumnLayout {
                    spacing: 8

                    Setting {
                        title: qsTr("Dark theme")
                        about: qsTr("Light text on dark colours. Off, the window is light.")

                        Toggle {
                            value: view.picker.dark
                            onChanged: (to) => view.picker.dark = to
                        }
                    }

                    Setting {
                        title: qsTr("Tint by the wallpaper")
                        about: qsTr("The window takes on the colours of the wallpaper whose details are open.")

                        Toggle {
                            value: view.picker.tint
                            onChanged: (to) => view.picker.tint = to
                        }
                    }
                }

                // System: what the wallpaper asks of the computer.
                ColumnLayout {
                    spacing: 8

                    Setting {
                        title: qsTr("Frame rate")
                        about: qsTr("Pictures a second, for scenes and web wallpapers. Wallpaper Engine's "
                                    + "presets: 10, 15, 25 and 30. More is smoother and costs more; "
                                    + "a heavy scene that cannot keep up simply draws fewer.")

                        Choice {
                            options: view.rates.map((r) => ({ name: r.name + " · " + r.fps, value: r.fps }))
                            current: view.picker.fps
                            onChosen: (value) => view.picker.fps = value
                        }
                    }

                    Setting {
                        title: qsTr("Follow the music")
                        about: qsTr("Wallpapers that answer to what the speakers play. Off, nothing "
                                    + "listens, and Plasma shows no microphone icon for it.")

                        Toggle {
                            value: view.picker.music
                            onChanged: (to) => view.picker.music = to
                        }
                    }

                    Setting {
                        title: qsTr("Follow the mouse")
                        about: qsTr("Particles that move away from the pointer, light that goes where it goes.")

                        Toggle {
                            value: view.picker.mouse
                            onChanged: (to) => view.picker.mouse = to
                        }
                    }

                    Setting {
                        title: qsTr("Parallax")
                        about: qsTr("Layers of a scene that lean as the mouse moves, for depth.")

                        Toggle {
                            value: view.picker.parallax
                            onChanged: (to) => view.picker.parallax = to
                        }
                    }

                    Setting {
                        title: qsTr("Particles")
                        about: qsTr("Smoke, sparks, snow and the like in scenes. They are most of what "
                                    + "a scene costs; off, they are neither worked out nor drawn.")

                        Toggle {
                            value: view.picker.particles
                            onChanged: (to) => view.picker.particles = to
                        }
                    }
                }

                // General.
                ColumnLayout {
                    spacing: 8

                    Setting {
                        title: qsTr("Language")
                        about: ""

                        Choice {
                            options: [{ name: "English", value: false }, { name: "Čeština", value: true }]
                            current: Language.czech
                            onChosen: (value) => Language.czech = value
                        }
                    }

                    Setting {
                        title: qsTr("Wallpaper sound")
                        about: qsTr("A wallpaper's own sound. It goes quiet by itself while another "
                                    + "program plays or a window covers the screen.")

                        Toggle {
                            value: !view.picker.muted
                            onChanged: (to) => view.picker.muted = !to
                        }
                    }

                    Setting {
                        title: qsTr("Volume")
                        about: ""

                        Slider {
                            implicitWidth: 220
                            from: 0
                            to: 1
                            value: view.picker.volume
                            enabled: !view.picker.muted
                            onMoved: view.picker.volume = value
                        }
                    }

                    Setting {
                        title: qsTr("Start when I log in")
                        about: qsTr("As a tray icon. The desktop keeps its own wallpaper until one is chosen.")

                        Toggle {
                            value: Autostart.enabled
                            onChanged: (to) => Autostart.enabled = to
                        }
                    }

                    RowLayout {
                        Layout.topMargin: 8

                        RoundedButton {
                            text: qsTr("Reset to defaults")
                            onClicked: view.resetToDefaults()
                        }

                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                            font.pixelSize: 11
                            opacity: 0.75
                            text: qsTr("Every setting above and in the other sections, except the "
                                       + "language and starting at login.")
                        }
                    }
                }
            }
        }
    }
}
