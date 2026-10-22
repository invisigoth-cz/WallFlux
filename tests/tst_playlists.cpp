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
                         {{QStringLiteral("seconds"), 20},
                          {QStringLiteral("videoEnds"), true},
                          {QStringLiteral("alwaysFirst"), true}});
        }

        Playlists reading;
        const QVariantMap one = reading.named(QStringLiteral("quick"));
        QCOMPARE(one.value(QStringLiteral("hours")).toInt(), 0);
        QCOMPARE(one.value(QStringLiteral("minutes")).toInt(), 0);
        QCOMPARE(one.value(QStringLiteral("seconds")).toInt(), 20);
        QCOMPARE(one.value(QStringLiteral("videoEnds")).toBool(), true);
        QCOMPARE(one.value(QStringLiteral("whilePaused")).toBool(), false);
        QCOMPARE(one.value(QStringLiteral("alwaysFirst")).toBool(), true);
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
                       {{QStringLiteral("hours"), hours},
                        {QStringLiteral("minutes"), minutes},
                        {QStringLiteral("seconds"), seconds}});

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
    /// boxes. Carrying the remainder rather than dropping it; the whole
    /// thing clamped rather than the hours alone -- capping the hours on
    /// their own let an interval get *shorter* when somebody added to it
    /// -- and lifted to the floor at the other end, so the boxes show the
    /// five seconds that will actually be waited.
    void turnsSecondsIntoAClock_data()
    {
        QTest::addColumn<int>("seconds");
        QTest::addColumn<int>("hours");
        QTest::addColumn<int>("minutes");
        QTest::addColumn<int>("rest");

        QTest::newRow("nothing is the floor") << 0 << 0 << 0 << 5;
        QTest::newRow("under the floor is the floor") << 3 << 0 << 0 << 5;
        QTest::newRow("under a minute") << 59 << 0 << 0 << 59;
        QTest::newRow("exactly a minute") << 60 << 0 << 1 << 0;
        QTest::newRow("a minute and a half") << 90 << 0 << 1 << 30;
        QTest::newRow("ninety-five minutes") << 95 * 60 << 1 << 35 << 0;
        QTest::newRow("the longest there is") << 23 * 3600 + 59 * 60 + 59 << 23 << 59 << 59;
        QTest::newRow("longer than that") << 99 * 3600 << 23 << 59 << 59;
        QTest::newRow("below nothing") << -5 << 0 << 0 << 5;
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

    /// "Always begin with the first wallpaper" holds index nought out of
    /// the shuffle and deals the rest. Twenty goes, because a shuffle that
    /// happened to leave the first one in place would pass a single run
    /// whatever it did.
    void alwaysFirstSurvivesTheShuffle()
    {
        const QStringList ids{QStringLiteral("first"), QStringLiteral("b"),
                              QStringLiteral("c"), QStringLiteral("d"),
                              QStringLiteral("e"), QStringLiteral("f")};

        Playlists playlists;
        playlists.save(QStringLiteral("held"), ids,
                       {{QStringLiteral("shuffle"), true},
                        {QStringLiteral("alwaysFirst"), true}});

        for (int go = 0; go < 20; ++go) {
            QSignalSpy plays(&playlists, &Playlists::plays);
            playlists.start(QStringLiteral("held"));
            QCOMPARE(plays.count(), 1);
            QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("first"));
            playlists.stop();
        }
    }

    /// Without it, the first wallpaper moves at least once in twenty goes.
    /// Not an assertion about randomness so much as about the setting
    /// doing something: with the hold-back broken both tests would pass
    /// only if this one failed.
    void shufflingWithoutItMovesTheFirstOne()
    {
        const QStringList ids{QStringLiteral("first"), QStringLiteral("b"),
                              QStringLiteral("c"), QStringLiteral("d"),
                              QStringLiteral("e"), QStringLiteral("f")};

        Playlists playlists;
        playlists.save(QStringLiteral("loose"), ids,
                       {{QStringLiteral("shuffle"), true}});

        bool moved = false;
        for (int go = 0; go < 20 && !moved; ++go) {
            QSignalSpy plays(&playlists, &Playlists::plays);
            playlists.start(QStringLiteral("loose"));
            moved = plays.takeFirst().at(0).toString() != QStringLiteral("first");
            playlists.stop();
        }
        QVERIFY(moved);
    }

    /// Holding stops the clock and letting go picks it up where it was,
    /// rather than starting the turn again.
    ///
    /// Real seconds, and so most of what this suite costs to run. It is
    /// the only thing that catches the two ways this goes wrong: the
    /// multiplier, where seconds read as minutes make a playlist sixty
    /// times too slow, and a hold that quietly restarts the turn instead
    /// of resuming it.
    void holdingKeepsWhatWasLeft()
    {
        Playlists playlists;
        playlists.save(QStringLiteral("held"), {QStringLiteral("a"), QStringLiteral("b")},
                       {{QStringLiteral("seconds"), 5}});

        QSignalSpy plays(&playlists, &Playlists::plays);
        playlists.start(QStringLiteral("held"));
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("a"));

        // Three seconds in, two to go.
        QTest::qWait(3000);
        playlists.hold(true);

        // Held, so the two seconds never run out however long it sits.
        QVERIFY(!plays.wait(3000));
        QCOMPARE(plays.count(), 0);

        // And the two seconds are still two, not five.
        playlists.hold(false);
        QVERIFY(plays.wait(3000));
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("b"));
        playlists.stop();
    }

    /// A video reaching its end is `next()`, and it winds the clock again
    /// rather than leaving the old one to go off a moment later.
    void nextMovesOnAndRewindsTheClock()
    {
        Playlists playlists;
        playlists.save(QStringLiteral("ends"), {QStringLiteral("a"), QStringLiteral("b")},
                       {{QStringLiteral("seconds"), 5}});

        QSignalSpy plays(&playlists, &Playlists::plays);
        playlists.start(QStringLiteral("ends"));
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("a"));

        QTest::qWait(3000);
        playlists.next();
        QCOMPARE(plays.count(), 1);
        QCOMPARE(plays.takeFirst().at(0).toString(), QStringLiteral("b"));

        // Five seconds from now, not the two that were left: without the
        // rewind this would turn again almost at once.
        QVERIFY(!plays.wait(3000));
        playlists.stop();
    }

    /// What is running says what it was told, and says nothing once it
    /// has stopped -- the window reads these to decide whether a video
    /// loops and whether a covered screen holds the clock.
    void saysWhatTheRunningPlaylistWasTold()
    {
        Playlists playlists;
        playlists.save(QStringLiteral("told"), {QStringLiteral("a")},
                       {{QStringLiteral("seconds"), 30},
                        {QStringLiteral("videoEnds"), true},
                        {QStringLiteral("whilePaused"), true}});

        QCOMPARE(playlists.changesOnVideoEnd(), false);
        QCOMPARE(playlists.changesWhilePaused(), false);

        playlists.start(QStringLiteral("told"));
        QCOMPARE(playlists.changesOnVideoEnd(), true);
        QCOMPARE(playlists.changesWhilePaused(), true);

        playlists.stop();
        QCOMPARE(playlists.changesOnVideoEnd(), false);
        QCOMPARE(playlists.changesWhilePaused(), false);
    }
};

QTEST_MAIN(TestPlaylists)
#include "tst_playlists.moc"
