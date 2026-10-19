// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "wallpapercore.h"

#include <QDir>
#include <QFile>
#include <QDirIterator>
#include <QFileInfo>
#include <QLocale>
#include <QProcess>
#include <QRegularExpression>
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
    if (lowered == QLatin1StringView("scene"))
        return WallpaperKind::Scene;
    return WallpaperKind::Unknown;
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

/// Reads one wallpaper folder, or nothing if it holds no readable
/// project.json.
std::optional<Wallpaper> readWallpaper(const QDir &folder)
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

    const QJsonObject root = document.object();

    // A preset carries no `type` at all -- it is somebody else's wallpaper
    // plus a file of settings, and the two in this library are exactly that.
    // Nothing here can draw one, or a type we have never seen, so neither
    // gets into the list in the first place.
    const WallpaperKind::Kind kind =
        kindFromType(root.value(QLatin1StringView("type")).toString());
    if (kind == WallpaperKind::Unknown) {
        qCDebug(lcScan) << "skipping" << folder.dirName() << "-- no type we can draw";
        return std::nullopt;
    }

    Wallpaper wallpaper;
    wallpaper.id = folder.dirName();
    // An untitled wallpaper is still worth listing, under its id.
    wallpaper.title = root.value(QLatin1StringView("title")).toString(wallpaper.id);
    wallpaper.kind = kind;
    wallpaper.source = entryPoint(folder, root.value(QLatin1StringView("file")).toString());
    wallpaper.preview = entryPoint(folder, root.value(QLatin1StringView("preview")).toString());
    return wallpaper;
}

} // namespace

WallpaperCore::WallpaperCore(QObject *parent)
    : QObject(parent)
{
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
    m_wallpapers = scan(steamRoots);
    // What was worked out about files on disk was worked out about the files
    // that were there before this.
    m_details.clear();

    const qsizetype withFile =
        std::ranges::count_if(m_wallpapers, [](const Wallpaper &w) { return w.hasFile(); });
    qCInfo(lcScan) << "found" << m_wallpapers.size() << "wallpapers," << withFile
                   << "with a file on disk";

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
    return true;
}

void WallpaperCore::clear()
{
    if (m_current.id.isEmpty())
        return;
    m_current = Wallpaper{};
    emit currentChanged();
}

namespace {

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

/// The picture size of a video file, as `1920x1080`, or nothing.
///
/// ffprobe rather than a decoder of our own: it is already on any machine
/// whose Qt plays video, since that backend is ffmpeg too, and it answers in
/// 53 milliseconds against these files -- measured.
///
/// ponytail: waits for the answer rather than going off and coming back, so
/// a stalled ffprobe would hold the interface still for up to two seconds.
/// It is asked once per wallpaper and only when its details are on screen.
/// Make it a signal if that stops being true.
QString videoResolution(const QString &file)
{
    QProcess probe;
    probe.start(QStringLiteral("ffprobe"),
                {QStringLiteral("-v"), QStringLiteral("error"),
                 QStringLiteral("-select_streams"), QStringLiteral("v:0"),
                 QStringLiteral("-show_entries"), QStringLiteral("stream=width,height"),
                 QStringLiteral("-of"), QStringLiteral("csv=p=0:s=x"), file});
    if (!probe.waitForFinished(2000) || probe.exitStatus() != QProcess::NormalExit
        || probe.exitCode() != 0) {
        qCInfo(lcScan) << "ffprobe said nothing about" << file
                       << "-- its resolution will not be shown";
        return {};
    }

    // "1920x1080\n", and nothing else on success.
    const QString answer = QString::fromLatin1(probe.readAllStandardOutput()).trimmed();
    static const QRegularExpression shape(QStringLiteral("^\\d+x\\d+$"));
    return shape.match(answer).hasMatch() ? answer : QString{};
}

} // namespace

QVariantMap WallpaperCore::details(const QString &id) const
{
    if (const auto known = m_details.constFind(id); known != m_details.cend())
        return *known;

    const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
    if (match == m_wallpapers.cend())
        return {};

    const QString file = match->source.toLocalFile();
    QVariantMap facts;

    // A web wallpaper is its whole folder; a video is the one file.
    if (!file.isEmpty()) {
        const qint64 bytes = match->kind == WallpaperKind::Web
                                 ? sizeUnder(QFileInfo(file).absolutePath())
                                 : QFileInfo(file).size();
        if (bytes > 0)
            facts.insert(QStringLiteral("size"), QLocale().formattedDataSize(bytes));
    }

    if (match->kind == WallpaperKind::Video && !file.isEmpty()) {
        if (const QString shape = videoResolution(file); !shape.isEmpty())
            facts.insert(QStringLiteral("resolution"), shape);
    }

    m_details.insert(id, facts);
    return facts;
}
