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
        store();
        emit allChanged();
        emit forgotten(name);
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
        emit renamed(from, name);
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
