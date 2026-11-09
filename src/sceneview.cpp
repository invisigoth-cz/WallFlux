// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <algorithm>
#include <clocale>
#include "sceneview.h"

#include "wallflux_scene.h"

#include <QDir>
#include <QElapsedTimer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QFileInfo>
#include <QLoggingCategory>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QOpenGLFunctions>
#include <QPointer>
#include <QQuickOpenGLUtils>
#include <QStandardPaths>
#include <QTimer>

#include <utility>

Q_LOGGING_CATEGORY(lcScene, "wallflux.scene")

namespace {

const void *lookUp(const char *name, void *)
{
    return reinterpret_cast<const void *>(QOpenGLContext::currentContext()->getProcAddress(name));
}

} // namespace

/// Lives on the render thread and owns the scene there: loading it compiles
/// shaders and uploads textures, which only the thread holding the context
/// can do.
class SceneRenderer : public QQuickFramebufferObject::Renderer
{
public:
    ~SceneRenderer() override { wallflux_scene_free(m_scene); }

    QOpenGLFramebufferObject *createFramebufferObject(const QSize &size) override
    {
        // Smoothing edges is the framebuffer's: the scene draws its layers
        // straight into it, and Qt resolves it before anybody reads it.
        QOpenGLFramebufferObjectFormat format;
        format.setSamples(m_samples);
        return new QOpenGLFramebufferObject(size, format);
    }

    /// The one moment both threads are still, so the only place the item is
    /// read from.
    void synchronize(QQuickFramebufferObject *item) override
    {
        auto *view = static_cast<SceneView *>(item);
        m_view = view;

        const QUrl source = view->m_source;
        const QString folder = source.isLocalFile()
                                   ? QFileInfo(source.toLocalFile()).absolutePath()
                                   : QString();
        if (folder != m_folder) {
            m_folder = folder;
            m_reload = true;
        }
        m_running = view->m_running;

        // A framebuffer's samples are fixed when it is made, so another
        // number is another framebuffer.
        if (view->m_msaa != m_samples) {
            m_samples = view->m_msaa;
            invalidateFramebufferObject();
        }
        // Pictures are loaded once, so another size is the scene again.
        if (view->m_halfTextures != m_half) {
            m_half = view->m_halfTextures;
            if (!m_folder.isEmpty())
                m_reload = true;
        }

        // As a fraction of the item, which is what the renderer wants and
        // what stays true of a framebuffer drawn at another size.
        std::optional<QPointF> pointer;
        const QSizeF size = view->size();
        if (view->m_pointer && size.width() > 0 && size.height() > 0) {
            const QPointF at(view->m_pointer->x() / size.width(),
                             view->m_pointer->y() / size.height());
            if (at.x() >= 0 && at.x() < 1 && at.y() >= 0 && at.y() < 1)
                pointer = at;
        }
        if (pointer != m_pointer) {
            m_pointer = pointer;
            m_pointed = false;
        }
    }

    void render() override
    {
        QOpenGLFunctions *gl = QOpenGLContext::currentContext()->functions();
        if (m_reload)
            load();

        if (m_scene && !m_pointed) {
            const QSize size = framebufferObject()->size();
            wallflux_scene_point_at(m_scene, m_pointer.has_value(),
                                    float(m_pointer.value_or(QPointF()).x()),
                                    float(m_pointer.value_or(QPointF()).y()), size.width(),
                                    size.height());
            m_pointed = true;
        }

        if (m_scene) {
            // Seconds the scene has been running, which is not the same as
            // seconds since it loaded: a covered screen stops its clock.
            // Nanoseconds: whole milliseconds dropped most of a millisecond off
            // every frame, and the scene ran four per cent slow at 60 a second.
            const qint64 since = m_clock.nsecsElapsed();
            m_clock.restart();
            if (m_running)
                m_seconds += since / 1e9;
            const QSize size = framebufferObject()->size();
            if (!wallflux_scene_draw(m_scene, size.width(), size.height(), float(m_seconds))) {
                // The renderer has said why on stderr already. One scene
                // failing is not a reason to take the program down with it.
                say(QStringLiteral("the renderer failed while drawing this wallpaper"));
                wallflux_scene_free(std::exchange(m_scene, nullptr));
            }
        } else {
            gl->glClearColor(0, 0, 0, 0);
            gl->glClear(GL_COLOR_BUFFER_BIT);
        }

        // The renderer leaves its own buffers, programs and blending set,
        // and the scene graph draws whatever comes next on the assumption
        // that nobody touched them. Forgetting this shows up in the picker,
        // not in the wallpaper.
        QQuickOpenGLUtils::resetOpenGLState();
    }

private:
    void load()
    {
        m_reload = false;
        wallflux_scene_free(std::exchange(m_scene, nullptr));
        if (m_folder.isEmpty()) {
            say({});
            return;
        }

        char error[512] = {};
        m_scene = wallflux_scene_load(m_folder.toUtf8().constData(), lookUp, nullptr, error,
                                      sizeof error);
        if (!m_scene) {
            qCWarning(lcScene) << "cannot draw" << m_folder << "--" << error;
            say(QString::fromUtf8(error));
            return;
        }
        qCInfo(lcScene) << "drawing" << m_folder;
        say({});
        // A new scene has been told nothing about the mouse yet.
        m_pointed = false;
        m_seconds = 0;
        m_clock.start();
    }

    /// Hands the item why the scene cannot be drawn, or that it can.
    ///
    /// Through the item's own thread rather than in synchronize(): a
    /// renderer asking to be drawn again is drawn again without being
    /// synchronized first, so a reason left for synchronize() to pick up
    /// waited for the next time somebody changed the item -- which, for a
    /// wallpaper that failed, is never.
    void say(const QString &error)
    {
        if (SceneView *view = m_view.data())
            QMetaObject::invokeMethod(view, [view, error] { view->setError(error); },
                                      Qt::QueuedConnection);
    }

    WallfluxScene *m_scene = nullptr;
    /// The item, as of the last synchronize(); a queued call to a context
    /// object that has gone is dropped rather than made.
    QPointer<SceneView> m_view;
    QString m_folder;
    QElapsedTimer m_clock;
    double m_seconds = 0;
    bool m_reload = false;
    int m_samples = 0;
    bool m_half = false;
    bool m_running = true;
    /// Where the mouse is, as a fraction of the item, and whether the scene
    /// has been told.
    std::optional<QPointF> m_pointer;
    bool m_pointed = false;
};

SceneView::SceneView(QQuickItem *parent)
    : QQuickFramebufferObject(parent)
{
    // A scene's scripts are read by QuickJS, whose JSON.parse goes through the
    // C library's strtod -- and QCoreApplication has switched that to the
    // user's locale, where "0.5" is nought and a comma is the decimal point.
    // Every object's scale and alpha cross that way, so on a Czech desktop a
    // layer at half its size was not drawn at all, and neither was anything
    // whose alpha was under one. A program drawing to a file never calls
    // setlocale and looks fine. Nothing here wants a locale's numbers from C.
    setlocale(LC_NUMERIC, "C");

    // OpenGL draws from the bottom up, and Qt shows this framebuffer as it
    // is: without this every scene came out upside down.
    setMirrorVertically(true);

    // A scene is drawn this often and no more. Asking for the next frame the
    // moment one was drawn ran at the screen's 144 a second, for a wallpaper.
    // Fifteen is Wallpaper Engine's own default: its welcome dialog picks the
    // Medium preset, which is 15 (Low 10, High 25, Ultra 30).
    // The Settings tab offers the four; see setFps().
    m_frames.setInterval(1000 / m_fps);
    m_frames.setTimerType(Qt::PreciseTimer);
    connect(&m_frames, &QTimer::timeout, this, [this] { update(); });
}

void SceneView::setFps(int fps)
{
    fps = std::clamp(fps, 1, 240);
    if (fps == m_fps)
        return;
    m_fps = fps;
    m_frames.setInterval(1000 / m_fps);
    handOptionsOn();
}

void SceneView::setMusic(bool music)
{
    if (music == m_music)
        return;
    m_music = music;
    handOptionsOn();
}

void SceneView::setParallax(bool parallax)
{
    if (parallax == m_parallax)
        return;
    m_parallax = parallax;
    handOptionsOn();
}

void SceneView::setParticles(bool particles)
{
    if (particles == m_particles)
        return;
    m_particles = particles;
    handOptionsOn();
}

void SceneView::setScaling(int scaling)
{
    scaling = std::clamp(scaling, 0, 2);
    if (scaling == m_scaling)
        return;
    m_scaling = scaling;
    handOptionsOn();
    update();
}

/// The renderer keeps these for every scene; a plain store, safe from any
/// thread, read when each frame is drawn.
void SceneView::setMsaa(int msaa)
{
    msaa = msaa >= 8 ? 8 : msaa >= 4 ? 4 : msaa >= 2 ? 2 : 0;
    if (msaa == m_msaa)
        return;
    m_msaa = msaa;
    handOptionsOn();
    update();
}

void SceneView::setBloom(bool bloom)
{
    if (bloom == m_bloom)
        return;
    m_bloom = bloom;
    handOptionsOn();
    update();
}

void SceneView::setHalfTextures(bool half)
{
    if (half == m_halfTextures)
        return;
    m_halfTextures = half;
    handOptionsOn();
    update();
}

void SceneView::handOptionsOn()
{
    wallflux_options(static_cast<uint32_t>(m_fps), m_music, m_parallax, m_particles,
                     static_cast<uint32_t>(m_scaling), m_bloom, m_halfTextures);
    emit optionsChanged();
}

/// Ticks while there is a scene to move: not while it is held, not with
/// nothing to show, and not once it has failed.
void SceneView::pace()
{
    if (m_running && !m_source.isEmpty() && m_error.isEmpty())
        m_frames.start();
    else
        m_frames.stop();
}

QQuickFramebufferObject::Renderer *SceneView::createRenderer() const
{
    return new SceneRenderer;
}

void SceneView::setSource(const QUrl &source)
{
    if (m_source == source)
        return;
    m_source = source;
    emit sourceChanged();
    // A new scene may draw where the last one failed; the renderer says so
    // again if it does not.
    setError({});
    pace();
    update();
}

void SceneView::setRunning(bool running)
{
    if (m_running == running)
        return;
    m_running = running;
    emit runningChanged();
    pace();
    update();
}

void SceneView::setPointer(const QVariant &pointer)
{
    const std::optional<QPointF> at = pointer.canConvert<QPointF>() && !pointer.isNull()
                                          ? std::optional(pointer.toPointF())
                                          : std::nullopt;
    if (m_pointer == at)
        return;
    m_pointer = at;
    emit pointerChanged();
    // The renderer is only synchronized when the item asks to be drawn --
    // asking to be drawn again from the render thread does not count -- so
    // a mouse that moves has to ask, or the scene never hears of it.
    update();
}

void SceneView::setError(const QString &error)
{
    if (m_error == error)
        return;
    m_error = error;
    emit errorChanged();
    pace();
}

SceneSounds::SceneSounds(QObject *parent)
    : QObject(parent)
{
}

QList<float> SceneSounds::speakers() const
{
    QList<float> bands(128);
    wallflux_speakers(bands.data());
    return bands;
}

QVariantList SceneSounds::of(const QUrl &source) const
{
    if (!source.isLocalFile())
        return {};
    const QFileInfo package(source.toLocalFile());
    const QByteArray folder = package.absolutePath().toUtf8();
    // A folder per wallpaper, so that two scenes that both call their music
    // sounds/theme.mp3 do not write over each other.
    const QByteArray cache = (QStandardPaths::writableLocation(QStandardPaths::CacheLocation)
                              + QStringLiteral("/scenes/") + package.dir().dirName())
                                 .toUtf8();

    QByteArray json(4096, '\0');
    size_t needed = wallflux_scene_sounds(folder.constData(), cache.constData(), json.data(),
                                          size_t(json.size()));
    if (needed >= size_t(json.size())) {
        json.resize(qsizetype(needed) + 1);
        needed = wallflux_scene_sounds(folder.constData(), cache.constData(), json.data(),
                                       size_t(json.size()));
    }
    json.truncate(qsizetype(needed));

    QVariantList sounds;
    for (const QJsonValue &one : QJsonDocument::fromJson(json).array()) {
        QVariantMap sound = one.toObject().toVariantMap();
        QVariantList files;
        for (const QVariant &file : sound.value(QStringLiteral("files")).toList())
            files.append(QUrl::fromLocalFile(file.toString()));
        sound.insert(QStringLiteral("files"), files);
        sounds.append(sound);
    }
    qCInfo(lcScene) << "playing" << sounds.size() << "sounds of" << package.dir().dirName();
    return sounds;
}

