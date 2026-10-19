// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "wallpapercore.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QLocale>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

namespace {

/// Writes one wallpaper folder into a pretend Steam install.
void makeWallpaper(const QString &root, const QString &id, const QByteArray &projectJson,
                   const QStringList &alsoWrite = {})
{
    const QString folder =
        root + QStringLiteral("/steamapps/workshop/content/431960/") + id;
    QVERIFY(QDir().mkpath(folder));

    QFile project(folder + QStringLiteral("/project.json"));
    QVERIFY(project.open(QIODevice::WriteOnly));
    QCOMPARE(project.write(projectJson), projectJson.size());
    project.close();

    for (const QString &name : alsoWrite) {
        QFile entry(folder + QLatin1Char('/') + name);
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

    /// Choosing the wallpaper that is already playing says nothing happened.
    /// It is what a person does by clicking the tile that is already lit, and
    /// announcing it stops the video player for good.
    void staysQuietWhenTheChoiceIsTheSame();

    /// What the picker shows when somebody looks at one card: the size of
    /// what is on disk, and nothing at all for a wallpaper that is not here.
    void tellsWhatIsOnDisk();

    /// What was played in this run, newest first and each one only once.
    void remembersWhatWasPlayedInThisRun();

    /// A wallpaper that appears on disk is noticed without being asked for,
    /// which is what makes a download in Steam turn up here by itself.
    void noticesAWallpaperArriving();
};

void TestWallpaperCore::readsWhatIsThere()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"A web one","type":"web","file":"index.html",)"
                  R"("preview":"preview.jpg"})",
                  {QStringLiteral("index.html"), QStringLiteral("preview.jpg")});
    // The library spells the type both ways; both have to land as Scene.
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"B capital scene","type":"Scene","file":"scene.json"})");
    // No title: the id stands in rather than the entry being dropped.
    makeWallpaper(steam.path(), QStringLiteral("333"), R"({"type":"video","file":"a.mp4"})",
                  {QStringLiteral("a.mp4")});
    // Not JSON at all. Must be skipped without taking the scan down.
    makeWallpaper(steam.path(), QStringLiteral("444"), "{ this is not json");
    // A preset: no type of its own, only settings for somebody else's
    // wallpaper. Nothing can draw it, so it must not be listed.
    makeWallpaper(steam.path(), QStringLiteral("555"),
                  R"({"title":"A preset","preset":{"ColorMode":1},"dependency":"111"})");
    // A type we have never seen. Also not listed.
    makeWallpaper(steam.path(), QStringLiteral("666"),
                  R"({"title":"A something else","type":"hologram","file":"a.txt"})",
                  {QStringLiteral("a.txt")});

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
    QVERIFY(found[1].playable());
    // What the picker puts in its grid, found the same guarded way as the
    // wallpaper's own file.
    QVERIFY(found[1].preview.toLocalFile().endsWith(QStringLiteral("/111/preview.jpg")));

    QCOMPARE(found[2].title, QStringLiteral("B capital scene"));
    QCOMPARE(found[2].kind, WallpaperKind::Scene);
    // scene.json lives inside scene.pkg, so there is nothing to point at.
    QVERIFY(found[2].source.isEmpty());
    QVERIFY(!found[2].hasFile());
    QVERIFY2(!found[2].playable(), "a scene wallpaper offered itself as playable");
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
                  {QStringLiteral("index.html")});

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

void TestWallpaperCore::staysQuietWhenTheChoiceIsTheSame()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());
    makeWallpaper(steam.path(), QStringLiteral("888"),
                  R"({"title":"The one playing","type":"video","file":"a.mp4"})",
                  {QStringLiteral("a.mp4")});

    WallpaperCore core;
    core.rescan({steam.path()});
    QSignalSpy currentChanges(&core, &WallpaperCore::currentChanged);

    QVERIFY(core.select(QStringLiteral("888")));
    QCOMPARE(currentChanges.count(), 1);

    // The same one again: it is chosen, so true, but nothing changed and
    // nothing may be announced.
    QVERIFY(core.select(QStringLiteral("888")));
    QCOMPARE(core.current().id, QStringLiteral("888"));
    QVERIFY2(currentChanges.count() == 1, "choosing the same wallpaper announced a change");

    // A rescan goes through select() too, and must stay just as quiet.
    core.rescan({steam.path()});
    QCOMPARE(currentChanges.count(), 1);
}

void TestWallpaperCore::tellsWhatIsOnDisk()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    // A web wallpaper, because its size is the whole folder rather than the
    // entry file -- which is the part that could be got wrong. No video
    // here: that would start ffprobe, and a test has no business needing it.
    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"A web one","type":"web","file":"index.html",)"
                  R"("preview":"preview.jpg","tags":["Game"],)"
                  R"("contentrating":"Everyone"})",
                  {QStringLiteral("index.html"), QStringLiteral("preview.jpg")});

    WallpaperCore core;
    core.rescan({steam.path()});

    const QString size = core.wallpapers().constFirst().sizeOnDisk;
    QVERIFY2(!size.isEmpty(), "the picker was given no size to show");

    // Compared with what index.html alone would come to rather than with a
    // number written out here: the point is that a web wallpaper is measured
    // as the folder it is, and that holds whatever else was put in it.
    const QString entryOnly = QLocale().formattedDataSize(
        QFileInfo(steam.path()
                  + QStringLiteral("/steamapps/workshop/content/431960/111/index.html"))
            .size());
    QVERIFY2(size != entryOnly,
             qPrintable(QStringLiteral("the entry file's own size was reported: %1").arg(size)));

    // The tags and the rating come out of project.json beside it, which is
    // the same read, so they are checked here rather than in a test of their
    // own.
    QCOMPARE(core.wallpapers().constFirst().tags, QStringList({QStringLiteral("Game")}));
    QCOMPARE(core.wallpapers().constFirst().rating, QStringLiteral("G"));
}

void TestWallpaperCore::remembersWhatWasPlayedInThisRun()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"First","type":"video","file":"a.mp4"})",
                  {QStringLiteral("a.mp4")});
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"Second","type":"video","file":"b.mp4"})",
                  {QStringLiteral("b.mp4")});

    WallpaperCore core;
    core.rescan({steam.path()});
    QVERIFY2(core.recent().isEmpty(), "something was played before anything was played");

    QVERIFY(core.select(QStringLiteral("111")));
    QVERIFY(core.select(QStringLiteral("222")));
    QVERIFY(core.select(QStringLiteral("111")));

    // Going back to the first one moves it to the front rather than putting
    // it in the list twice, which an order in a menu has to get right.
    QCOMPARE(core.recent(), QStringList({QStringLiteral("111"), QStringLiteral("222")}));
}

void TestWallpaperCore::noticesAWallpaperArriving()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"Already here","type":"web","file":"index.html"})",
                  {QStringLiteral("index.html")});

    WallpaperCore core;
    core.rescan({steam.path()});
    QCOMPARE(core.wallpapers().size(), 1);

    QSignalSpy arrivals(&core, &WallpaperCore::wallpapersChanged);

    // What Steam does when somebody subscribes: a folder turns up in the one
    // being watched.
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"Just downloaded","type":"web","file":"index.html"})",
                  {QStringLiteral("index.html")});

    // Longer than the two seconds the scan waits for the writing to stop,
    // with room for a filesystem that takes its time telling anyone.
    QVERIFY2(arrivals.wait(10000), "nothing noticed the wallpaper arriving");
    QCOMPARE(core.wallpapers().size(), 2);
}

QTEST_MAIN(TestWallpaperCore)
#include "tst_wallpapercore.moc"
