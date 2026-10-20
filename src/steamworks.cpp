// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "steamworks.h"

#include <QFileInfo>
#include <QLoggingCategory>
#include <QVariantMap>

#include <cstddef>

#include <dlfcn.h>

Q_LOGGING_CATEGORY(lcSteam, "wallflux.steam")

namespace {

/// Wallpaper Engine. Steam checks that the account running this owns it, and
/// will not answer otherwise -- which is the whole reason the workshop needs
/// Steam at all.
constexpr auto WallpaperEngineAppId = "431960";

/// The version of ISteamUGC these calls are written against. The client
/// keeps older interface versions working, so pinning one is what makes this
/// survive a newer Steam rather than what breaks it.
constexpr auto UgcInterfaceVersion = "STEAMUGC_INTERFACE_VERSION018";

/// Where libsteam_api.so is looked for, in order.
///
/// Nothing here is shipped by WallFlux. The first is for anyone whose copy
/// is somewhere of their own choosing, and is also how this gets tested; the
/// second is where the steamworks-sdk package puts it; the last lets the
/// dynamic loader look in the usual places.
QStringList candidates()
{
    QStringList paths;
    if (const QByteArray told = qgetenv("WALLFLUX_STEAM_API"); !told.isEmpty())
        paths << QString::fromLocal8Bit(told);
    paths << QStringLiteral("/opt/steamworks-sdk/redistributable_bin/linux64/libsteam_api.so")
          << QStringLiteral("libsteam_api.so");
    return paths;
}


// What Steam's answers look like in memory.
//
// Valve's headers are not here on purpose, so these are written out -- and
// every offset was measured by compiling against those headers once and
// printing offsetof, not guessed. The asserts are the measurement: get a
// field wrong and this stops at build time instead of reading somebody
// else's bytes at run time. Steamworks SDK 1.58.
#pragma pack(push, 4)

/// One wallpaper, as the workshop describes it.
struct UgcDetails
{
    quint64 id;                   // 0
    qint32 result;                // 8
    qint32 fileType;              // 12
    char _apps[8];                // 16  creator and consumer app id
    char title[129];              // 24
    char description[8000];       // 153
    char _untilOwner[3];          // 8153
    quint64 owner;                // 8156
    quint32 created;              // 8164
    quint32 updated;              // 8168
    char _untilBanned[8];         // 8172 added to list, visibility
    char banned;                  // 8180
    char acceptedForUse;          // 8181
    char tagsTruncated;           // 8182
    char tags[1025];              // 8183  comma separated
    char _untilSize[276];         // 9208  file handles and the cloud name
    qint32 fileSize;              // 9484
    qint32 previewFileSize;       // 9488
    char url[256];                // 9492
    quint32 votesUp;              // 9748
    quint32 votesDown;            // 9752
    float score;                  // 9756
    quint32 children;             // 9760
};

/// What arrives when a query has finished. Only the head of it is read.
struct QueryCompleted
{
    quint64 handle;               // 0
    qint32 result;                // 8
    quint32 returned;             // 12
    quint32 total;                // 16
    char cached;                  // 20
    char cursor[256];             // 21
};

/// The envelope every callback arrives in.
struct CallbackMsg
{
    qint32 user;
    qint32 callback;
    void *param;
    qint32 paramSize;
};

/// The envelope that says an asked-for call has an answer waiting.
struct CallCompleted
{
    quint64 call;
    qint32 callback;
    qint32 paramSize;
};

/// What arrives when one download has finished, or given up. Unlike a
/// query, this is not an answer to a call we made: it turns up on the pipe
/// by itself, with its own callback number, which is why pump() has to look
/// for it rather than collect() reading it.
struct DownloadFinished
{
    quint32 app;                  // 0
    quint64 id;                   // 4
    qint32 result;                // 12
};

#pragma pack(pop)

static_assert(sizeof(UgcDetails) == 9764);
static_assert(offsetof(UgcDetails, title) == 24);
static_assert(offsetof(UgcDetails, owner) == 8156);
static_assert(offsetof(UgcDetails, tags) == 8183);
static_assert(offsetof(UgcDetails, fileSize) == 9484);
static_assert(offsetof(UgcDetails, votesUp) == 9748);
static_assert(sizeof(QueryCompleted) == 280);
static_assert(sizeof(CallbackMsg) == 20);
static_assert(offsetof(CallbackMsg, param) == 8);
static_assert(sizeof(CallCompleted) == 16);
static_assert(sizeof(DownloadFinished) == 16);
static_assert(offsetof(DownloadFinished, id) == 4);
static_assert(offsetof(DownloadFinished, result) == 12);

/// Callback numbers, and the few enum values worth naming. Measured the same
/// way.
constexpr int CallCompletedId = 703;
constexpr int QueryCompletedId = 3401;
constexpr int DownloadFinishedId = 3406;
constexpr qint32 ResultOk = 1;
constexpr int MatchingItemsReadyToUse = 2;

/// The ranking a free-text search is the only sensible one under: Steam
/// orders by how well the text matched, so any other ranking would throw
/// the match away.
constexpr int RankedByTextSearch = 11;

/// EUGCQuery, for the three orders the picker offers.
int ugcOrder(Steamworks::Order order)
{
    switch (order) {
    case Steamworks::Newest:
        return 1; // k_EUGCQuery_RankedByPublicationDate
    case Steamworks::MostSubscribed:
        return 12; // k_EUGCQuery_RankedByTotalUniqueSubscriptions
    case Steamworks::Trending:
        break;
    }
    return 3; // k_EUGCQuery_RankedByTrend
}

/// A fixed char array out of Steam, which is not promised to end in a zero.
QString textIn(const char *field, int room)
{
    return QString::fromUtf8(field, qstrnlen(field, room));
}

} // namespace

Steamworks::Steamworks(QObject *parent)
    : QObject(parent)
{
    if (!open())
        qCInfo(lcSteam) << "no workshop today:" << m_trouble;
    else
        qCInfo(lcSteam) << "the workshop can be asked";
}

bool Steamworks::open()
{
    for (const QString &path : candidates()) {
        m_library = dlopen(path.toLocal8Bit().constData(), RTLD_LAZY | RTLD_LOCAL);
        if (m_library) {
            qCDebug(lcSteam) << "opened" << path;
            break;
        }
    }

    if (!m_library) {
        m_trouble = tr("The Steamworks library is not on this machine.");
        return false;
    }

    // The flat C entry points, by name. Valve's own headers are not here --
    // see the class comment for why -- so what is wanted is spelled out.
    const auto initSafe = reinterpret_cast<bool (*)()>(dlsym(m_library, "SteamAPI_InitSafe"));
    const auto steamUser = reinterpret_cast<int (*)()>(dlsym(m_library, "SteamAPI_GetHSteamUser"));
    const auto findInterface = reinterpret_cast<void *(*)(int, const char *)>(
        dlsym(m_library, "SteamInternal_FindOrCreateUserInterface"));
    m_shutdown = reinterpret_cast<void (*)()>(dlsym(m_library, "SteamAPI_Shutdown"));

    const auto dispatchInit = reinterpret_cast<void (*)()>(
        dlsym(m_library, "SteamAPI_ManualDispatch_Init"));
    const auto steamPipe = reinterpret_cast<qint32 (*)()>(
        dlsym(m_library, "SteamAPI_GetHSteamPipe"));

#define WALLFLUX_RESOLVE(member, type, name) \
    member = reinterpret_cast<type>(dlsym(m_library, name))

    WALLFLUX_RESOLVE(m_runFrame, void (*)(qint32), "SteamAPI_ManualDispatch_RunFrame");
    WALLFLUX_RESOLVE(m_nextCallback, bool (*)(qint32, void *),
                     "SteamAPI_ManualDispatch_GetNextCallback");
    WALLFLUX_RESOLVE(m_freeCallback, void (*)(qint32), "SteamAPI_ManualDispatch_FreeLastCallback");
    WALLFLUX_RESOLVE(m_callResult, bool (*)(qint32, quint64, void *, int, int, bool *),
                     "SteamAPI_ManualDispatch_GetAPICallResult");
    WALLFLUX_RESOLVE(m_createQuery, quint64 (*)(void *, int, int, quint32, quint32, quint32),
                     "SteamAPI_ISteamUGC_CreateQueryAllUGCRequestPage");
    WALLFLUX_RESOLVE(m_sendQuery, quint64 (*)(void *, quint64),
                     "SteamAPI_ISteamUGC_SendQueryUGCRequest");
    WALLFLUX_RESOLVE(m_queryResult, bool (*)(void *, quint64, quint32, void *),
                     "SteamAPI_ISteamUGC_GetQueryUGCResult");
    WALLFLUX_RESOLVE(m_previewUrl, bool (*)(void *, quint64, quint32, char *, quint32),
                     "SteamAPI_ISteamUGC_GetQueryUGCPreviewURL");
    WALLFLUX_RESOLVE(m_releaseQuery, bool (*)(void *, quint64),
                     "SteamAPI_ISteamUGC_ReleaseQueryUGCRequest");
    WALLFLUX_RESOLVE(m_requireTag, bool (*)(void *, quint64, const char *),
                     "SteamAPI_ISteamUGC_AddRequiredTag");
    WALLFLUX_RESOLVE(m_searchText, bool (*)(void *, quint64, const char *),
                     "SteamAPI_ISteamUGC_SetSearchText");
    WALLFLUX_RESOLVE(m_subscribe, quint64 (*)(void *, quint64),
                     "SteamAPI_ISteamUGC_SubscribeItem");
    WALLFLUX_RESOLVE(m_downloadItem, bool (*)(void *, quint64, bool),
                     "SteamAPI_ISteamUGC_DownloadItem");
    WALLFLUX_RESOLVE(m_downloadInfo, bool (*)(void *, quint64, quint64 *, quint64 *),
                     "SteamAPI_ISteamUGC_GetItemDownloadInfo");

#undef WALLFLUX_RESOLVE

    if (!initSafe || !steamUser || !findInterface || !dispatchInit || !steamPipe || !m_runFrame
        || !m_nextCallback || !m_freeCallback || !m_callResult || !m_createQuery || !m_sendQuery
        || !m_queryResult || !m_previewUrl || !m_releaseQuery || !m_subscribe
        || !m_downloadItem || !m_downloadInfo || !m_requireTag || !m_searchText) {
        m_trouble = tr("The Steamworks library on this machine is not one this "
                       "program knows how to talk to.");
        return false;
    }

    // Set before starting: without it the library has no idea which
    // application is asking, and asks the Steam client to relaunch us.
    qputenv("SteamAppId", WallpaperEngineAppId);
    qputenv("SteamGameId", WallpaperEngineAppId);

    if (!initSafe()) {
        // Almost always: Steam is not running, or the account signed in to it
        // does not own Wallpaper Engine.
        m_trouble = tr("Steam did not answer. It has to be running, and signed in to the "
                       "account that owns Wallpaper Engine.");
        return false;
    }

    m_ugc = findInterface(steamUser(), UgcInterfaceVersion);
    if (!m_ugc) {
        m_trouble = tr("Steam answered but offered no workshop to ask about.");
        return false;
    }

    // Steam runs no thread of its own in this process: its answers sit in a
    // queue until somebody asks for them, which is what pump() does.
    dispatchInit();
    m_pipe = steamPipe();

    m_pump.setInterval(100);
    connect(&m_pump, &QTimer::timeout, this, &Steamworks::pump);

    return true;
}

void Steamworks::browse(const QString &key, Steamworks::Order order, int page,
                        const QStringList &tags, const QString &search)
{
    if (!m_ugc)
        return;

    // One at a time: a query handle has to be released before the next is
    // made, so a question asked while one is outstanding waits its turn.
    if (m_call != 0) {
        m_queued.append({key, order, page, tags, search});
        return;
    }

    m_asking = key;

    const quint32 app = QByteArrayView(WallpaperEngineAppId).toByteArray().toUInt();
    const int ranking = search.isEmpty() ? ugcOrder(order) : RankedByTextSearch;
    m_query = m_createQuery(m_ugc, ranking, MatchingItemsReadyToUse, 0, app,
                            static_cast<quint32>(qMax(1, page)));
    if (!m_query) {
        qCWarning(lcSteam) << "the workshop would not take the question";
        return;
    }

    // Narrowing has to be said before the question is sent, never after.
    for (const QString &tag : tags)
        m_requireTag(m_ugc, m_query, tag.toUtf8().constData());
    if (!search.isEmpty())
        m_searchText(m_ugc, m_query, search.toUtf8().constData());

    m_call = m_sendQuery(m_ugc, m_query);
    if (m_call == 0) {
        m_releaseQuery(m_ugc, m_query);
        m_query = 0;
        qCWarning(lcSteam) << "the question was not asked";
        return;
    }

    qCDebug(lcSteam) << "asking" << key << "page" << page << "tags" << tags
                     << "search" << search;
    m_pump.start();
    emit busyChanged();
}

void Steamworks::download(const QString &id)
{
    if (!m_ugc)
        return;

    bool numeric = false;
    const quint64 fileId = id.toULongLong(&numeric);
    if (!numeric || fileId == 0) {
        qCWarning(lcSteam) << "not a published file id:" << id;
        return;
    }

    // Already on its way. Asking twice would have Steam start over.
    if (m_downloading.contains(fileId))
        return;

    // Two calls, because they are two different things. Subscribing is what
    // makes Steam keep the wallpaper and show it as the person's own; the
    // download is asked for separately so it starts now rather than
    // whenever Steam gets round to it.
    //
    // Subscribing answers with a call result of its own, which is not waited
    // for: it tells us nothing the download does not, and a subscription
    // that fails shows up as a download that fails. pump() ignores and frees
    // it like any other callback it was not asked about.
    m_subscribe(m_ugc, fileId);

    if (!m_downloadItem(m_ugc, fileId, true)) {
        qCWarning(lcSteam) << "Steam would not download" << fileId;
        m_downloads.insert(id, -1);
        emit downloadsChanged();
        return;
    }

    qCInfo(lcSteam) << "downloading" << fileId;
    m_downloading.append(fileId);
    m_downloads.insert(id, 0);
    m_pump.start();
    emit downloadsChanged();
}

void Steamworks::measure()
{
    for (const quint64 id : m_downloading) {
        quint64 done = 0;
        quint64 total = 0;
        // False before Steam has started and after it has finished, so what
        // was there last is left alone rather than reset to nothing.
        if (m_downloadInfo(m_ugc, id, &done, &total) && total > 0)
            m_downloads.insert(QString::number(id),
                               static_cast<int>(done * 100 / total));
    }
    emit downloadsChanged();
}

void Steamworks::pump()
{
    m_runFrame(m_pipe);

    CallbackMsg msg{};
    while (m_nextCallback(m_pipe, &msg)) {
        if (msg.callback == CallCompletedId && msg.param) {
            const auto *done = static_cast<const CallCompleted *>(msg.param);
            if (done->call == m_call)
                collect(done->call, done->callback, done->paramSize);
        } else if (msg.callback == DownloadFinishedId && msg.param
                   && msg.paramSize >= qint32(sizeof(DownloadFinished))) {
            const auto *done = static_cast<const DownloadFinished *>(msg.param);
            if (m_downloading.removeOne(done->id)) {
                const bool ok = done->result == ResultOk;
                if (!ok)
                    qCWarning(lcSteam) << "download of" << done->id << "failed, result"
                                       << done->result;
                else
                    qCInfo(lcSteam) << "Steam has" << done->id << "on the disk";
                // Left at 100 rather than taken out: the folder watcher
                // needs a moment to notice, and until it does the card
                // should say the thing is there, not offer to fetch it again.
                m_downloads.insert(QString::number(done->id), ok ? 100 : -1);
                emit downloadsChanged();
            }
        }
        // Freed whatever it was: the queue does not move on otherwise.
        m_freeCallback(m_pipe);
    }

    if (!m_downloading.isEmpty())
        measure();

    if (!waiting())
        m_pump.stop();
}

void Steamworks::collect(quint64 call, int callbackId, int paramSize)
{
    QByteArray answer(paramSize, '\0');
    bool failed = false;
    const bool got = m_callResult(m_pipe, call, answer.data(), answer.size(), callbackId, &failed);

    m_call = 0;

    if (!got || failed || callbackId != QueryCompletedId
        || answer.size() < qsizetype(sizeof(QueryCompleted))) {
        qCWarning(lcSteam) << "the workshop's answer could not be read";
        m_releaseQuery(m_ugc, m_query);
        m_query = 0;
        emit busyChanged();
        askNext();
        return;
    }

    const auto *finished = reinterpret_cast<const QueryCompleted *>(answer.constData());
    QVariantList landing;

    if (finished->result != ResultOk) {
        qCWarning(lcSteam) << "the workshop refused the question, result" << finished->result;
    } else {
        for (quint32 i = 0; i < finished->returned; ++i) {
            UgcDetails item{};
            if (!m_queryResult(m_ugc, m_query, i, &item) || item.result != ResultOk)
                continue;

            // Long enough for what Steam puts in it; the call says how much
            // room it was given.
            char preview[1024] = {};
            m_previewUrl(m_ugc, m_query, i, preview, sizeof(preview));

            landing.append(QVariantMap{
                {QStringLiteral("id"), QString::number(item.id)},
                {QStringLiteral("title"), textIn(item.title, sizeof(item.title))},
                {QStringLiteral("preview"), textIn(preview, sizeof(preview))},
                {QStringLiteral("tags"),
                 textIn(item.tags, sizeof(item.tags)).split(QLatin1Char(','), Qt::SkipEmptyParts)},
                {QStringLiteral("bytes"), item.fileSize},
                {QStringLiteral("votesUp"), item.votesUp},
            });
        }
        qCInfo(lcSteam) << m_asking << "returned" << landing.size() << "of" << finished->total;
    }

    // Filed even when it came back empty, so a row that found nothing can
    // say so rather than sit there looking like it is still asking.
    m_found.insert(m_asking, landing);
    m_totals.insert(m_asking, static_cast<int>(finished->total));

    m_releaseQuery(m_ugc, m_query);
    m_query = 0;
    emit foundChanged();
    emit busyChanged();

    askNext();
}

void Steamworks::askNext()
{
    if (m_queued.isEmpty())
        return;
    const Question next = m_queued.takeFirst();
    browse(next.key, next.order, next.page, next.tags, next.search);
}

Steamworks::~Steamworks()
{
    // Told to stop before the library goes, so Steam does not go on thinking
    // Wallpaper Engine is running.
    if (m_ugc && m_shutdown)
        m_shutdown();
    if (m_library)
        dlclose(m_library);
}
