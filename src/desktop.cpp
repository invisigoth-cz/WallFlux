// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "desktop.h"

#include <LayerShellQt/Window>

#include <QLoggingCategory>
#include <QQuickWindow>
#include <QScreen>

Q_LOGGING_CATEGORY(lcDesktop, "wallflux.desktop")

Desktop::Desktop(QObject *parent)
    : QObject(parent)
{
}

bool Desktop::asWallpaper(QQuickWindow *window)
{
    if (!window)
        return false;

    LayerShellQt::Window *layer = LayerShellQt::Window::get(window);
    if (!layer) {
        qCInfo(lcDesktop) << "no layer shell here; the window stays an ordinary one";
        return false;
    }

    using Layer = LayerShellQt::Window;

    // Bottom rather than background. Plasma draws its own desktop --
    // wallpaper, icons and widgets -- into a background-layer surface of
    // its own, and clicking the desktop raises that surface above ours
    // within the same layer, which took the wallpaper off the screen.
    // Layers are a fixed order the compositor will not reshuffle, and
    // bottom is still below every ordinary window.
    layer->setLayer(Layer::LayerBottom);

    // Anchored to all four edges, which is what makes the compositor give
    // it the whole screen.
    layer->setAnchors(Layer::Anchors(Layer::AnchorTop | Layer::AnchorBottom
                                     | Layer::AnchorLeft | Layer::AnchorRight));

    // A wallpaper reserves no space of its own and never takes the
    // keyboard, so panels keep their room and focus never lands here.
    layer->setExclusiveZone(-1);
    layer->setKeyboardInteractivity(Layer::KeyboardInteractivityNone);
    layer->setActivateOnShow(false);

    // The compositor reads the scope as the surface's role; KWin sees this
    // one as a desktop rather than as a panel.
    layer->setScope(QStringLiteral("desktop"));

    qCInfo(lcDesktop) << "wallpaper surface asked for on"
                      << (window->screen() ? window->screen()->name() : QStringLiteral("no screen"));
    return true;
}
