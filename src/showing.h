// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include "wallpapercore.h"

#include <QObject>
#include <QPointer>
#include <QQmlEngine>

/// The wallpaper one monitor shows -- or, cloned, every monitor.
///
/// The library is `WallpaperCore`'s and is the same for everybody; what is
/// on show is each monitor's own, so it lives here, one of these to a
/// monitor.
class Showing : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    /// The library to look wallpapers up in -- `Wallpapers` in QML.
    Q_PROPERTY(WallpaperCore *library READ library WRITE setLibrary NOTIFY libraryChanged)
    /// What is on show, or an empty wallpaper while nothing is.
    Q_PROPERTY(Wallpaper current READ current NOTIFY currentChanged)

public:
    explicit Showing(QObject *parent = nullptr);

    WallpaperCore *library() const { return m_library; }
    void setLibrary(WallpaperCore *library);
    Wallpaper current() const { return m_current; }

    /// Puts the wallpaper with this id on show. Returns false, and changes
    /// nothing, if the library has no wallpaper of that id.
    Q_INVOKABLE bool select(const QString &id);

    /// Shows nothing.
    Q_INVOKABLE void clear();

signals:
    void libraryChanged();
    void currentChanged();

private:
    /// The library was read again: what is on show may have moved, learnt
    /// its size, or gone.
    void reread();

    QPointer<WallpaperCore> m_library;
    Wallpaper m_current;
};
