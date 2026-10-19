// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>

/// Whether this machine can be asked about the Steam Workshop, and the way in.
///
/// Nothing of Valve's is shipped with WallFlux and nothing is linked against
/// at build time. The library is looked for when it is first wanted and
/// opened with dlopen; where it is not there, this reports as much and the
/// interface falls back to sending somebody to the Steam client by hand.
///
/// Why not ship it: the licence that travels with the Steamworks SDK permits
/// redistributing it only in its entirety, unmodified, not integrated with
/// another program, and with its own agreement put in front of the end user
/// -- and the agreement that actually governs the SDK is behind a partner
/// login and cannot be read. Somebody who has the SDK accepted those terms
/// themselves; WallFlux stays out of it by never being the one to hand it on.
///
/// The GPL exception in LICENSE-EXCEPTION is what makes even this much
/// allowed from our side.
class Steamworks : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Steamworks)
    QML_SINGLETON

    /// Whether Steam answered and the workshop interface is there to ask.
    Q_PROPERTY(bool available READ isAvailable CONSTANT)

    /// Why not, in words an interface can show. Empty while all is well.
    Q_PROPERTY(QString trouble READ trouble CONSTANT)

public:
    explicit Steamworks(QObject *parent = nullptr);
    ~Steamworks() override;

    bool isAvailable() const { return m_ugc != nullptr; }
    QString trouble() const { return m_trouble; }

private:
    /// Opens libsteam_api.so from wherever it is, or says where it looked.
    bool open();

    void *m_library = nullptr;
    /// The ISteamUGC pointer, which is what everything about the workshop is
    /// asked through. Null until open() has worked.
    void *m_ugc = nullptr;
    QString m_trouble;

    void (*m_shutdown)() = nullptr;
};
