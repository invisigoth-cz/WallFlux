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

    /// Every playlist, each `{ name, ids, hours, minutes, seconds, shuffle,
    /// videoEnds, whilePaused, alwaysFirst, transition, transitionMs }`.
    Q_PROPERTY(QVariantList all READ all NOTIFY allChanged)

    /// The name of the one playing, or empty while none is.
    Q_PROPERTY(QString playing READ playing NOTIFY playingChanged)

    /// Whether the playlist now running turns when a video ends rather
    /// than when the clock says so. False while nothing is running.
    ///
    /// The window reads this: it is what decides whether a video loops for
    /// ever or plays once, and only the window knows whether what is on
    /// the screen is a video at all.
    Q_PROPERTY(bool changesOnVideoEnd READ changesOnVideoEnd NOTIFY playingChanged)

    /// Whether the playlist now running keeps counting while the wallpaper
    /// is stopped behind a window. False while nothing is running, which
    /// is the quieter answer: a playlist that is not running is not
    /// counting either.
    Q_PROPERTY(bool changesWhilePaused READ changesWhilePaused NOTIFY playingChanged)

    /// How the playlist now running gets from one wallpaper to the next:
    /// `""` for straight away, `"soft"` to hold the last frame over the
    /// gap, `"none"` for straight away said out loud, `"fade"` to cross
    /// from one to the other, `"black"` to go out through black. Empty
    /// while nothing is running, which is the same as no transition.
    ///
    /// A word rather than the number of a row in a drop down: the rows
    /// have been reordered twice already and a number in a file would have
    /// meant something else each time.
    Q_PROPERTY(QString transition READ transition NOTIFY playingChanged)

    /// How long that takes, in milliseconds.
    Q_PROPERTY(int transitionMs READ transitionMs NOTIFY playingChanged)

public:
    explicit Playlists(QObject *parent = nullptr);

    QVariantList all() const { return m_all; }
    QString playing() const { return m_playing; }
    bool changesOnVideoEnd() const { return m_videoEnds; }
    bool changesWhilePaused() const { return m_whilePaused; }
    QString transition() const { return m_transition; }
    int transitionMs() const { return m_transitionMs; }

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

    /// Throws one away, and stops it if it was the one playing.
    Q_INVOKABLE void forget(const QString &name);

    /// Calls one by another name, and goes on playing it if it was. Refused,
    /// with false, where the new name is empty or another playlist has it:
    /// renaming over one would throw that one away without asking, which is
    /// what Delete is for.
    Q_INVOKABLE bool rename(const QString &from, const QString &to);

    /// One playlist by name, or an empty map. For the window, which has to
    /// show what a playlist holds before it changes any of it.
    Q_INVOKABLE QVariantMap named(const QString &name) const;

    /// Starts one: the first wallpaper goes on now and the rest follow.
    /// Starting the one already playing starts it again from the top.
    Q_INVOKABLE void start(const QString &name);

    /// Stops whichever is playing. What is on the screen stays there --
    /// stopping a playlist is not the same as turning the wallpaper off.
    Q_INVOKABLE void stop();

    /// On to the next one now, and the clock starts again from the top.
    /// What a video reaching its end means, for a playlist that was told
    /// to turn on that rather than on the clock.
    Q_INVOKABLE void next();

    /// Back to the one before, and the clock starts again from the top.
    Q_INVOKABLE void previous();

    /// Stops the clock without stopping the playlist, and starts it again
    /// where it left off rather than from the beginning.
    ///
    /// *Why* it is held is the window's business and not this one's:
    /// because a window is covering the wallpaper and this playlist was
    /// not told to go on regardless, or because a video is what decides
    /// when to turn and it has not finished. The window works out whether
    /// either is true and says so here.
    Q_INVOKABLE void hold(bool held);

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

    /// Puts the next wallpaper on and winds the clock again. The one place
    /// that does it, whether the clock ran out or a video did.
    void goOn(int step = 1);

    QVariantList m_all;
    QString m_playing;

    /// The ids of the playlist that is running, in the order they will
    /// come -- already shuffled where that was asked for, so the order is
    /// decided once rather than guessed at every turn.
    QStringList m_order;
    int m_at = 0;

    /// How long each wallpaper of the running playlist stays, in seconds,
    /// and what the running playlist was told about videos and pausing.
    /// Read once when it starts rather than looked up at every turn.
    int m_stay = 0;
    bool m_videoEnds = false;
    bool m_whilePaused = false;
    QString m_transition;
    int m_transitionMs = 0;

    /// What was left on the clock when it was held, in milliseconds, or -1
    /// where it is not being held.
    int m_left = -1;

    /// Single shot, and wound again at every turn. A repeating timer
    /// cannot be stopped and picked up where it was, which is the whole of
    /// what holding one is.
    QTimer m_turn;
};
