// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include "kwinscript.h"

#include <QObject>
#include <QPointF>
#include <QQmlEngine>

/// Where the mouse is on the desktop, for the scenes that follow it.
///
/// The wallpaper's surfaces take no input, and must not: a click on the
/// desktop belongs to Plasma's own desktop underneath. So nothing about the
/// mouse ever reaches them, and the position has to come from the
/// compositor. On KWin a script is loaded that reads `workspace.cursorPos`
/// and calls back here -- only while somebody is `watching`, so a desktop
/// with no scene on it sends nothing at all. Elsewhere the mouse is never
/// `known`, and a scene runs as it would with nobody at the computer.
class Pointer : public QObject, protected QDBusContext
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Pointer)
    QML_SINGLETON
    Q_CLASSINFO("D-Bus Interface", "cz.invisigoth.WallFlux.Pointer")

    /// In the desktop's own coordinates, the ones `Screen.virtualX` counts in.
    Q_PROPERTY(QPointF position READ position NOTIFY positionChanged)
    Q_PROPERTY(bool known READ known NOTIFY positionChanged)
    /// Whether to ask the compositor at all.
    Q_PROPERTY(bool watching READ watching WRITE setWatching NOTIFY watchingChanged)

public:
    explicit Pointer(QObject *parent = nullptr);

    QPointF position() const { return m_position; }
    bool known() const { return m_known; }
    bool watching() const { return m_watching; }
    void setWatching(bool watching);

public slots:
    /// Called by the KWin script over D-Bus, which sends whole pixels.
    void setPosition(int x, int y);

signals:
    void positionChanged();
    void watchingChanged();

private:
    KWinScript m_script{QStringLiteral("wallflux-pointer")};
    bool m_registered = false;
    bool m_watching = false;
    bool m_known = false;
    QPointF m_position;
};
