// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QList>
#include <QObject>
#include <QQmlEngine>

namespace KWayland::Client {
class Dpms;
class DpmsManager;
class Registry;
}

/// Whether nobody can see the screens: every one of them switched off by
/// power management, or the session locked.
///
/// Off is KWin's own word over Wayland (org_kde_kwin_dpms, through
/// KWayland): neither KWin's scripting nor anything on the session bus says
/// it -- both were looked through. Locked is the screen saver's
/// ActiveChanged. Under the lock the wallpaper was still playing, sound and
/// all, while nothing of it could be seen.
class Asleep : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Asleep)
    QML_SINGLETON

    Q_PROPERTY(bool asleep READ asleep NOTIFY changed)

public:
    explicit Asleep(QObject *parent = nullptr);

    bool asleep() const { return m_dark || m_locked; }

public slots:
    /// The screen saver's ActiveChanged: the session locked or unlocked.
    void setLocked(bool locked);

signals:
    void changed();

private:
    void watch(quint32 name, quint32 version);
    void settle();

    KWayland::Client::Registry *m_registry = nullptr;
    KWayland::Client::DpmsManager *m_manager = nullptr;
    QList<KWayland::Client::Dpms *> m_screens;
    bool m_dark = false;
    bool m_locked = false;
};
