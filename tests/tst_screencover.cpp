// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

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
        cover.setShownOn({QStringLiteral("eDP-1")});
        QSignalSpy changes(&cover, &ScreenCover::coveredChanged);

        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        QCOMPARE(changes.count(), 1);
        QVERIFY(cover.isCovered());

        cover.setCoveredScreens(QString());
        QCOMPARE(changes.count(), 2);
        QVERIFY(!cover.isCovered());
    }

    void forgetsTheWindowsWhileTheDesktopIsBeingShown()
    {
        ScreenCover cover;
        cover.setShownOn({QStringLiteral("eDP-1")});
        cover.setCoveredScreens(QStringLiteral("eDP-1"));
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
        cover.setShownOn({QStringLiteral("eDP-1")});
        QSignalSpy changes(&cover, &ScreenCover::coveredChanged);

        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        QVERIFY2(changes.count() == 1,
                 "the players were handed a change that was not one, which stops them");
    }

    /// One wallpaper on two screens is covered only when both are: a window
    /// filling the laptop leaves the monitor beside it in view.
    void isCoveredOnlyWhereEveryScreenIs()
    {
        ScreenCover cover;
        cover.setShownOn({QStringLiteral("eDP-1"), QStringLiteral("HDMI-A-1")});

        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        QVERIFY2(!cover.isCovered(), "stopped on a monitor nobody had covered");

        cover.setCoveredScreens(QStringLiteral("HDMI-A-1,eDP-1"));
        QVERIFY(cover.isCovered());

        // The screen with the window taken off the wallpaper.
        cover.setShownOn({QStringLiteral("HDMI-A-1")});
        QVERIFY(cover.isCovered());
        cover.setCoveredScreens(QStringLiteral("eDP-1"));
        QVERIFY(!cover.isCovered());
    }

    /// Another program's window with the keyboard is said once, and not
    /// while the desktop is being shown, which hands the keyboard to nobody.
    void saysWhenAnotherProgramHasTheKeyboard()
    {
        ScreenCover cover;
        QSignalSpy changes(&cover, &ScreenCover::focusedChanged);
        QVERIFY(!cover.isFocused());

        cover.setFocused(true);
        cover.setFocused(true);
        QVERIFY(cover.isFocused());
        QCOMPARE(changes.count(), 1);

        cover.setShowingDesktop(true);
        QVERIFY(!cover.isFocused());
        cover.setShowingDesktop(false);
        QVERIFY(cover.isFocused());

        cover.setFocused(false);
        QVERIFY(!cover.isFocused());
        QCOMPARE(changes.count(), 4);
    }
};

QTEST_MAIN(TestScreenCover)
#include "tst_screencover.moc"
