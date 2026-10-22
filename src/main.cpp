// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "wallpapercore.h"


#include <QApplication>
#include <QDir>
#include <QLockFile>
#include <QNetworkAccessManager>
#include <QNetworkDiskCache>
#include <QQmlApplicationEngine>
#include <QQmlNetworkAccessManagerFactory>
#include <QLoggingCategory>
#include <QStandardPaths>
#include <QtWebEngineQuick>

namespace {

/// What QML fetches pictures with, with somewhere to keep them.
///
/// QML's own keeps nothing: every workshop preview is fetched again every
/// time it is shown, and the workshop answers fifty pictures at a time --
/// so turning back a page, or coming back to the tab, meant the whole lot
/// over the network again. Steam says a preview is good for a week
/// (`Cache-Control: max-age=604800`, read off the response), which is
/// exactly what a cache on the disk is for.
class CachingNetwork : public QQmlNetworkAccessManagerFactory
{
public:
    QNetworkAccessManager *create(QObject *parent) override
    {
        auto *network = new QNetworkAccessManager(parent);
        auto *cache = new QNetworkDiskCache(network);
        cache->setCacheDirectory(QStandardPaths::writableLocation(QStandardPaths::CacheLocation)
                                 + QStringLiteral("/previews"));
        // A preview is some twenty kilobytes at the size the cards ask for,
        // so this is room for thousands of them and still nothing much on
        // a disk. Qt throws the oldest out when it fills.
        cache->setMaximumCacheSize(200 * 1024 * 1024);
        network->setCache(cache);
        return network;
    }
};

} // namespace

int main(int argc, char *argv[])
{
    // Chromium wants a shared OpenGL context that exists before any window
    // does. Called after QGuiApplication is built, this aborts at startup
    // instead of failing quietly, so it stays the first line of the program.
    QtWebEngineQuick::initialize();

    // A wallpaper is somebody else's JavaScript, and its console is its own
    // business: one of the three here reports a WebGL complaint several
    // hundred times a minute and would otherwise bury everything we say.
    // Set before the application is built, so QT_LOGGING_RULES still wins
    // and `QT_LOGGING_RULES=js=true` brings the messages back for anyone
    // debugging a wallpaper.
    QLoggingCategory::setFilterRules(QStringLiteral("js=false"));

    // QApplication rather than QGuiApplication: Qt.labs.platform, which the
    // tray icon comes from, falls back to the Qt Widgets tray where the
    // desktop has no native one of its own, and that fallback needs this.
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("wallflux"));
    app.setOrganizationName(QStringLiteral("WallFlux"));
    // KWin reads the desktop file name to decide what the window is; a
    // wallpaper that claims to be an ordinary application gets treated as one.
    app.setDesktopFileName(QStringLiteral("wallflux"));

    // One wallpaper to a session. Starting at login means the program is
    // usually already running by the time somebody starts it again by hand,
    // and two copies is two videos drawn over each other and two lots of
    // sound coming out of the speakers.
    //
    // The runtime directory is per-user and emptied at logout, which is the
    // lifetime this lock wants. Where a system has none, a temporary file
    // named after the user does the same job.
    const QString runtime = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    QLockFile running(runtime.isEmpty()
                          ? QDir::tempPath() + QStringLiteral("/wallflux-")
                                + qEnvironmentVariable("USER", QStringLiteral("user"))
                                + QStringLiteral(".lock")
                          : runtime + QStringLiteral("/wallflux.lock"));
    if (!running.tryLock()) {
        // Not a failure: the wallpaper this copy was asked for is on the
        // screen already, put there by the copy that got here first.
        qWarning("wallflux: another copy is already running, leaving the screen to it");
        return 0;
    }

    // WallpaperCore declares itself a singleton of the WallFlux QML module,
    // so there is nothing to register here: the engine builds it when the QML
    // first asks for `Wallpapers`, and it scans the Steam folders as it is
    // built.
    // Before the engine, so it is still there when the engine goes: the
    // engine is handed the factory and does not take it over.
    CachingNetwork network;

    QQmlApplicationEngine engine;
    engine.setNetworkAccessManagerFactory(&network);

    // Without this the program would sit there with no window and no reason
    // given when the QML fails to load.
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("WallFlux", "Main");

    if (engine.rootObjects().isEmpty()) {
        qWarning("wallflux: the QML did not load");
        return 1;
    }

    // No window is shown from here any more. A wallpaper is a layer-shell
    // surface and a layer-shell surface belongs to one screen, so there is
    // one window per screen and they come and go with the monitors --
    // which only QML is in a position to notice. `Desktop::asWallpaper()`
    // is what used to be written out below, called by each window while it
    // is still hidden, for the same reason it was done before `show()`:
    // a surface that has reached the compositor cannot be made into a
    // layer-shell one afterwards.

    return app.exec();
}
