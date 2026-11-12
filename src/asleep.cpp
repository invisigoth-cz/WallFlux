// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "asleep.h"

#include <KWayland/Client/connection_thread.h>
#include <KWayland/Client/dpms.h>
#include <KWayland/Client/output.h>
#include <KWayland/Client/registry.h>
#include <QDBusConnection>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QLoggingCategory>
#include <algorithm>

Q_LOGGING_CATEGORY(lcAsleep, "wallflux.asleep")

using namespace KWayland::Client;

namespace {
const QString Saver = QStringLiteral("org.freedesktop.ScreenSaver");
const QString SaverPath = QStringLiteral("/org/freedesktop/ScreenSaver");
}

Asleep::Asleep(QObject *parent)
    : QObject(parent)
{
    // The lock, from the screen saver. Connected by its name, so only that
    // service's signal is heard.
    QDBusConnection bus = QDBusConnection::sessionBus();
    bus.connect(Saver, SaverPath, Saver, QStringLiteral("ActiveChanged"), this, SLOT(setLocked(bool)));
    auto *asked = new QDBusPendingCallWatcher(
        bus.asyncCall(QDBusMessage::createMethodCall(Saver, SaverPath, Saver, QStringLiteral("GetActive"))),
        this);
    connect(asked, &QDBusPendingCallWatcher::finished, this, [this](QDBusPendingCallWatcher *call) {
        const QDBusPendingReply<bool> reply = *call;
        if (reply.isValid())
            setLocked(reply.value());
        call->deleteLater();
    });

    // The screens, from KWin, where there is a Wayland connection at all.
    ConnectionThread *connection = ConnectionThread::fromApplication(this);
    if (!connection) {
        qCInfo(lcAsleep) << "not on Wayland: only the lock is watched";
        return;
    }
    m_registry = new Registry(this);
    m_registry->create(connection);
    connect(m_registry, &Registry::interfacesAnnounced, this, [this] {
        const Registry::AnnouncedInterface dpms = m_registry->interface(Registry::Interface::Dpms);
        if (dpms.name == 0) {
            qCInfo(lcAsleep) << "the compositor says nothing of screens switched off";
            return;
        }
        m_manager = m_registry->createDpmsManager(dpms.name, dpms.version, this);
        for (const Registry::AnnouncedInterface &output : m_registry->interfaces(Registry::Interface::Output))
            watch(output.name, output.version);
    });
    // A monitor plugged in later.
    connect(m_registry, &Registry::outputAnnounced, this, [this](quint32 name, quint32 version) {
        if (m_manager)
            watch(name, version);
    });
    m_registry->setup();
}

void Asleep::watch(quint32 name, quint32 version)
{
    Output *output = m_registry->createOutput(name, version, this);
    Dpms *dpms = m_manager->getDpms(output, output);
    m_screens.append(dpms);
    connect(dpms, &Dpms::modeChanged, this, &Asleep::settle);
    connect(dpms, &Dpms::supportedChanged, this, &Asleep::settle);
    connect(output, &Output::removed, this, [this, output, dpms] {
        m_screens.removeOne(dpms);
        output->deleteLater();
        settle();
    });
    settle();
}

void Asleep::settle()
{
    // Dark only when every screen is: one still lit is a wallpaper somebody
    // can see. The mode is believed whatever `isSupported` says: KWin's
    // virtual outputs say false and report Off all the same.
    const bool dark = !m_screens.isEmpty() && std::ranges::all_of(m_screens, [](const Dpms *one) {
        return one->mode() != Dpms::Mode::On;
    });
    if (dark == m_dark)
        return;
    m_dark = dark;
    qCInfo(lcAsleep) << (dark ? "every screen is off" : "a screen is on again");
    emit changed();
}

void Asleep::setLocked(bool locked)
{
    if (locked == m_locked)
        return;
    const bool was = asleep();
    m_locked = locked;
    qCInfo(lcAsleep) << (locked ? "the session is locked" : "the session is unlocked");
    if (asleep() != was)
        emit changed();
}
