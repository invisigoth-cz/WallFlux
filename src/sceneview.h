// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QPointF>
#include <QQmlEngine>
#include <QQuickFramebufferObject>
#include <QTimer>
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
    /// What the Settings tab allows: pictures a second, and whether a scene
    /// follows the music, leans with the mouse and has particles. They hold
    /// for every scene at once, so each view hands all four on together.
    Q_PROPERTY(int fps READ fps WRITE setFps NOTIFY optionsChanged)
    Q_PROPERTY(bool music READ music WRITE setMusic NOTIFY optionsChanged)
    Q_PROPERTY(bool parallax READ parallax WRITE setParallax NOTIFY optionsChanged)
    Q_PROPERTY(bool particles READ particles WRITE setParticles NOTIFY optionsChanged)
    /// How a scene meets a screen of another shape: 0 cropped to fill it,
    /// 1 fitted inside it with bars, 2 stretched to it.
    Q_PROPERTY(int scaling READ scaling WRITE setScaling NOTIFY optionsChanged)
    /// The quality preset's three: samples a pixel for smoothing edges (0, 2,
    /// 4 or 8), whether a scene may glow, and pictures at half their size.
    Q_PROPERTY(int msaa READ msaa WRITE setMsaa NOTIFY optionsChanged)
    Q_PROPERTY(bool bloom READ bloom WRITE setBloom NOTIFY optionsChanged)
    Q_PROPERTY(bool halfTextures READ halfTextures WRITE setHalfTextures NOTIFY optionsChanged)
    /// The wallpaper's properties as set in the panel: a JSON object of name
    /// to value, laid over project.json's. Another value is the scene loaded
    /// again with it.
    Q_PROPERTY(QString values READ values WRITE setValues NOTIFY valuesChanged)

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

    int fps() const { return m_fps; }
    void setFps(int fps);
    bool music() const { return m_music; }
    void setMusic(bool music);
    bool parallax() const { return m_parallax; }
    void setParallax(bool parallax);
    bool particles() const { return m_particles; }
    void setParticles(bool particles);
    int scaling() const { return m_scaling; }
    void setScaling(int scaling);
    int msaa() const { return m_msaa; }
    void setMsaa(int msaa);
    bool bloom() const { return m_bloom; }
    void setBloom(bool bloom);
    bool halfTextures() const { return m_halfTextures; }
    void setHalfTextures(bool half);
    QString values() const { return m_values; }
    void setValues(const QString &values);

signals:
    void sourceChanged();
    void runningChanged();
    void errorChanged();
    void pointerChanged();
    void optionsChanged();
    void valuesChanged();

    /// A scene was loaded and is being drawn, with what of it the renderer
    /// had to leave out -- nothing, for a scene drawn whole. Not said again
    /// when a changed setting loads the same scene over.
    void loaded(const QStringList &skipped);

private:
    friend class SceneRenderer;
    void setError(const QString &error);
    void pace();
    void handOptionsOn();

    QUrl m_source;
    bool m_running = true;
    QString m_error;
    std::optional<QPointF> m_pointer;
    QTimer m_frames;
    int m_fps = 15;
    bool m_music = true;
    bool m_parallax = true;
    bool m_particles = true;
    int m_scaling = 0;
    int m_msaa = 0;
    bool m_bloom = true;
    bool m_halfTextures = false;
    QString m_values;
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

    /// What the speakers are playing, as a web wallpaper's audio listener
    /// wants it: 64 bands of the left ear and 64 of the right, nought to one.
    /// Reading it is what keeps the renderer listening.
    Q_INVOKABLE QList<float> speakers() const;
};

