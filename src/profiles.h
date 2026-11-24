// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QVariantList>
#include <QVariantMap>

/// The whole arrangement of the monitors under a name, Wallpaper Engine's
/// profiles: the layout, and for each monitor by connector name what it
/// shows, which playlist it runs, whether it is muted and the wallpaper's
/// settings there.
///
/// Kept in `profiles.json` beside the playlists. Nothing here knows which
/// monitors are plugged in; a profile that names one that is not is kept
/// as it is, and the window applies that part when the monitor comes.
/// Loading one is somebody's say-so, which is why it does not break the
/// rule that nothing plays when the program starts.
class Profiles : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Profiles)
    QML_SINGLETON

    /// Every profile, each `{ name, layout, displays, excluded, ... }` as
    /// it was handed to `save()`.
    Q_PROPERTY(QVariantList all READ all NOTIFY allChanged)

public:
    explicit Profiles(QObject *parent = nullptr);

    QVariantList all() const { return m_all; }

    /// Writes one down under this name, replacing whatever had it. A name
    /// of nothing but spaces is refused.
    Q_INVOKABLE void save(const QString &name, const QVariantMap &arrangement);

    /// Throws one away.
    Q_INVOKABLE void forget(const QString &name);

    /// Calls one by another name. Refused, with false, where the new name
    /// is empty or another profile has it, as a playlist's is.
    Q_INVOKABLE bool rename(const QString &from, const QString &to);

    /// One profile by name, or an empty map.
    Q_INVOKABLE QVariantMap named(const QString &name) const;

signals:
    void allChanged();

private:
    static QString whereProfilesLive();
    void load();
    void store() const;

    QVariantList m_all;
};
