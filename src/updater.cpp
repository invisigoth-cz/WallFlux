// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "updater.h"

#include <QCryptographicHash>

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLoggingCategory>
#include <QNetworkReply>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QVersionNumber>

Q_LOGGING_CATEGORY(lcUpdate, "wallflux.update")

namespace {

const QUrl Latest(QStringLiteral("https://api.github.com/repos/invisigoth-cz/WallFlux/releases/latest"));
const QString Installed = QStringLiteral("/usr/bin/wallflux");

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
    connect(&m_pacman, &QProcess::finished, this, [this](int code, QProcess::ExitStatus how) {
        if (how == QProcess::NormalExit && code == 0) {
            qCInfo(lcUpdate) << "installed" << m_latest << "-- starting it";
            // The new copy would knock on this one's door and leave, so it
            // waits for this process to be gone first.
            QProcess::startDetached(
                QStringLiteral("/bin/sh"),
                {QStringLiteral("-c"),
                 QStringLiteral("while kill -0 %1 2>/dev/null; do sleep 0.2; done; exec %2 --show")
                     .arg(QCoreApplication::applicationPid())
                     .arg(Installed)});
            QCoreApplication::quit();
            return;
        }
        // 126 is pkexec's for a password window that was dismissed.
        const QString said = code == 126 ? QStringLiteral("cancelled")
                                         : QString::fromLocal8Bit(m_pacman.readAllStandardError()).trimmed();
        qCWarning(lcUpdate) << "pacman did not install it:" << code << said;
        set(Failed, said.right(400));
    });
}

bool Updater::canInstall() const
{
    return QCoreApplication::applicationFilePath() == Installed
           && !QStandardPaths::findExecutable(QStringLiteral("pkexec")).isEmpty()
           && !QStandardPaths::findExecutable(QStringLiteral("pacman")).isEmpty();
}

bool Updater::newer(const QString &candidate, const QString &current)
{
    return QVersionNumber::fromString(candidate) > QVersionNumber::fromString(current);
}

Updater::Package Updater::packageIn(const QJsonObject &release)
{
    // Named as makepkg names it, and nothing else: whatever else is attached
    // to a release is not for pacman.
    static const QRegularExpression package(
        QStringLiteral(R"(^wallflux-[0-9.]+-[0-9]+-x86_64\.pkg\.tar\.zst$)"));
    for (const QJsonValue &asset : release.value(QLatin1StringView("assets")).toArray()) {
        const QJsonObject one = asset.toObject();
        if (package.match(one.value(QLatin1StringView("name")).toString()).hasMatch()) {
            const QUrl url(one.value(QLatin1StringView("browser_download_url")).toString());
            // GitHub states every asset's SHA-256 as "sha256:<hex>"; a
            // package without one is not taken, since nothing could then
            // say it is the file that was published.
            static const QRegularExpression digest(QStringLiteral(R"(^sha256:([0-9a-f]{64})$)"));
            const auto said = digest.match(one.value(QLatin1StringView("digest")).toString());
            if (url.scheme() == QLatin1StringView("https") && url.host() == QLatin1StringView("github.com")
                && said.hasMatch())
                return {url, said.captured(1).toLatin1()};
        }
    }
    return {};
}

void Updater::set(State state, const QString &detail)
{
    m_state = state;
    m_detail = detail;
    emit changed();
}

void Updater::check()
{
    if (m_state == Checking || m_state == Downloading || m_state == Installing)
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
        m_package = packageIn(release);
        const bool later = newer(tag, QCoreApplication::applicationVersion());
        qCInfo(lcUpdate) << "the newest release is" << tag << (later ? "-- newer" : "-- this one");
        set(later ? Available : Current);
    });
}

void Updater::install()
{
    if (m_state != Available || !canInstall() || m_package.url.isEmpty())
        return;
    m_progress = 0;
    set(Downloading);
    QNetworkReply *reply = m_network.get(asking(m_package.url));
    connect(reply, &QNetworkReply::downloadProgress, this, [this](qint64 got, qint64 of) {
        if (of > 0) {
            m_progress = int(got * 100 / of);
            emit changed();
        }
    });
    connect(reply, &QNetworkReply::finished, this, [this, reply] {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            set(Failed, reply->errorString());
            return;
        }
        const QString folder = QStandardPaths::writableLocation(QStandardPaths::CacheLocation)
                               + QStringLiteral("/update");
        QDir().mkpath(folder);
        const QByteArray bytes = reply->readAll();
        // Checked here, so that a wrong file never gets as far as asking
        // for the password -- and again as root, below.
        const QByteArray got = QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex();
        if (got != m_package.sha256) {
            qCWarning(lcUpdate) << "the package is not the one published:" << got << "against"
                                << m_package.sha256;
            set(Failed, QStringLiteral("the download is not the published package"));
            return;
        }
        QFile file(folder + u'/' + m_package.url.fileName());
        if (!file.open(QIODevice::WriteOnly) || file.write(bytes) < 0) {
            set(Failed, file.errorString());
            return;
        }
        file.close();
        installFrom(file.fileName(), m_package.sha256);
    });
}

void Updater::installFrom(const QString &file, const QByteArray &sha256)
{
    set(Installing);
    qCInfo(lcUpdate) << "installing" << file;
    // The file sits in the user's own cache, where anything running as the
    // user could swap it between the check above and pacman reading it --
    // with the password already given. So root copies it somewhere only
    // root can write, checks it there against GitHub's checksum, and
    // installs that copy. The file and the sum are arguments, never part
    // of the script.
    static const QString script = QStringLiteral(
        "set -eu; dir=$(mktemp -d); trap 'rm -rf \"$dir\"' EXIT; "
        "cp -- \"$1\" \"$dir/wallflux.pkg.tar.zst\"; "
        "printf '%s  %s\\n' \"$2\" \"$dir/wallflux.pkg.tar.zst\" | sha256sum --check --quiet; "
        "pacman -U --noconfirm \"$dir/wallflux.pkg.tar.zst\"");
    m_pacman.start(QStringLiteral("pkexec"),
                   {QStringLiteral("/bin/sh"), QStringLiteral("-c"), script,
                    QStringLiteral("wallflux-update"), file, QString::fromLatin1(sha256)});
}
