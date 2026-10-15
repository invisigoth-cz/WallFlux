// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "wallpapercore.h"

#include <LayerShellQt/Window>

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <QtWebEngineQuick>

int main(int argc, char *argv[])
{
    // Chromium wants a shared OpenGL context that exists before any window
    // does. Called after QGuiApplication is built, this aborts at startup
    // instead of failing quietly, so it stays the first line of the program.
    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("wallflux"));
    app.setOrganizationName(QStringLiteral("WallFlux"));
    // KWin reads the desktop file name to decide what the window is; a
    // wallpaper that claims to be an ordinary application gets treated as one.
    app.setDesktopFileName(QStringLiteral("wallflux"));

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
