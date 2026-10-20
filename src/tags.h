// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QHash>
#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QStringList>
#include <QVariantList>

/// What the workshop's tags are called, and which of them name a genre.
///
/// **This is not a list of tags.** Which tags exist comes from the data --
/// `project.json` beside each installed wallpaper, and the `tags` of each
/// answer to a workshop query. Measured over 22 live queries, about 1100
/// wallpapers, Steam uses 65 of them. Two things that cannot be read off
/// that data are here instead: what to call one on screen, and which 25 of
/// them a person thinks of as a genre.
///
/// The names are Valve's own and are not typed out here. Wallpaper Engine's
/// install carries `locale/` in 36 languages, so a tag is called whatever
/// Valve calls it -- `Cartoon` is "Skica" in Czech, `Vehicle` is "Dopravní
/// Prostředky" -- and `wallflux_cs.ts` never sees a tag name. Where Valve
/// has none, which is true of 11 of the 65, the tag itself is shown.
class Tags : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Tags)
    QML_SINGLETON

    /// The genres, spelled as Steam spells them -- `MMD`, not the
    /// "MMD (Miku-Miku Dance)" it is displayed as.
    ///
    /// The only tag list written out in this program, and it is written out
    /// because it is nowhere on the disk: `locale/` and `ui/` were both
    /// searched and neither says which tags are a genre. Every one of these
    /// was seen coming back from Steam.
    Q_PROPERTY(QStringList genres READ genres CONSTANT)

    /// What a wallpaper is: the tag Wallpaper Engine marks the kind with.
    Q_PROPERTY(QStringList types READ types CONSTANT)

    /// Steam's three, which are not Wallpaper Engine's three. The catalogue
    /// files them as "G - All ages", "PG13 - Parental guidance" and
    /// "R - Mature"; what Steam actually tags with is these.
    Q_PROPERTY(QStringList ratings READ ratings CONSTANT)

    /// Every picture size a monitor has. The telephone ones Wallpaper
    /// Engine also tags are left out.
    Q_PROPERTY(QStringList resolutions READ resolutions CONSTANT)

    /// What a wallpaper does rather than what it is of: audio responsive,
    /// customizable, approved and the rest.
    Q_PROPERTY(QStringList properties READ properties CONSTANT)

public:
    explicit Tags(QObject *parent = nullptr);

    QStringList genres() const;
    QStringList types() const;
    QStringList ratings() const;
    QStringList resolutions() const;
    QStringList properties() const;

    /// What `tag` is called in `locale` ("cs-cz", "en-us"), or `tag` itself
    /// where Valve has no name for it.
    ///
    /// Looked up by key rather than by English name, which is what reaches
    /// the two that differ: `Unspecified` is keyed `..._unspecified` but
    /// reads "Unspecified genre", and `3840 x 2160` reads "3840 x 2160 - 4K".
    /// Each language is read off the disk once and kept.
    Q_INVOKABLE QString name(const QString &tag, const QString &locale);

    /// The same, for a whole list, as `{ v, l }` pairs ready for a row of
    /// chips: what the tag means to Steam and what it says to a person.
    /// Ordered by what it says, the way the language orders words, since
    /// what it says is what is read.
    Q_INVOKABLE QVariantList labelled(const QStringList &tags, const QString &locale);

private:
    /// One language's names, read off the disk, keyed by the part of the
    /// key after `ui_workshop_tags_`.
    QHash<QString, QString> read(const QString &locale) const;

    /// locale -> the part of the key after `ui_workshop_tags_` -> the name.
    QHash<QString, QHash<QString, QString>> m_read;
};
