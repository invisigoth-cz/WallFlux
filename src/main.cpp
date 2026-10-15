// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include <QGuiApplication>
#include <QQmlApplicationEngine>
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

    return app.exec();
}
