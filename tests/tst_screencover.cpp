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
    void startsWithNothingCovered()
    {
        ScreenCover cover;
        QVERIFY(cover.coveredScreens().isEmpty());
    }

    /// Each screen a window covers, by name, so that each monitor's
    /// wallpaper can stand still on its own -- and said once per change.
    void listsTheCoveredScreensAndNoneWhileTheDesktopIsShown()
    {
        ScreenCover cover;
        QSignalSpy changes(&cover, &ScreenCover::coveredScreensChanged);

        cover.setCoveredScreens(QStringLiteral("eDP-1,HDMI-A-1"));
        QCOMPARE(cover.coveredScreens(), (QStringList{"eDP-1", "HDMI-A-1"}));
        QCOMPARE(changes.count(), 1);

        cover.setCoveredScreens(QStringLiteral("eDP-1,HDMI-A-1"));
        QVERIFY2(changes.count() == 1,
                 "the players were handed a change that was not one, which stops them");

        cover.setShowingDesktop(true);
        QVERIFY2(cover.coveredScreens().isEmpty(),
                 "somebody asked to see the wallpaper and it stayed stopped");

        cover.setShowingDesktop(false);
        QCOMPARE(cover.coveredScreens(), (QStringList{"eDP-1", "HDMI-A-1"}));
        QCOMPARE(changes.count(), 3);

        cover.setCoveredScreens(QString());
        QVERIFY(cover.coveredScreens().isEmpty());
        QCOMPARE(changes.count(), 4);
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
