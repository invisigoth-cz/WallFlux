// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "tags.h"
#include "wallpapercore.h"

#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QRegularExpression>
#include <QVariantMap>

#include <algorithm>

Q_LOGGING_CATEGORY(lcTags, "wallflux.tags")

namespace {

/// What every workshop tag's key starts with in Wallpaper Engine's
/// catalogues. 103 keys carry it.
constexpr QLatin1StringView KeyPrefix("ui_workshop_tags_");

/// The key a tag is filed under: lowercased, with every run of anything
/// else turned into one underscore. `Sci-Fi` is `sci_fi`, `Pixel art` is
/// `pixel_art`, `3840 x 2160` is `3840_x_2160`.
///
/// Only ever used this way round. Backwards -- name to key -- it is wrong
/// for 38 of the 103, because the `asset_type_` and `dual_` keys carry a
/// prefix their name does not and the resolutions carry a "- 4K" their key
/// does not.
QString keyFor(const QString &tag)
{
    static const QRegularExpression anythingElse(QStringLiteral("[^a-z0-9]+"));
    QString key = tag.toLower().replace(anythingElse, QStringLiteral("_"));
    while (key.endsWith(u'_'))
        key.chop(1);
    while (key.startsWith(u'_'))
        key.remove(0, 1);
    return key;
}

/// Wallpaper Engine's own translations, wherever Steam put them.
QString catalogueDir()
{
    for (const QString &root : WallpaperCore::steamRoots()) {
        const QString dir =
            root + QLatin1StringView("/steamapps/common/wallpaper_engine/locale");
        if (QFileInfo::exists(dir))
            return dir;
    }
    return {};
}

} // namespace

Tags::Tags(QObject *parent)
    : QObject(parent)
{
}

QStringList Tags::genres() const
{
    // Alphabetical by the English, which is the order Wallpaper Engine's own
    // list is in.
    return {
        QStringLiteral("Abstract"),   QStringLiteral("Animal"),
        QStringLiteral("Anime"),      QStringLiteral("Cartoon"),
        QStringLiteral("CGI"),        QStringLiteral("Cyberpunk"),
        QStringLiteral("Fantasy"),    QStringLiteral("Game"),
        QStringLiteral("Girls"),      QStringLiteral("Guys"),
        QStringLiteral("Landscape"),  QStringLiteral("Medieval"),
        QStringLiteral("Memes"),      QStringLiteral("MMD"),
        QStringLiteral("Music"),      QStringLiteral("Nature"),
        QStringLiteral("Pixel art"),  QStringLiteral("Relaxing"),
        QStringLiteral("Retro"),      QStringLiteral("Sci-Fi"),
        QStringLiteral("Sports"),     QStringLiteral("Technology"),
        QStringLiteral("Television"), QStringLiteral("Vehicle"),
        QStringLiteral("Unspecified"),
    };
}

QStringList Tags::types() const
{
    // What was seen, not what the catalogue lists: the catalogue also has
    // Application and Asset pack, neither of which turned up in 1100
    // wallpapers, and a filter for something nobody has is a dead chip.
    return {
        QStringLiteral("Scene"), QStringLiteral("Video"),  QStringLiteral("Web"),
        QStringLiteral("Image"), QStringLiteral("Preset"), QStringLiteral("Asset"),
    };
}

QStringList Tags::ratings() const
{
    return {
        QStringLiteral("Everyone"),
        QStringLiteral("Questionable"),
        QStringLiteral("Mature"),
    };
}

QStringList Tags::resolutions() const
{
    // ponytail: the ones seen over 22 queries, in reading order rather than
    // alphabetical. The catalogue knows of 8K and a few more that did not
    // turn up; add them here if somebody misses one.
    return {
        QStringLiteral("1280 x 720"),
        QStringLiteral("1366 x 768"),
        QStringLiteral("1920 x 1080"),
        QStringLiteral("2560 x 1440"),
        QStringLiteral("3840 x 2160"),
        QStringLiteral("Ultrawide 2560 x 1080"),
        QStringLiteral("Ultrawide 3440 x 1440"),
        QStringLiteral("Dual 5120 x 1440"),
        QStringLiteral("Dual 7680 x 2160"),
        QStringLiteral("Triple 7680 x 1440"),
        QStringLiteral("Portrait 720 x 1280"),
        QStringLiteral("Portrait 1080 x 1920"),
        QStringLiteral("Portrait 1440 x 2560"),
        QStringLiteral("Portrait 2160 x 3840"),
        QStringLiteral("Standard Definition"),
        QStringLiteral("Ultrawide Standard Definition"),
        QStringLiteral("Portrait Standard Definition"),
        QStringLiteral("Dynamic resolution"),
        QStringLiteral("Other resolution"),
    };
}

QVariantList Tags::labelled(const QStringList &tags, const QString &locale)
{
    QStringList sorted = tags;
    std::sort(sorted.begin(), sorted.end(),
              [this, &locale](const QString &first, const QString &second) {
                  return QString::localeAwareCompare(name(first, locale),
                                                     name(second, locale)) < 0;
              });

    QVariantList chips;
    chips.reserve(sorted.size());
    for (const QString &tag : std::as_const(sorted)) {
        chips.append(QVariantMap{{QStringLiteral("v"), tag},
                                 {QStringLiteral("l"), name(tag, locale)}});
    }
    return chips;
}

QString Tags::name(const QString &tag, const QString &locale)
{
    const auto known = m_read.constFind(locale);
    if (known != m_read.constEnd())
        return known->value(keyFor(tag), tag);

    QHash<QString, QString> names;

    const QString dir = catalogueDir();
    if (dir.isEmpty()) {
        // Wallpaper Engine is not installed, or not where Steam usually
        // puts it. Every tag then shows as Steam spells it, which is
        // readable English rather than nothing.
        qCInfo(lcTags) << "no Wallpaper Engine catalogue; tags stay as Steam spells them";
        m_read.insert(locale, names);
        return tag;
    }

    QFile catalogue(dir + QLatin1StringView("/ui_") + locale + QLatin1StringView(".json"));
    if (!catalogue.open(QIODevice::ReadOnly)) {
        qCInfo(lcTags) << "no catalogue for" << locale << "at" << catalogue.fileName();
        m_read.insert(locale, names);
        return tag;
    }

    QJsonParseError trouble;
    const QJsonDocument read = QJsonDocument::fromJson(catalogue.readAll(), &trouble);
    if (trouble.error != QJsonParseError::NoError || !read.isObject()) {
        qCWarning(lcTags) << catalogue.fileName() << "is not an object:" << trouble.errorString();
        m_read.insert(locale, names);
        return tag;
    }

    const QJsonObject all = read.object();
    for (auto entry = all.constBegin(); entry != all.constEnd(); ++entry) {
        if (!entry.key().startsWith(KeyPrefix))
            continue;
        const QString called = entry.value().toString();
        if (!called.isEmpty())
            names.insert(entry.key().sliced(KeyPrefix.size()), called);
    }

    qCInfo(lcTags) << "read" << names.size() << "tag names for" << locale;
    m_read.insert(locale, names);
    return names.value(keyFor(tag), tag);
}
