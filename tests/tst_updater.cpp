// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <QJsonArray>
#include <QJsonObject>
#include <QTest>

#include "updater.h"

/// What a release is newer than, and which of its files is the package.
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

    void takesThePackageAndNothingElse()
    {
        const auto asset = [](const char *name, const char *url) {
            return QJsonObject{{QStringLiteral("name"), QString::fromLatin1(name)},
                               {QStringLiteral("browser_download_url"), QString::fromLatin1(url)}};
        };
        const QJsonObject release{{QStringLiteral("assets"), QJsonArray{
            asset("notes.txt", "https://example.org/notes.txt"),
            asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst.sig", "https://example.org/a.sig"),
            asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst", "https://example.org/w.pkg.tar.zst")}}};
        QCOMPARE(Updater::packageIn(release), QUrl(QStringLiteral("https://example.org/w.pkg.tar.zst")));

        // Not over plain http, which anybody on the way could swap.
        const QJsonObject plain{{QStringLiteral("assets"), QJsonArray{
            asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst", "http://example.org/w")}}};
        QVERIFY(Updater::packageIn(plain).isEmpty());
        QVERIFY(Updater::packageIn({}).isEmpty());
    }
};

QTEST_GUILESS_MAIN(TestUpdater)
#include "tst_updater.moc"
