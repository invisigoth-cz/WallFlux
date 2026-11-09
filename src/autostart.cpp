// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "autostart.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QLoggingCategory>
#include <QSaveFile>
#include <QStandardPaths>

Q_LOGGING_CATEGORY(lcAutostart, "wallflux.autostart")

namespace {

const QString kName = QStringLiteral("autostart/wallflux.desktop");

/// Whether a desktop entry says it is hidden: `Hidden=true` in its
/// `[Desktop Entry]` group, which is the only group these files have.
bool saysHidden(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return false;
    while (!file.atEnd()) {
        const QByteArray line = file.readLine().trimmed();
        if (line.startsWith("Hidden="))
            return line.mid(7).trimmed().toLower() == "true";
    }
    return false;
}

} // namespace

Autostart::Autostart(QObject *parent)
    : QObject(parent)
{
}

QString Autostart::userEntry()
{
    return QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation) + u'/' + kName;
}

bool Autostart::isEnabled() const
{
    const QString own = userEntry();
    if (QFileInfo::exists(own))
        return !saysHidden(own);
    // No entry of the user's own: whatever the system's says, if there is one.
    const QStringList all = QStandardPaths::locateAll(QStandardPaths::GenericConfigLocation, kName);
    for (const QString &path : all) {
        if (path != own)
            return !saysHidden(path);
    }
    return false;
}

void Autostart::setEnabled(bool enabled)
{
    if (enabled == isEnabled())
        return;

    const QString path = userEntry();
    QDir().mkpath(QFileInfo(path).absolutePath());
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        qCWarning(lcAutostart) << "cannot write" << path << file.errorString();
        return;
    }
    // The same entry the package installs, so that switching it back on
    // starts it exactly as the package would have.
    file.write("[Desktop Entry]\n"
               "Type=Application\n"
               "Name=WallFlux\n"
               "Icon=preferences-desktop-wallpaper\n"
               "Exec=wallflux\n"
               "Terminal=false\n"
               "StartupNotify=false\n"
               "X-KDE-autostart-after=panel\n");
    file.write(enabled ? "Hidden=false\n" : "Hidden=true\n");
    if (!file.commit()) {
        qCWarning(lcAutostart) << "cannot write" << path << file.errorString();
        return;
    }
    emit enabledChanged();
}
