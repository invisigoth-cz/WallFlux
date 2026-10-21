// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QList>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

/// Wallpapers that take turns.
///
/// A playlist is a name, the wallpapers in it, how long each one stays and
/// whether they come in order or at random. This holds the list of them,
/// keeps them on the disk, and while one is playing says which wallpaper
/// should be on the screen now.
///
/// **It does not play anything.** It says `plays(id)` and the window hands
/// that to `Wallpapers.select()`, the same way a click on a card does --
/// nothing here knows what a wallpaper is or how one is drawn.
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

    /// Every playlist, each `{ name, ids, hours, minutes, shuffle }`.
    Q_PROPERTY(QVariantList all READ all NOTIFY allChanged)

    /// The name of the one playing, or empty while none is.
    Q_PROPERTY(QString playing READ playing NOTIFY playingChanged)

public:
    explicit Playlists(QObject *parent = nullptr);

    QVariantList all() const { return m_all; }
    QString playing() const { return m_playing; }

    /// Writes one down, replacing whatever had that name. An empty `ids`
    /// is refused: a playlist of nothing has nothing to take turns.
    Q_INVOKABLE void save(const QString &name, const QStringList &ids, int hours, int minutes,
                          bool shuffle);

    /// Throws one away, and stops it if it was the one playing.
    Q_INVOKABLE void forget(const QString &name);

    /// One playlist by name, or an empty map. For the window, which has to
    /// show what a playlist holds before it changes any of it.
    Q_INVOKABLE QVariantMap named(const QString &name) const;

    /// Starts one: the first wallpaper goes on now and the rest follow.
    /// Starting the one already playing starts it again from the top.
    Q_INVOKABLE void start(const QString &name);

    /// Stops whichever is playing. What is on the screen stays there --
    /// stopping a playlist is not the same as turning the wallpaper off.
    Q_INVOKABLE void stop();

signals:
    void allChanged();
    void playingChanged();

    /// The wallpaper that should be on the screen now.
    void plays(const QString &id);

private:
    /// Where the file is, which is also what the directory is made for.
    static QString wherePlaylistsLive();

    void load();
    void store() const;

    /// The next one in the running playlist, and the timer set for it.
    void turn();

    QVariantList m_all;
    QString m_playing;

    /// The ids of the playlist that is running, in the order they will
    /// come -- already shuffled where that was asked for, so the order is
    /// decided once rather than guessed at every turn.
    QStringList m_order;
    int m_at = 0;

    QTimer m_turn;
};
