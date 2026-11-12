// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "playlists.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QRandomGenerator>
#include <QSaveFile>
#include <QStandardPaths>

Q_LOGGING_CATEGORY(lcPlaylists, "wallflux.playlists")

namespace {

/// The shortest a wallpaper may stay, whatever the window was told. A
/// playlist set to nought would be a wallpaper changing every tick of the
/// timer, which is a seizure rather than a feature. Five seconds rather
/// than the minute it used to be, now that the window can ask in seconds.
constexpr int LeastSeconds = 5;

} // namespace

int Playlists::staysFor(const QVariantMap &one)
{
    const int asked = one.value(QStringLiteral("hours")).toInt() * 3600
                      + one.value(QStringLiteral("minutes")).toInt() * 60
                      + one.value(QStringLiteral("seconds")).toInt();
    return qMax(LeastSeconds, asked);
}

QString Playlists::wherePlaylistsLive()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation)
           + QStringLiteral("/playlists.json");
}

Playlists::Playlists(QObject *parent)
    : QObject(parent)
{
    // Single shot, wound again at every turn: a repeating timer cannot be
    // stopped and picked up where it was.
    m_turn.setSingleShot(true);
    connect(&m_turn, &QTimer::timeout, this, &Playlists::turn);
    load();
}

void Playlists::load()
{
    QFile file(wherePlaylistsLive());
    if (!file.exists())
        return;

    if (!file.open(QIODevice::ReadOnly)) {
        qCWarning(lcPlaylists) << "cannot read" << file.fileName() << file.errorString();
        return;
    }

    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        qCWarning(lcPlaylists) << "unreadable playlists file:" << error.errorString();
        return;
    }

    m_all.clear();
    const QJsonArray saved = document.array();
    for (const QJsonValue &value : saved) {
        const QJsonObject one = value.toObject();
        const QString name = one.value(QLatin1StringView("name")).toString();
        QStringList ids;
        const QJsonArray held = one.value(QLatin1StringView("ids")).toArray();
        for (const QJsonValue &id : held)
            ids << id.toString();

        // A playlist with no name or nothing in it cannot be shown or
        // played, and keeping it would only make the list longer.
        if (name.isEmpty() || ids.isEmpty())
            continue;

        m_all.append(QVariantMap{
            {QStringLiteral("name"), name},
            {QStringLiteral("ids"), ids},
            {QStringLiteral("hours"), one.value(QLatin1StringView("hours")).toInt(0)},
            {QStringLiteral("minutes"), one.value(QLatin1StringView("minutes")).toInt(15)},
            {QStringLiteral("seconds"), one.value(QLatin1StringView("seconds")).toInt(0)},
            {QStringLiteral("shuffle"), one.value(QLatin1StringView("shuffle")).toBool(false)},
            {QStringLiteral("videoEnds"), one.value(QLatin1StringView("videoEnds")).toBool(false)},
            {QStringLiteral("whilePaused"),
             one.value(QLatin1StringView("whilePaused")).toBool(false)},
            {QStringLiteral("alwaysFirst"),
             one.value(QLatin1StringView("alwaysFirst")).toBool(false)},
            {QStringLiteral("transition"),
             one.value(QLatin1StringView("transition")).toString()},
            {QStringLiteral("transitionMs"),
             one.value(QLatin1StringView("transitionMs")).toInt(1500)},
        });
    }

    qCInfo(lcPlaylists) << "read" << m_all.size() << "playlists";
    emit allChanged();
}

void Playlists::store() const
{
    const QString path = wherePlaylistsLive();
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        qCWarning(lcPlaylists) << "cannot make room for" << path;
        return;
    }

    QJsonArray saved;
    for (const QVariant &value : m_all) {
        const QVariantMap one = value.toMap();
        QJsonArray ids;
        for (const QString &id : one.value(QStringLiteral("ids")).toStringList())
            ids.append(id);

        saved.append(QJsonObject{
            {QStringLiteral("name"), one.value(QStringLiteral("name")).toString()},
            {QStringLiteral("ids"), ids},
            {QStringLiteral("hours"), one.value(QStringLiteral("hours")).toInt()},
            {QStringLiteral("minutes"), one.value(QStringLiteral("minutes")).toInt()},
            {QStringLiteral("seconds"), one.value(QStringLiteral("seconds")).toInt()},
            {QStringLiteral("shuffle"), one.value(QStringLiteral("shuffle")).toBool()},
            {QStringLiteral("videoEnds"), one.value(QStringLiteral("videoEnds")).toBool()},
            {QStringLiteral("whilePaused"), one.value(QStringLiteral("whilePaused")).toBool()},
            {QStringLiteral("alwaysFirst"), one.value(QStringLiteral("alwaysFirst")).toBool()},
            {QStringLiteral("transition"), one.value(QStringLiteral("transition")).toString()},
            {QStringLiteral("transitionMs"), one.value(QStringLiteral("transitionMs")).toInt()},
        });
    }

    // Written whole or not at all: a half-written file here is a list of
    // playlists that cannot be read at the next start.
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        qCWarning(lcPlaylists) << "cannot write" << path << file.errorString();
        return;
    }
    file.write(QJsonDocument(saved).toJson(QJsonDocument::Indented));
    if (!file.commit())
        qCWarning(lcPlaylists) << "could not finish writing" << path;
}

QVariantMap Playlists::asClock(int seconds) const
{
    constexpr int LongestDay = 23 * 3600 + 59 * 60 + 59;
    // The same floor `staysFor()` applies, applied here so the three boxes
    // show it rather than a line of small print under them explaining it.
    // Type three noughts and they read five seconds, which is what will
    // happen; the window says the truth instead of a footnote.
    const int total = qBound(LeastSeconds, seconds, LongestDay);

    return {
        {QStringLiteral("hours"), total / 3600},
        {QStringLiteral("minutes"), (total % 3600) / 60},
        {QStringLiteral("seconds"), total % 60},
    };
}

void Playlists::save(const QString &name, const QStringList &ids, const QVariantMap &how)
{
    if (name.isEmpty() || ids.isEmpty()) {
        qCWarning(lcPlaylists) << "a playlist needs a name and something in it";
        return;
    }

    const QVariantMap one{
        {QStringLiteral("name"), name},
        {QStringLiteral("ids"), ids},
        {QStringLiteral("hours"), qMax(0, how.value(QStringLiteral("hours")).toInt())},
        {QStringLiteral("minutes"), qMax(0, how.value(QStringLiteral("minutes")).toInt())},
        {QStringLiteral("seconds"), qMax(0, how.value(QStringLiteral("seconds")).toInt())},
        {QStringLiteral("shuffle"), how.value(QStringLiteral("shuffle")).toBool()},
        {QStringLiteral("videoEnds"), how.value(QStringLiteral("videoEnds")).toBool()},
        {QStringLiteral("whilePaused"), how.value(QStringLiteral("whilePaused")).toBool()},
        {QStringLiteral("alwaysFirst"), how.value(QStringLiteral("alwaysFirst")).toBool()},
        {QStringLiteral("transition"), how.value(QStringLiteral("transition")).toString()},
        {QStringLiteral("transitionMs"),
         qBound(0, how.value(QStringLiteral("transitionMs")).toInt(), 3000)},
    };

    // The name is what a playlist is known by, so saving under one that is
    // taken is a change to that playlist rather than a second of the same
    // name.
    for (qsizetype i = 0; i < m_all.size(); ++i) {
        if (m_all[i].toMap().value(QStringLiteral("name")).toString() == name) {
            m_all[i] = one;
            store();
            emit allChanged();
            qCInfo(lcPlaylists) << "changed" << name << "--" << ids.size() << "wallpapers";
            return;
        }
    }

    m_all.append(one);
    store();
    emit allChanged();
    qCInfo(lcPlaylists) << "saved" << name << "--" << ids.size() << "wallpapers";
}

void Playlists::forget(const QString &name)
{
    for (qsizetype i = 0; i < m_all.size(); ++i) {
        if (m_all[i].toMap().value(QStringLiteral("name")).toString() != name)
            continue;

        m_all.removeAt(i);
        if (m_playing == name)
            stop();
        store();
        emit allChanged();
        qCInfo(lcPlaylists) << "forgot" << name;
        return;
    }
}

bool Playlists::rename(const QString &from, const QString &to)
{
    const QString name = to.trimmed();
    if (name.isEmpty() || (name != from && !named(name).isEmpty()))
        return false;

    for (qsizetype i = 0; i < m_all.size(); ++i) {
        QVariantMap one = m_all[i].toMap();
        if (one.value(QStringLiteral("name")).toString() != from)
            continue;

        one.insert(QStringLiteral("name"), name);
        m_all[i] = one;
        store();
        emit allChanged();
        if (m_playing == from) {
            m_playing = name;
            emit playingChanged();
        }
        qCInfo(lcPlaylists) << "renamed" << from << "to" << name;
        return true;
    }
    return false;
}

QVariantMap Playlists::named(const QString &name) const
{
    for (const QVariant &value : m_all) {
        const QVariantMap one = value.toMap();
        if (one.value(QStringLiteral("name")).toString() == name)
            return one;
    }
    return {};
}

void Playlists::start(const QString &name)
{
    const QVariantMap one = named(name);
    if (one.isEmpty()) {
        qCWarning(lcPlaylists) << "no playlist called" << name;
        return;
    }

    m_order = one.value(QStringLiteral("ids")).toStringList();
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

    m_stay = staysFor(one);
    m_videoEnds = one.value(QStringLiteral("videoEnds")).toBool();
    m_whilePaused = one.value(QStringLiteral("whilePaused")).toBool();
    m_transition = one.value(QStringLiteral("transition")).toString();
    m_transitionMs = one.value(QStringLiteral("transitionMs")).toInt();

    m_at = 0;
    m_left = -1;
    m_playing = name;
    m_turn.start(m_stay * 1000);
    emit playingChanged();

    qCInfo(lcPlaylists) << "playing" << name << "--" << m_order.size() << "wallpapers,"
                        << m_stay << "seconds each; on a video ending:" << m_videoEnds
                        << "; while paused:" << m_whilePaused;
    emit plays(m_order.constFirst());
}

void Playlists::next()
{
    if (m_playing.isEmpty())
        return;
    goOn();
}

void Playlists::previous()
{
    if (m_playing.isEmpty())
        return;
    goOn(-1);
}

void Playlists::hold(bool held)
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
        qCDebug(lcPlaylists) << "held with" << m_left << "ms left";
        return;
    }

    if (m_turn.isActive() || m_left < 0)
        return;
    m_turn.start(m_left);
    m_left = -1;
}

void Playlists::stop()
{
    if (m_playing.isEmpty())
        return;

    qCInfo(lcPlaylists) << "stopped" << m_playing;
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

void Playlists::turn()
{
    goOn();
}

void Playlists::goOn(int step)
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

    qCInfo(lcPlaylists) << "turning to" << m_order.at(m_at);
    emit plays(m_order.at(m_at));
}
