// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "tags.h"

#include <QTest>

/// Whether a tag gets Valve's own word for it.
///
/// Reads the catalogue Wallpaper Engine installed rather than a fake one:
/// what is being tested is that the key is built the way Valve files them,
/// and a fake catalogue would only prove the fake was read. Where Wallpaper
/// Engine is not installed there is nothing to be right or wrong about, so
/// those checks stand down and the fallback is still checked.
class TestTags : public QObject
{
    Q_OBJECT

private slots:
    void saysTheTagItselfWhenValveHasNoName();
    void callsATagWhatValveCallsIt();
    void findsTheOnesWhoseNameIsNotTheirKey();
    void namesEveryGenreItOffers();
    void findsEveryPictureSizeInTheCatalogue();

private:
    Tags m_tags;

    /// Whether the catalogue is there at all. Asked with a tag whose name
    /// is not the tag -- `Cartoon` would answer "Cartoon" either way, which
    /// is no answer.
    bool installed()
    {
        return m_tags.name(QStringLiteral("Unspecified"), QStringLiteral("en-us"))
               != QLatin1StringView("Unspecified");
    }
};

void TestTags::saysTheTagItselfWhenValveHasNoName()
{
    // Three of the real 65 are an age rating, which Wallpaper Engine files
    // under quite different words, and eight more are newer than any
    // catalogue on the disk. All of them have to come out readable.
    QCOMPARE(m_tags.name(QStringLiteral("Everyone"), QStringLiteral("cs-cz")),
             QStringLiteral("Everyone"));
    QCOMPARE(m_tags.name(QStringLiteral("Puppet Warp"), QStringLiteral("cs-cz")),
             QStringLiteral("Puppet Warp"));
    QCOMPARE(m_tags.name(QStringLiteral("Nothing Valve Ever Heard Of"), QStringLiteral("cs-cz")),
             QStringLiteral("Nothing Valve Ever Heard Of"));
    // And a language nobody ships must not lose the tag either.
    QCOMPARE(m_tags.name(QStringLiteral("Cartoon"), QStringLiteral("xx-xx")),
             QStringLiteral("Cartoon"));
}

void TestTags::callsATagWhatValveCallsIt()
{
    if (!installed())
        QSKIP("Wallpaper Engine is not installed, so there is no catalogue to read");

    QCOMPARE(m_tags.name(QStringLiteral("Cartoon"), QStringLiteral("cs-cz")),
             QStringLiteral("Skica"));
    QCOMPARE(m_tags.name(QStringLiteral("Vehicle"), QStringLiteral("cs-cz")),
             QStringLiteral("Dopravní Prostředky"));
    // Two words, so the underscore in the key has to be put there.
    QCOMPARE(m_tags.name(QStringLiteral("Audio responsive"), QStringLiteral("cs-cz")),
             QStringLiteral("Reaguje na zvuk"));
    // A dash, which is not a space.
    QCOMPARE(m_tags.name(QStringLiteral("Sci-Fi"), QStringLiteral("cs-cz")),
             QStringLiteral("Sci-fi"));
}

void TestTags::findsTheOnesWhoseNameIsNotTheirKey()
{
    if (!installed())
        QSKIP("Wallpaper Engine is not installed, so there is no catalogue to read");

    // The point of looking a tag up by its key. Steam says `Unspecified`
    // and `3840 x 2160`; Valve's name for each says more than that, so
    // matching on the name would find neither.
    QCOMPARE(m_tags.name(QStringLiteral("Unspecified"), QStringLiteral("cs-cz")),
             QStringLiteral("Nespecifikovaný žánr"));
    QCOMPARE(m_tags.name(QStringLiteral("3840 x 2160"), QStringLiteral("en-us")),
             QStringLiteral("3840 x 2160 - 4K"));
    QCOMPARE(m_tags.name(QStringLiteral("MMD"), QStringLiteral("en-us")),
             QStringLiteral("MMD (Miku-Miku Dance)"));
}

void TestTags::namesEveryGenreItOffers()
{
    if (!installed())
        QSKIP("Wallpaper Engine is not installed, so there is no catalogue to read");

    // A genre nobody has a word for is a chip that reads as a Steam tag in
    // the middle of a translated list, which is how a wrong spelling here
    // would show itself.
    const QStringList genres = m_tags.genres();
    QCOMPARE(genres.size(), 25);
    for (const QString &genre : genres) {
        const QString called = m_tags.name(genre, QStringLiteral("cs-cz"));
        QVERIFY2(!called.isEmpty(), qPrintable(genre));
        // Czech differs from the English for most of them; these are the
        // ones Valve leaves alone, so only the rest prove a name was found.
        static const QStringList sameInBoth = {
            QStringLiteral("Anime"), QStringLiteral("CGI"), QStringLiteral("Cyberpunk"),
            QStringLiteral("Fantasy"), QStringLiteral("Pixel art"), QStringLiteral("Retro"),
            QStringLiteral("MMD"),
        };
        if (!sameInBoth.contains(genre))
            QVERIFY2(called != genre, qPrintable(genre + QLatin1StringView(" got no name")));
    }
}

/// Every picture size the filter offers has to be one Valve knows, or the
/// chip reads as a raw Steam tag among translated ones -- and, worse, is a
/// tag no wallpaper carries, so ticking it finds nothing. A size spelled
/// wrong here looks exactly like a size nobody uses.
void TestTags::findsEveryPictureSizeInTheCatalogue()
{
    if (!installed())
        QSKIP("Wallpaper Engine is not installed, so there is no catalogue to read");

    const QStringList sizes = m_tags.resolutions();
    QVERIFY(!sizes.isEmpty());

    for (const QString &size : sizes) {
        const QString called = m_tags.name(size, QStringLiteral("cs-cz"));
        QVERIFY2(!called.isEmpty(), qPrintable(size));
        // The catalogue gives the numbers back unchanged, so a name that
        // matches is no proof on its own. What proves it is the ones
        // Valve words: the four portrait sizes and the three standard
        // definitions all read differently from the tag itself.
        static const QStringList valveWordsThese = {
            QStringLiteral("Standard Definition"),
            QStringLiteral("Ultrawide Standard Definition"),
            QStringLiteral("Portrait Standard Definition"),
            QStringLiteral("Dynamic resolution"),
            QStringLiteral("Other resolution"),
        };
        if (valveWordsThese.contains(size))
            QVERIFY2(called != size, qPrintable(size + QLatin1StringView(" got no name")));
    }

    // The five that were missing, by name, so that taking them out again
    // breaks this rather than quietly shortening a list.
    QVERIFY(sizes.contains(QStringLiteral("Portrait 720 x 1280")));
    QVERIFY(sizes.contains(QStringLiteral("Portrait 1080 x 1920")));
    QVERIFY(sizes.contains(QStringLiteral("Portrait 1440 x 2560")));
    QVERIFY(sizes.contains(QStringLiteral("Portrait 2160 x 3840")));
    QVERIFY(sizes.contains(QStringLiteral("Portrait Standard Definition")));

    // And each of those really is a key the catalogue has, rather than
    // five strings that happen to come back unchanged.
    QCOMPARE(m_tags.name(QStringLiteral("Portrait Standard Definition"),
                         QStringLiteral("en-us")),
             QStringLiteral("Portrait standard"));
}

QTEST_MAIN(TestTags)
#include "tst_tags.moc"
