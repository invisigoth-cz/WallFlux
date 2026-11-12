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
        const QString sum = QStringLiteral("a996beb0ca3a8ac33f0773c3b4fdc0e48eac9c1076f3b6ef4566652663a5158e");
        const auto asset = [&](const char *name, const char *url, bool summed = true) {
            QJsonObject one{{QStringLiteral("name"), QString::fromLatin1(name)},
                            {QStringLiteral("browser_download_url"), QString::fromLatin1(url)}};
            if (summed)
                one.insert(QStringLiteral("digest"), QStringLiteral("sha256:") + sum);
            return one;
        };
        const char *from = "https://github.com/invisigoth-cz/WallFlux/releases/download/v0.33.0/w.pkg.tar.zst";
        const QJsonObject release{{QStringLiteral("assets"), QJsonArray{
            asset("notes.txt", "https://github.com/x/notes.txt"),
            asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst.sig", "https://github.com/x/a.sig"),
            asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst", from)}}};
        const Updater::Package found = Updater::packageIn(release);
        QCOMPARE(found.url, QUrl(QString::fromLatin1(from)));
        QCOMPARE(found.sha256, sum.toLatin1());

        // Not over plain http, which anybody on the way could swap; not from
        // anywhere but GitHub; and not without the checksum GitHub states.
        const auto alone = [&](QJsonObject one) {
            return QJsonObject{{QStringLiteral("assets"), QJsonArray{one}}};
        };
        QVERIFY(Updater::packageIn(alone(asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst",
                                               "http://github.com/w"))).url.isEmpty());
        QVERIFY(Updater::packageIn(alone(asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst",
                                               "https://example.org/w"))).url.isEmpty());
        QVERIFY(Updater::packageIn(alone(asset("wallflux-0.33.0-1-x86_64.pkg.tar.zst", from,
                                               false))).url.isEmpty());
        QVERIFY(Updater::packageIn({}).url.isEmpty());
    }
};

QTEST_GUILESS_MAIN(TestUpdater)
#include "tst_updater.moc"
