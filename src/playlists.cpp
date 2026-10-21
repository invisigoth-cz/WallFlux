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
/// timer, which is a seizure rather than a feature.
constexpr int LeastMinutes = 1;

} // namespace

QString Playlists::wherePlaylistsLive()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation)
           + QStringLiteral("/playlists.json");
}

Playlists::Playlists(QObject *parent)
    : QObject(parent)
{
    m_turn.setSingleShot(false);
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
            {QStringLiteral("shuffle"), one.value(QLatin1StringView("shuffle")).toBool(false)},
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
            {QStringLiteral("shuffle"), one.value(QStringLiteral("shuffle")).toBool()},
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

void Playlists::save(const QString &name, const QStringList &ids, int hours, int minutes,
                     bool shuffle)
{
    if (name.isEmpty() || ids.isEmpty()) {
        qCWarning(lcPlaylists) << "a playlist needs a name and something in it";
        return;
    }

    const QVariantMap one{
        {QStringLiteral("name"), name},
        {QStringLiteral("ids"), ids},
        {QStringLiteral("hours"), qMax(0, hours)},
        {QStringLiteral("minutes"), qMax(0, minutes)},
        {QStringLiteral("shuffle"), shuffle},
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
        // Shuffled once, when it starts, rather than a new guess at every
        // turn: picking at random each time plays the same wallpaper twice
        // in a row sooner or later, which looks like the playlist stopped.
        for (qsizetype i = m_order.size() - 1; i > 0; --i)
            m_order.swapItemsAt(i, QRandomGenerator::global()->bounded(int(i) + 1));
    }

    const int hours = one.value(QStringLiteral("hours")).toInt();
    const int minutes = one.value(QStringLiteral("minutes")).toInt();
    const int stay = qMax(LeastMinutes, hours * 60 + minutes);

    m_at = 0;
    m_playing = name;
    m_turn.start(stay * 60 * 1000);
    emit playingChanged();

    qCInfo(lcPlaylists) << "playing" << name << "--" << m_order.size() << "wallpapers,"
                        << stay << "minutes each";
    emit plays(m_order.constFirst());
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
    emit playingChanged();
}

void Playlists::turn()
{
    if (m_order.isEmpty()) {
        stop();
        return;
    }

    m_at = (m_at + 1) % int(m_order.size());
    qCInfo(lcPlaylists) << "turning to" << m_order.at(m_at);
    emit plays(m_order.at(m_at));
}
