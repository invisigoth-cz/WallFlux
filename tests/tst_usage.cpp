// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <QSignalSpy>
#include <QTest>

#include "usage.h"

/// Reading what /proc says, and a reading of this very process.
class TestUsage : public QObject
{
    Q_OBJECT

private slots:
    /// A name with a space and a bracket in it does not shift the fields.
    void readsAStatWhoseNameHasBrackets()
    {
        const QByteArray stat = "4242 (Web Content (x)) S 4200 4242 4242 0 -1 4194560 1 0 0 0 "
                                "150 50 0 0 20 0 1 0 100 1000 200";
        qint64 parent = 0, ticks = 0;
        QVERIFY(Usage::readStat(stat, &parent, &ticks));
        QCOMPARE(parent, 4200);
        QCOMPARE(ticks, 200);
    }

    void readsResidentMemory()
    {
        QCOMPARE(Usage::residentKb("Name:\twallflux\nVmPeak:\t 900 kB\nVmRSS:\t  420108 kB\n"), 420108);
        QCOMPARE(Usage::residentKb("Name:\tkthread\n"), 0);
    }

    /// amdgpu's own words: a client, and nanoseconds an engine was busy.
    void readsAGraphicsClient()
    {
        QHash<QString, qint64> engines;
        const QString client = Usage::drmClient("pos:\t0\nflags:\t02100002\ndrm-driver:\tamdgpu\n"
                                                "drm-client-id:\t1234\ndrm-engine-gfx:\t5000000 ns\n"
                                                "drm-engine-compute:\t0 ns\ndrm-memory-vram:\t1024 KiB\n",
                                                &engines);
        QCOMPARE(client, QStringLiteral("1234"));
        QCOMPARE(engines.value(QStringLiteral("gfx")), 5000000);
        QCOMPARE(engines.value(QStringLiteral("compute")), 0);
        QCOMPARE(engines.size(), 2);

        QHash<QString, qint64> none;
        QVERIFY(Usage::drmClient("pos:\t0\nflags:\t02\n", &none).isEmpty());
    }

    /// This process holds some memory and the numbers come through.
    void readsItself()
    {
        Usage usage;
        QSignalSpy changed(&usage, &Usage::changed);
        usage.setWatching(true);
        QVERIFY(changed.wait(2000));
        QVERIFY(usage.ram() > 1);
        QVERIFY(usage.cpu() >= 0);
        usage.setWatching(false);
    }
};

QTEST_GUILESS_MAIN(TestUsage)
#include "tst_usage.moc"
