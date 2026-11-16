// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <QTest>

#include "updater.h"

/// What a release is newer than.
class TestUpdater : public QObject
{
    Q_OBJECT

private slots:
    void comparesNumbersNotText()
    {
        QVERIFY(Updater::newer(QStringLiteral("0.33.0"), QStringLiteral("0.32.0")));
        QVERIFY(Updater::newer(QStringLiteral("0.10.0"), QStringLiteral("0.9.0")));
        QVERIFY(Updater::newer(QStringLiteral("1.0.0"), QStringLiteral("0.99.9")));
        QVERIFY(!Updater::newer(QStringLiteral("0.32.0"), QStringLiteral("0.32.0")));
        QVERIFY(!Updater::newer(QStringLiteral("0.31.0"), QStringLiteral("0.32.0")));
    }
};

QTEST_GUILESS_MAIN(TestUpdater)
#include "tst_updater.moc"
