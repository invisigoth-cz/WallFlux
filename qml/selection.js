// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

/// Choosing several wallpapers in the library, as Wallpaper Engine does:
/// Ctrl adds or takes away one, Shift takes everything between, a band
/// dragged over the grid takes every card it touches. Plain functions of
/// lists, so a test can reach them; `tests/tst_selection.qml`.

.pragma library

/// The ids with this one added, or taken away where it was there.
function toggled(ids, id) {
    return ids.indexOf(id) >= 0 ? ids.filter(one => one !== id) : ids.concat([id]);
}

/// Every id from `from` to `to` in the grid's order, either way round. With
/// no start, or one no longer in the grid, just `to`.
function between(order, from, to) {
    const a = order.indexOf(from);
    const b = order.indexOf(to);
    if (b < 0)
        return [];
    if (a < 0)
        return [to];
    return order.slice(Math.min(a, b), Math.max(a, b) + 1);
}

/// The indices of the cards a band touches, in a grid of `count` cards laid
/// out `columns` across in cells of `cellWidth` by `cellHeight`, the band in
/// the grid's own coordinates and drawn any way round.
function inBand(count, columns, cellWidth, cellHeight, band) {
    const left = Math.min(band.x, band.x + band.width);
    const right = Math.max(band.x, band.x + band.width);
    const top = Math.min(band.y, band.y + band.height);
    const bottom = Math.max(band.y, band.y + band.height);
    const touched = [];
    for (let i = 0; i < count; ++i) {
        const x = (i % columns) * cellWidth;
        const y = Math.floor(i / columns) * cellHeight;
        if (x < right && x + cellWidth > left && y < bottom && y + cellHeight > top)
            touched.push(i);
    }
    return touched;
}
