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
#include <QSaveFile>
#include <QSet>

#include <utility>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QLoggingCategory>

#include <algorithm>
#include <functional>
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

/// Whether a file begins the way a video container does.
///
/// A video wallpaper is played by Qt's FFmpeg backend, which goes by what a
/// file holds rather than its name and cannot be told what it may open --
/// so a playlist or a concat list saved as `.mp4` would be read as one, and
/// could reach other files or the network. Only these containers are let
/// through: MP4/MOV, Matroska/WebM, AVI, ASF/WMV, FLV, Ogg, MPEG-TS.
bool looksLikeVideo(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return false;
    const QByteArray head = file.read(192);
    const auto at = [&](qsizetype i, QByteArrayView what) { return head.mid(i, what.size()) == what; };
    for (const char *box : {"ftyp", "moov", "mdat", "wide", "free", "skip", "pnot"}) {
        if (at(4, box))
            return true;
    }
    return at(0, "\x1A\x45\xDF\xA3") || (at(0, "RIFF") && at(8, "AVI "))
           || at(0, "\x30\x26\xB2\x75\x8E\x66\xCF\x11") || at(0, "FLV") || at(0, "OggS")
           || (head.size() > 188 && head.at(0) == 0x47 && head.at(188) == 0x47);
}

/// What a wallpaper that states no type is, going by its file. Four of
/// Wallpaper Engine's own say nothing -- Audiophile, Ricepod and Techno name
/// a .json, Sheep an .exe -- and Wallpaper Engine lists them all the same.
WallpaperKind::Kind kindFromFile(const QString &file)
{
    const QString suffix = QFileInfo(file).suffix().toLower();
    if (suffix == QLatin1StringView("json"))
        return WallpaperKind::Scene;
    if (suffix == QLatin1StringView("html") || suffix == QLatin1StringView("htm"))
        return WallpaperKind::Web;
    if (suffix == QLatin1StringView("exe"))
        return WallpaperKind::Application;
    return WallpaperKind::Unknown;
}

/// The ones of Wallpaper Engine's own its `visibility.json` hides: those
/// set to `false`, which is what removing one writes.
QSet<QString> hiddenIn(const QDir &shipped)
{
    QFile file(shipped.filePath(QStringLiteral("visibility.json")));
    if (!file.open(QIODevice::ReadOnly))
        return {};
    QSet<QString> hidden;
    const QJsonObject said = QJsonDocument::fromJson(file.readAll()).object();
    for (auto it = said.begin(); it != said.end(); ++it) {
        if (it.value().isBool() && !it.value().toBool())
            hidden.insert(it.key());
    }
    return hidden;
}


/// What Steam's record of the workshop says of each item: when the copy on
/// the disk was published and the newest Steam knows of, by id. The record
/// is Valve's KeyValues text; every item is a block of plain pairs, so a
/// block is found by its braces and read by its keys.
QHash<QString, std::pair<qint64, qint64>> workshopTimes(const QString &acf)
{
    QFile file(acf);
    if (!file.open(QIODevice::ReadOnly))
        return {};
    const QString text = QString::fromUtf8(file.readAll());
    static const QRegularExpression block(QStringLiteral(R"re("(\d+)"\s*\{([^{}]*)\})re"));
    static const QRegularExpression installed(QStringLiteral(R"re("timeupdated"\s*"(\d+)")re"));
    static const QRegularExpression latest(QStringLiteral(R"re("latest_timeupdated"\s*"(\d+)")re"));
    QHash<QString, std::pair<qint64, qint64>> times;
    for (auto it = block.globalMatch(text); it.hasNext();) {
        const QRegularExpressionMatch one = it.next();
        const QString body = one.captured(2);
        auto &[at, newest] = times[one.captured(1)];
        // The item is in two blocks, and only one of them says the latest.
        at = std::max(at, installed.match(body).captured(1).toLongLong());
        newest = std::max(newest, latest.match(body).captured(1).toLongLong());
    }
    return times;
}

/// What to ask ffprobe for.
QStringList probeArguments(const QString &file)
{
    return {QStringLiteral("-v"),        QStringLiteral("error"),
            QStringLiteral("-show_entries"),   QStringLiteral("stream=codec_type,width,height"),
            QStringLiteral("-of"),       QStringLiteral("csv=p=0"),
            // A local file and a video container only: see ONLY_A_VIDEO in
            // the renderer's video.rs, which says why.
            QStringLiteral("-protocol_whitelist"), QStringLiteral("file"),
            QStringLiteral("-format_whitelist"),
            QStringLiteral("mov,mp4,m4a,3gp,3g2,mj2,matroska,webm,avi,asf,mpegts,flv,ogg"),
            file};
}

/// What to ask ffmpeg about the loudest moment of the first sound track:
/// the sound alone decoded, a quarter of a second for 26 s of 4K video.
QStringList peakArguments(const QString &file)
{
    return {QStringLiteral("-nostdin"), QStringLiteral("-hide_banner"), QStringLiteral("-nostats"),
            QStringLiteral("-v"), QStringLiteral("info"),
            // As ffprobe is told, and for the same reason.
            QStringLiteral("-protocol_whitelist"), QStringLiteral("file"),
            QStringLiteral("-format_whitelist"),
            QStringLiteral("mov,mp4,m4a,3gp,3g2,mj2,matroska,webm,avi,asf,mpegts,flv,ogg"),
            QStringLiteral("-vn"), QStringLiteral("-sn"), QStringLiteral("-dn"),
            QStringLiteral("-i"), file,
            QStringLiteral("-map"), QStringLiteral("0:a:0"),
            QStringLiteral("-af"), QStringLiteral("astats=measure_perchannel=none:measure_overall=Peak_level"),
            QStringLiteral("-f"), QStringLiteral("null"), QStringLiteral("-")};
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
    const QString type = drawn->value(QLatin1StringView("type")).toString();
    const WallpaperKind::Kind kind = type.isEmpty()
        ? kindFromFile(drawn->value(QLatin1StringView("file")).toString())
        : kindFromType(type);
    if (kind == WallpaperKind::Unknown) {
        qCDebug(lcScan) << "skipping" << folder.dirName() << "-- no type we can draw";
        return std::nullopt;
    }

    Wallpaper wallpaper;
    wallpaper.id = folder.dirName();
    // An untitled wallpaper is still worth listing, under its id.
    wallpaper.title = WallpaperCore::readable(root.value(QLatin1StringView("title")).toString(wallpaper.id));
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
    if (wallpaper.kind == WallpaperKind::Video && wallpaper.hasFile()
        && !looksLikeVideo(wallpaper.source.toLocalFile())) {
        qCWarning(lcScan) << "wallpaper" << wallpaper.id << "says it is a video, and" << named
                          << "is not one: not played";
        wallpaper.drawable = false;
    }
    if (wallpaper.kind == WallpaperKind::Web || wallpaper.kind == WallpaperKind::Scene)
        wallpaper.properties = webProperties(*drawn, preset, folder);
    wallpaper.preview = entryPoint(folder, root.value(QLatin1StringView("preview")).toString());

    for (const QJsonValue &tag : root.value(QLatin1StringView("tags")).toArray())
        wallpaper.tags.append(tag.toString());
    wallpaper.rating = ratingFrom(root.value(QLatin1StringView("contentrating")).toString());
    wallpaper.author = root.value(QLatin1StringView("authorsteamid")).toString();

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

/// A property's condition, read rather than run: see `WallpaperCore::holds`.
/// Recursive descent over JavaScript's own precedence -- `||` under `&&`
/// under `!` under a comparison -- with its loose and strict equality.
class Condition
{
public:
    Condition(const QString &text, const QVariantMap &values)
        : m_text(text)
        , m_values(values)
    {
    }

    /// The answer, or nothing when the text is not something this reads.
    std::optional<bool> holds()
    {
        const QVariant answer = either();
        skipSpace();
        if (m_failed || m_at != m_text.size())
            return std::nullopt;
        return truthy(answer);
    }

private:
    static bool isNumber(const QVariant &v)
    {
        return v.typeId() == QMetaType::Bool || v.typeId() == QMetaType::Int
               || v.typeId() == QMetaType::Double || v.typeId() == QMetaType::LongLong;
    }
    static bool isText(const QVariant &v) { return v.typeId() == QMetaType::QString; }

    static double asNumber(const QVariant &v)
    {
        if (isText(v)) {
            const QString t = v.toString().trimmed();
            bool ok = false;
            const double d = t.isEmpty() ? 0 : t.toDouble(&ok);
            return t.isEmpty() || ok ? d : qQNaN();
        }
        return v.isValid() ? v.toDouble() : qQNaN();
    }

    static bool truthy(const QVariant &v)
    {
        if (isText(v))
            return !v.toString().isEmpty();
        if (isNumber(v)) {
            const double d = v.toDouble();
            return d != 0 && !qIsNaN(d);
        }
        return false;
    }

    static bool strictlyEqual(const QVariant &a, const QVariant &b)
    {
        if (isText(a) || isText(b))
            return isText(a) && isText(b) && a.toString() == b.toString();
        const bool aBool = a.typeId() == QMetaType::Bool, bBool = b.typeId() == QMetaType::Bool;
        if (aBool != bBool)
            return false;
        if (!a.isValid() || !b.isValid())
            return a.isValid() == b.isValid();
        return a.toDouble() == b.toDouble();
    }

    static bool looselyEqual(const QVariant &a, const QVariant &b)
    {
        if (!a.isValid() || !b.isValid())
            return a.isValid() == b.isValid();
        if (isText(a) && isText(b))
            return a.toString() == b.toString();
        return asNumber(a) == asNumber(b);
    }

    void skipSpace()
    {
        while (m_at < m_text.size() && m_text.at(m_at).isSpace())
            ++m_at;
    }

    bool take(QLatin1StringView token)
    {
        skipSpace();
        if (QStringView(m_text).mid(m_at).startsWith(token)) {
            m_at += token.size();
            return true;
        }
        return false;
    }

    QVariant fail()
    {
        m_failed = true;
        return {};
    }

    QVariant either()
    {
        QVariant left = both();
        while (!m_failed && take(QLatin1StringView("||"))) {
            const QVariant right = both();
            left = truthy(left) ? left : right;
        }
        return left;
    }

    QVariant both()
    {
        QVariant left = negated();
        while (!m_failed && take(QLatin1StringView("&&"))) {
            const QVariant right = negated();
            left = truthy(left) ? right : left;
        }
        return left;
    }

    QVariant negated()
    {
        skipSpace();
        if (m_at + 1 < m_text.size() && m_text.at(m_at) == QLatin1Char('!')
            && m_text.at(m_at + 1) != QLatin1Char('=')) {
            ++m_at;
            return !truthy(negated());
        }
        return compared();
    }

    QVariant compared()
    {
        const QVariant left = single();
        if (m_failed)
            return {};
        // Longest first, so that `===` is not read as `==` and a stray `=`.
        if (take(QLatin1StringView("===")))
            return strictlyEqual(left, single());
        if (take(QLatin1StringView("!==")))
            return !strictlyEqual(left, single());
        if (take(QLatin1StringView("==")))
            return looselyEqual(left, single());
        if (take(QLatin1StringView("!=")))
            return !looselyEqual(left, single());
        const auto order = [&](auto compare) -> QVariant {
            const QVariant right = single();
            if (isText(left) && isText(right))
                return compare(left.toString().compare(right.toString()), 0);
            return compare(asNumber(left), asNumber(right));
        };
        if (take(QLatin1StringView("<=")))
            return order(std::less_equal<>());
        if (take(QLatin1StringView(">=")))
            return order(std::greater_equal<>());
        if (take(QLatin1StringView("<")))
            return order(std::less<>());
        if (take(QLatin1StringView(">")))
            return order(std::greater<>());
        return left;
    }

    QVariant single()
    {
        skipSpace();
        if (m_at >= m_text.size())
            return fail();
        const QChar c = m_text.at(m_at);

        if (c == QLatin1Char('(')) {
            ++m_at;
            const QVariant inside = either();
            return take(QLatin1StringView(")")) ? inside : fail();
        }
        if (c == QLatin1Char('\'') || c == QLatin1Char('"')) {
            const qsizetype end = m_text.indexOf(c, m_at + 1);
            if (end < 0)
                return fail();
            const QString text = m_text.mid(m_at + 1, end - m_at - 1);
            m_at = end + 1;
            return text;
        }
        if (c.isDigit() || c == QLatin1Char('-') || c == QLatin1Char('.')) {
            qsizetype end = m_at + 1;
            while (end < m_text.size() && (m_text.at(end).isDigit() || m_text.at(end) == QLatin1Char('.')))
                ++end;
            bool ok = false;
            const double number = m_text.mid(m_at, end - m_at).toDouble(&ok);
            if (!ok)
                return fail();
            m_at = end;
            return number;
        }
        if (!c.isLetter() && c != QLatin1Char('_') && c != QLatin1Char('$'))
            return fail();

        const QString name = word();
        if (name == QLatin1StringView("true"))
            return true;
        if (name == QLatin1StringView("false"))
            return false;
        if (!m_values.contains(name) || !take(QLatin1StringView(".value")))
            return fail();
        const QVariant value = m_values.value(name);

        if (!take(QLatin1StringView(".")))
            return value;
        const QString method = word();
        if (!take(QLatin1StringView("(")))
            return fail();
        const QString argument = single().toString();
        if (m_failed || !take(QLatin1StringView(")")))
            return fail();
        const QString self = value.toString();
        if (method == QLatin1StringView("startsWith"))
            return self.startsWith(argument);
        if (method == QLatin1StringView("endsWith"))
            return self.endsWith(argument);
        if (method == QLatin1StringView("includes"))
            return self.contains(argument);
        return fail();
    }

    QString word()
    {
        skipSpace();
        const qsizetype start = m_at;
        while (m_at < m_text.size()
               && (m_text.at(m_at).isLetterOrNumber() || m_text.at(m_at) == QLatin1Char('_')
                   || m_text.at(m_at) == QLatin1Char('$')))
            ++m_at;
        return m_text.mid(start, m_at - start);
    }

    const QString m_text;
    const QVariantMap m_values;
    qsizetype m_at = 0;
    bool m_failed = false;
};

} // namespace

bool WallpaperCore::holds(const QString &condition, const QVariantMap &values)
{
    if (condition.trimmed().isEmpty())
        return true;
    return Condition(condition, values).holds().value_or(true);
}

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

bool audible(const QByteArray &printed)
{
    // "[Parsed_astats_0 @ 0x...] Peak level dB: -inf" -- nought throughout
    // reads as -inf. Below -70 dB nobody hears it over a room either.
    static const QRegularExpression peak(QStringLiteral("Peak level dB: (\\S+)"));
    const auto said = peak.match(QString::fromLatin1(printed));
    if (!said.hasMatch())
        return true;
    if (said.captured(1) == QLatin1StringView("-inf"))
        return false;
    bool number = false;
    const double dB = said.captured(1).toDouble(&number);
    return !number || dB > -70.0;
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
    return steamRoots({
        home + QLatin1StringView("/.local/share/Steam"),
        home + QLatin1StringView("/.steam/steam"),
        home + QLatin1StringView("/.steam/root"),
        home + QLatin1StringView("/.var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home + QLatin1StringView("/Steam"),
    });
}

QStringList WallpaperCore::steamRoots(const QStringList &installs)
{
    // Steam writes its libraries as `"path"  "/mnt/games/SteamLibrary"`, with
    // a backslash before a backslash or a quote, as in a C string.
    static const QRegularExpression pathLine(
        QStringLiteral(R"re("path"\s+"((?:[^"\\]|\\.)*)")re"));
    static const QRegularExpression escaped(QStringLiteral(R"re(\\(.))re"));

    // Most of the installs are symlinks to the first one, and every install
    // lists itself as a library. Resolving each to its real path and
    // dropping repeats is what stops every wallpaper being found twice over.
    QStringList roots;
    const auto add = [&roots](const QString &path) {
        const QString real = QFileInfo(path).canonicalFilePath();
        if (!real.isEmpty() && !roots.contains(real))
            roots.append(real);
    };
    for (const QString &install : installs) {
        add(install);
        QFile vdf(install + QLatin1StringView("/steamapps/libraryfolders.vdf"));
        if (!vdf.open(QIODevice::ReadOnly))
            continue;
        const QString text = QString::fromUtf8(vdf.readAll());
        for (const auto &match : pathLine.globalMatch(text))
            add(QString(match.captured(1)).replace(escaped, QStringLiteral("\\1")));
    }
    return roots;
}

QList<Wallpaper> WallpaperCore::scan(const QStringList &steamRoots)
{
    QList<Wallpaper> found;

    // One id, once. Wallpaper Engine installed into another library leaves
    // its workshop folder behind and Steam downloads every item again beside
    // the app (19 November, the laptop: 57 of 70 in both). Steam keeps the
    // copy in the library that holds the app, so that one is read first and
    // a later one of the same id is passed over.
    QStringList libraries = steamRoots;
    std::ranges::stable_partition(libraries, [](const QString &root) {
        return QFile::exists(root + QLatin1StringView("/steamapps/appmanifest_")
                             + WallpaperEngineAppId + QLatin1StringView(".acf"));
    });
    QSet<QString> seen;

    for (const QString &root : libraries) {
        QDir content(root + QLatin1StringView("/steamapps/workshop/content/")
                     + WallpaperEngineAppId);
        if (!content.exists())
            continue;

        const auto times = workshopTimes(root + QLatin1StringView("/steamapps/workshop/appworkshop_")
                                         + WallpaperEngineAppId + QLatin1StringView(".acf"));
        const QStringList folders =
            content.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);
        for (const QString &folder : folders) {
            if (seen.contains(folder))
                continue;
            std::optional<Wallpaper> wallpaper = readWallpaper(QDir(content.filePath(folder)));
            if (!wallpaper)
                continue;
            std::tie(wallpaper->installedAt, wallpaper->latestAt) = times.value(folder);
            seen.insert(folder);
            found.append(*wallpaper);
        }
    }

    // The ones that come with Wallpaper Engine, which no workshop folder holds:
    // every one Wallpaper Engine lists, less those its visibility.json hides.
    // ponytail: by name, the four of its scenes the renderer draws. The rest are
    // 3D models or come out black, and are listed dimmed as Wallpaper Engine
    // lists them, so that they can be read about and removed. Add to the list
    // as the renderer learns them.
    static const QStringList scenesThatDraw = {
        QStringLiteral("beach"), QStringLiteral("deep_space"),
        QStringLiteral("razer_bedroom"), QStringLiteral("razer_vortex")};
    for (const QString &root : steamRoots) {
        const QDir shipped(root + QLatin1StringView("/steamapps/common/wallpaper_engine/projects/defaultprojects"));
        const QSet<QString> hidden = hiddenIn(shipped);
        for (const QString &folder : shipped.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
            if (hidden.contains(folder))
                continue;
            std::optional<Wallpaper> wallpaper = readWallpaper(QDir(shipped.filePath(folder)));
            if (!wallpaper)
                continue;
            wallpaper->official = true;
            wallpaper->drawable =
                wallpaper->kind != WallpaperKind::Scene || scenesThatDraw.contains(folder);
            found.append(*wallpaper);
        }
    }

    // By title, because that is the order a person would look for one in.
    std::ranges::sort(found, [](const Wallpaper &a, const Wallpaper &b) {
        return readsBefore(a.title, b.title);
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

    emit wallpapersChanged();
}

Wallpaper WallpaperCore::find(const QString &id) const
{
    const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
    return match == m_wallpapers.cend() ? Wallpaper{} : *match;
}

void WallpaperCore::played(const QString &id)
{
    if (find(id).id.isEmpty())
        return;
    // Newest first, and only once in the list: picking the same wallpaper
    // again with another in between should move it, not list it twice.
    m_recent.removeAll(id);
    m_recent.prepend(id);
    emit recentChanged();
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

    const bool named = folder.dirName() == id;
    const bool scanned = std::ranges::any_of(m_roots, [&folder](const QString &root) {
        return folder.absolutePath().startsWith(QDir(root).absolutePath()
                                                + QLatin1Char('/'));
    });

    if (match->official) {
        QDir shipped = folder;
        if (!named || !shipped.cdUp() || shipped.dirName() != QLatin1StringView("defaultprojects")
            || !scanned) {
            qCWarning(lcScan) << "refusing to hide" << folder.absolutePath()
                              << "-- that is not one of Wallpaper Engine's own";
            return false;
        }
        const QString path = shipped.filePath(QStringLiteral("visibility.json"));
        QJsonObject visibility;
        if (QFile was(path); was.open(QIODevice::ReadOnly))
            visibility = QJsonDocument::fromJson(was.readAll()).object();
        visibility.insert(id, false);
        // Written whole or not at all: Wallpaper Engine reads this file too.
        QSaveFile file(path);
        if (!file.open(QIODevice::WriteOnly)
            || file.write(QJsonDocument(visibility).toJson()) < 0 || !file.commit()) {
            qCWarning(lcScan) << "could not hide" << id << "in" << path << "--"
                              << file.errorString();
            return false;
        }
        qCInfo(lcScan) << "hiding" << id << "in" << path << "as Wallpaper Engine does";
        rescan(m_roots);
        return true;
    }

    QDir workshop = folder;
    const bool belongs = workshop.cdUp()
                         && workshop.dirName() == QStringLiteral("431960");

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

            if (!told.resolution.isEmpty())
                match->resolution = told.resolution;
            // A sound track is heard out before the panel is told of it,
            // so that it does not say yes and then no.
            if (!told.sound) {
                match->soundKnown = true;
                match->hasSound = false;
            } else {
                listenTo(id, match->source.toLocalFile());
            }
            emit wallpapersChanged();
        });
        probe->start(QStringLiteral("ffprobe"), probeArguments(wallpaper.source.toLocalFile()));
    }
}

void WallpaperCore::listenTo(const QString &id, const QString &file)
{
    auto *peak = new QProcess(this);
    peak->setProcessChannelMode(QProcess::MergedChannels);
    m_probes.append(peak);
    connect(peak, &QProcess::finished, this, [this, peak, id]() {
        m_probes.removeAll(peak);
        peak->deleteLater();

        const auto match = std::ranges::find(m_wallpapers, id, &Wallpaper::id);
        if (match == m_wallpapers.end())
            return;
        match->soundKnown = true;
        match->hasSound = audible(peak->readAllStandardOutput());
        if (!match->hasSound)
            qCInfo(lcScan) << "the sound track of" << id << "is silent";
        emit wallpapersChanged();
    });
    peak->start(QStringLiteral("ffmpeg"), peakArguments(file));
}

WallpaperCore::~WallpaperCore()
{
    // Nobody is left who wanted the answers.
    stopProbes();
}

QString WallpaperCore::readable(const QString &text)
{
    const auto east = [](char32_t c) {
        switch (QChar::script(c)) {
        case QChar::Script_Han:
        case QChar::Script_Hiragana:
        case QChar::Script_Katakana:
        case QChar::Script_Hangul:
        case QChar::Script_Bopomofo:
            return true;
        default:
            // Their punctuation is Common to Unicode: the 【】 and 「」 a
            // title is wrapped in, and the ideographic comma and full stop.
            return (c >= 0x3000 && c <= 0x303F) || (c >= 0xFE30 && c <= 0xFE4F)
                || (c >= 0xFF5F && c <= 0xFFEF);
        }
    };

    QString kept;
    bool any = false;
    for (char32_t c : text.toUcs4()) {
        if (east(c)) {
            any = true;
            kept += u' ';
            continue;
        }
        // Full-width ASCII is ASCII: "（" closes a "(" as well as "）" does.
        if (c >= 0xFF01 && c <= 0xFF5E)
            c -= 0xFEE0;
        kept += QString::fromUcs4(&c, 1);
    }
    if (!any)
        return text;

    static const QRegularExpression emptied(QStringLiteral(R"(\(\s*\)|\[\s*\]|\{\s*\}|<\s*>)"));
    kept.replace(emptied, QString());

    // What separated the words that went: "A-完美【x+y】-B" leaves "A- + -B".
    // Marks are peeled off the ends of each word, a run of them is cut to
    // its first, and none is left at either end.
    const auto mark = [](QChar c) { return QStringView(u"|/·•~_+&-–—").contains(c); };
    QStringList words;
    bool afterMark = true;
    const auto add = [&](const QString &piece) {
        if (piece.isEmpty())
            return;
        const bool marks = std::all_of(piece.begin(), piece.end(), mark);
        if (marks && afterMark)
            return;
        words << piece;
        afterMark = marks;
    };
    for (QString word : kept.split(u' ', Qt::SkipEmptyParts)) {
        qsizetype from = 0, to = word.size();
        while (from < to && mark(word.at(from)))
            ++from;
        while (to > from && mark(word.at(to - 1)))
            --to;
        add(word.left(from));
        add(word.mid(from, to - from));
        add(word.mid(to));
    }
    if (afterMark && !words.isEmpty())
        words.removeLast();

    static const QRegularExpression ends(QStringLiteral(R"(^[\s,:;.]+|[\s,:;]+$)"));
    kept = words.join(u' ').replace(ends, QString());

    for (const QChar c : std::as_const(kept)) {
        if (c.isLetterOrNumber())
            return kept;
    }
    return text;
}

bool WallpaperCore::readsBefore(const QString &a, const QString &b)
{
    // Latin letters first, then digits, then everything else -- the user's
    // word of 10 November: A to Z is where a name is looked for, and a
    // title starting "3" or "【" or in another script is the exception.
    const auto group = [](const QString &title) {
        const QString t = title.trimmed();
        if (t.isEmpty())
            return 2;
        const QChar c = t.at(0);
        if (c.isLetter() && c.script() == QChar::Script_Latin)
            return 0;
        return c.isDigit() ? 1 : 2;
    };
    const int ga = group(a);
    const int gb = group(b);
    if (ga != gb)
        return ga < gb;
    return a.localeAwareCompare(b) < 0;
}
