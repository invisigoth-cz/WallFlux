// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

.pragma library

// WallFlux's icon in the window's colours, for the tray: a tile in the
// theme's own shade with a W drawn in the accent, so that it sits with
// whichever theme -- or accent, or desktop colour scheme -- is chosen.
// The shape is icons/wallflux.svg's, which is what is installed for the
// application menu; change one, change the other.
//
// Handed back as a data URL, which the tray takes as it takes a file:
// measured, Plasma got the 64 by 64 picture through StatusNotifierItem.

function hex(c) {
    const two = v => ("0" + Math.round(Math.max(0, Math.min(1, v)) * 255).toString(16)).slice(-2);
    return "#" + two(c.r) + two(c.g) + two(c.b);
}

function mix(a, b, t) {
    return { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t };
}

/// The accent turned round the colour wheel by `turn` (0 to 1), as a
/// plain { r, g, b }.
function turned(c, turn) {
    const h = ((c.hslHue < 0 ? 0 : c.hslHue) + turn) % 1;
    const s = c.hslSaturation, l = c.hslLightness;
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    const f = t => {
        t = (t + 1) % 1;
        return t < 1 / 6 ? p + (q - p) * 6 * t : t < 1 / 2 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p;
    };
    return { r: f(h + 1 / 3), g: f(h), b: f(h - 1 / 3) };
}

/// `set` is one of the picker's themes ({ dark, window, base, highlight })
/// and `accent` the chosen colour, both as QML colours.
function url(set, accent) {
    const dark = set.dark;
    const ink = { r: accent.r, g: accent.g, b: accent.b };
    // The tile: the theme's own ground, a little of the accent in it, and
    // darker towards the bottom -- so it stands off a panel of either kind.
    const top = mix(set.base, ink, dark ? 0.16 : 0.08);
    const bottom = mix(set.window, { r: 0, g: 0, b: 0 }, dark ? 0.35 : 0.06);
    // The W: lighter than the accent where it starts, the accent itself in
    // the middle and turned towards violet at the end, as the default is.
    // On a light tile the start is not lightened, or it would vanish.
    const from = dark ? mix(ink, { r: 1, g: 1, b: 1 }, 0.3) : ink;
    const to = turned(accent, 0.1);
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">'
        + '<defs><linearGradient id="t" x1="0" y1="0" x2="0" y2="1">'
        + '<stop offset="0" stop-color="' + hex(top) + '"/><stop offset="1" stop-color="' + hex(bottom) + '"/>'
        + '</linearGradient><linearGradient id="s" x1="0" y1="1" x2="1" y2="0">'
        + '<stop offset="0" stop-color="' + hex(from) + '"/><stop offset="0.55" stop-color="' + hex(ink) + '"/>'
        + '<stop offset="1" stop-color="' + hex(to) + '"/></linearGradient></defs>'
        + '<rect x="3" y="3" width="58" height="58" rx="15" fill="url(#t)"'
        // A light tile on a light panel needs its edge said.
        + (dark ? '' : ' stroke="' + hex(mix(set.window, { r: 0, g: 0, b: 0 }, 0.18)) + '" stroke-width="1.5"')
        + '/>'
        + '<path d="M13 20 L22 45 L32 26 L42 45 L51 20" fill="none" stroke="url(#s)" stroke-width="6.5" '
        + 'stroke-linecap="round" stroke-linejoin="round"/></svg>';
    return "data:image/svg+xml;utf8," + encodeURIComponent(svg);
}
