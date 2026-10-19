// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "steamworks.h"

#include <QFileInfo>
#include <QLoggingCategory>

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

    if (!initSafe || !steamUser || !findInterface) {
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

    return true;
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
