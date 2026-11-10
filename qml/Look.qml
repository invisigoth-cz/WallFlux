// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

pragma Singleton

import QtQuick

/// What a theme is besides its palette: the colours a palette has no role
/// for, and how the card the keyboard is on is marked. The user's word of
/// 10 November: every theme its own, so that nothing in the window stays
/// Breeze's green and blue under Steam's colours, and each picks a card out
/// in a way of its own.
///
/// The picker owns the theme and the accent and hands both in; anything in
/// the window reads them from here, the Steam tab included, which cannot
/// see the picker.
QtObject {
    /// Set by the picker.
    property string theme: "dark"
    property color highlight: "#3daee9"

    /// Per theme:
    /// - `positive` and `negative`: Download, Save, Confirm and Remove.
    /// - `playable` and `listed`: the band round a card that plays, and
    ///   round one that is only listed.
    /// - `tags`: a tag's pill -- `soft` grey, `tinted` in the accent, or
    ///   `outline` for the high contrast pair.
    /// - `ring`: how the card the keyboard is on stands out --
    ///   `glow` a halo in the accent; `lift` raised on a shadow, for a light
    ///   window where a glow is lost; `outline` KDE's focus ring, a gap and a
    ///   line; `focus` Steam's Big Picture, grown and lit; `double` a thick
    ///   band and a second line, hard edged.
    readonly property var styles: ({
        dark: { positive: "#27ae60", negative: "#da4453", playable: "#27ae60",
                listed: "#f67400", tags: "soft", ring: "glow" },
        light: { positive: "#27ae60", negative: "#da4453", playable: "#27ae60",
                 listed: "#f67400", tags: "tinted", ring: "lift" },
        system: { positive: "#27ae60", negative: "#da4453", playable: "#27ae60",
                  listed: "#f67400", tags: "soft", ring: "outline" },
        steam: { positive: "#5c7e10", negative: "#a33a35", playable: "#4c6b22",
                 listed: "#a0742a", tags: "tinted", ring: "focus" },
        hardLight: { positive: "#006b2e", negative: "#b00020", playable: "#006b2e",
                     listed: "#8a4600", tags: "outline", ring: "double" },
        hardDark: { positive: "#00e070", negative: "#ff4d4d", playable: "#00e070",
                    listed: "#ffaa00", tags: "outline", ring: "double" }
    })
    readonly property var current: styles[theme] || styles.dark

    readonly property color positive: current.positive
    readonly property color negative: current.negative
    readonly property color playable: current.playable
    readonly property color listed: current.listed
    readonly property string tags: current.tags
    readonly property string ring: current.ring

    /// Black or white, whichever reads on `c`.
    function ink(c) {
        return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) > 0.6 ? "#000000" : "#ffffff";
    }
}
