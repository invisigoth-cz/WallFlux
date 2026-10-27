// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "sceneview.h"
#include "wallflux_scene.h"
#include "wallpapercore.h"

#include <QFileInfo>
#include <QGuiApplication>
#include <QOffscreenSurface>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QQuickWindow>
#include <QSignalSpy>
#include <QTest>

#include <cstdio>

/// The renderer drawn the way the program draws it: into a SceneView, on
/// Qt's render thread, with Qt's OpenGL context. What `shot` cannot say is
/// whether that arrangement works -- the context shared with the scene
/// graph, the state handed back to it, the framebuffer Qt gives.
///
/// Needs a display and the scene wallpapers on this machine, and skips
/// without them: the library is the only scene there is to draw.
class TestSceneView : public QObject
{
    Q_OBJECT

private:
    /// Every scene wallpaper on the machine, by id.
    static QHash<QString, QUrl> scenes()
    {
        QHash<QString, QUrl> found;
        for (const Wallpaper &one : WallpaperCore::scan(WallpaperCore::steamRoots())) {
            if (one.kind == WallpaperKind::Scene && one.playable())
                found.insert(one.id, one.source);
        }
        return found;
    }

    /// The same scene drawn straight through the C interface into a
    /// framebuffer of the test's own, the way `shot` draws it, and read back
    /// the right way up -- which is what the item has to agree with.
    static QImage drawnDirectly(const QUrl &source, const QSize &size)
    {
        QOffscreenSurface surface;
        surface.create();
        QOpenGLContext context;
        if (!context.create() || !context.makeCurrent(&surface))
            return {};
        QImage picture;
        {
            QOpenGLFramebufferObject frame(size);
            frame.bind();
            const auto lookUp = [](const char *name, void *) -> const void * {
                return reinterpret_cast<const void *>(
                    QOpenGLContext::currentContext()->getProcAddress(name));
            };
            char error[256] = {};
            WallfluxScene *scene = wallflux_scene_load(
                QFileInfo(source.toLocalFile()).absolutePath().toUtf8().constData(), lookUp,
                nullptr, error, sizeof error);
            if (scene) {
                wallflux_scene_draw(scene, size.width(), size.height(), 0);
                wallflux_scene_free(scene);
                picture = frame.toImage().convertToFormat(QImage::Format_RGB32);
            }
        }
        context.doneCurrent();
        return picture;
    }

    /// How far apart two pictures of the same size are, nought to 255.
    static double apart(const QImage &a, const QImage &b)
    {
        double sum = 0;
        int n = 0;
        for (int y = 0; y < a.height(); y += 3) {
            for (int x = 0; x < a.width(); x += 3, ++n)
                sum += std::abs(qGray(a.pixel(x, y)) - qGray(b.pixel(x, y)));
        }
        return n ? sum / n : 255;
    }

private slots:
    void initTestCase()
    {
        QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
        if (scenes().isEmpty())
            QSKIP("no scene wallpapers on this machine");
    }

    /// Warhammer: effects, composition layers, a post-processing layer and
    /// particles -- most of what the renderer does, in one picture.
    void drawsASceneIntoTheItem()
    {
        const QUrl source = scenes().value(QStringLiteral("3356737002"));
        if (source.isEmpty())
            QSKIP("Warhammer is not on this machine");

        QQuickWindow window;
        window.resize(640, 360);
        SceneView view(window.contentItem());
        view.setSize(QSizeF(640, 360));
        // Standing still at nought seconds, so the picture is one moment and
        // not whichever moment the grab happened to land on.
        view.setRunning(false);
        view.setSource(source);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));

        const QImage picture = window.grabWindow();
        QVERIFY(view.error().isEmpty());
        if (qEnvironmentVariableIsSet("WALLFLUX_SCENE_GRAB"))
            picture.save(qEnvironmentVariable("WALLFLUX_SCENE_GRAB"));

        // A scene that loaded and drew nothing is the failure to catch: the
        // framebuffer would be the clear colour from edge to edge.
        qint64 lit = 0;
        for (int y = 0; y < picture.height(); y += 4) {
            for (int x = 0; x < picture.width(); x += 4)
                lit += qGray(picture.pixel(x, y)) > 16;
        }
        QVERIFY2(lit > (picture.width() / 4) * (picture.height() / 4) / 2,
                 qPrintable(QStringLiteral("only %1 lit samples").arg(lit)));

        // And the right way up. Drawn through the item, the first try came out
        // upside down: OpenGL draws from the bottom and Qt showed the
        // framebuffer as it was. Upside down is far further from the direct
        // drawing than the same picture is.
        const QImage direct = drawnDirectly(source, picture.size());
        QVERIFY(!direct.isNull());
        const QImage seen = picture.convertToFormat(QImage::Format_RGB32);
        const double same = apart(seen, direct);
        const double flipped = apart(seen, direct.flipped(Qt::Vertical));
        QVERIFY2(same < 8 && same * 3 < flipped,
                 qPrintable(QStringLiteral("%1 from the direct drawing, %2 from it upside down")
                                .arg(same)
                                .arg(flipped)));
    }

    /// A scene with nothing in it the renderer can draw says why, rather
    /// than leaving a black screen and no word about it.
    void saysWhyASceneCannotBeDrawn()
    {
        const QUrl source = scenes().value(QStringLiteral("1904095896"));
        if (source.isEmpty())
            QSKIP("Diablo IV Lilith, whose one image is a 3D model, is not on this machine");

        QQuickWindow window;
        window.resize(320, 180);
        SceneView view(window.contentItem());
        view.setSize(QSizeF(320, 180));
        QSignalSpy said(&view, &SceneView::errorChanged);
        view.setSource(source);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));

        QTRY_VERIFY(!view.error().isEmpty());
        QVERIFY(view.error().contains(QStringLiteral("nothing")));
    }
};

// Not QTEST_MAIN: without a display the application cannot even be built,
// and a package built somewhere with no screen should be told this test has
// nothing to do rather than that it failed.
int main(int argc, char *argv[])
{
    if (qEnvironmentVariableIsEmpty("WAYLAND_DISPLAY") && qEnvironmentVariableIsEmpty("DISPLAY")) {
        std::puts("SKIP: no display to draw a scene on");
        return 0;
    }
    QGuiApplication app(argc, argv);
    TestSceneView test;
    return QTest::qExec(&test, argc, argv);
}

#include "tst_sceneview.moc"
