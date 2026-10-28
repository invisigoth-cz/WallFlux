// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "kwinscript.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDir>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcKWin, "wallflux.kwin")

namespace {

QDBusInterface scripting()
{
    return QDBusInterface(QStringLiteral("org.kde.KWin"), QStringLiteral("/Scripting"),
                          QStringLiteral("org.kde.kwin.Scripting"),
                          QDBusConnection::sessionBus());
}

} // namespace

KWinScript::KWinScript(QString name)
    : m_name(std::move(name))
{
    m_file.setFileTemplate(QDir::tempPath() + QStringLiteral("/") + m_name
                           + QStringLiteral("-XXXXXX.js"));
}

KWinScript::~KWinScript()
{
    unload();
}

bool KWinScript::load(const QString &source)
{
    QDBusInterface kwin = scripting();
    if (!kwin.isValid()) {
        qCInfo(lcKWin) << "no KWin to run" << m_name << "in (on wlroots compositors this is"
                       << "expected)";
        return false;
    }

    if (!m_file.isOpen() && !m_file.open()) {
        qCWarning(lcKWin) << "cannot write" << m_name << "--" << m_file.errorString();
        return false;
    }
    m_file.resize(0);
    m_file.write(source.toUtf8());
    // KWin reads the file when it is asked to load it, so what is written
    // has to be on disk by then rather than sitting in a buffer.
    m_file.flush();

    // Unloaded first, every time. Asked to load a script it already has
    // under this name, KWin hands back the one that is loaded and reads
    // nothing -- and after a restart of this program that would be a script
    // still calling the bus name of the copy that is gone.
    kwin.call(QStringLiteral("unloadScript"), m_name);

    const QDBusReply<int> loaded = kwin.call(QStringLiteral("loadScript"), m_file.fileName(), m_name);
    if (!loaded.isValid()) {
        qCWarning(lcKWin) << "KWin would not take" << m_name << "--" << loaded.error().message();
        return false;
    }
    // ponytail: the script is not reloaded if KWin restarts. On Wayland a
    // KWin that goes away takes the session with it, so there is nothing to
    // come back to. Watch the service here if that ever stops being true.
    const QDBusReply<void> started = kwin.call(QStringLiteral("start"));
    if (!started.isValid()) {
        qCWarning(lcKWin) << "KWin would not start" << m_name << "--" << started.error().message();
        return false;
    }
    return true;
}

void KWinScript::unload()
{
    QDBusInterface kwin = scripting();
    if (kwin.isValid())
        kwin.call(QStringLiteral("unloadScript"), m_name);
}
