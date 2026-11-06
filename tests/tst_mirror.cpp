// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "mirror.h"

#include <QGuiApplication>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QTest>

#include <memory>

/// One window's picture shown in another, the way a second screen clones the
/// first: the right way up, cropped to the second window's shape, and kept up
/// to date when the first one changes.
///
/// Needs a display, as every window does.
class TestMirror : public QObject
{
    Q_OBJECT

    static bool near(QColor got, QColor wanted)
    {
        return qAbs(got.red() - wanted.red()) < 8 && qAbs(got.green() - wanted.green()) < 8
               && qAbs(got.blue() - wanted.blue()) < 8;
    }

private slots:
    void showsTheOtherWindowsPicture()
    {
        // Wider than the mirror: a black band down each side that the crop
        // has to take off, and red over green between them.
        QQmlEngine engine;
        QQmlComponent component(&engine);
        component.setData(R"(
            import QtQuick
            Window {
                width: 200; height: 100; visible: true; color: "black"
                property alias picture: picture
                property alias top: top
                Item {
                    id: picture
                    anchors.fill: parent
                    layer.enabled: true
                    Rectangle { anchors.fill: parent; color: "black" }
                    Rectangle { id: top; x: 50; width: 100; height: 50; color: "red" }
                    Rectangle { x: 50; y: 50; width: 100; height: 50; color: "lime" }
                }
            })", QUrl());
        std::unique_ptr<QObject> from(component.create());
        QVERIFY2(from, qPrintable(component.errorString()));
        auto *picture = from->property("picture").value<QQuickItem *>();
        QVERIFY(picture);

        QQuickWindow to;
        to.resize(100, 100);
        Mirror mirror(to.contentItem());
        mirror.setSize(QSizeF(100, 100));
        mirror.setSource(picture);
        to.show();
        QVERIFY(QTest::qWaitForWindowExposed(&to));
        QTest::qWait(500);

        QImage got = to.grabWindow();
        const qreal scale = got.width() / 100.0;
        const auto at = [&](int x, int y) { return got.pixelColor(int(x * scale), int(y * scale)); };
        QVERIFY2(near(at(50, 10), Qt::red), qPrintable(at(50, 10).name()));
        QVERIFY2(near(at(50, 90), QColor(0, 255, 0)), qPrintable(at(50, 90).name()));
        // No black band: the sides of the source are what the crop took off.
        QVERIFY2(near(at(2, 10), Qt::red), qPrintable(at(2, 10).name()));
        QVERIFY2(near(at(97, 90), QColor(0, 255, 0)), qPrintable(at(97, 90).name()));

        // And the next picture of the source reaches the mirror.
        from->property("top").value<QObject *>()->setProperty("color", QColor(Qt::blue));
        QTest::qWait(500);
        got = to.grabWindow();
        QVERIFY2(near(at(50, 10), Qt::blue), qPrintable(at(50, 10).name()));
    }
};

int main(int argc, char *argv[])
{
    // As the program has them: the windows' contexts share their textures,
    // which is what a mirror reads, and the scene graph is OpenGL.
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
    QGuiApplication app(argc, argv);
    TestMirror test;
    return QTest::qExec(&test, argc, argv);
}

#include "tst_mirror.moc"
