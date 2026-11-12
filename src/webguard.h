// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include "wallpapercore.h"

#include <QFileInfo>
#include <QWebEngineUrlRequestInterceptor>
#include <algorithm>

/// Keeps a web wallpaper's page to the wallpapers' own folders.
///
/// A page is somebody's off the workshop, loaded from a file, and Chromium
/// lets a page loaded from a file read any other file it can name -- left
/// alone, `fetch("file:///home/...")` would hand it whatever is in the home
/// folder. So a file is fetched only from under a folder wallpapers live in:
/// the workshop's, or Wallpaper Engine's own install. Everything else that
/// is not a file is Chromium's business as before.
class WallpaperFilesOnly : public QWebEngineUrlRequestInterceptor
{
public:
    explicit WallpaperFilesOnly(QObject *parent)
        : QWebEngineUrlRequestInterceptor(parent)
    {
        for (const QString &root : WallpaperCore::steamRoots()) {
            for (const char *under : {"/steamapps/workshop/content/431960", "/steamapps/common/wallpaper_engine"}) {
                const QString folder = QFileInfo(root + QLatin1StringView(under)).canonicalFilePath();
                if (!folder.isEmpty())
                    m_allowed.append(folder + QLatin1Char('/'));
            }
        }
    }

    static bool inside(const QString &path, const QStringList &folders)
    {
        // Canonical, so that neither `..` nor a symbolic link leads out.
        const QString real = QFileInfo(path).canonicalFilePath();
        return !real.isEmpty()
               && std::ranges::any_of(folders, [&](const QString &f) { return real.startsWith(f); });
    }

    void interceptRequest(QWebEngineUrlRequestInfo &info) override
    {
        const QUrl url = info.requestUrl();
        if (!url.isLocalFile() || inside(url.toLocalFile(), m_allowed))
            return;
        qWarning("a web wallpaper asked for %s, outside the wallpapers' folders: refused",
                 qPrintable(url.toString()));
        info.block(true);
    }

private:
    QStringList m_allowed;
};
