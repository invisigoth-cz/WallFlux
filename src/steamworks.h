// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QList>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

/// Whether this machine can be asked about the Steam Workshop, and the way in.
///
/// Nothing of Valve's is shipped with WallFlux and nothing is linked against
/// at build time. The library is looked for when it is first wanted and
/// opened with dlopen; where it is not there, this reports as much and the
/// interface falls back to sending somebody to the Steam client by hand.
///
/// Why not ship it: the licence that travels with the Steamworks SDK permits
/// redistributing it only in its entirety, unmodified, not integrated with
/// another program, and with its own agreement put in front of the end user
/// -- and the agreement that actually governs the SDK is behind a partner
/// login and cannot be read. Somebody who has the SDK accepted those terms
/// themselves; WallFlux stays out of it by never being the one to hand it on.
///
/// The GPL exception in LICENSE-EXCEPTION is what makes even this much
/// allowed from our side.
class Steamworks : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Steamworks)
    QML_SINGLETON

    /// Whether Steam answered and the workshop interface is there to ask.
    ///
    /// Not constant: Steam can be started after WallFlux was, and asking
    /// again is what `retry()` is for.
    Q_PROPERTY(bool available READ isAvailable NOTIFY availableChanged)

    /// Why not -- as a reason, not as a sentence.
    ///
    /// A sentence built here is built once, in whatever language was
    /// installed at that moment, and stays that way: `retranslate()`
    /// refreshes bindings that do their own translating, and a string
    /// handed over from C++ is not one. Measured -- the heading changed
    /// with the language and the reason under it did not. So the words
    /// belong to the interface, and this says only which words.
    Q_PROPERTY(Steamworks::Trouble trouble READ trouble NOTIFY availableChanged)

    /// What each question came back with, under the name the asker gave
    /// it: a list of maps, each with `id`, `title`, `preview`, `tags`,
    /// `bytes` and `votesUp`.
    ///
    /// Named rather than one shared list, because the window shows many
    /// rows at once -- Trending, New and a row per genre -- and filling
    /// them from one list would mean every row after the first showing the
    /// tail of the one before.
    Q_PROPERTY(QVariantMap found READ found NOTIFY foundChanged)

    /// How many wallpapers the workshop says match each question, which is
    /// what a page count is worked out from. Fifty come back at a time.
    Q_PROPERTY(QVariantMap totals READ totals NOTIFY foundChanged)

    /// Whether a question is outstanding.
    Q_PROPERTY(bool busy READ isBusy NOTIFY busyChanged)

    /// How far along each download is, keyed by published file id: 0 to 100
    /// while it runs, 100 once Steam says it is there, and -1 where Steam
    /// would not do it. An id that is absent was never asked for.
    ///
    /// One map rather than a signal per wallpaper, because what wants to
    /// know is a button on a card and a binding is how a card asks.
    Q_PROPERTY(QVariantMap downloads READ downloads NOTIFY downloadsChanged)

    /// The ids of the wallpapers the account has made favourites on Steam.
    Q_PROPERTY(QStringList favorites READ favorites NOTIFY favoritesChanged)

public:
    /// Why the workshop cannot be asked.
    enum Trouble {
        Fine,            ///< Nothing the matter.
        NoLibrary,       ///< No libsteam_api.so anywhere this looks.
        UnknownLibrary,  ///< One is there, but not one this can talk to.
        SteamSilent,     ///< Steam is not running, or does not own the app.
        NoWorkshop,      ///< Steam answered, but offered no workshop.
    };
    Q_ENUM(Trouble)

    /// What to put at the top, in the order the picker's own menu has them.
    enum Order {
        Trending,     ///< What the workshop is making a fuss of now.
        Newest,       ///< By the date it was published.
        MostSubscribed, ///< By how many people took it.
        MyFavorites,  ///< The account's own favourites, newest first.
        FriendsFavorites, ///< What the account's friends made favourites.
    };
    Q_ENUM(Order)

    explicit Steamworks(QObject *parent = nullptr);
    ~Steamworks() override;

    bool isAvailable() const { return m_ugc != nullptr; }
    Trouble trouble() const { return m_trouble; }
    QVariantMap found() const { return m_found; }
    QVariantMap totals() const { return m_totals; }
    bool isBusy() const { return m_call != 0; }
    QVariantMap downloads() const { return m_downloads; }
    QStringList favorites() const { return m_favorites; }

    /// Asks the workshop for one page of wallpapers, fifty at a time, which
    /// is what Steam gives, and files the answer under `key`.
    ///
    /// `tags` narrows it to wallpapers carrying all of them, spelled as
    /// Steam spells them -- `Sci-Fi`, `MMD`, `Everyone`, `3840 x 2160` --
    /// which is what makes a genre row and the filters one mechanism. A tag
    /// with a `-` in front leaves out what carries it instead: `-Anime` is
    /// Wallpaper Engine's "Hide Anime".
    /// `search` is free text; giving any switches the order to ranking by
    /// text, since that is the only ranking a search means anything under.
    ///
    /// Asking again while one is outstanding is allowed: the questions are
    /// asked one after another, since a query handle has to be let go of
    /// before the next is made. Asking the same `key` again replaces what
    /// was under it -- in the queue as well, so three clicks on Next mean
    /// page four rather than pages two, three and four in turn.
    ///
    /// `urgent` is for a question somebody is sitting in front of waiting
    /// for: it goes ahead of whatever is still queued. Discover asks one
    /// question per row and builds a dozen rows, and a question takes
    /// between a tenth of a second and three quarters of one -- measured --
    /// so a search typed on arriving at the tab sat behind several seconds
    /// of rows nobody had looked at yet.
    ///
    /// `creator` narrows it to one account's published work, as a
    /// steamID64 -- which is a different call of Steam's, with its own
    /// list of orders, so `order` is ignored while it is set.
    Q_INVOKABLE void browse(const QString &key, Steamworks::Order order, int page = 1,
                            const QStringList &tags = {}, const QString &search = {},
                            bool urgent = false, const QString &creator = {});

    /// Subscribes to one wallpaper and asks Steam to fetch it now. Nothing
    /// is fetched here: Steam downloads it into the folder it already owns,
    /// and the watcher on that folder is what puts it in the library -- so
    /// there is nothing to do once this has been called but watch
    /// `downloads` and wait.
    Q_INVOKABLE void download(const QString &id);

    /// Gives one wallpaper back.
    ///
    /// Unsubscribing is the whole of it: the folder is Steam's, Steam takes
    /// it away, and the watcher on that folder is what takes the card out
    /// of the grid. Nothing here deletes a file, and subscribing again is
    /// all it takes to have it back.
    Q_INVOKABLE void remove(const QString &id);

    /// Asks Steam again, for when it was not running a moment ago and is
    /// now. Does nothing where the workshop can already be asked.
    ///
    /// Without this, being told to start Steam would be useless advice:
    /// the library is opened once when the workshop is first wanted, and
    /// starting Steam afterwards would change nothing until WallFlux was
    /// restarted.
    Q_INVOKABLE void retry();

    /// Makes one wallpaper one of the account's favourites on Steam, or
    /// takes it off -- the heart Wallpaper Engine draws, which is the
    /// workshop's own favourite and so the same on every machine the
    /// account signs in to. `favorites` says so at once rather than when
    /// Steam answers, since the heart is what was clicked.
    Q_INVOKABLE void favorite(const QString &id, bool on);

signals:
    void availableChanged();
    void foundChanged();
    void busyChanged();
    void downloadsChanged();
    void favoritesChanged();

private:
    /// Opens libsteam_api.so from wherever it is, or says where it looked.
    bool open();

    /// Steam has no thread of its own here: its answers are collected by
    /// asking, which is what this does while a question is outstanding.
    void pump();

    /// Reads one finished query out of Steam and into `found`.
    void collect(quint64 call, int callbackId, int paramSize);

    /// Asks whatever question has been waiting its turn.
    void askNext();

    /// Asks Steam how far each outstanding download has got.
    void measure();

    /// Asks for the account's favourites from the first page: the answers
    /// come under `FavoritesKey` and a page number and are gathered into
    /// `m_favorites` rather than `found`.
    void askFavorites(int page);

    /// Whether the pump still has anything to pump for.
    bool waiting() const
    {
        return m_call != 0 || !m_downloading.isEmpty() || m_draining > 0;
    }

    /// How many more turns of the pump to take for callbacks nobody is
    /// waiting for. Unsubscribing answers with one, and a callback that is
    /// never freed stops the queue for everything after it -- so the pump
    /// is left running for a moment rather than stopped the instant the
    /// last thing anybody wanted has arrived.
    int m_draining = 0;

    void *m_library = nullptr;
    /// The ISteamUGC pointer, which is what everything about the workshop is
    /// asked through. Null until open() has worked.
    void *m_ugc = nullptr;
    Trouble m_trouble = Fine;

    /// One question, whole, so it can wait its turn without being taken
    /// apart first.
    struct Question
    {
        QString key;
        Order order = Trending;
        int page = 1;
        QStringList tags;
        QString search;
        QString creator;
    };

    /// What is being asked now, so the answer lands under the right name,
    /// and what is still waiting to be asked.
    QString m_asking;
    QList<Question> m_queued;
    QVariantMap m_found;
    QVariantMap m_totals;

    /// What is on its way down, and how far each has got. The list is the
    /// ids to ask about; the map is what the interface reads.
    QList<quint64> m_downloading;
    QVariantMap m_downloads;

    /// The account's favourites by id, and the pages of them gathered so
    /// far while they are being asked for.
    QStringList m_favorites;
    QStringList m_gathering;

    /// The account signed in to Steam, as a steamID64, for the questions
    /// about its own favourites. Nought where it could not be asked.
    quint64 m_self = 0;

    /// The pipe callbacks come down, and what is outstanding on it.
    qint32 m_pipe = 0;
    quint64 m_query = 0;
    quint64 m_call = 0;
    QTimer m_pump;

    // The library's own entry points, found by name. Spelled out rather
    // than taken from Valve's headers, which are deliberately not here.
    void (*m_shutdown)() = nullptr;
    void (*m_runFrame)(qint32) = nullptr;
    bool (*m_nextCallback)(qint32, void *) = nullptr;
    void (*m_freeCallback)(qint32) = nullptr;
    bool (*m_callResult)(qint32, quint64, void *, int, int, bool *) = nullptr;
    quint64 (*m_createQuery)(void *, int, int, quint32, quint32, quint32) = nullptr;
    quint64 (*m_createUserQuery)(void *, quint32, int, int, int, quint32, quint32,
                                 quint32) = nullptr;
    quint64 (*m_sendQuery)(void *, quint64) = nullptr;
    bool (*m_queryResult)(void *, quint64, quint32, void *) = nullptr;
    bool (*m_previewUrl)(void *, quint64, quint32, char *, quint32) = nullptr;
    bool (*m_releaseQuery)(void *, quint64) = nullptr;
    bool (*m_requireTag)(void *, quint64, const char *) = nullptr;
    bool (*m_excludeTag)(void *, quint64, const char *) = nullptr;
    bool (*m_searchText)(void *, quint64, const char *) = nullptr;
    quint64 (*m_subscribe)(void *, quint64) = nullptr;
    quint64 (*m_unsubscribe)(void *, quint64) = nullptr;
    quint64 (*m_addFavorite)(void *, quint32, quint64) = nullptr;
    quint64 (*m_removeFavorite)(void *, quint32, quint64) = nullptr;
    bool (*m_downloadItem)(void *, quint64, bool) = nullptr;
    bool (*m_downloadInfo)(void *, quint64, quint64 *, quint64 *) = nullptr;
};
