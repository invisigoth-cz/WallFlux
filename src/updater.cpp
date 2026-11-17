// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "updater.h"

#include <QCoreApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QNetworkReply>
#include <QProcess>
#include <QStandardPaths>
#include <QVersionNumber>

Q_LOGGING_CATEGORY(lcUpdate, "wallflux.update")

namespace {

const QUrl Latest(QStringLiteral("https://api.github.com/repos/invisigoth-cz/WallFlux/releases/latest"));

QNetworkRequest asking(const QUrl &url)
{
    QNetworkRequest request(url);
    // GitHub refuses a request that does not say what it comes from.
    request.setHeader(QNetworkRequest::UserAgentHeader,
                      QStringLiteral("WallFlux/") + QCoreApplication::applicationVersion());
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                         QNetworkRequest::NoLessSafeRedirectPolicy);
    return request;
}

} // namespace

Updater::Updater(QObject *parent)
    : QObject(parent)
{
}

QString Updater::command()
{
    return QStringLiteral("curl -fsSL https://raw.githubusercontent.com/invisigoth-cz/WallFlux/main/install.sh | bash");
}

bool Updater::install()
{
    // Konsole, being KDE's and so on every machine WallFlux runs on. Started
    // detached: the script quits this WallFlux and starts the new one, and
    // the terminal has to outlive the program that opened it.
    const QString line = command()
        + QStringLiteral("; echo; read -rsn1 -p 'Press any key to close this window.'");
    const QStringList inKonsole{QStringLiteral("konsole"), QStringLiteral("-e"), QStringLiteral("bash"),
                                QStringLiteral("-c"), line};
    // In a scope of its own where there is a user manager. Started as a
    // child of a WallFlux that runs as a service -- which an update starts
    // it as -- the terminal and the script in it belong to that service,
    // and systemd ends them with it the moment the script quits the old
    // WallFlux: on 17 November the update stopped there and nothing
    // started again.
    const bool scoped = !QStandardPaths::findExecutable(QStringLiteral("systemd-run")).isEmpty();
    const bool started = scoped
        ? QProcess::startDetached(QStringLiteral("systemd-run"),
                                  QStringList{QStringLiteral("--user"), QStringLiteral("--scope"),
                                              QStringLiteral("--quiet"), QStringLiteral("--collect")}
                                      + inKonsole)
        : QProcess::startDetached(inKonsole.first(), inKonsole.mid(1));
    if (!started)
        qCWarning(lcUpdate) << "Konsole could not be started";
    return started;
}

bool Updater::newer(const QString &candidate, const QString &current)
{
    return QVersionNumber::fromString(candidate) > QVersionNumber::fromString(current);
}

void Updater::set(State state, const QString &detail)
{
    m_state = state;
    m_detail = detail;
    emit changed();
}

void Updater::check()
{
    if (m_state == Checking)
        return;
    set(Checking);
    QNetworkReply *reply = m_network.get(asking(Latest));
    connect(reply, &QNetworkReply::finished, this, [this, reply] {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            qCWarning(lcUpdate) << "GitHub did not say:" << reply->errorString();
            set(Failed, reply->errorString());
            return;
        }
        const QJsonObject release = QJsonDocument::fromJson(reply->readAll()).object();
        QString tag = release.value(QLatin1StringView("tag_name")).toString();
        if (tag.startsWith(u'v'))
            tag.remove(0, 1);
        if (tag.isEmpty()) {
            set(Failed, QStringLiteral("no release"));
            return;
        }
        m_latest = tag;
        m_notes = release.value(QLatin1StringView("body")).toString();
        // Opened in the browser on a click: so a page of GitHub's, and
        // nothing a forged answer could make it -- a file, or `steam://`.
        const QUrl page(release.value(QLatin1StringView("html_url")).toString());
        m_page = page.scheme() == QLatin1StringView("https") && page.host() == QLatin1StringView("github.com")
                     ? page : QUrl(QStringLiteral("https://github.com/invisigoth-cz/WallFlux/releases"));
        const bool later = newer(tag, QCoreApplication::applicationVersion());
        qCInfo(lcUpdate) << "the newest release is" << tag << (later ? "-- newer" : "-- this one");
        set(later ? Available : Current);
    });
}
