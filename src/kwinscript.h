// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

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
