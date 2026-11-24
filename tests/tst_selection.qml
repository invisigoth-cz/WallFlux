// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

import QtQuick
import QtTest
import "../qml/selection.js" as Selection

/// Choosing several wallpapers in the library, as Wallpaper Engine does:
/// Ctrl adds or takes away one, Shift takes everything between, and a band
/// dragged over the grid takes every card it touches.
TestCase {
    name: "selection"

    readonly property var order: ["a", "b", "c", "d", "e", "f", "g"]

    function test_ctrl_adds_and_takes_away() {
        compare(Selection.toggled([], "c"), ["c"]);
        compare(Selection.toggled(["a", "c"], "c"), ["a"]);
        compare(Selection.toggled(["a"], "b"), ["a", "b"]);
    }

    function test_shift_takes_everything_between_either_way_round() {
        compare(Selection.between(order, "b", "e"), ["b", "c", "d", "e"]);
        compare(Selection.between(order, "e", "b"), ["b", "c", "d", "e"]);
        compare(Selection.between(order, "d", "d"), ["d"]);
    }

    function test_shift_without_a_start_takes_the_one() {
        compare(Selection.between(order, "", "c"), ["c"]);
        compare(Selection.between(order, "gone", "c"), ["c"]);
    }

    function test_a_band_takes_every_card_it_touches() {
        // Three columns of 100 by 120: cards 0 1 2 / 3 4 5 / 6.
        compare(Selection.inBand(7, 3, 100, 120, Qt.rect(50, 60, 100, 100)), [0, 1, 3, 4]);
        // Drawn up and to the left counts the same.
        compare(Selection.inBand(7, 3, 100, 120, Qt.rect(150, 160, -100, -100)), [0, 1, 3, 4]);
        // Past the last card there is nothing.
        compare(Selection.inBand(7, 3, 100, 120, Qt.rect(110, 250, 150, 50)), []);
        compare(Selection.inBand(7, 3, 100, 120, Qt.rect(10, 250, 20, 20)), [6]);
    }
}
