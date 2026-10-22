// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "playlists.h"

#include <QSignalSpy>
#include <QStandardPaths>
#include <QTest>

/// What a playlist remembers, and how long a wallpaper stays.
///
/// The configuration directory is redirected into a temporary one, so this
/// never reads or writes the playlists somebody actually has.
class TestPlaylists : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase() { QStandardPaths::setTestModeEnabled(true); }

    void init()
    {
        // A clean slate per test: each one makes its own Playlists, and
        // the file is what they would otherwise share.
        Playlists first;
        const QVariantList all = first.all();
        for (const QVariant &one : all)
            first.forget(one.toMap().value(QStringLiteral("name")).toString());
    }

    void remembersSeconds()
    {
        {
            Playlists writing;
            writing.save(QStringLiteral("quick"), {QStringLiteral("1"), QStringLiteral("2")},
                         0, 0, 20, false);
        }

        Playlists reading;
        const QVariantMap one = reading.named(QStringLiteral("quick"));
        QCOMPARE(one.value(QStringLiteral("hours")).toInt(), 0);
        QCOMPARE(one.value(QStringLiteral("minutes")).toInt(), 0);
        QCOMPARE(one.value(QStringLiteral("seconds")).toInt(), 20);
    }

    /// Hours, minutes and seconds are added up rather than one of them
    /// winning, and nought is lifted to the floor rather than taken at its
    /// word.
    void addsTheThreeUpAndKeepsAFloor_data()
    {
        QTest::addColumn<int>("hours");
        QTest::addColumn<int>("minutes");
        QTest::addColumn<int>("seconds");
        QTest::addColumn<int>("stays");

        QTest::newRow("seconds alone") << 0 << 0 << 20 << 20;
        QTest::newRow("all three") << 1 << 2 << 3 << 3723;
        QTest::newRow("nought is the floor") << 0 << 0 << 0 << 5;
        QTest::newRow("under the floor is the floor") << 0 << 0 << 1 << 5;
    }

    void addsTheThreeUpAndKeepsAFloor()
    {
        QFETCH(int, hours);
        QFETCH(int, minutes);
        QFETCH(int, seconds);
        QFETCH(int, stays);

        Playlists playlists;
        playlists.save(QStringLiteral("timed"), {QStringLiteral("1"), QStringLiteral("2")},
                       hours, minutes, seconds, false);

        QSignalSpy plays(&playlists, &Playlists::plays);
        playlists.start(QStringLiteral("timed"));
        // The first wallpaper goes on at once; what is being measured is
        // how long until the second.
        QCOMPARE(plays.count(), 1);

        // Reading it off the playlist rather than off the timer, which is
        // private -- the two come from the same place.
        QCOMPARE(Playlists::staysFor(playlists.named(QStringLiteral("timed"))), stays);
        playlists.stop();
    }

    /// What the settings popup does with whatever was typed into its three
    /// boxes. Carrying the remainder rather than dropping it, and the
    /// whole thing clamped rather than the hours alone -- capping the
    /// hours on their own let an interval get *shorter* when somebody
    /// added to it.
    void turnsSecondsIntoAClock_data()
    {
        QTest::addColumn<int>("seconds");
        QTest::addColumn<int>("hours");
        QTest::addColumn<int>("minutes");
        QTest::addColumn<int>("rest");

        QTest::newRow("nothing") << 0 << 0 << 0 << 0;
        QTest::newRow("under a minute") << 59 << 0 << 0 << 59;
        QTest::newRow("exactly a minute") << 60 << 0 << 1 << 0;
        QTest::newRow("a minute and a half") << 90 << 0 << 1 << 30;
        QTest::newRow("ninety-five minutes") << 95 * 60 << 1 << 35 << 0;
        QTest::newRow("the longest there is") << 23 * 3600 + 59 * 60 + 59 << 23 << 59 << 59;
        QTest::newRow("longer than that") << 99 * 3600 << 23 << 59 << 59;
        QTest::newRow("below nothing") << -5 << 0 << 0 << 0;
    }

    void turnsSecondsIntoAClock()
    {
        QFETCH(int, seconds);
        QFETCH(int, hours);
        QFETCH(int, minutes);
        QFETCH(int, rest);

        Playlists playlists;
        const QVariantMap clock = playlists.asClock(seconds);
        QCOMPARE(clock.value(QStringLiteral("hours")).toInt(), hours);
        QCOMPARE(clock.value(QStringLiteral("minutes")).toInt(), minutes);
        QCOMPARE(clock.value(QStringLiteral("seconds")).toInt(), rest);
    }

    /// A playlist set to seconds really does turn, rather than the window
    /// having been told one thing and the timer another.
    void turnsWhenTheSecondsAreUp()
    {
        Playlists playlists;
        playlists.save(QStringLiteral("fast"), {QStringLiteral("a"), QStringLiteral("b")},
                       0, 0, 5, false);

        QSignalSpy plays(&playlists, &Playlists::plays);
        playlists.start(QStringLiteral("fast"));
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("a"));

        // Five seconds of suite time, deliberately. The one way to get
        // this wrong is the multiplier -- seconds read as minutes is a
        // playlist sixty times too slow, and nothing but a real turn
        // catches it.
        QVERIFY(plays.wait(7000));
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("b"));
        playlists.stop();
    }
};

QTEST_MAIN(TestPlaylists)
#include "tst_playlists.moc"
