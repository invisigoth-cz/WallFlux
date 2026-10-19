// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#pragma once

#include <QHash>
#include <QList>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QUrl>
#include <QVariantMap>

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
    Scene,   ///< Wallpaper Engine's own packed format. Nothing here draws it.
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
    Q_PROPERTY(bool hasFile READ hasFile CONSTANT)
    Q_PROPERTY(bool playable READ playable CONSTANT)

public:
    /// The workshop id, which is the name of the folder it sits in. The
    /// `workshopid` field inside project.json is missing from twenty of the
    /// forty-eight wallpapers here, so the folder is the only reliable name.
    QString id;
    QString title;
    WallpaperKind::Kind kind = WallpaperKind::Unknown;

    /// Absolute path to the file that *is* the wallpaper: index.html for a
    /// web one, the video file for a video one.
    ///
    /// Empty when there is nothing to point at -- which is the normal case
    /// for a scene wallpaper, whose entry file lives inside scene.pkg rather
    /// than on disk.
    QUrl source;

    /// The still or animated image Wallpaper Engine shows for this one in
    /// its own library. Every wallpaper here has one, but it is read from
    /// downloaded content like everything else, so it may be empty.
    QUrl preview;

    /// Whether the file this wallpaper is made of was found on disk.
    ///
    /// Not a promise that it can be drawn: a scene wallpaper keeps its own
    /// format even when the file is there.
    bool hasFile() const { return !source.isEmpty(); }

    /// Whether this is one we can actually put on the screen today.
    ///
    /// Asked by the picker, to tell apart what a click can do something with
    /// from the scene wallpapers that are only listed. Kept here rather than
    /// repeated in QML so there is one answer, not two that can drift.
    bool playable() const
    {
        return hasFile() && (kind == WallpaperKind::Web || kind == WallpaperKind::Video);
    }
};

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

public:
    /// Scans on construction: a freshly built one already knows what is
    /// installed.
    explicit WallpaperCore(QObject *parent = nullptr);

    QList<Wallpaper> wallpapers() const { return m_wallpapers; }
    Wallpaper current() const { return m_current; }

    /// Looks through the Steam folders again. Called once at startup.
    Q_INVOKABLE void rescan();

    /// Looks through these Steam installs rather than the ones found on this
    /// machine, so a test can work on a folder of its own making.
    void rescan(const QStringList &steamRoots);

    /// Makes the wallpaper with this id the current one. Returns false, and
    /// changes nothing, if no wallpaper has that id.
    Q_INVOKABLE bool select(const QString &id);

    /// Forgets the current wallpaper, leaving the screen blank.
    Q_INVOKABLE void clear();

    /// What this wallpaper takes up on disk and, for a video, how big its
    /// picture is: `{ "size": "87.4 MiB", "resolution": "1920x1080" }`.
    ///
    /// Asked for one wallpaper at a time, when the picker is showing the
    /// details of one, because neither answer is free and neither is wanted
    /// for the other fifty. Both are missing from project.json -- checked
    /// across all 53 of them -- so they are read off the disk instead, and
    /// the answer is kept once it has been worked out. An entry is empty
    /// where it could not be had, which the picker shows as "Nativní".
    Q_INVOKABLE QVariantMap details(const QString &id) const;

    /// Reads the wallpapers under each of these Steam installs.
    ///
    /// Separate from rescan() so a test can point it at a folder of its own
    /// making rather than at whatever Steam happens to have installed.
    static QList<Wallpaper> scan(const QStringList &steamRoots);

    /// The places a Steam install is normally found on Linux, reduced to the
    /// ones that exist.
    static QStringList steamRoots();

signals:
    void wallpapersChanged();
    void currentChanged();

private:
    /// Worked out the first time the picker asks and kept afterwards, since
    /// nothing about a file on disk changes while the program runs. Cleared
    /// by a rescan, which is when it might have.
    mutable QHash<QString, QVariantMap> m_details;

    QList<Wallpaper> m_wallpapers;
    Wallpaper m_current;
};
