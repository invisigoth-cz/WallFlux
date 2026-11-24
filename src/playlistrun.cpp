// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "playlistrun.h"

#include "playlists.h"

#include <QLoggingCategory>
#include <QRandomGenerator>

Q_LOGGING_CATEGORY(lcRun, "wallflux.playlists")

PlaylistRun::PlaylistRun(QObject *parent)
    : QObject(parent)
{
    // Single shot, wound again at every turn: a repeating timer cannot be
    // stopped and picked up where it was.
    m_turn.setSingleShot(true);
    connect(&m_turn, &QTimer::timeout, this, &PlaylistRun::turn);
}

void PlaylistRun::start(const QVariantMap &one)
{
    const QString name = one.value(QStringLiteral("name")).toString();
    const QStringList ids = one.value(QStringLiteral("ids")).toStringList();
    if (ids.isEmpty()) {
        qCWarning(lcRun) << "nothing to play in" << name;
        return;
    }

    m_order = ids;
    if (one.value(QStringLiteral("shuffle")).toBool()) {
        // "Always begin with the first wallpaper" is what the first index
        // is held back from the shuffle for: the rest are dealt out, and
        // the one at the top of the list is still the one that comes up.
        // Without it the setting means nothing at all, since an unshuffled
        // playlist begins with its first wallpaper anyway.
        const qsizetype keep = one.value(QStringLiteral("alwaysFirst")).toBool() ? 1 : 0;

        // Shuffled once, when it starts, rather than a new guess at every
        // turn: picking at random each time plays the same wallpaper twice
        // in a row sooner or later, which looks like the playlist stopped.
        for (qsizetype i = m_order.size() - 1; i > keep; --i) {
            const int at = int(keep) + QRandomGenerator::global()->bounded(int(i - keep) + 1);
            m_order.swapItemsAt(i, at);
        }
    }

    m_stay = Playlists::staysFor(one);
    m_videoEnds = one.value(QStringLiteral("videoEnds")).toBool();
    m_whilePaused = one.value(QStringLiteral("whilePaused")).toBool();
    m_transition = one.value(QStringLiteral("transition")).toString();
    m_transitionMs = one.value(QStringLiteral("transitionMs")).toInt();

    m_at = 0;
    m_left = -1;
    m_playing = name;
    m_turn.start(m_stay * 1000);
    emit playingChanged();

    qCInfo(lcRun) << "playing" << name << "--" << m_order.size() << "wallpapers,"
                        << m_stay << "seconds each; on a video ending:" << m_videoEnds
                        << "; while paused:" << m_whilePaused;
    emit plays(m_order.constFirst());
}

void PlaylistRun::next()
{
    if (m_playing.isEmpty())
        return;
    goOn();
}

void PlaylistRun::previous()
{
    if (m_playing.isEmpty())
        return;
    goOn(-1);
}

void PlaylistRun::hold(bool held)
{
    if (m_playing.isEmpty())
        return;

    if (held) {
        if (!m_turn.isActive())
            return;
        // What was left, so it is picked up where it stopped rather than
        // started again -- a wallpaper covered for a second should not
        // have its whole turn back.
        m_left = m_turn.remainingTime();
        m_turn.stop();
        qCDebug(lcRun) << "held with" << m_left << "ms left";
        return;
    }

    if (m_turn.isActive() || m_left < 0)
        return;
    m_turn.start(m_left);
    m_left = -1;
}

void PlaylistRun::stop()
{
    if (m_playing.isEmpty())
        return;

    qCInfo(lcRun) << "stopped" << m_playing;
    m_turn.stop();
    m_playing.clear();
    m_order.clear();
    m_at = 0;
    m_left = -1;
    m_videoEnds = false;
    m_whilePaused = false;
    m_transition.clear();
    m_transitionMs = 0;
    emit playingChanged();
}

void PlaylistRun::turn()
{
    goOn();
}

void PlaylistRun::goOn(int step)
{
    if (m_order.isEmpty()) {
        stop();
        return;
    }

    const int count = int(m_order.size());
    m_at = ((m_at + step) % count + count) % count;
    // Wound again here rather than left to repeat: a single-shot timer is
    // what lets it be stopped and picked up where it was.
    m_left = -1;
    m_turn.start(m_stay * 1000);

    qCInfo(lcRun) << "turning to" << m_order.at(m_at);
    emit plays(m_order.at(m_at));
}

void PlaylistRun::renamed(const QString &from, const QString &to)
{
    if (m_playing.isEmpty() || m_playing != from)
        return;
    m_playing = to;
    emit playingChanged();
}

void PlaylistRun::forgotten(const QString &name)
{
    if (!m_playing.isEmpty() && m_playing == name)
        stop();
}
