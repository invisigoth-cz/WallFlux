// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QList>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

/// Wallpapers that take turns.
///
/// A playlist is a name, the wallpapers in it, how long each one stays and
/// whether they come in order or at random. This holds the list of them,
/// keeps them on the disk. Running one is `PlaylistRun`'s, of which each
/// monitor has its own.
///
/// This is the first thing WallFlux remembers between runs. Everything
/// else about it is deliberately forgotten -- which wallpaper was playing,
/// what was searched for, what was ticked -- and a playlist is the one
/// thing where that would make the feature pointless. Asked for and
/// decided; see CLAUDE.md.
class Playlists : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Playlists)
    QML_SINGLETON

    /// Every playlist, each `{ name, ids, hours, minutes, seconds, shuffle,
    /// videoEnds, whilePaused, alwaysFirst, transition, transitionMs }`.
    Q_PROPERTY(QVariantList all READ all NOTIFY allChanged)

public:
    explicit Playlists(QObject *parent = nullptr);

    QVariantList all() const { return m_all; }

    /// Writes one down, replacing whatever had that name. An empty `ids`
    /// is refused: a playlist of nothing has nothing to take turns.
    ///
    /// `how` is everything else about it, by name rather than by position:
    /// `hours`, `minutes`, `seconds`, `shuffle`, `videoEnds`,
    /// `whilePaused`, `alwaysFirst`, `transition`, `transitionMs`.
    /// Anything left out is nought, false or empty.
    /// A map rather than nine arguments, because `save(n, ids, 0, 15, 0,
    /// true, false, true, false)` is a line nobody can read or get right.
    Q_INVOKABLE void save(const QString &name, const QStringList &ids, const QVariantMap &how);

    /// Throws one away, and says so, so that wherever it runs it stops.
    Q_INVOKABLE void forget(const QString &name);

    /// Calls one by another name, and says so, so that wherever it runs it
    /// goes on under the new one. Refused,
    /// with false, where the new name is empty or another playlist has it:
    /// renaming over one would throw that one away without asking, which is
    /// what Delete is for.
    Q_INVOKABLE bool rename(const QString &from, const QString &to);

    /// One playlist by name, or an empty map. For the window, which has to
    /// show what a playlist holds before it changes any of it.
    Q_INVOKABLE QVariantMap named(const QString &name) const;

    /// How long one wallpaper of a playlist stays, in seconds, floor and
    /// all -- hours, minutes and seconds added up, and never less than the
    /// floor, since a playlist set to nought would be a wallpaper changing
    /// every tick of the timer. Public because it is a sum with a floor
    /// and nothing else, and the test says what it should come to.
    static int staysFor(const QVariantMap &one);

    /// The other half of that sum: a number of seconds back into
    /// `{ hours, minutes, seconds }`, carrying the remainder rather than
    /// dropping it, never shorter than the floor `staysFor()` keeps and
    /// never longer than a day less a second. An exact inverse of it, so
    /// the window can never show an interval that is not the one that
    /// will run.
    ///
    /// It is here and not in the window because it is the same rule read
    /// the other way round, and because a rule in QML is a rule no test
    /// can reach. The settings popup hands it whatever was typed into its
    /// three boxes and writes back what comes out, which is what turns
    /// ninety seconds into a minute and a half.
    Q_INVOKABLE QVariantMap asClock(int seconds) const;

signals:
    void allChanged();

    /// One was called by another name; every run of it follows.
    void renamed(const QString &from, const QString &to);

    /// One was thrown away; every run of it stops.
    void forgotten(const QString &name);

private:
    /// Where the file is, which is also what the directory is made for.
    static QString wherePlaylistsLive();

    void load();
    void store() const;

    QVariantList m_all;
};
