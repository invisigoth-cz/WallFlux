// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QDateTime>
#include <QFileSystemWatcher>
#include <QList>
#include <QProcess>
#include <QTimer>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QUrl>

namespace WallpaperKind {
Q_NAMESPACE
QML_ELEMENT

/// What a wallpaper is made of, which is what decides who can draw it.
///
/// A namespace rather than a member enum so that QML can name the values
/// (`WallpaperKind.Web`) without having to reach inside a value type.
enum Kind {
    Unknown, ///< No `type` in project.json, or one we have never seen.
             ///< A scan drops these, so it is only what `current` holds
             ///< while nothing is chosen.
    Web,     ///< HTML, CSS and JavaScript. QtWebEngine draws these.
    Video,   ///< A plain video file. QtMultimedia draws these.
    Scene,   ///< Wallpaper Engine's own packed format, drawn by the renderer under crates/.
    Application, ///< A program of its own, shipped as a wallpaper. Nothing
                 ///< here runs one, and none were installed on the machine
                 ///< this was written on -- it is listed so the filter can
                 ///< name every kind the library has rather than only the
                 ///< kinds that happen to be here.
};
Q_ENUM_NS(Kind)
}

/// One wallpaper found on disk.
///
/// A value type, not an object: a scan hands back a plain list of these and
/// QML reads them straight out of it.
class Wallpaper
{
    Q_GADGET
    QML_VALUE_TYPE(wallpaper)

    Q_PROPERTY(QString id MEMBER id CONSTANT)
    Q_PROPERTY(QString title MEMBER title CONSTANT)
    Q_PROPERTY(WallpaperKind::Kind kind MEMBER kind CONSTANT)
    Q_PROPERTY(QUrl source MEMBER source CONSTANT)
    Q_PROPERTY(QUrl preview MEMBER preview CONSTANT)
    Q_PROPERTY(QStringList tags MEMBER tags CONSTANT)
    Q_PROPERTY(QString rating MEMBER rating CONSTANT)
    Q_PROPERTY(QString resolution MEMBER resolution CONSTANT)
    Q_PROPERTY(QString properties MEMBER properties CONSTANT)
    Q_PROPERTY(bool soundKnown MEMBER soundKnown CONSTANT)
    Q_PROPERTY(bool hasSound MEMBER hasSound CONSTANT)
    Q_PROPERTY(QString sizeOnDisk MEMBER sizeOnDisk CONSTANT)
    Q_PROPERTY(QDateTime added MEMBER added CONSTANT)
    Q_PROPERTY(qint64 bytes MEMBER bytes CONSTANT)
    Q_PROPERTY(qint64 installedAt MEMBER installedAt CONSTANT)
    Q_PROPERTY(qint64 latestAt MEMBER latestAt CONSTANT)
    Q_PROPERTY(bool hasFile READ hasFile CONSTANT)
    Q_PROPERTY(bool playable READ playable CONSTANT)
    Q_PROPERTY(bool official MEMBER official CONSTANT)
    Q_PROPERTY(QString author MEMBER author CONSTANT)

public:
    /// The workshop id, which is the name of the folder it sits in. The
    /// `workshopid` field inside project.json is missing from twenty of the
    /// forty-eight wallpapers here, so the folder is the only reliable name.
    QString id;
    QString title;
    WallpaperKind::Kind kind = WallpaperKind::Unknown;

    /// Absolute path to the file that *is* the wallpaper: index.html for a
    /// web one, the video file for a video one, scene.pkg for a scene -- the
    /// file project.json names for a scene lives inside that package rather
    /// than on disk.
    ///
    /// Empty when there is nothing to point at.
    QUrl source;

    /// The still or animated image Wallpaper Engine shows for this one in
    /// its own library. Every wallpaper here has one, but it is read from
    /// downloaded content like everything else, so it may be empty.
    QUrl preview;

    /// What the workshop page filed it under: Game, Sci-Fi, Landscape and
    /// the rest. Straight out of project.json, spelled as Wallpaper Engine
    /// spells it.
    QStringList tags;

    /// How old somebody should be, as `G`, `PG13` or `R`, or empty where
    /// project.json says nothing. Wallpaper Engine writes Everyone,
    /// Questionable and Mature; these are the same three under the names
    /// the interface shows. On the machine this was written on all 49 that
    /// carry one say Everyone.
    QString rating;

    /// The picture size of a video, as `1920x1080`, or empty.
    ///
    /// Read with ffprobe during the scan: nothing in project.json carries it
    /// -- checked across all 53 -- and the preview is a square thumbnail, so
    /// there is nowhere cheaper to get it. A scene or web wallpaper has no
    /// one answer and leaves this empty.
    QString resolution;

    /// The settings project.json declares, with their values and a preset's
    /// laid over them, as JSON in the shape Wallpaper Engine hands a page
    /// through `applyUserProperties`. For web wallpapers and scenes -- the
    /// panel beside the grid offers them -- and empty for a video, whose one
    /// setting is Windows' scheme colour.
    QString properties;

    /// Whether the wallpaper plays a sound of its own, once it is known --
    /// which is not the same question as the workshop's "Audio responsive",
    /// a wallpaper that follows the music somebody else is playing. The two
    /// were read as one: a panel saying "not stated" over a wallpaper with
    /// its own soundtrack looked like a panel getting it wrong.
    ///
    /// A scene says so in its scene.json, read during the scan at about a
    /// millisecond a package. A video says so to ffprobe, and is known when
    /// that answers. A web page can start a sound whenever it likes, so
    /// nothing short of playing it could say, and it is never known.
    bool soundKnown = false;
    bool hasSound = false;

    /// What the folder takes up, written out for a person to read.
    QString sizeOnDisk;

    /// When this wallpaper arrived on the disk, which is the folder's own
    /// creation time. Worth knowing that on the machine this was written on
    /// all 53 of them carry the same date, because Steam downloaded them in
    /// one go -- so ordering by it says less than it sounds like it should.
    QDateTime added;

    /// Everything in the wallpaper's folder, added up. Measured at 3
    /// milliseconds for all 53, which is why it is read during a scan rather
    /// than when somebody asks: ordering by size needs all of them at once.
    qint64 bytes = 0;

    /// When the copy on the disk was published, and the newest version
    /// Steam has heard of, in seconds since 1970 -- `timeupdated` and
    /// `latest_timeupdated` in Steam's `appworkshop_431960.acf`. A newer one
    /// than is on the disk is an update Steam has not fetched. Nought where
    /// Steam says nothing, which is never an update.
    qint64 installedAt = 0;
    qint64 latestAt = 0;

    /// Whether it came with Wallpaper Engine rather than from the workshop:
    /// one of its own, under `projects/defaultprojects`, named rather than
    /// numbered. Removing one hides it, as Wallpaper Engine does.
    bool official = false;

    /// Who made it, as a steamID64 string, where project.json says so --
    /// only Wallpaper Engine's own do, in `authorsteamid`. For a workshop
    /// item the workshop is asked instead.
    QString author;

    /// Whether the renderer can draw it. False only for those of Wallpaper
    /// Engine's own scenes it is known not to: 3D models, or black.
    bool drawable = true;

    /// Whether the file this wallpaper is made of was found on disk.
    ///
    bool hasFile() const { return !source.isEmpty(); }

    /// Whether this is one we can actually put on the screen today.
    ///
    /// Asked by the picker, to tell apart what a click can do something with
    /// from the scene wallpapers that are only listed. Kept here rather than
    /// repeated in QML so there is one answer, not two that can drift.
    bool playable() const
    {
        return hasFile() && drawable
               && (kind == WallpaperKind::Web || kind == WallpaperKind::Video
                   || kind == WallpaperKind::Scene);
    }
};

/// What ffprobe said about a video's streams.
struct Probed
{
    /// Whether it said anything at all.
    bool answered = false;
    /// The picture size as `1920x1080`, or empty.
    QString resolution;
    /// Whether there is a sound track. Six of the ten videos here carry one.
    bool sound = false;
};

/// What `ffprobe -show_entries stream=codec_type,width,height -of csv=p=0`
/// printed, read back.
Probed probedFrom(const QByteArray &printed);

/// Finds the Wallpaper Engine wallpapers installed by Steam and keeps the
/// one that was chosen.
///
/// Knows nothing about how a wallpaper is drawn; it only says what exists
/// and which one is wanted.
class WallpaperCore : public QObject
{
    Q_OBJECT
    // Declared here rather than registered from main() so that qmllint and
    // the QML compiler know the type. A runtime registration works too, but
    // it is invisible to both, and every use of it reads as an unqualified
    // name they have to guess at.
    QML_NAMED_ELEMENT(Wallpapers)
    QML_SINGLETON

    Q_PROPERTY(QList<Wallpaper> wallpapers READ wallpapers NOTIFY wallpapersChanged)
    Q_PROPERTY(Wallpaper current READ current NOTIFY currentChanged)
    Q_PROPERTY(QStringList recent READ recent NOTIFY recentChanged)

    /// The ids of everything on the disk, for anyone who has an id and
    /// wants to know whether it is already here -- the workshop's cards
    /// ask, so they can stop offering to fetch what is sitting there. A
    /// property rather than a question, so an answer that changes when the
    /// folder watcher finds something reaches whoever asked.
    Q_PROPERTY(QStringList installedIds READ installedIds NOTIFY wallpapersChanged)

public:
    /// Scans on construction: a freshly built one already knows what is
    /// installed.
    explicit WallpaperCore(QObject *parent = nullptr);
    ~WallpaperCore() override;

    QList<Wallpaper> wallpapers() const { return m_wallpapers; }
    Wallpaper current() const { return m_current; }

    /// The wallpapers chosen in this run, the last one first.
    ///
    /// This run only: what was played yesterday would have to be written
    /// down, and this program deliberately writes nothing down. Empty at
    /// startup, which is what ordering by it shows until something is
    /// played.
    QStringList recent() const { return m_recent; }

    /// Every id on the disk, so somebody holding an id can tell whether it
    /// is already here.
    QStringList installedIds() const
    {
        QStringList ids;
        ids.reserve(m_wallpapers.size());
        for (const Wallpaper &one : m_wallpapers)
            ids.append(one.id);
        return ids;
    }

    /// Looks through the Steam folders again. Called once at startup, and
    /// by itself whenever Steam puts something new in them.
    Q_INVOKABLE void rescan();

    /// Looks through these Steam installs rather than the ones found on this
    /// machine, so a test can work on a folder of its own making.
    void rescan(const QStringList &steamRoots);

    /// Makes the wallpaper with this id the current one. Returns false, and
    /// changes nothing, if no wallpaper has that id.
    Q_INVOKABLE bool select(const QString &id);

    /// Forgets the current wallpaper, leaving the screen blank.
    Q_INVOKABLE void clear();

    /// Takes one wallpaper off the disk, folder and all, and says whether
    /// it went.
    ///
    /// The other half of giving a wallpaper back, and only ever to be
    /// called once Steam has been told to unsubscribe it. Steam does not
    /// do this part: measured, `UnsubscribeItem` answers k_EResultOK and
    /// the item leaves the subscribed list -- `GetNumSubscribedItems`
    /// drops by one and `GetSubscribedItems` no longer carries it -- and
    /// then the folder stays exactly where it was, both while this program
    /// is running and a minute after it has quit. Since Steam no longer
    /// thinks it is subscribed, nothing fetches it again.
    ///
    /// Refuses anything that is not the workshop folder named after `id`,
    /// under app 431960, under a root this program scanned. It is deleting
    /// somebody's files; it does not take anybody's word for which ones.
    ///
    /// **One of Wallpaper Engine's own is hidden instead, not deleted** --
    /// what its `removeOfficialWallpaper` does: `false` against its name in
    /// `projects/defaultprojects/visibility.json`, which Wallpaper Engine
    /// reads too. Its files belong to the install, and Steam would put them
    /// back the next time it checked.
    Q_INVOKABLE bool discard(const QString &id);

    /// Reads the wallpapers under each of these Steam installs.
    ///
    /// Separate from rescan() so a test can point it at a folder of its own
    /// making rather than at whatever Steam happens to have installed.
    static QList<Wallpaper> scan(const QStringList &steamRoots);

    /// The places a Steam install is normally found on Linux, reduced to the
    /// ones that exist.
    static QStringList steamRoots();

    /// A name or a label without its Chinese, Japanese and Korean, the
    /// user's word of 10 November: for somebody who reads neither it is
    /// only noise and width. What is left is tidied -- emptied brackets and
    /// doubled separators go -- and a text that would be left with no
    /// letter or digit at all is handed back whole, since a card has to be
    /// called something.
    Q_INVOKABLE static QString readable(const QString &text);

    /// Whether title `a` comes before `b` in the library's name order:
    /// Latin letters A to Z first, then digits, then anything else, each
    /// group in the locale's own order.
    static bool readsBefore(const QString &a, const QString &b);

signals:
    void wallpapersChanged();
    void currentChanged();
    void recentChanged();

private:
    QStringList m_recent;

    /// Asks ffprobe how big each video's picture is, without waiting.
    ///
    /// Started after every scan. Waiting for it would put a third of a
    /// second on startup here and multiply by every scan a test does --
    /// measured, it took the test suite from 0.03 to 3.2 seconds. Each
    /// answer fills itself in and says so, so the picker shows what it has
    /// and gains the rest a moment later.
    void readResolutions();

    /// Takes every outstanding question back.
    void stopProbes();

    /// Watches the folders Steam downloads into, so that a wallpaper
    /// fetched from the workshop turns up here without being asked for.
    ///
    /// Steam is the only thing that can fetch one -- a workshop item for a
    /// paid app is downloadable only by an account that owns the app -- so
    /// what WallFlux can do is notice. Somebody subscribes in Steam, Steam
    /// downloads, and the grid gains a wallpaper on its own.
    void watch(const QStringList &steamRoots);

    /// One per video still being asked about.
    QList<QProcess *> m_probes;

    QFileSystemWatcher m_watcher;
    /// A download is a folder appearing and then filling up over minutes, so
    /// the scan waits for the writing to stop rather than running per file.
    QTimer m_settle;
    /// The roots the last scan used, so the one this triggers looks in the
    /// same place -- which for a test is a folder of its own making.
    QStringList m_roots;

    QList<Wallpaper> m_wallpapers;
    Wallpaper m_current;
};
