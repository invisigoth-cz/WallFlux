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

/// Callback numbers, and the few enum values worth naming. Measured the same
/// way.
constexpr int CallCompletedId = 703;
constexpr int QueryCompletedId = 3401;
constexpr qint32 ResultOk = 1;
constexpr int MatchingItemsReadyToUse = 2;

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

#undef WALLFLUX_RESOLVE

    if (!initSafe || !steamUser || !findInterface || !dispatchInit || !steamPipe || !m_runFrame
        || !m_nextCallback || !m_freeCallback || !m_callResult || !m_createQuery || !m_sendQuery
        || !m_queryResult || !m_previewUrl || !m_releaseQuery) {
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

void Steamworks::browse(Steamworks::Order order, int page)
{
    if (!m_ugc)
        return;

    // One at a time: a query handle has to be released before the next is
    // made, so a question asked while one is outstanding waits its turn.
    if (m_call != 0) {
        m_queued.append({order, page});
        return;
    }

    m_asking = order;

    const quint32 app = QByteArrayView(WallpaperEngineAppId).toByteArray().toUInt();
    m_query = m_createQuery(m_ugc, ugcOrder(order), MatchingItemsReadyToUse, 0, app,
                            static_cast<quint32>(qMax(1, page)));
    if (!m_query) {
        qCWarning(lcSteam) << "the workshop would not take the question";
        return;
    }

    m_call = m_sendQuery(m_ugc, m_query);
    if (m_call == 0) {
        m_releaseQuery(m_ugc, m_query);
        m_query = 0;
        qCWarning(lcSteam) << "the question was not asked";
        return;
    }

    m_pump.start();
    emit busyChanged();
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
        }
        // Freed whatever it was: the queue does not move on otherwise.
        m_freeCallback(m_pipe);
    }
}

void Steamworks::collect(quint64 call, int callbackId, int paramSize)
{
    QByteArray answer(paramSize, '\0');
    bool failed = false;
    const bool got = m_callResult(m_pipe, call, answer.data(), answer.size(), callbackId, &failed);

    m_pump.stop();
    m_call = 0;

    if (!got || failed || callbackId != QueryCompletedId
        || answer.size() < qsizetype(sizeof(QueryCompleted))) {
        qCWarning(lcSteam) << "the workshop's answer could not be read";
        m_releaseQuery(m_ugc, m_query);
        m_query = 0;
        emit busyChanged();
        if (!m_queued.isEmpty()) {
            const auto next = m_queued.takeFirst();
            browse(next.first, next.second);
        }
        return;
    }

    const auto *finished = reinterpret_cast<const QueryCompleted *>(answer.constData());
    QVariantList &landing = m_asking == Newest ? m_newest : m_trending;
    landing.clear();

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
        qCInfo(lcSteam) << "the workshop returned" << landing.size() << "of"
                        << finished->total << "wallpapers";
    }

    m_releaseQuery(m_ugc, m_query);
    m_query = 0;
    emit foundChanged();
    emit busyChanged();

    if (!m_queued.isEmpty()) {
        const auto next = m_queued.takeFirst();
        browse(next.first, next.second);
    }
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
