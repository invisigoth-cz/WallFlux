// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <QFile>
#include <QStandardPaths>
#include <QTest>

#include "autostart.h"

/// Starting with a login, switched by the user's own entry. Run in Qt's test
/// mode, so the entry lands under ~/.qttest and the real one is left alone.
class TestAutostart : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        QStandardPaths::setTestModeEnabled(true);
        QFile::remove(Autostart::userEntry());
    }

    void cleanupTestCase() { QFile::remove(Autostart::userEntry()); }

    /// Off writes an entry saying hidden; on writes it again saying not.
    void switchesWithTheUsersOwnEntry()
    {
        // Off and then on, so the entry is written whatever the machine has:
        // with WallFlux installed its system entry says on and only off
        // writes; on a clean build machine there is none, it is off already
        // and only on writes.
        Autostart autostart;
        autostart.setEnabled(false);
        QVERIFY(!autostart.isEnabled());
        autostart.setEnabled(true);
        QVERIFY(autostart.isEnabled());

        QFile entry(Autostart::userEntry());
        QVERIFY(entry.open(QIODevice::ReadOnly));
        const QByteArray text = entry.readAll();
        QVERIFY(text.contains("Exec=wallflux"));
        QVERIFY(text.contains("Hidden=false"));
        entry.close();

        // Off is the entry kept and saying hidden, not taken away: taken
        // away, the system's entry would start it all the same.
        autostart.setEnabled(false);
        QVERIFY(!autostart.isEnabled());
        QVERIFY(QFile::exists(Autostart::userEntry()));
    }
};

QTEST_GUILESS_MAIN(TestAutostart)
#include "tst_autostart.moc"
