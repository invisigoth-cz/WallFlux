// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QtQml/qqmlregistration.h>

/// Whether WallFlux starts when somebody logs in.
///
/// The package puts `wallflux.desktop` in `/etc/xdg/autostart`, which is
/// everybody or nobody. A file of the same name in the user's own
/// `~/.config/autostart` takes its place (the XDG autostart rules), and
/// `Hidden=true` in it means "not for me". So switching it off writes that
/// file, and switching it on writes it again saying the opposite -- which
/// also starts it where the package's file is missing, as in a build run
/// from the source tree.
class Autostart : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Autostart)
    QML_SINGLETON

    Q_PROPERTY(bool enabled READ isEnabled WRITE setEnabled NOTIFY enabledChanged)

public:
    explicit Autostart(QObject *parent = nullptr);

    bool isEnabled() const;
    void setEnabled(bool enabled);

    /// The user's own entry, which is the one this writes.
    static QString userEntry();

signals:
    void enabledChanged();
};
