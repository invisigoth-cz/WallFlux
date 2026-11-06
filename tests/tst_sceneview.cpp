// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include <clocale>
#include "sceneview.h"
#include "wallflux_scene.h"
#include "wallpapercore.h"

#include <QDataStream>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QOffscreenSurface>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QQuickOpenGLUtils>
#include <QQuickWindow>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QTest>

#include <memory>
#include <vector>

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

    /// Future Clock's red is a layer whose alpha a script's JSON carries,
    /// which is nought wherever the C library reads a comma for a point.
    /// Czech is what it was found on; any such locale does it.
    void readsItsNumbersWhateverTheLocale()
    {
        const QUrl source = scenes().value(QStringLiteral("2127129941"));
        if (source.isEmpty())
            QSKIP("Future Clock is not on this machine");
        if (!setlocale(LC_NUMERIC, "cs_CZ.UTF-8"))
            QSKIP("no locale with a decimal comma on this machine");

        QQuickWindow window;
        window.resize(640, 360);
        SceneView view(window.contentItem());
        view.setSize(QSizeF(640, 360));
        view.setRunning(false);
        view.setSource(source);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        const QImage picture = window.grabWindow();
        int reddest = 0;
        for (int y = 0; y < picture.height(); y += 2) {
            for (int x = 0; x < picture.width(); x += 2) {
                const QRgb pixel = picture.pixel(x, y);
                reddest = qMax(reddest, qRed(pixel) - qGreen(pixel));
            }
        }
        QVERIFY2(reddest > 100, qPrintable(QStringLiteral("reddest by %1").arg(reddest)));
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

    /// Particles drawn the second frame as they were the first, after Qt has
    /// put its own state back between them -- which the item does after
    /// every frame. Resetting binds no index buffer, and the bound vertex
    /// array is what remembers which one it uses: the particles' array, left
    /// bound by the frame before, lost its indices and every smoke in the
    /// library was drawn without them on the desktop, while `shot`, which
    /// resets nothing, drew them all. Arkham Knight because it is nothing but
    /// a picture and five fogs.
    void survivesQtPuttingItsStateBack()
    {
        const QUrl source = scenes().value(QStringLiteral("1562429365"));
        if (source.isEmpty())
            QSKIP("Arkham Knight is not on this machine");

        const QSize size(480, 270);
        const auto run = [&](bool reset) {
            QOffscreenSurface surface;
            surface.create();
            QOpenGLContext context;
            if (!context.create() || !context.makeCurrent(&surface))
                return QImage();
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
                    // Six seconds a frame at a time, which is when the fog
                    // has filled in.
                    for (int i = 0; i <= 360; ++i) {
                        wallflux_scene_draw(scene, size.width(), size.height(), i / 60.0f);
                        if (reset) {
                            QQuickOpenGLUtils::resetOpenGLState();
                            frame.bind();
                        }
                    }
                    wallflux_scene_free(scene);
                    picture = frame.toImage().convertToFormat(QImage::Format_RGB32);
                }
            }
            context.doneCurrent();
            return picture;
        };
        const QImage left = run(false);
        const QImage reset = run(true);
        QVERIFY(!left.isNull() && !reset.isNull());
        const double apartBy = apart(left, reset);
        QVERIFY2(apartBy < 0.5,
                 qPrintable(QStringLiteral("%1 apart with Qt's state put back").arg(apartBy)));
    }

    /// A scene's music is inside its package, out of a media player's
    /// reach, and has to come out to be played. Arkham Knight carries one
    /// track and loops it.
    void unpacksTheMusicOfAScene()
    {
        const QUrl source = scenes().value(QStringLiteral("1562429365"));
        if (source.isEmpty())
            QSKIP("Arkham Knight is not on this machine");
        QStandardPaths::setTestModeEnabled(true);

        const QVariantList sounds = SceneSounds().of(source);
        QCOMPARE(sounds.size(), 1);
        const QVariantMap sound = sounds.first().toMap();
        QCOMPARE(sound.value(QStringLiteral("mode")).toString(), QStringLiteral("loop"));
        const QVariantList files = sound.value(QStringLiteral("files")).toList();
        QCOMPARE(files.size(), 1);
        const QFileInfo file(files.first().toUrl().toLocalFile());
        QVERIFY2(file.exists() && file.size() > 1'000'000, qPrintable(file.filePath()));
        QVERIFY(file.filePath().endsWith(QStringLiteral(".mp3")));
    }

    /// A running scene is drawn fifteen times a second, not as often as the
    /// screen would let it. Counted over two seconds, with room for a timer
    /// that lands between two frames of the screen.
    void drawsFifteenTimesASecond()
    {
        const QUrl source = scenes().value(QStringLiteral("2149578679"));
        if (source.isEmpty())
            QSKIP("Blue Dawn is not on this machine");

        QQuickWindow window;
        window.resize(320, 180);
        SceneView view(window.contentItem());
        view.setSize(QSizeF(320, 180));
        view.setSource(source);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTest::qWait(500);

        QSignalSpy frames(&window, &QQuickWindow::frameSwapped);
        QTest::qWait(2000);
        QVERIFY2(frames.size() >= 25 && frames.size() <= 34, qPrintable(QString::number(frames.size())));

        // And not at all while it is held.
        view.setRunning(false);
        QTest::qWait(200);
        frames.clear();
        QTest::qWait(1000);
        QVERIFY2(frames.size() <= 1, qPrintable(QString::number(frames.size())));
    }

    /// Where the mouse is reaches the scene. Three of Blue Dawn side by
    /// side, running together: the outer two with the mouse on the left,
    /// which have to agree, and the middle one with it on the right, which
    /// has to differ from them by much more -- the layers lean away from the
    /// pointer. Blue Dawn because it draws the same picture twice, which
    /// Warhammer does not, and leans a long way. The two that agree are the two loaded furthest apart, so
    /// whatever the moments between the loads change counts against the
    /// test rather than for it. The trap
    /// it is there for: a running renderer draws itself again without being
    /// synchronized, so a mouse that only changed the item was never heard.
    void hearsWhereTheMouseIs()
    {
        const QUrl source = scenes().value(QStringLiteral("2149578679"));
        if (source.isEmpty())
            QSKIP("Blue Dawn is not on this machine");

        const QSize one(320, 180);
        QQuickWindow window;
        window.resize(one.width() * 3, one.height());
        std::vector<std::unique_ptr<SceneView>> views;
        for (int i = 0; i < 3; ++i) {
            auto view = std::make_unique<SceneView>(window.contentItem());
            view->setSize(one);
            view->setX(i * one.width());
            view->setSource(source);
            views.push_back(std::move(view));
        }
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        // Set once the scenes are running, which is when the trap bites.
        QTest::qWait(300);
        views[0]->setPointer(QPointF(one.width() * 0.05, one.height() / 2));
        views[1]->setPointer(QPointF(one.width() * 0.95, one.height() / 2));
        views[2]->setPointer(QPointF(one.width() * 0.05, one.height() / 2));
        QTest::qWait(2000);

        const QImage all = window.grabWindow().convertToFormat(QImage::Format_RGB32);
        const QSize grabbed(all.width() / 3, all.height());
        const auto part = [&](int i) { return all.copy(QRect(QPoint(i * grabbed.width(), 0), grabbed)); };
        const double together = apart(part(0), part(2));
        const double moved = apart(part(0), part(1));
        QVERIFY2(moved > 3 * together && moved > 2,
                 qPrintable(QStringLiteral("%1 apart with the mouse elsewhere, %2 with it in the same place")
                                .arg(moved)
                                .arg(together)));
    }

    /// A scene with nothing in it the renderer can draw says why, rather
    /// than leaving a black screen and no word about it.
    ///
    /// The scene is made here, a package holding a scene.json with no
    /// objects. It used to be Diablo IV Lilith, taken for a 3D model, until
    /// its one image turned out to be a video and began to play.
    void saysWhyASceneCannotBeDrawn()
    {
        QTemporaryDir folder;
        QVERIFY(folder.isValid());
        const QByteArray scene = R"({"general": {}, "objects": []})";
        QFile package(folder.filePath(QStringLiteral("scene.pkg")));
        QVERIFY(package.open(QIODevice::WriteOnly));
        QDataStream out(&package);
        out.setByteOrder(QDataStream::LittleEndian);
        const auto text = [&](const QByteArray &bytes) {
            out << quint32(bytes.size());
            out.writeRawData(bytes.constData(), int(bytes.size()));
        };
        // A package is its version, then each entry's name, offset and size,
        // then the entries' bytes.
        text("PKGV0001");
        out << quint32(1);
        text("scene.json");
        out << quint32(0) << quint32(scene.size());
        out.writeRawData(scene.constData(), int(scene.size()));
        package.close();
        const QUrl source = QUrl::fromLocalFile(package.fileName());

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
