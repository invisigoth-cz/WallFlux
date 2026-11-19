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
#include <QJsonDocument>
#include <QJsonObject>
#include <QScopeGuard>
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
        // A video's file begins as an MP4 does, or it would not be played.
        entry.write(name.endsWith(QLatin1StringView(".mp4"))
                        ? QByteArray("\0\0\0\x18" "ftypmp42", 12)
                        : QByteArray("<html></html>"));
    }
}

/// Writes a scene.pkg holding the one scene.json, laid out as Wallpaper
/// Engine lays its packages out: a version, a count, a table of names with
/// offsets and sizes, and the payload after it.
void writePackage(const QString &file, const QByteArray &sceneJson)
{
    QByteArray bytes;
    const auto number = [&bytes](qint32 n) {
        bytes.append(reinterpret_cast<const char *>(&n), sizeof n);
    };
    const auto text = [&](const QByteArray &s) {
        number(qint32(s.size()));
        bytes.append(s);
    };
    text("PKGV0019");
    number(1);
    text("scene.json");
    number(0);
    number(qint32(sceneJson.size()));
    bytes.append(sceneJson);

    QFile out(file);
    QVERIFY(out.open(QIODevice::WriteOnly));
    QCOMPARE(out.write(bytes), bytes.size());
}

} // namespace

class TestWallpaperCore : public QObject
{
    Q_OBJECT

private slots:
    void findsALibraryOnAnotherDrive();
    /// Wallpaper Engine moved to another library leaves its workshop
    /// behind, and Steam downloads it all again beside the app: one id in
    /// two folders, shown once, from where Wallpaper Engine is.
    void showsAWallpaperInTwoLibrariesOnce();
    void leavesOutWhatOnlyEastAsiaReads();
    void putsLettersBeforeDigitsBeforeTheRest();
    void readsAConditionWithoutRunningIt();
    void playsOnlyWhatIsReallyAVideo();
    void drawsAPresetAsTheWallpaperItSitsOn();
    void listsWhatComesWithWallpaperEngine();
    void hidesOneOfWallpaperEnginesOwn();
    /// The things a scan has to get right, on one made-up Steam install.
    void readsWhatIsThere();
    void readsWhenSteamFetchedEachOne();

    /// A `file` that climbs out of the wallpaper's folder is refused. The
    /// name comes from downloaded content, so this is a boundary, not a
    /// nicety.
    void refusesAFileOutsideTheWallpaper();

    /// A scene's package is named after the scene file project.json names:
    /// SCP Terminal Lockdown is gifscene.json in gifscene.pkg, and taking
    /// scene.pkg for granted listed it dimmed with nothing to play.
    void findsAScenePackageNamedAfterItsFile();

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

    /// Whether a wallpaper plays a sound of its own, which is what the panel
    /// says beside "follows the music" so that the two are not read as one.
    void saysWhetherItHasASoundOfItsOwn();

    /// Giving a wallpaper back takes its folder with it -- and anything
    /// that is not that folder is left alone.
    void takesOneOffTheDisk();
};

void TestWallpaperCore::findsALibraryOnAnotherDrive()
{
    // Steam in the home folder, Wallpaper Engine in a library elsewhere --
    // whose name has a quote in it, which Steam writes escaped.
    QTemporaryDir home;
    QTemporaryDir drive;
    const QString steam = home.path() + QStringLiteral("/Steam");
    const QString library = drive.path() + QStringLiteral("/Games \"2\"");
    QVERIFY(QDir().mkpath(steam + QStringLiteral("/steamapps")));
    const QString content = library + QStringLiteral("/steamapps/workshop/content/431960/7");
    QVERIFY(QDir().mkpath(content));

    QString escaped = library;
    escaped.replace(QStringLiteral("\""), QStringLiteral("\\\""));
    QFile vdf(steam + QStringLiteral("/steamapps/libraryfolders.vdf"));
    QVERIFY(vdf.open(QIODevice::WriteOnly));
    vdf.write(QStringLiteral("\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"%1\"\n"
                             "\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"%2\"\n\t\t\"apps\"\n\t\t{\n"
                             "\t\t\t\"431960\"\t\t\"1\"\n\t\t}\n\t}\n}\n")
                  .arg(steam, escaped).toUtf8());
    vdf.close();

    QFile project(content + QStringLiteral("/project.json"));
    QVERIFY(project.open(QIODevice::WriteOnly));
    project.write(R"({"title":"Far away","type":"web","file":"index.html"})");
    project.close();
    QFile page(content + QStringLiteral("/index.html"));
    QVERIFY(page.open(QIODevice::WriteOnly));
    page.close();

    // The install lists itself too, and a missing one is left out.
    const QStringList roots = WallpaperCore::steamRoots(
        {steam, steam, home.path() + QStringLiteral("/nowhere")});
    QCOMPARE(roots, QStringList({QFileInfo(steam).canonicalFilePath(),
                                 QFileInfo(library).canonicalFilePath()}));

    const QList<Wallpaper> found = WallpaperCore::scan(roots);
    QCOMPARE(found.size(), 1);
    QCOMPARE(found.at(0).title, QStringLiteral("Far away"));
}

void TestWallpaperCore::showsAWallpaperInTwoLibrariesOnce()
{
    QTemporaryDir left;
    QTemporaryDir now;
    const QByteArray page = R"({"title":"Twice","type":"web","file":"index.html"})";
    makeWallpaper(left.path(), QStringLiteral("7"), page, {QStringLiteral("index.html")});
    makeWallpaper(left.path(), QStringLiteral("8"),
                  R"({"title":"Only here","type":"web","file":"index.html"})",
                  {QStringLiteral("index.html")});
    makeWallpaper(now.path(), QStringLiteral("7"), page, {QStringLiteral("index.html")});
    QFile manifest(now.path() + QStringLiteral("/steamapps/appmanifest_431960.acf"));
    QVERIFY(manifest.open(QIODevice::WriteOnly));
    manifest.close();

    const QList<Wallpaper> found = WallpaperCore::scan({left.path(), now.path()});
    QCOMPARE(found.size(), 2);
    QCOMPARE(found.at(0).title, QStringLiteral("Only here"));
    QCOMPARE(found.at(1).title, QStringLiteral("Twice"));
    QVERIFY(found.at(1).source.toLocalFile().startsWith(now.path() + QLatin1Char('/')));
}

void TestWallpaperCore::drawsAPresetAsTheWallpaperItSitsOn()
{
    QTemporaryDir steam;
    const QString content = steam.path() + QStringLiteral("/steamapps/workshop/content/431960/");
    const auto write = [&content](const QString &id, const QByteArray &json) {
        QVERIFY(QDir().mkpath(content + id));
        QFile file(content + id + QStringLiteral("/project.json"));
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(json);
        file.close();
        QFile page(content + id + QStringLiteral("/index.html"));
        QVERIFY(page.open(QIODevice::WriteOnly));
        QFile picture(content + id + QStringLiteral("/a.jpg"));
        QVERIFY(picture.open(QIODevice::WriteOnly));
    };
    write(QStringLiteral("100"), R"({"title":"Base","type":"web","file":"index.html","general":{"properties":
          {"image":{"type":"file","value":null},"speed":{"type":"slider","value":1},"kept":{"type":"bool","value":true}}}})");
    write(QStringLiteral("200"), R"({"title":"Preset","dependency":"100","preset":{"image":"a.jpg","speed":5,"nobody":1}})");
    write(QStringLiteral("300"), R"({"title":"Out","dependency":"../../..","preset":{"speed":5}})");
    write(QStringLiteral("400"), R"({"title":"Lost","dependency":"999","preset":{"speed":5}})");

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 2);
    QCOMPARE(found.at(0).title, QStringLiteral("Base"));
    QCOMPARE(found.at(1).title, QStringLiteral("Preset"));
    QCOMPARE(found.at(1).kind, WallpaperKind::Web);
    QVERIFY(found.at(1).source.toLocalFile().endsWith(QStringLiteral("/100/index.html")));

    const QJsonObject told = QJsonDocument::fromJson(found.at(1).properties.toUtf8()).object();
    QCOMPARE(told.value(QStringLiteral("speed")).toObject().value(QStringLiteral("value")).toInt(), 5);
    QVERIFY(told.value(QStringLiteral("kept")).toObject().value(QStringLiteral("value")).toBool());
    QVERIFY(told.value(QStringLiteral("image")).toObject().value(QStringLiteral("value")).toString()
                .endsWith(QStringLiteral("/200/a.jpg")));
    QVERIFY(!told.contains(QStringLiteral("nobody")));
    QVERIFY(!told.value(QStringLiteral("image")).toObject().value(QStringLiteral("value")).toString()
                 .startsWith(u'/'));
}

void TestWallpaperCore::listsWhatComesWithWallpaperEngine()
{
    QTemporaryDir steam;
    const QString shipped = steam.path() + QStringLiteral("/steamapps/common/wallpaper_engine/projects/defaultprojects/");
    const auto write = [&shipped](const QString &name, const QString &file, const QByteArray &json) {
        QVERIFY(QDir().mkpath(shipped + name));
        QFile project(shipped + name + file);
        QVERIFY(project.open(QIODevice::WriteOnly));
        project.write(json);
    };
    write(QStringLiteral("deep_space"), QStringLiteral("/project.json"),
          R"({"title":"Deep Space","type":"scene","file":"scene.json","official":true,)"
          R"("authorsteamid":"76561197975619619"})");
    write(QStringLiteral("deep_space"), QStringLiteral("/scene.json"), "{}");
    // One the renderer cannot draw is listed all the same, as Wallpaper Engine
    // lists it, and cannot be played.
    write(QStringLiteral("arsenal"), QStringLiteral("/project.json"), R"({"title":"Arsenal","type":"scene","file":"scene.json"})");
    write(QStringLiteral("arsenal"), QStringLiteral("/scene.json"), "{}");
    // No type at all: a scene by its file, as Techno's is.
    write(QStringLiteral("techno"), QStringLiteral("/project.json"), R"({"title":"Techno","file":"techno.json"})");
    write(QStringLiteral("techno"), QStringLiteral("/techno.json"), "{}");
    // Hidden by Wallpaper Engine's own list, so not listed here either.
    write(QStringLiteral("dino_run"), QStringLiteral("/project.json"), R"({"title":"Dino Run","type":"scene","file":"scene.json"})");
    write(QStringLiteral("dino_run"), QStringLiteral("/scene.json"), "{}");
    write(QString(), QStringLiteral("visibility.json"), R"({"dino_run":false,"techno":true})");

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QStringList ids;
    for (const Wallpaper &one : found)
        ids.append(one.id);
    QCOMPARE(ids, QStringList({QStringLiteral("arsenal"), QStringLiteral("deep_space"),
                               QStringLiteral("techno")}));
    for (const Wallpaper &one : found)
        QVERIFY2(one.official, qPrintable(one.id));
    QCOMPARE(found.at(1).author, QStringLiteral("76561197975619619"));
    QVERIFY(found.at(1).source.toLocalFile().endsWith(QStringLiteral("/deep_space/scene.json")));
    QVERIFY(found.at(1).playable());
    QVERIFY(!found.at(0).playable());
    QCOMPARE(found.at(2).kind, WallpaperKind::Scene);
}

/// Removing one of Wallpaper Engine's own hides it the way Wallpaper Engine
/// does, in visibility.json, and leaves its files where the install put them.
void TestWallpaperCore::hidesOneOfWallpaperEnginesOwn()
{
    QTemporaryDir steam;
    const QString shipped = steam.path() + QStringLiteral("/steamapps/common/wallpaper_engine/projects/defaultprojects/");
    for (const QString &name : {QStringLiteral("beach"), QStringLiteral("retro")}) {
        QVERIFY(QDir().mkpath(shipped + name));
        QFile project(shipped + name + QStringLiteral("/project.json"));
        QVERIFY(project.open(QIODevice::WriteOnly));
        project.write(R"({"title":"x","type":"scene","file":"scene.json","preview":"preview.jpg"})");
        QFile preview(shipped + name + QStringLiteral("/preview.jpg"));
        QVERIFY(preview.open(QIODevice::WriteOnly));
    }
    QFile visibility(shipped + QStringLiteral("visibility.json"));
    QVERIFY(visibility.open(QIODevice::WriteOnly));
    visibility.write(R"({"audiophile":false})");
    visibility.close();

    WallpaperCore core;
    core.rescan({steam.path()});
    QCOMPARE(core.wallpapers().size(), 2);

    QVERIFY(core.discard(QStringLiteral("beach")));
    QCOMPARE(core.wallpapers().size(), 1);
    QCOMPARE(core.wallpapers().constFirst().id, QStringLiteral("retro"));
    QVERIFY2(QFile::exists(shipped + QStringLiteral("beach/project.json")),
             "the install's own files were deleted");

    QVERIFY(visibility.open(QIODevice::ReadOnly));
    const QJsonObject said = QJsonDocument::fromJson(visibility.readAll()).object();
    QCOMPARE(said.value(QStringLiteral("beach")), QJsonValue(false));
    QCOMPARE(said.value(QStringLiteral("audiophile")), QJsonValue(false));
}

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
                  R"({"title":"B capital scene","type":"Scene","file":"scene.json"})",
                  {QStringLiteral("scene.pkg")});
    // No title: the id stands in rather than the entry being dropped.
    makeWallpaper(steam.path(), QStringLiteral("333"), R"({"type":"video","file":"a.mp4"})",
                  {QStringLiteral("a.mp4")});
    // Not JSON at all. Must be skipped without taking the scan down.
    makeWallpaper(steam.path(), QStringLiteral("444"), "{ this is not json");
    // A preset of a wallpaper that is not here: nothing can draw it.
    makeWallpaper(steam.path(), QStringLiteral("555"),
                  R"({"title":"A preset","preset":{"ColorMode":1},"dependency":"999"})");
    // A type we have never seen. Also not listed.
    makeWallpaper(steam.path(), QStringLiteral("666"),
                  R"({"title":"A something else","type":"hologram","file":"a.txt"})",
                  {QStringLiteral("a.txt")});

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 3);

    // Sorted by title, letters before digits, so the untitled one -- listed
    // under its id -- comes last.
    QCOMPARE(found[0].title, QStringLiteral("A web one"));
    QCOMPARE(found[0].kind, WallpaperKind::Web);
    QVERIFY(found[0].source.isLocalFile());
    QVERIFY(found[0].source.toLocalFile().endsWith(QStringLiteral("/111/index.html")));
    QVERIFY(found[0].hasFile());
    QVERIFY(found[0].playable());
    // What the picker puts in its grid, found the same guarded way as the
    // wallpaper's own file.
    QVERIFY(found[0].preview.toLocalFile().endsWith(QStringLiteral("/111/preview.jpg")));

    QCOMPARE(found[1].title, QStringLiteral("B capital scene"));
    QCOMPARE(found[1].kind, WallpaperKind::Scene);
    // scene.json lives inside scene.pkg, so the package is what is drawn.
    QVERIFY(found[1].source.toLocalFile().endsWith(QStringLiteral("/222/scene.pkg")));
    QVERIFY(found[1].playable());

    QCOMPARE(found[2].id, QStringLiteral("333"));
    QCOMPARE(found[2].title, QStringLiteral("333"));
    QCOMPARE(found[2].kind, WallpaperKind::Video);
    QVERIFY(found[2].hasFile());
}

/// Steam's own record, in its own shape: one item in both sections, one
/// whose newer version Steam has heard of and not fetched, and one Steam
/// says nothing about.
void TestWallpaperCore::readsWhenSteamFetchedEachOne()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());
    for (const char *id : {"111", "222", "333"})
        makeWallpaper(steam.path(), QString::fromLatin1(id),
                      R"({"title":"x","type":"web","file":"index.html"})",
                      {QStringLiteral("index.html")});

    QFile acf(steam.path() + QStringLiteral("/steamapps/workshop/appworkshop_431960.acf"));
    QVERIFY(acf.open(QIODevice::WriteOnly));
    acf.write(R"("AppWorkshop"
{
	"appid"		"431960"
	"WorkshopItemsInstalled"
	{
		"111"
		{
			"size"		"100"
			"timeupdated"		"1600000000"
			"manifest"		"1"
		}
		"222"
		{
			"size"		"200"
			"timeupdated"		"1500000000"
		}
	}
	"WorkshopItemDetails"
	{
		"111"
		{
			"timeupdated"		"1600000000"
			"latest_timeupdated"		"1600000000"
		}
		"222"
		{
			"timeupdated"		"1500000000"
			"latest_timeupdated"		"1700000000"
		}
	}
}
)");
    acf.close();

    QHash<QString, Wallpaper> byId;
    for (const Wallpaper &one : WallpaperCore::scan({steam.path()}))
        byId.insert(one.id, one);
    QCOMPARE(byId.size(), 3);
    QCOMPARE(byId[QStringLiteral("111")].installedAt, 1600000000);
    QCOMPARE(byId[QStringLiteral("111")].latestAt, 1600000000);
    QCOMPARE(byId[QStringLiteral("222")].installedAt, 1500000000);
    QCOMPARE(byId[QStringLiteral("222")].latestAt, 1700000000);
    QCOMPARE(byId[QStringLiteral("333")].installedAt, 0);
    QCOMPARE(byId[QStringLiteral("333")].latestAt, 0);
}

void TestWallpaperCore::findsAScenePackageNamedAfterItsFile()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());
    makeWallpaper(steam.path(), QStringLiteral("666"),
                  R"({"title":"Terminal","type":"scene","file":"gifscene.json"})",
                  {QStringLiteral("gifscene.pkg")});

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 1);
    QVERIFY(found[0].hasFile());
    QVERIFY(found[0].source.toLocalFile().endsWith(QStringLiteral("/666/gifscene.pkg")));
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
    //
    // The rating is kept in Steam's own word rather than turned into G,
    // PG13 or R: a workshop query answers with these, so holding both to
    // the same word is what lets one filter chip be given to both.
    QCOMPARE(core.wallpapers().constFirst().tags, QStringList({QStringLiteral("Game")}));
    QCOMPARE(core.wallpapers().constFirst().rating, QStringLiteral("Everyone"));
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

void TestWallpaperCore::leavesOutWhatOnlyEastAsiaReads()
{
    QCOMPARE(WallpaperCore::readable(QStringLiteral("【4K】原神 雷电将军 Raiden Shogun")),
             QStringLiteral("4K Raiden Shogun"));
    QCOMPARE(WallpaperCore::readable(QStringLiteral("夜の街 | Night City | 夜景")),
             QStringLiteral("Night City"));
    QCOMPARE(WallpaperCore::readable(QStringLiteral("Raiden将军Shogun (原神)")),
             QStringLiteral("Raiden Shogun"));
    QCOMPARE(WallpaperCore::readable(QStringLiteral("벚꽃 Cherry blossom")),
             QStringLiteral("Cherry blossom"));
    QCOMPARE(WallpaperCore::readable(QStringLiteral("This is just a Code Rain(只是一个代码雨）")),
             QStringLiteral("This is just a Code Rain"));
    QCOMPARE(WallpaperCore::readable(
                 QStringLiteral("Perfect Wallpaper-完美壁纸【落樱粒子+多风格+自定义】-steam独此一家")),
             QStringLiteral("Perfect Wallpaper - steam"));
    QCOMPARE(WallpaperCore::readable(QStringLiteral("质量效应3 诺曼底号 SR-2 飞行壁纸")),
             QStringLiteral("3 SR-2"));
    // Nothing to read without it: kept whole.
    QCOMPARE(WallpaperCore::readable(QStringLiteral("雷电将军")), QStringLiteral("雷电将军"));
    // Untouched where there is nothing to leave out.
    QCOMPARE(WallpaperCore::readable(QStringLiteral("DOOM -- Eternal (4K)")),
             QStringLiteral("DOOM -- Eternal (4K)"));
}

void TestWallpaperCore::playsOnlyWhatIsReallyAVideo()
{
    QTemporaryDir steam;
    makeWallpaper(steam.path(), QStringLiteral("1"), R"({"type":"video","file":"a.mp4"})",
                  {QStringLiteral("a.mp4")});
    makeWallpaper(steam.path(), QStringLiteral("2"), R"({"type":"video","file":"b.mp4"})");
    QFile playlist(steam.path() + QStringLiteral("/steamapps/workshop/content/431960/2/b.mp4"));
    QVERIFY(playlist.open(QIODevice::WriteOnly));
    playlist.write("#EXTM3U\nfile:///etc/hostname\n");
    playlist.close();

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 2);
    for (const Wallpaper &one : found)
        QCOMPARE(one.playable(), one.id == QLatin1StringView("1"));
}

void TestWallpaperCore::readsAConditionWithoutRunningIt()
{
    const QVariantMap values{{QStringLiteral("mode"), 3},
                             {QStringLiteral("on"), true},
                             {QStringLiteral("off"), false},
                             {QStringLiteral("style"), 1},
                             {QStringLiteral("effect"), QStringLiteral("rainbowwave")},
                             {QStringLiteral("empty"), QString()}};
    const auto holds = [&](const char *c) { return WallpaperCore::holds(QString::fromUtf8(c), values); };

    // What the library's own conditions look like.
    QVERIFY(holds("mode.value == 3"));
    QVERIFY(!holds("mode.value == 2"));
    QVERIFY(holds("on.value == true && mode.value != 1"));
    QVERIFY(!holds("off.value == true || mode.value === 2"));
    QVERIFY(holds("(mode.value == 2 && on.value) || (mode.value == 3 && off.value == false )"));
    QVERIFY(holds("style.value=='1'"));      // loose: a number and its digits
    QVERIFY(!holds("style.value==='1'"));    // strict: not the same type
    QVERIFY(holds("effect.value.startsWith('rainbow') === true"));
    QVERIFY(holds("effect.value.endsWith(\"wave\")"));
    QVERIFY(!holds("effect.value.endsWith('spiral') === true"));
    QVERIFY(holds("mode.value > 0 && mode.value <= 3"));
    QVERIFY(holds("!off.value"));
    QVERIFY(!holds("empty.value != ''"));
    QVERIFY(!holds("on.value === 1"));

    // Anything else is shown, as an error was -- and never run.
    QVERIFY(holds(""));
    QVERIFY(holds("unknown.value == 3"));
    QVERIFY(holds("Wallpapers.discard('1')"));
    QVERIFY(holds("mode.value == 3; Qt.quit()"));
    QVERIFY(holds("mode.value.constructor('return 1')()"));
}

void TestWallpaperCore::putsLettersBeforeDigitsBeforeTheRest()
{
    // Within a group the order is the locale's, and a clean build machine
    // has none -- LANG=C sorts by code point, "Zed" before "alien". Said
    // here, so the test asks the same of every machine.
    const QLocale before;
    QLocale::setDefault(QLocale(QLocale::English, QLocale::UnitedStates));
    const auto restore = qScopeGuard([&] { QLocale::setDefault(before); });

    QStringList titles{QStringLiteral("三体"), QStringLiteral("3 SR-2"), QStringLiteral("Zed"),
                       QStringLiteral("[4K] Dune"), QStringLiteral("Árie"), QStringLiteral("alien"),
                       QStringLiteral("2077")};
    std::ranges::sort(titles, WallpaperCore::readsBefore);
    QCOMPARE(titles.mid(0, 3),
             (QStringList{QStringLiteral("alien"), QStringLiteral("Árie"), QStringLiteral("Zed")}));
    QCOMPARE(titles.mid(3, 2), (QStringList{QStringLiteral("2077"), QStringLiteral("3 SR-2")}));
    QCOMPARE(titles.mid(5).size(), 2);
}

QTEST_MAIN(TestWallpaperCore)

void TestWallpaperCore::takesOneOffTheDisk()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"Going","type":"web","file":"index.html",)"
                  R"("preview":"preview.jpg","tags":["Game"]})",
                  {QStringLiteral("index.html"), QStringLiteral("preview.jpg")});
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"Staying","type":"web","file":"index.html",)"
                  R"("preview":"preview.jpg","tags":["Game"]})",
                  {QStringLiteral("index.html"), QStringLiteral("preview.jpg")});

    WallpaperCore core;
    core.rescan({steam.path()});
    QCOMPARE(core.wallpapers().size(), 2);

    const QString content =
        steam.path() + QStringLiteral("/steamapps/workshop/content/431960/");

    // Nothing here is called that, so nothing is deleted.
    QVERIFY(!core.discard(QStringLiteral("333")));
    QCOMPARE(core.wallpapers().size(), 2);

    QVERIFY(core.discard(QStringLiteral("111")));
    QVERIFY2(!QDir(content + QStringLiteral("111")).exists(),
             "the folder was left behind");
    QVERIFY2(QDir(content + QStringLiteral("222")).exists(),
             "the wrong folder went");
    QCOMPARE(core.wallpapers().size(), 1);
    QCOMPARE(core.wallpapers().constFirst().id, QStringLiteral("222"));
}

void TestWallpaperCore::saysWhetherItHasASoundOfItsOwn()
{
    QTemporaryDir steam;
    QVERIFY(steam.isValid());
    const QString content = steam.path() + QStringLiteral("/steamapps/workshop/content/431960/");

    makeWallpaper(steam.path(), QStringLiteral("111"),
                  R"({"title":"A web one","type":"web","file":"index.html"})",
                  {QStringLiteral("index.html")});
    makeWallpaper(steam.path(), QStringLiteral("222"),
                  R"({"title":"B scene with music","type":"scene","file":"scene.json"})");
    writePackage(content + QStringLiteral("222/scene.pkg"),
                 R"({"objects":[{"name":"music","sound":["sounds/a.mp3"]}]})");
    // A sound object switched off is not a sound the wallpaper plays.
    makeWallpaper(steam.path(), QStringLiteral("333"),
                  R"({"title":"C quiet scene","type":"scene","file":"scene.json"})");
    writePackage(content + QStringLiteral("333/scene.pkg"),
                 R"({"objects":[{"name":"off","sound":["sounds/a.mp3"],"visible":false}]})");

    const QList<Wallpaper> found = WallpaperCore::scan({steam.path()});
    QCOMPARE(found.size(), 3);
    // A web page can start a sound whenever it likes; nothing says.
    QVERIFY(!found[0].soundKnown);
    QVERIFY(found[1].soundKnown);
    QVERIFY(found[1].hasSound);
    QVERIFY(found[2].soundKnown);
    QVERIFY(!found[2].hasSound);

    // A video is answered by ffprobe, a line a stream, in either order.
    const Probed both = probedFrom("audio\nvideo,1920,1080\n");
    QVERIFY(both.answered);
    QVERIFY(both.sound);
    QCOMPARE(both.resolution, QStringLiteral("1920x1080"));
    const Probed silent = probedFrom("video,3840,2160\n");
    QVERIFY(!silent.sound);
    QCOMPARE(silent.resolution, QStringLiteral("3840x2160"));
    QVERIFY(!probedFrom("").answered);
}

#include "tst_wallpapercore.moc"
