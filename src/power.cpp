// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "power.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcPower, "wallflux.power")

namespace {

const QString Service = QStringLiteral("org.freedesktop.UPower");
const QString Daemon = QStringLiteral("/org/freedesktop/UPower");
const QString Display = QStringLiteral("/org/freedesktop/UPower/devices/DisplayDevice");
/// UPower's number for a battery among the kinds of device it knows.
constexpr uint Battery = 2;

} // namespace

Power::Power(QObject *parent)
    : QObject(parent)
{
    QDBusConnection bus = QDBusConnection::systemBus();
    if (!bus.isConnected()) {
        qCInfo(lcPower) << "no system bus, so no battery is known of";
        return;
    }
    for (const QString &path : {Daemon, Display}) {
        bus.connect(Service, path, QStringLiteral("org.freedesktop.DBus.Properties"),
                    QStringLiteral("PropertiesChanged"), this, SLOT(read()));
    }
    read();
}

void Power::read()
{
    QDBusConnection bus = QDBusConnection::systemBus();
    QDBusInterface daemon(Service, Daemon, Service, bus);
    QDBusInterface display(Service, Display, Service + QStringLiteral(".Device"), bus);

    const bool hasBattery = display.property("IsPresent").toBool()
                            && display.property("Type").toUInt() == Battery;
    const bool onBattery = daemon.property("OnBattery").toBool();
    const double percent = hasBattery ? display.property("Percentage").toDouble() : 100;

    if (hasBattery == m_hasBattery && onBattery == m_onBattery && qFuzzyCompare(percent, m_percent))
        return;
    m_hasBattery = hasBattery;
    m_onBattery = onBattery;
    m_percent = percent;
    qCDebug(lcPower) << "battery" << hasBattery << "on it" << onBattery << percent << "%";
    emit changed();
}
