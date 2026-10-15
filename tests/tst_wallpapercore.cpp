// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "wallpapercore.h"

#include <QDir>
#include <QFile>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

namespace {

/// Writes one wallpaper folder into a pretend Steam install.
void makeWallpaper(const QString &root, const QString &id, const QByteArray &projectJson,
                   const QString &alsoWrite = {})
{
    const QString folder =
        root + QStringLiteral("/steamapps/workshop/content/431960/") + id;
    QVERIFY(QDir().mkpath(folder));

    QFile project(folder + QStringLiteral("/project.json"));
    QVERIFY(project.open(QIODevice::WriteOnly));
    QCOMPARE(project.write(projectJson), projectJson.size());
    project.close();

    if (!alsoWrite.isEmpty()) {
        QFile entry(folder + QLatin1Char('/') + alsoWrite);
        QVERIFY(entry.open(QIODevice::WriteOnly));
        entry.write("<html></html>");
    }
}

} // namespace

class TestWallpaperCore : public QObject
{
    Q_OBJECT

private slots:
    /// The things a scan has to get right, on one made-up Steam install.
    void readsWhatIsThere();

    /// A `file` that climbs out of the wallpaper's folder is refused. The
    /// name comes from downloaded content, so this is a boundary, not a
    /// nicety.
    void refusesAFileOutsideTheWallpaper();

    /// Selecting is by id, and an id nobody has changes nothing.
    void selectsByIdAndRefusesUnknownOnes();
};

void TestWallpaperCore::readsWhatIsThere()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"A web one","type":"web","file":"index.html"})",
                  QStringLiteral("index.html"));
    // The library spells the type both ways; both have to land as Scene.
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"B capital scene","type":"Scene","file":"scene.json"})");
    // No title: the id stands in rather than the entry being dropped.
    makeWallpaper(steam.path(), QStringLiteral("333"), R"({"type":"video","file":"a.mp4"})",
                  QStringLiteral("a.mp4"));
    // Not JSON at all. Must be skipped without taking the scan down.
    makeWallpaper(steam.path(), QStringLiteral("444"), "{ this is not json");

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 3);

    // Sorted by title, and digits sort before letters, so the untitled one
    // -- listed under its id -- comes first.
    QCOMPARE(found[0].id, QStringLiteral("333"));
    QCOMPARE(found[0].title, QStringLiteral("333"));
    QCOMPARE(found[0].kind, WallpaperKind::Video);
    QVERIFY(found[0].hasFile());

    QCOMPARE(found[1].title, QStringLiteral("A web one"));
    QCOMPARE(found[1].kind, WallpaperKind::Web);
    QVERIFY(found[1].source.isLocalFile());
    QVERIFY(found[1].source.toLocalFile().endsWith(QStringLiteral("/111/index.html")));
    QVERIFY(found[1].hasFile());

    QCOMPARE(found[2].title, QStringLiteral("B capital scene"));
    QCOMPARE(found[2].kind, WallpaperKind::Scene);
    // scene.json lives inside scene.pkg, so there is nothing to point at.
    QVERIFY(found[2].source.isEmpty());
    QVERIFY(!found[2].hasFile());
}

void TestWallpaperCore::refusesAFileOutsideTheWallpaper()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    // A real file to aim at, so the test fails for the right reason: it is
    // refused because it is outside, not because it is missing.
    QFile secret(steam.path() + QStringLiteral("/secret.html"));
    QVERIFY(secret.open(QIODevice::WriteOnly));
    secret.write("<html>private</html>");
    secret.close();

    makeWallpaper(steam.path(), QStringLiteral("555"),
                  R"({"title":"Escaper","type":"web",)"
                  R"("file":"../../../../secret.html"})");

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 1);
    QCOMPARE(found[0].title, QStringLiteral("Escaper"));
    QVERIFY2(found[0].source.isEmpty(), "a wallpaper reached outside its own folder");
    QVERIFY(!found[0].hasFile());
}

void TestWallpaperCore::selectsByIdAndRefusesUnknownOnes()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());
    makeWallpaper(steam.path(), QStringLiteral("777"),
                  R"({"title":"Only one","type":"web","file":"index.html"})",
                  QStringLiteral("index.html"));

    WallpaperCore core;
    QSignalSpy currentChanges(&core, &WallpaperCore::currentChanged);

    core.rescan({steam.path()});
    QCOMPARE(core.wallpapers().size(), 1);
    QVERIFY(core.current().id.isEmpty());

    QVERIFY(!core.select(QStringLiteral("nobody")));
    QVERIFY2(core.current().id.isEmpty(), "an unknown id changed the choice");
    QCOMPARE(currentChanges.count(), 0);

    QVERIFY(core.select(QStringLiteral("777")));
    QCOMPARE(core.current().id, QStringLiteral("777"));
    QCOMPARE(core.current().title, QStringLiteral("Only one"));
    QCOMPARE(currentChanges.count(), 1);

    core.clear();
    QVERIFY(core.current().id.isEmpty());
    QCOMPARE(currentChanges.count(), 2);
}

QTEST_MAIN(TestWallpaperCore)
#include "tst_wallpapercore.moc"
