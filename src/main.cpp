// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "compatibility.h"
#include "wallpapercore.h"
#include "webguard.h"


#include <QApplication>
#include <QDir>
#include <QDirIterator>
#include <QFontDatabase>
#include <QIcon>
#include <QFileInfo>
#include <QLocalServer>
#include <QLocalSocket>
#include <QLockFile>
#include <QNetworkAccessManager>
#include <QNetworkDiskCache>
#include <QQmlApplicationEngine>
#include <QQmlNetworkAccessManagerFactory>
#include <QQuickWindow>
#include <QLoggingCategory>
#include <QStandardPaths>
#include <QtWebEngineQuick>
#include <algorithm>
#include <QQuickWebEngineProfile>

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

    // Scene wallpapers are drawn with OpenGL into the scene graph's own
    // context, so the scene graph has to be OpenGL. It already is on the
    // machine this was written on; said out loud, because Qt is free to pick
    // Vulkan elsewhere, and the scenes would then draw nothing at all.
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);

    // Every screen after the first shows the first one's picture, which is a
    // texture of another window's context; only contexts that share can read
    // each other's. QtWebEngine no longer asks for this itself -- measured:
    // without it the two windows were in different share groups and the
    // second screen read whatever its own texture of that number held.
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);

    // A wallpaper is somebody else's JavaScript, and its console is its own
    // business: one of the three here reports a WebGL complaint several
    // hundred times a minute and would otherwise bury everything we say.
    // Set before the application is built, so QT_LOGGING_RULES still wins
    // and `QT_LOGGING_RULES=js=true` brings the messages back for anyone
    // debugging a wallpaper.
    QLoggingCategory::setFilterRules(QStringLiteral("js=false"));

    // Started by Compatibility to try the library out, unseen and unheard:
    // a program of its own, so that a wallpaper that brings it down brings
    // down nothing else.
    if (argc > 1 && qstrcmp(argv[1], "--grade") == 0)
        return Compatibility::grade(argc, argv);

    // QApplication rather than QGuiApplication: Qt.labs.platform, which the
    // tray icon comes from, falls back to the Qt Widgets tray where the
    // desktop has no native one of its own, and that fallback needs this.
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("wallflux"));
    // Read in QML as `Application.version`, which is what the footer's
    // About icon says when it is hovered.
    app.setApplicationVersion(QStringLiteral(WALLFLUX_VERSION));
    app.setOrganizationName(QStringLiteral("WallFlux"));
    // KWin reads the desktop file name to decide what the window is; a
    // wallpaper that claims to be an ordinary application gets treated as one.
    app.setDesktopFileName(QStringLiteral("wallflux"));
    app.setWindowIcon(QIcon::fromTheme(QStringLiteral("wallflux"), QIcon(QStringLiteral(":/icons/wallflux.svg"))));

    // The typefaces Settings > Appearance offers, out of the binary. The
    // desktop's own is the default and needs nothing loading.
    for (QDirIterator font(QStringLiteral(":/fonts"), {QStringLiteral("*.ttf")}); font.hasNext();) {
        if (QFontDatabase::addApplicationFont(font.next()) < 0)
            qWarning("could not load the typeface %s", qPrintable(font.filePath()));
    }

    // One wallpaper to a session. Starting at login means the program is
    // usually already running by the time somebody starts it again by hand,
    // and two copies is two videos drawn over each other and two lots of
    // sound coming out of the speakers.
    //
    // The runtime directory is per-user and emptied at logout, which is the
    // lifetime this lock wants. Where a system has none, a temporary file
    // named after the user does the same job.
    const QString runtime = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    const QString place = runtime.isEmpty()
                              ? QDir::tempPath() + QStringLiteral("/wallflux-")
                                    + qEnvironmentVariable("USER", QStringLiteral("user"))
                              : runtime + QStringLiteral("/wallflux");
    QLockFile running(place + QStringLiteral(".lock"));
    // Where the copy that is running listens for the next one. Started again
    // from the menu, WallFlux shows the window of the copy already running --
    // which is the only way to reach it with the tray icon switched off.
    const QString door = place + QStringLiteral(".socket");
    if (!running.tryLock()) {
        // Not a failure: the wallpaper this copy was asked for is on the
        // screen already, put there by the copy that got here first.
        QLocalSocket knock;
        knock.connectToServer(door);
        if (knock.waitForConnected(1000)) {
            knock.write("show\n");
            knock.waitForBytesWritten(1000);
        }
        qWarning("wallflux: another copy is already running; asked it to show its window");
        return 0;
    }

    // WallpaperCore declares itself a singleton of the WallFlux QML module,
    // so there is nothing to register here: the engine builds it when the QML
    // first asks for `Wallpapers`, and it scans the Steam folders as it is
    // built.
    // Before the engine, so it is still there when the engine goes: the
    // engine is handed the factory and does not take it over.
    CachingNetwork network;

    QQuickWebEngineProfile::defaultProfile()->setUrlRequestInterceptor(new WallpaperFilesOnly(&app));

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

    QLocalServer::removeServer(door);
    QLocalServer listening;
    if (!listening.listen(door))
        qWarning("wallflux: cannot listen for a second start: %s", qPrintable(listening.errorString()));
    QObject *root = engine.rootObjects().constFirst();
    QObject::connect(&listening, &QLocalServer::newConnection, &listening, [&listening, root] {
        while (QLocalSocket *knock = listening.nextPendingConnection()) {
            QObject::connect(knock, &QLocalSocket::disconnected, knock, &QObject::deleteLater);
            QMetaObject::invokeMethod(root, "showPicker");
        }
    });
    // Started again by an update, which is somebody's click: the window is
    // what they were looking at, so it is what they get back.
    if (app.arguments().contains(QStringLiteral("--show")))
        QMetaObject::invokeMethod(root, "showPicker", Qt::QueuedConnection);

    return app.exec();
}
