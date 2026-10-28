// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "pointer.h"

#include <QSignalSpy>
#include <QTest>

/// What this side decides about the mouse: that it is only known while it
/// is watched, and that a position it already had is not news. The test
/// runs with no session bus to reach (see CMakeLists.txt), so nothing is
/// loaded into the compositor of whoever is running it.
class TestPointer : public QObject
{
    Q_OBJECT

private slots:
    void isKnownOnlyWhileWatched()
    {
        Pointer pointer;
        QSignalSpy moves(&pointer, &Pointer::positionChanged);

        // A call still on the bus from a script that has been unloaded.
        pointer.setPosition(10, 20);
        QVERIFY(!pointer.known());
        QCOMPARE(moves.count(), 0);

        pointer.setWatching(true);
        pointer.setPosition(10, 20);
        QVERIFY(pointer.known());
        QCOMPARE(pointer.position(), QPointF(10, 20));
        QCOMPARE(moves.count(), 1);

        // Every move is a binding re-evaluated on every screen.
        pointer.setPosition(10, 20);
        QCOMPARE(moves.count(), 1);

        pointer.setWatching(false);
        QVERIFY(!pointer.known());
        QCOMPARE(moves.count(), 2);
    }
};

QTEST_GUILESS_MAIN(TestPointer)
#include "tst_pointer.moc"
