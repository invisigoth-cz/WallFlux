// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>

/// Whether the computer runs on its battery, and how full that is.
///
/// Asked of UPower on the system bus, which every desktop Linux has and
/// which KDE's own battery applet reads: `OnBattery` on the daemon, and
/// the "display device" -- all the batteries summed up as one -- for the
/// rest. A desktop has no battery and says so; without UPower this reports
/// mains power for ever and nothing else goes wrong.
class Power : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Power)
    QML_SINGLETON

    Q_PROPERTY(bool hasBattery READ hasBattery NOTIFY changed)
    Q_PROPERTY(bool onBattery READ onBattery NOTIFY changed)
    /// Per cent, nought to a hundred; meaningless without a battery.
    Q_PROPERTY(double percent READ percent NOTIFY changed)

public:
    explicit Power(QObject *parent = nullptr);

    bool hasBattery() const { return m_hasBattery; }
    bool onBattery() const { return m_onBattery; }
    double percent() const { return m_percent; }

signals:
    void changed();

private slots:
    /// UPower announces every change as a PropertiesChanged on the object
    /// that changed; whichever it was, all three are read again.
    void read();

private:
    bool m_hasBattery = false;
    bool m_onBattery = false;
    double m_percent = 100;
};
