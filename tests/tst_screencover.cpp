// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "screencover.h"

#include <QSignalSpy>
#include <QTest>

/// The deciding is done by the script inside KWin, which a test cannot ask
/// anything of; what is tested here is the one thing on this side that can
/// be got wrong -- saying nothing when the answer has not changed.
///
/// The test runs with no session bus to reach (see CMakeLists.txt), so
/// nothing is loaded into the compositor of whoever is running it.
class TestScreenCover : public QObject
{
    Q_OBJECT

private slots:
    void startsWithTheWallpaperInView()
    {
        ScreenCover cover;
        QVERIFY(!cover.isCovered());
    }

    void reportsEachChangeOnce()
    {
        ScreenCover cover;
        QSignalSpy changes(&cover, &ScreenCover::coveredChanged);

        cover.setCovered(true);
        QCOMPARE(changes.count(), 1);
        QVERIFY(cover.isCovered());

        cover.setCovered(false);
        QCOMPARE(changes.count(), 2);
        QVERIFY(!cover.isCovered());
    }

    void forgetsTheWindowsWhileTheDesktopIsBeingShown()
    {
        ScreenCover cover;
        cover.setCovered(true);
        QVERIFY(cover.isCovered());

        cover.setShowingDesktop(true);
        QVERIFY2(!cover.isCovered(),
                 "somebody asked to see the wallpaper and it stayed stopped");

        cover.setShowingDesktop(false);
        QVERIFY2(cover.isCovered(),
                 "the window in front of the wallpaper was forgotten about");
    }

    void staysQuietWhenTheAnswerIsTheSame()
    {
        ScreenCover cover;
        QSignalSpy changes(&cover, &ScreenCover::coveredChanged);

        cover.setCovered(true);
        cover.setCovered(true);
        cover.setCovered(true);
        QVERIFY2(changes.count() == 1,
                 "the players were handed a change that was not one, which stops them");
    }
};

QTEST_MAIN(TestScreenCover)
#include "tst_screencover.moc"
