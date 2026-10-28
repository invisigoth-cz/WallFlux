// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QPointF>
#include <QQmlEngine>
#include <QQuickFramebufferObject>
#include <QUrl>
#include <QVariantList>

#include <optional>

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
    /// Where the mouse is over this item, in its own pixels, or undefined
    /// where it is not over it or nobody knows. What the scene does with it
    /// is its own: a layer that leans away, a particle force that clears a
    /// hole round the pointer.
    Q_PROPERTY(QVariant pointer READ pointer WRITE setPointer NOTIFY pointerChanged)

public:
    explicit SceneView(QQuickItem *parent = nullptr);

    Renderer *createRenderer() const override;

    QUrl source() const { return m_source; }
    void setSource(const QUrl &source);
    bool running() const { return m_running; }
    void setRunning(bool running);
    QString error() const { return m_error; }

    QVariant pointer() const { return m_pointer ? QVariant(*m_pointer) : QVariant(); }
    void setPointer(const QVariant &pointer);

signals:
    void sourceChanged();
    void runningChanged();
    void errorChanged();
    void pointerChanged();

private:
    friend class SceneRenderer;
    void setError(const QString &error);

    QUrl m_source;
    bool m_running = true;
    QString m_error;
    std::optional<QPointF> m_pointer;
};

/// The music a scene wallpaper plays.
///
/// A scene keeps its sound inside `scene.pkg`, where a media player cannot
/// reach it. The renderer reads the package and writes the files out to the
/// cache; playing them is QML's, with the player video wallpapers use, and
/// once for the program rather than once per screen.
class SceneSounds : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit SceneSounds(QObject *parent = nullptr);

    /// What the scene at `source` -- its `scene.pkg` -- plays: a list of
    /// `{ files, mode, volume, mintime, maxtime, startsilent }`, one per
    /// sound object, with `files` as URLs. Empty for a scene with no sound,
    /// or one that cannot be read.
    ///
    /// Writes the files out the first time, which for a wallpaper carrying
    /// an album is a moment's work on this thread.
    Q_INVOKABLE QVariantList of(const QUrl &source) const;
};

