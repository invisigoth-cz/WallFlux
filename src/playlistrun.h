// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QTimer>
#include <QVariantMap>

/// One playlist running: which wallpaper should be on a monitor now, and
/// when the next one comes. Every monitor has one of these; the playlists
/// themselves are `Playlists`', which this knows nothing of beyond the one
/// it was handed.
///
/// **It does not play anything.** It says `plays(id)` and the window puts
/// that on its monitor, the same way a click on a card does -- nothing here
/// knows what a wallpaper is or how one is drawn.
class PlaylistRun : public QObject
{
    Q_OBJECT
    QML_ELEMENT

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
    explicit PlaylistRun(QObject *parent = nullptr);

    QString playing() const { return m_playing; }
    bool changesOnVideoEnd() const { return m_videoEnds; }
    bool changesWhilePaused() const { return m_whilePaused; }
    QString transition() const { return m_transition; }
    int transitionMs() const { return m_transitionMs; }

    /// Starts one -- `Playlists.named(name)` -- with the first wallpaper now
    /// and the rest to follow. Starting the one already playing starts it
    /// again from the top. A playlist with no wallpapers is refused.
    Q_INVOKABLE void start(const QVariantMap &one);

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

public slots:
    /// The playlist was called by another name: if it is this one, it goes
    /// on under the new name.
    void renamed(const QString &from, const QString &to);

    /// The playlist was thrown away: if it is this one, it stops.
    void forgotten(const QString &name);

signals:
    void playingChanged();

    /// The wallpaper that should be on the screen now.
    void plays(const QString &id);

private:
    /// The next one in the running playlist, and the timer set for it.
    void turn();

    /// Puts the next wallpaper on and winds the clock again. The one place
    /// that does it, whether the clock ran out or a video did.
    void goOn(int step = 1);

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
