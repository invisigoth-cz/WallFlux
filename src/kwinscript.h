// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QDBusConnectionInterface>
#include <QDBusContext>
#include <QDBusReply>

#include <QString>
#include <QTemporaryFile>

/// A script run inside KWin on this program's behalf, which is how a
/// Wayland client learns what only the compositor knows: which windows are
/// where, and where the mouse is.
///
/// Loaded over D-Bus from a file, under a name of its own, and unloaded
/// again when this goes. Where there is no KWin, loading says so and
/// nothing else happens.
class KWinScript
{
public:
    explicit KWinScript(QString name);
    ~KWinScript();

    KWinScript(const KWinScript &) = delete;
    KWinScript &operator=(const KWinScript &) = delete;

    /// Writes `source` out and has KWin run it, in place of any script of
    /// the same name. False where there is no KWin or it would not.
    bool load(const QString &source);
    void unload();

private:
    QString m_name;
    /// KWin loads a script from a path, so the script has to be a real
    /// file, and one that stays put while it runs.
    QTemporaryFile m_file;
};

/// Whether a call came from KWin, or was not made over D-Bus at all.
///
/// What the scripts call back is on the session bus, where any program of
/// the session could call it too -- to stop the wallpaper as if covered, or
/// move a scene's mouse. Only KWin's own connection is listened to. Its
/// name is remembered, and looked up again only when a call comes from
/// another, so a mouse moving sixty times a second costs no lookups.
inline bool fromKWin(const QDBusContext &call)
{
    if (!call.calledFromDBus())
        return true;
    static QString kwin;
    const QString sender = call.message().service();
    if (!kwin.isEmpty() && sender == kwin)
        return true;
    const QDBusReply<QString> owner =
        call.connection().interface()->serviceOwner(QStringLiteral("org.kde.KWin"));
    kwin = owner.isValid() ? owner.value() : QString();
    if (!kwin.isEmpty() && sender == kwin)
        return true;
    qWarning("a D-Bus call from %s, which is not KWin: refused", qPrintable(sender));
    return false;
}
