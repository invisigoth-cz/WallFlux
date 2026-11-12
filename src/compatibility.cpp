// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "compatibility.h"
#include "wallflux_scene.h"
#include "webguard.h"

#include <QCoreApplication>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QMediaPlayer>
#include <QOffscreenSurface>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QTimer>
#include <QWebEnginePage>
#include <QWebEngineProfile>
#include <clocale>
#include <malloc.h>
#include <unistd.h>

Q_LOGGING_CATEGORY(lcTrial, "wallflux.trial")

namespace {

/// What the trial writes in front of each line meant for the window, so
/// that anything else that reaches its standard output is passed over.
constexpr QByteArrayView Mark = "wallflux-trial ";

struct Verdict
{
    QString grade;
    QStringList why;
};

void say(const QJsonObject &what)
{
    QByteArray line = Mark.toByteArray() + QJsonDocument(what).toJson(QJsonDocument::Compact) + '\n';
    ::write(STDOUT_FILENO, line.constData(), size_t(line.size()));
}

const void *lookUp(const char *name, void *)
{
    return reinterpret_cast<const void *>(QOpenGLContext::currentContext()->getProcAddress(name));
}

/// Whether nothing at all was drawn: every pixel black, give or take.
bool black(const QImage &picture)
{
    const QImage rgb = picture.convertToFormat(QImage::Format_RGB32);
    for (int y = 0; y < rgb.height(); ++y) {
        const auto *row = reinterpret_cast<const QRgb *>(rgb.constScanLine(y));
        for (int x = 0; x < rgb.width(); ++x) {
            if (qRed(row[x]) > 4 || qGreen(row[x]) > 4 || qBlue(row[x]) > 4)
                return false;
        }
    }
    return true;
}

/// A scene drawn for three seconds at fifteen a second, off screen, small:
/// the renderer says what it left out, and the last picture says whether it
/// drew anything at all.
Verdict scene(const QUrl &source, const QString &values, QOpenGLContext &context,
              QOffscreenSurface &surface)
{
    if (!context.makeCurrent(&surface))
        return {QStringLiteral("broken"), {QStringLiteral("No OpenGL to draw it with")}};

    const QSize size(640, 360);
    QOpenGLFramebufferObject frame(size);
    const QString folder = QFileInfo(source.toLocalFile()).absolutePath();
    const QByteArray settings = values.toUtf8();
    char error[512] = {};
    WallfluxScene *drawn = wallflux_scene_load_with(folder.toUtf8().constData(),
                                                    settings.isEmpty() ? nullptr : settings.constData(),
                                                    lookUp, nullptr, error, sizeof error);
    if (!drawn)
        return {QStringLiteral("broken"), {QString::fromUtf8(error)}};

    QByteArray left(int(wallflux_scene_skipped(drawn, nullptr, 0)) + 1, '\0');
    wallflux_scene_skipped(drawn, left.data(), size_t(left.size()));
    const QStringList skipped = QString::fromUtf8(left.constData()).split(u'\n', Qt::SkipEmptyParts);

    bool drew = true;
    for (int step = 0; step <= 45 && drew; ++step) {
        frame.bind();
        drew = wallflux_scene_draw(drawn, uint32_t(size.width()), uint32_t(size.height()), float(step) / 15);
    }
    const QImage picture = frame.toImage();
    wallflux_scene_free(drawn);
    malloc_trim(0);

    if (!drew)
        return {QStringLiteral("broken"), {QStringLiteral("The renderer stopped while drawing it")}};
    if (black(picture))
        return {QStringLiteral("broken"), {QStringLiteral("It drew nothing but black")}};
    return {skipped.isEmpty() ? QStringLiteral("perfect") : QStringLiteral("minor"), skipped};
}

/// Waits in an event loop for `done`, or for `seconds`.
template<typename Start>
Verdict awaited(int seconds, const QString &late, Start start)
{
    Verdict verdict{QStringLiteral("broken"), {late}};
    QEventLoop loop;
    QTimer::singleShot(seconds * 1000, &loop, &QEventLoop::quit);
    start(verdict, loop);
    loop.exec();
    return verdict;
}

/// A video opened, as the window grades one: it plays if it loads.
/// No audio output is attached, so it is not heard.
Verdict video(const QUrl &source)
{
    QMediaPlayer player;
    return awaited(15, QStringLiteral("It did not open within 15 seconds"), [&](Verdict &verdict, QEventLoop &loop) {
        QObject::connect(&player, &QMediaPlayer::mediaStatusChanged, &loop, [&](QMediaPlayer::MediaStatus status) {
            if (status == QMediaPlayer::LoadedMedia || status == QMediaPlayer::BufferedMedia) {
                verdict = {QStringLiteral("perfect"), {}};
                loop.quit();
            } else if (status == QMediaPlayer::InvalidMedia) {
                verdict = {QStringLiteral("broken"), {player.errorString()}};
                loop.quit();
            }
        });
        QObject::connect(&player, &QMediaPlayer::errorOccurred, &loop, [&](QMediaPlayer::Error, const QString &said) {
            verdict = {QStringLiteral("broken"), {said}};
            loop.quit();
        });
        player.setSource(source);
    });
}

/// A page loaded, as the window grades one: it plays if it loads. Muted,
/// and kept to the wallpapers' folders like the window's own.
Verdict page(const QUrl &source)
{
    QWebEnginePage page;
    page.setAudioMuted(true);
    return awaited(20, QStringLiteral("It did not load within 20 seconds"), [&](Verdict &verdict, QEventLoop &loop) {
        QObject::connect(&page, &QWebEnginePage::loadFinished, &loop, [&](bool ok) {
            verdict = ok ? Verdict{QStringLiteral("perfect"), {}}
                         : Verdict{QStringLiteral("broken"), {QStringLiteral("The page did not load")}};
            loop.quit();
        });
        page.load(source);
    });
}

QVariantMap parsed(const QByteArray &line)
{
    return QJsonDocument::fromJson(line.mid(Mark.size())).object().toVariantMap();
}

} // namespace

Compatibility::Compatibility(QObject *parent)
    : QObject(parent)
{
}

Compatibility::~Compatibility()
{
    stop();
}

void Compatibility::test(const QVariantList &wallpapers)
{
    if (running())
        return;
    m_left.clear();
    for (const QVariant &one : wallpapers)
        m_left.append(one.toMap());
    m_total = int(m_left.size());
    m_done = 0;
    qCInfo(lcTrial) << "trying" << m_total << "wallpapers";
    launch();
}

void Compatibility::stop()
{
    m_left.clear();
    m_current.clear();
    if (m_child) {
        // Out of the way first: killing it delivers `finished`, which must
        // not start another.
        QProcess *child = std::exchange(m_child, nullptr);
        child->disconnect(this);
        child->kill();
        child->waitForFinished(3000);
        delete child;
    }
    emit progress();
}

void Compatibility::launch()
{
    m_pending.clear();
    m_current.clear();
    if (m_left.isEmpty()) {
        qCInfo(lcTrial) << "tried them all";
        emit progress();
        return;
    }

    m_child = new QProcess(this);
    m_child->setProgram(QCoreApplication::applicationFilePath());
    m_child->setArguments({QStringLiteral("--grade")});
    // Its messages go where the program's own go; only its answers come here.
    m_child->setProcessChannelMode(QProcess::ForwardedErrorChannel);
    // Out of the way of whatever is being done on the machine meanwhile.
    m_child->setChildProcessModifier([] { (void)::nice(10); });
    connect(m_child, &QProcess::readyReadStandardOutput, this, &Compatibility::read);
    connect(m_child, &QProcess::finished, this, &Compatibility::ended);
    m_child->start();

    for (const QVariantMap &one : std::as_const(m_left))
        m_child->write(QJsonDocument(QJsonObject::fromVariantMap(one)).toJson(QJsonDocument::Compact) + '\n');
    m_child->closeWriteChannel();
    emit progress();
}

void Compatibility::read()
{
    if (!m_child)
        return;
    m_pending += m_child->readAllStandardOutput();
    for (qsizetype end; (end = m_pending.indexOf('\n')) >= 0;) {
        const QByteArray line = m_pending.left(end);
        m_pending.remove(0, end + 1);
        if (!line.startsWith(Mark))
            continue;
        const QVariantMap said = parsed(line);
        const QString id = said.value(QStringLiteral("id")).toString();
        if (said.contains(QStringLiteral("trying"))) {
            m_current = id;
        } else {
            const QString grade = said.value(QStringLiteral("grade")).toString();
            const QStringList why = said.value(QStringLiteral("why")).toStringList();
            m_left.removeIf([&](const QVariantMap &one) { return one.value(QStringLiteral("id")) == id; });
            m_current.clear();
            ++m_done;
            qCInfo(lcTrial) << id << grade << why;
            emit graded(id, grade, why);
        }
        emit progress();
    }
}

void Compatibility::ended()
{
    read();
    QProcess *child = std::exchange(m_child, nullptr);
    const bool crashed = child->exitStatus() != QProcess::NormalExit || child->exitCode() != 0;
    child->deleteLater();

    // What it was trying when it went is what brought it down.
    if (!m_current.isEmpty()) {
        const QString id = std::exchange(m_current, QString());
        qCWarning(lcTrial) << id << "brought the trial down";
        m_left.removeIf([&](const QVariantMap &one) { return one.value(QStringLiteral("id")) == id; });
        ++m_done;
        emit graded(id, QStringLiteral("broken"), {QStringLiteral("It brought the trial down")});
        launch();
        return;
    }
    // Gone without starting on anything: another start would go the same way.
    if (!m_left.isEmpty())
        qCWarning(lcTrial) << "the trial ended" << (crashed ? "badly" : "early") << "with"
                           << m_left.size() << "left";
    m_left.clear();
    emit progress();
}

int Compatibility::grade(int argc, char **argv)
{
    QGuiApplication app(argc, argv);
    // What SceneView's constructor does, for the same reason: QuickJS reads
    // numbers with strtod, which on a Czech desktop reads "0.5" as nought.
    std::setlocale(LC_NUMERIC, "C");
    // Without the music nothing listens to the speakers, so Plasma shows no
    // microphone; the rest as the defaults.
    wallflux_options(15, false, false, true, 0, true, false);
    QWebEngineProfile::defaultProfile()->setUrlRequestInterceptor(new WallpaperFilesOnly(&app));

    QOffscreenSurface surface;
    surface.create();
    QOpenGLContext context;
    if (!context.create())
        qCWarning(lcTrial) << "no OpenGL context: scenes will be marked broken";

    QFile in;
    if (!in.open(stdin, QIODevice::ReadOnly))
        return 1;
    while (!in.atEnd()) {
        const QVariantMap one = QJsonDocument::fromJson(in.readLine()).object().toVariantMap();
        const QString id = one.value(QStringLiteral("id")).toString();
        if (id.isEmpty())
            continue;
        say({{QStringLiteral("id"), id}, {QStringLiteral("trying"), true}});

        const QString kind = one.value(QStringLiteral("kind")).toString();
        const QUrl source(one.value(QStringLiteral("source")).toString());
        const Verdict verdict = kind == QLatin1StringView("scene")
                                    ? scene(source, one.value(QStringLiteral("values")).toString(), context, surface)
                                : kind == QLatin1StringView("video") ? video(source)
                                : kind == QLatin1StringView("web")   ? page(source)
                                    : Verdict{QStringLiteral("broken"), {QStringLiteral("Not something WallFlux plays")}};
        say({{QStringLiteral("id"), id},
             {QStringLiteral("grade"), verdict.grade},
             {QStringLiteral("why"), QJsonArray::fromStringList(verdict.why)}});
    }
    return 0;
}
