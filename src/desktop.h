// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
// The whole type, not a forward declaration: a Q_INVOKABLE taking a
// pointer needs a meta-type for it, and a meta-type needs the definition.
#include <QQuickWindow>

/// Turns a window into a wallpaper.
///
/// A wallpaper is a layer-shell surface: anchored to all four edges of one
/// screen, below every ordinary window, taking no input and reserving no
/// room. Asking for that is the one thing QML cannot do for itself, and it
/// has to be asked before the window reaches the compositor -- afterwards
/// it is an ordinary surface and cannot be made into a layer-shell one.
///
/// So the window is built hidden, handed here, and shown afterwards. It
/// used to be `main()` that did this to the one window there was; there is
/// one per screen now, and they come and go as monitors are plugged in, so
/// it is QML that says when.
class Desktop : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Desktop)
    QML_SINGLETON

public:
    explicit Desktop(QObject *parent = nullptr);

    /// Asks the compositor to treat this window as the desktop's wallpaper.
    /// Answers false where there is no layer shell -- under X11, or on a
    /// compositor without the protocol -- and the window is then an
    /// ordinary one, which is what the fullscreen fallback in Main.qml is
    /// for.
    Q_INVOKABLE bool asWallpaper(QQuickWindow *window);
};
