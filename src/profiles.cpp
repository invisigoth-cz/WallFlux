// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "profiles.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QSaveFile>
#include <QStandardPaths>

Q_LOGGING_CATEGORY(lcProfiles, "wallflux.profiles")

QString Profiles::whereProfilesLive()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation)
           + QStringLiteral("/profiles.json");
}

Profiles::Profiles(QObject *parent)
    : QObject(parent)
{
    load();
}

void Profiles::load()
{
    QFile file(whereProfilesLive());
    if (!file.exists())
        return;
    if (!file.open(QIODevice::ReadOnly)) {
        qCWarning(lcProfiles) << "cannot read" << file.fileName() << file.errorString();
        return;
    }

    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        qCWarning(lcProfiles) << "unreadable profiles file:" << error.errorString();
        return;
    }

    m_all.clear();
    const QJsonArray saved = document.array();
    for (const QJsonValue &value : saved) {
        const QVariantMap one = value.toObject().toVariantMap();
        if (one.value(QStringLiteral("name")).toString().trimmed().isEmpty())
            continue;
        m_all.append(one);
    }
    qCInfo(lcProfiles) << "read" << m_all.size() << "profiles";
    emit allChanged();
}

void Profiles::store() const
{
    const QString path = whereProfilesLive();
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        qCWarning(lcProfiles) << "cannot make room for" << path;
        return;
    }

    // Written whole or not at all, as the playlists are.
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        qCWarning(lcProfiles) << "cannot write" << path << file.errorString();
        return;
    }
    file.write(QJsonDocument(QJsonArray::fromVariantList(m_all)).toJson(QJsonDocument::Indented));
    if (!file.commit())
        qCWarning(lcProfiles) << "could not finish writing" << path;
}

void Profiles::save(const QString &name, const QVariantMap &arrangement)
{
    const QString called = name.trimmed();
    if (called.isEmpty())
        return;

    QVariantMap one = arrangement;
    one.insert(QStringLiteral("name"), called);

    bool replaced = false;
    for (QVariant &value : m_all) {
        if (value.toMap().value(QStringLiteral("name")).toString() == called) {
            value = one;
            replaced = true;
            break;
        }
    }
    if (!replaced)
        m_all.append(one);

    store();
    emit allChanged();
    qCInfo(lcProfiles) << "saved" << called;
}

void Profiles::forget(const QString &name)
{
    for (qsizetype i = 0; i < m_all.size(); ++i) {
        if (m_all[i].toMap().value(QStringLiteral("name")).toString() != name)
            continue;
        m_all.removeAt(i);
        store();
        emit allChanged();
        qCInfo(lcProfiles) << "forgot" << name;
        return;
    }
}

bool Profiles::rename(const QString &from, const QString &to)
{
    const QString name = to.trimmed();
    if (name.isEmpty() || (name != from && !named(name).isEmpty()))
        return false;

    for (QVariant &value : m_all) {
        QVariantMap one = value.toMap();
        if (one.value(QStringLiteral("name")).toString() != from)
            continue;
        one.insert(QStringLiteral("name"), name);
        value = one;
        store();
        emit allChanged();
        return true;
    }
    return false;
}

QVariantMap Profiles::named(const QString &name) const
{
    for (const QVariant &value : m_all) {
        const QVariantMap one = value.toMap();
        if (one.value(QStringLiteral("name")).toString() == name)
            return one;
    }
    return {};
}
