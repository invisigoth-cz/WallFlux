// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "wallpapercore.h"
#include "wallflux_scene.h"

#include <QDir>
#include <QFile>
#include <QDirIterator>
#include <QFileInfo>
#include <QJsonArray>
#include <QLocale>
#include <QProcess>
#include <QRegularExpression>

#include <utility>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QLoggingCategory>

#include <algorithm>
#include <optional>

Q_LOGGING_CATEGORY(lcScan, "wallflux.scan")

namespace {

/// Wallpaper Engine on Steam. Its workshop content sits in a folder named
/// after it.
constexpr QLatin1StringView WallpaperEngineAppId("431960");

WallpaperKind::Kind kindFromType(const QString &type)
{
    // Spelled both "scene" and "Scene" in this library -- three of the
    // forty-one use the capital -- so the comparison cannot be case
    // sensitive.
    const QString lowered = type.toLower();
    if (lowered == QLatin1StringView("web"))
        return WallpaperKind::Web;
    if (lowered == QLatin1StringView("video"))
        return WallpaperKind::Video;
    if (lowered == QLatin1StringView("application"))
        return WallpaperKind::Application;
    if (lowered == QLatin1StringView("scene"))
        return WallpaperKind::Scene;
    return WallpaperKind::Unknown;
}


/// What to ask ffprobe for.
QStringList probeArguments(const QString &file)
{
    return {QStringLiteral("-v"),        QStringLiteral("error"),
            QStringLiteral("-show_entries"),   QStringLiteral("stream=codec_type,width,height"),
            QStringLiteral("-of"),       QStringLiteral("csv=p=0"),
            file};
}

/// Everything under this folder, added up.
///
/// A web wallpaper is a folder of files and its `index.html` is the least of
/// them -- measured on this machine: 13 kB of HTML in 3.6 MB of wallpaper.
/// Reporting the entry file's size would be reporting nothing.
qint64 sizeUnder(const QString &folder)
{
    qint64 total = 0;
    QDirIterator files(folder, QDir::Files, QDirIterator::Subdirectories);
    while (files.hasNext())
        total += QFileInfo(files.next()).size();
    return total;
}

/// A file that project.json names, resolved inside the wallpaper's own
/// folder.
///
/// That name comes out of content downloaded from the workshop, so it is
/// checked rather than trusted: a `file` of "../../../.bashrc" must not be
/// able to hand the browser something outside the wallpaper.
QUrl entryPoint(const QDir &folder, const QString &name)
{
    if (name.isEmpty())
        return {};

    const QString inside = folder.absolutePath() + QLatin1Char('/');
    const QString resolved = QDir::cleanPath(folder.absoluteFilePath(name));
    if (!resolved.startsWith(inside)) {
        qCWarning(lcScan) << "wallpaper" << folder.dirName()
                          << "names a file outside its own folder, ignoring it:" << name;
        return {};
    }

    // A scene wallpaper names a file that lives inside scene.pkg rather than
    // on disk, so missing is the ordinary case here, not an error.
    if (!QFileInfo::exists(resolved))
        return {};

    return QUrl::fromLocalFile(resolved);
}

/// The content rating in project.json, as Steam spells it.
///
/// Kept in Steam's own words rather than turned into G, PG13 and R, so that
/// one wallpaper's rating means the same thing whether it came off the disk
/// or out of a workshop query -- and so one filter chip can be given to
/// both. Wallpaper Engine writes one of three words; anything else,
/// including nothing at all, leaves it empty rather than guessing.
QString ratingFrom(const QString &contentRating)
{
    const QString lowered = contentRating.toLower();
    if (lowered == QLatin1StringView("everyone"))
        return QStringLiteral("Everyone");
    if (lowered == QLatin1StringView("questionable"))
        return QStringLiteral("Questionable");
    if (lowered == QLatin1StringView("mature"))
        return QStringLiteral("Mature");
    return {};
}

/// A folder's project.json, or nothing where it cannot be read.
std::optional<QJsonObject> readProject(const QDir &folder)
{
    QFile file(folder.filePath(QStringLiteral("project.json")));
    if (!file.open(QIODevice::ReadOnly))
        return std::nullopt;

    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        qCWarning(lcScan) << "wallpaper" << folder.dirName() << "has an unreadable project.json:"
                          << error.errorString();
        return std::nullopt;
    }
    return document.object();
}

/// What a web wallpaper is told about its settings: each as project.json
/// declares it, with a preset's value laid over it. A file a preset names lives
/// in the preset's own folder and the page is handed where that is, the way
/// Wallpaper Engine does -- pages put `file:///` in front of what they get.
QString webProperties(const QJsonObject &project, const QJsonObject &preset, const QDir &presetFolder)
{
    QJsonObject properties = project.value(QLatin1StringView("general")).toObject()
                                 .value(QLatin1StringView("properties")).toObject();
    for (auto it = preset.begin(); it != preset.end(); ++it) {
        if (!properties.contains(it.key()))
            continue;
        QJsonObject property = properties.value(it.key()).toObject();
        const QString type = property.value(QLatin1StringView("type")).toString();
        QJsonValue value = it.value();
        if (type == QLatin1StringView("file") || type == QLatin1StringView("directory"))
            // Wallpaper Engine hands over a Windows path and pages put three
            // slashes in front of it, so a path here goes without its first.
            // Four slashes is a file that cannot be played: 三体's music was silent.
            value = entryPoint(presetFolder, value.toString()).toLocalFile().mid(1);
        property.insert(QStringLiteral("value"), value);
        properties.insert(it.key(), property);
    }
    return QString::fromUtf8(QJsonDocument(properties).toJson(QJsonDocument::Compact));
}

/// Reads one wallpaper folder, or nothing if it holds no readable
/// project.json.
std::optional<Wallpaper> readWallpaper(const QDir &folder)
{
    const std::optional<QJsonObject> own = readProject(folder);
    if (!own)
        return std::nullopt;
    const QJsonObject &root = *own;

    // A preset carries no `type` at all: it is somebody else's wallpaper, named
    // by its folder, plus a file of settings. Two here sit on top of Perfect
    // Wallpaper and are what somebody subscribed to; it is that wallpaper that
    // gets drawn. The name comes off the internet, so it has to be a number.
    QDir drawnFrom = folder;
    std::optional<QJsonObject> drawn = root;
    const QJsonObject preset = root.value(QLatin1StringView("preset")).toObject();
    const QString dependency = root.value(QLatin1StringView("dependency")).toString();
    static const QRegularExpression workshopId(QStringLiteral("^\\d+$"));
    if (!preset.isEmpty() && workshopId.match(dependency).hasMatch()) {
        drawnFrom.setPath(folder.absoluteFilePath(QStringLiteral("../") + dependency));
        drawnFrom.makeAbsolute();
        drawn = readProject(drawnFrom);
        if (!drawn) {
            qCDebug(lcScan) << "skipping" << folder.dirName() << "-- its wallpaper is not here";
            return std::nullopt;
        }
    }

    // A type we have never seen, or a preset of a preset, cannot be drawn and
    // does not get into the list in the first place.
    const WallpaperKind::Kind kind =
        kindFromType(drawn->value(QLatin1StringView("type")).toString());
    if (kind == WallpaperKind::Unknown) {
        qCDebug(lcScan) << "skipping" << folder.dirName() << "-- no type we can draw";
        return std::nullopt;
    }

    Wallpaper wallpaper;
    wallpaper.id = folder.dirName();
    // An untitled wallpaper is still worth listing, under its id.
    wallpaper.title = root.value(QLatin1StringView("title")).toString(wallpaper.id);
    wallpaper.kind = kind;
    // A scene's own file is inside a package named after it -- scene.json in
    // scene.pkg almost always, gifscene.json in gifscene.pkg once here -- and
    // the package is what is drawn. The ones Wallpaper Engine ships are not
    // packed, and there the file itself says where the wallpaper is.
    const QString named = drawn->value(QLatin1StringView("file")).toString();
    const QString package = named.endsWith(QLatin1StringView(".json"))
        ? named.chopped(5) + QLatin1StringView(".pkg")
        : QStringLiteral("scene.pkg");
    wallpaper.source = entryPoint(drawnFrom, wallpaper.kind == WallpaperKind::Scene ? package : named);
    if (wallpaper.kind == WallpaperKind::Scene && wallpaper.source.isEmpty())
        wallpaper.source = entryPoint(drawnFrom, named);
    if (wallpaper.kind == WallpaperKind::Web)
        wallpaper.properties = webProperties(*drawn, preset, folder);
    wallpaper.preview = entryPoint(folder, root.value(QLatin1StringView("preview")).toString());

    for (const QJsonValue &tag : root.value(QLatin1StringView("tags")).toArray())
        wallpaper.tags.append(tag.toString());
    wallpaper.rating = ratingFrom(root.value(QLatin1StringView("contentrating")).toString());

    const QFileInfo here(folder.absolutePath());
    // birthTime where the filesystem keeps one -- btrfs does, measured --
    // and the last change where it does not, which is the nearest thing to
    // when it arrived.
    wallpaper.added = here.birthTime().isValid() ? here.birthTime() : here.lastModified();
    wallpaper.bytes = sizeUnder(folder.absolutePath());
    if (wallpaper.bytes > 0)
        wallpaper.sizeOnDisk = QLocale().formattedDataSize(wallpaper.bytes);

    if (wallpaper.kind == WallpaperKind::Scene && wallpaper.hasFile()) {
        wallpaper.soundKnown = true;
        wallpaper.hasSound = wallflux_scene_has_sound(QFile::encodeName(drawnFrom.absolutePath()).constData());
    }

    return wallpaper;
}

} // namespace

/// What ffprobe printed, read back.
///
/// ffprobe rather than a decoder of our own: it is already on any machine
/// whose Qt plays video, since that backend is ffmpeg too, and it answers in
/// 53 milliseconds against these files -- measured. Nothing in project.json
/// carries a resolution (checked across all 53) and the preview is a square
/// thumbnail, so there is nowhere cheaper to ask.
Probed probedFrom(const QByteArray &printed)
{
    // A line a stream, in whatever order the file keeps them:
    // "video,1920,1080" and "audio" -- the audio first in two of the ten
    // here. Nothing at all where the file could not be read.
    Probed told;
    static const QRegularExpression video(QStringLiteral("^video,(\\d+),(\\d+)$"));
    for (const QString &line : QString::fromLatin1(printed).split(u'\n')) {
        const QString stream = line.trimmed();
        told.answered = told.answered || !stream.isEmpty();
        if (stream.startsWith(QLatin1StringView("audio")))
            told.sound = true;
        else if (const auto shape = video.match(stream); shape.hasMatch() && told.resolution.isEmpty())
            told.resolution = shape.captured(1) + u'x' + shape.captured(2);
    }
    return told;
}

WallpaperCore::WallpaperCore(QObject *parent)
    : QObject(parent)
{
    m_settle.setSingleShot(true);
    // Long enough that a download writing a file at a time does not start a
    // scan per file, short enough that somebody watching the grid sees it
    // happen. A 280 MB video takes minutes; the folder appears at the start
    // of that, so there will be several of these before it settles.
    m_settle.setInterval(2000);
    connect(&m_settle, &QTimer::timeout, this, [this]() { rescan(m_roots); });
    connect(&m_watcher, &QFileSystemWatcher::directoryChanged, this,
            [this]() { m_settle.start(); });

    rescan();
}

QStringList WallpaperCore::steamRoots()
{
    const QString home = QDir::homePath();
    const QStringList candidates = {
        home + QLatin1StringView("/.local/share/Steam"),
        home + QLatin1StringView("/.steam/steam"),
        home + QLatin1StringView("/.steam/root"),
        home + QLatin1StringView("/.var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home + QLatin1StringView("/Steam"),
    };

    // Most of those are symlinks to the first one. Resolving each to its real
    // path and dropping repeats is what stops every wallpaper being found
    // three times over.
    //
    // ponytail: only the default library of each install. A second Steam
    // library on another drive is listed in steamapps/libraryfolders.vdf;
    // parse that here if anyone turns out to keep wallpapers on one.
    QStringList roots;
    for (const QString &candidate : candidates) {
        const QString real = QFileInfo(candidate).canonicalFilePath();
        if (!real.isEmpty() && !roots.contains(real))
            roots.append(real);
    }
    return roots;
}

QList<Wallpaper> WallpaperCore::scan(const QStringList &steamRoots)
{
    QList<Wallpaper> found;

    for (const QString &root : steamRoots) {
        QDir content(root + QLatin1StringView("/steamapps/workshop/content/")
                     + WallpaperEngineAppId);
        if (!content.exists())
            continue;

        const QStringList folders =
            content.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);
        for (const QString &folder : folders) {
            if (const std::optional<Wallpaper> wallpaper =
                    readWallpaper(QDir(content.filePath(folder))))
                found.append(*wallpaper);
        }
    }

    // The ones that come with Wallpaper Engine, which no workshop folder holds.
    // ponytail: by name, the four of its nineteen that draw. The rest are 3D
    // models, or come out black, and a grid of cards that show nothing is worse
    // than a shorter grid. Add to the list as the renderer learns them.
    static const QStringList shippedThatDraw = {
        QStringLiteral("beach"), QStringLiteral("deep_space"),
        QStringLiteral("razer_bedroom"), QStringLiteral("razer_vortex")};
    for (const QString &root : steamRoots) {
        const QDir shipped(root + QLatin1StringView("/steamapps/common/wallpaper_engine/projects/defaultprojects"));
        for (const QString &folder : shippedThatDraw) {
            if (const std::optional<Wallpaper> wallpaper = readWallpaper(QDir(shipped.filePath(folder))))
                found.append(*wallpaper);
        }
    }

    // By title, because that is the order a person would look for one in.
    std::ranges::sort(found, [](const Wallpaper &a, const Wallpaper &b) {
        return a.title.localeAwareCompare(b.title) < 0;
    });
    return found;
}

void WallpaperCore::rescan()
{
    rescan(steamRoots());
}

void WallpaperCore::rescan(const QStringList &steamRoots)
{
    m_roots = steamRoots;
    m_wallpapers = scan(steamRoots);
    watch(steamRoots);

    const qsizetype withFile =
        std::ranges::count_if(m_wallpapers, [](const Wallpaper &w) { return w.hasFile(); });
    qCInfo(lcScan) << "found" << m_wallpapers.size() << "wallpapers," << withFile
                   << "with a file on disk";

    readResolutions();

    // A wallpaper that was chosen before may be gone now.
    if (!m_current.id.isEmpty() && !select(m_current.id))
        clear();

    emit wallpapersChanged();
}

bool WallpaperCore::select(const QString &id)
{
    const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
    if (match == m_wallpapers.cend())
        return false;

    // Saying nothing when nothing has changed is not an optimisation here,
    // it is the difference between a wallpaper that plays and one that does
    // not. Clicking the wallpaper that is already playing used to report a
    // change; QML then handed the video player the same file again, which
    // stops it and reports no change of its own, so nothing was left to
    // start it. The screen went black until something else was chosen.
    //
    // A rescan still refreshes what we hold: the same wallpaper whose file
    // has moved is a change, and is announced as one.
    if (m_current.id == match->id && m_current.source == match->source)
        return true;

    m_current = *match;
    emit currentChanged();

    // Newest first, and only once in the list: picking the same wallpaper
    // again with another in between should move it, not list it twice.
    m_recent.removeAll(m_current.id);
    m_recent.prepend(m_current.id);
    emit recentChanged();

    return true;
}

void WallpaperCore::clear()
{
    if (m_current.id.isEmpty())
        return;
    m_current = Wallpaper{};
    emit currentChanged();
}

bool WallpaperCore::discard(const QString &id)
{
    const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
    if (match == m_wallpapers.cend()) {
        qCWarning(lcScan) << "nothing installed here is called" << id;
        return false;
    }

    // Every wallpaper has a preview beside its project.json -- checked
    // across every one of them -- so the folder is that file's folder.
    const QDir folder = QFileInfo(match->preview.toLocalFile()).absoluteDir();

    QDir workshop = folder;
    const bool named = folder.dirName() == id;
    const bool belongs = workshop.cdUp()
                         && workshop.dirName() == QStringLiteral("431960");
    const bool scanned = std::ranges::any_of(m_roots, [&folder](const QString &root) {
        return folder.absolutePath().startsWith(QDir(root).absolutePath()
                                                + QLatin1Char('/'));
    });

    if (!named || !belongs || !scanned) {
        qCWarning(lcScan) << "refusing to delete" << folder.absolutePath()
                          << "-- that is not the workshop folder of" << id;
        return false;
    }

    qCInfo(lcScan) << "deleting" << folder.absolutePath();
    const bool gone = QDir(folder).removeRecursively();
    if (!gone)
        qCWarning(lcScan) << "could not delete all of" << folder.absolutePath();

    rescan(m_roots);
    return gone;
}

void WallpaperCore::watch(const QStringList &steamRoots)
{
    const QStringList watched = m_watcher.directories();
    if (!watched.isEmpty())
        m_watcher.removePaths(watched);

    for (const QString &root : steamRoots) {
        const QString content = root + QLatin1StringView("/steamapps/workshop/content/")
                                + WallpaperEngineAppId;
        // Only one that is there: asked to watch a path that does not exist,
        // QFileSystemWatcher warns and carries on watching nothing.
        if (QFileInfo::exists(content))
            m_watcher.addPath(content);
    }
}

void WallpaperCore::stopProbes()
{
    // Taken out of the list and disconnected before any of them is deleted.
    // Destroying a running QProcess waits for its program to go and delivers
    // the finished signal on the way, and the handler for that edits this
    // very list -- which is a crash in the middle of deleting it. Measured,
    // as a segfault in the test suite.
    const QList<QProcess *> going = std::exchange(m_probes, {});
    for (QProcess *probe : going) {
        probe->disconnect(this);
        probe->kill();
        delete probe;
    }
}

void WallpaperCore::readResolutions()
{
    stopProbes();

    for (const Wallpaper &wallpaper : std::as_const(m_wallpapers)) {
        if (wallpaper.kind != WallpaperKind::Video || wallpaper.source.isEmpty())
            continue;

        auto *probe = new QProcess(this);
        m_probes.append(probe);
        connect(probe, &QProcess::finished, this, [this, probe, id = wallpaper.id]() {
            m_probes.removeAll(probe);
            probe->deleteLater();

            const Probed told = probedFrom(probe->readAllStandardOutput());
            if (!told.answered) {
                qCInfo(lcScan) << "ffprobe said nothing about" << id;
                return;
            }

            const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
            if (match == m_wallpapers.end())
                return;

            const auto tell = [&told](Wallpaper &one) {
                if (!told.resolution.isEmpty())
                    one.resolution = told.resolution;
                one.soundKnown = true;
                one.hasSound = told.sound;
            };
            tell(*match);
            // The one that is playing carries its own copy.
            if (m_current.id == id)
                tell(m_current);
            emit wallpapersChanged();
        });
        probe->start(QStringLiteral("ffprobe"), probeArguments(wallpaper.source.toLocalFile()));
    }
}

WallpaperCore::~WallpaperCore()
{
    // Nobody is left who wanted the answers.
    stopProbes();
}
