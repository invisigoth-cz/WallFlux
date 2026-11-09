// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <QTest>

#include "power.h"

/// Without UPower there is no battery and the mains never fail: nothing
/// is held or turned down for a reason nobody can see.
class TestPower : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        // Nowhere, so the answer is the one for a machine without UPower
        // whatever this one has.
        qputenv("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/wallflux-test");
    }

    void saysMainsWithoutUPower()
    {
        Power power;
        QVERIFY(!power.hasBattery());
        QVERIFY(!power.onBattery());
        QCOMPARE(power.percent(), 100.0);
    }
};

QTEST_GUILESS_MAIN(TestPower)
#include "tst_power.moc"
