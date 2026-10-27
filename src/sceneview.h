// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QQmlEngine>
#include <QQuickFramebufferObject>
#include <QUrl>

/// A scene wallpaper, drawn by the renderer under crates/.
///
/// An item rather than a window of its own, so that a scene is one more
/// thing on the wallpaper's surface the way a video is: the transitions
/// grab it, the cover pauses it, and there is one layer-shell surface per
/// screen rather than two from two programs. The renderer draws into this
/// item's framebuffer on Qt's render thread, with Qt's OpenGL context --
/// which is why the program asks for OpenGL before any window exists.
class SceneView : public QQuickFramebufferObject
{
    Q_OBJECT
    QML_ELEMENT

    /// The wallpaper's `scene.pkg`. Empty draws nothing and lets the last
    /// scene go.
    Q_PROPERTY(QUrl source READ source WRITE setSource NOTIFY sourceChanged)
    /// Whether the scene moves. Stopped, its clock stands still and nothing
    /// is drawn again until it is started -- which is what a covered screen
    /// asks for.
    Q_PROPERTY(bool running READ running WRITE setRunning NOTIFY runningChanged)
    /// Why the last scene could not be drawn, or empty.
    Q_PROPERTY(QString error READ error NOTIFY errorChanged)

public:
    explicit SceneView(QQuickItem *parent = nullptr);

    Renderer *createRenderer() const override;

    QUrl source() const { return m_source; }
    void setSource(const QUrl &source);
    bool running() const { return m_running; }
    void setRunning(bool running);
    QString error() const { return m_error; }

signals:
    void sourceChanged();
    void runningChanged();
    void errorChanged();

private:
    friend class SceneRenderer;
    void setError(const QString &error);

    QUrl m_source;
    bool m_running = true;
    QString m_error;
};
