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
        Autostart autostart;
        autostart.setEnabled(false);
        QVERIFY(QFile::exists(Autostart::userEntry()));
        QVERIFY(!autostart.isEnabled());

        autostart.setEnabled(true);
        QVERIFY(autostart.isEnabled());

        QFile entry(Autostart::userEntry());
        QVERIFY(entry.open(QIODevice::ReadOnly));
        const QByteArray text = entry.readAll();
        QVERIFY(text.contains("Exec=wallflux"));
        QVERIFY(text.contains("Hidden=false"));
    }
};

QTEST_GUILESS_MAIN(TestAutostart)
#include "tst_autostart.moc"
