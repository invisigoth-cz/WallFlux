// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "wallpapercore.h"

#include <LayerShellQt/Window>

#include <QApplication>
#include <QDir>
#include <QLockFile>
#include <QQmlApplicationEngine>
#include <QLoggingCategory>
#include <QQuickWindow>
#include <QStandardPaths>
#include <QtWebEngineQuick>

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
    QQmlApplicationEngine engine;

    // Without this the program would sit there with no window and no reason
    // given when the QML fails to load.
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("WallFlux", "Main");

    const QList<QObject *> roots = engine.rootObjects();
    auto *window = roots.isEmpty() ? nullptr : qobject_cast<QQuickWindow *>(roots.first());
    if (!window) {
        qWarning("wallflux: the QML did not give us a window to show");
        return 1;
    }

    // Asked for while the window is still only a QWindow: once it has reached
    // the compositor it is an ordinary surface and cannot be made into a
    // layer-shell one. This is why Main.qml leaves the window hidden.
    if (LayerShellQt::Window *layer = LayerShellQt::Window::get(window)) {
        using Layer = LayerShellQt::Window;
        layer->setLayer(Layer::LayerBackground);
        // Anchored to all four edges, which is what makes the compositor give
        // it the whole screen.
        layer->setAnchors(Layer::Anchors(Layer::AnchorTop | Layer::AnchorBottom
                                        | Layer::AnchorLeft | Layer::AnchorRight));
        // A wallpaper reserves no space of its own and never takes the
        // keyboard, so panels keep their room and focus never lands here.
        layer->setExclusiveZone(-1);
        layer->setKeyboardInteractivity(Layer::KeyboardInteractivityNone);
        layer->setActivateOnShow(false);
        // The compositor reads the scope as the surface's role; KWin sees
        // this one as a desktop rather than as a panel.
        layer->setScope(QStringLiteral("desktop"));
    }

    window->show();

    return app.exec();
}
