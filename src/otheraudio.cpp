// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "otheraudio.h"

#include <QCoreApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcAudio, "wallflux.audio")

namespace {

constexpr auto PactlCommand = "pactl";

/// The process that started this one, or 0 where that cannot be read.
///
/// /proc/<pid>/stat holds the command name in brackets, and a command may
/// contain both spaces and brackets of its own -- so the fields are counted
/// from the last closing bracket rather than from the beginning.
qint64 parentOf(qint64 pid)
{
    QFile stat(QStringLiteral("/proc/%1/stat").arg(pid));
    if (!stat.open(QIODevice::ReadOnly))
        return 0;

    const QByteArray line = stat.readLine();
    const int afterName = line.lastIndexOf(')');
    if (afterName < 0)
        return 0;

    // What follows is " R 1234 ...": the state, then the parent.
    const QList<QByteArray> fields = line.mid(afterName + 1).simplified().split(' ');
    if (fields.size() < 2)
        return 0;

    bool read = false;
    const qint64 parent = fields.at(1).toLongLong(&read);
    return read ? parent : 0;
}

/// Whether this process is the given one or was started by it.
///
/// QtWebEngine plays a web wallpaper's sound from a child process of its
/// own, so asking about our own pid alone would have the wallpaper mute
/// itself the moment it made a noise.
bool belongsToUs(qint64 pid, qint64 ourPid)
{
    // Deep enough for any sane process tree; a cycle cannot happen, but a
    // bound costs nothing and a loop here would hang the program.
    for (int step = 0; pid > 1 && step < 32; ++step) {
        if (pid == ourPid)
            return true;
        pid = parentOf(pid);
    }
    return false;
}

/// Whether a stream is one of ours rather than another program's.
///
/// Two ways of asking, because there are two sorts of stream here. Anything
/// going through the PulseAudio interface carries the pid it came from, and
/// a web wallpaper's sound comes from a QtWebEngine process of our own, so
/// that one is answered by walking up the process tree.
///
/// A video wallpaper's sound does not. Qt plays it as a native PipeWire
/// client, and those carry no pid at all: measured, the stream has only
/// `client.id` and a `node.name` and `media.name` of "wallflux", which is
/// this program's application name. Without this the wallpaper hears its own
/// sound as somebody else's and quietens itself the moment it makes one --
/// measured as muting itself a fraction of a second after starting.
bool isOurs(const QJsonObject &properties, qint64 ourPid, const QString &ourName)
{
    const qint64 pid =
        properties.value(QLatin1StringView("application.process.id")).toString().toLongLong();
    if (pid > 0)
        return belongsToUs(pid, ourPid);

    if (ourName.isEmpty())
        return false;

    for (const auto key : {"application.name", "node.name", "media.name"}) {
        if (properties.value(QLatin1StringView(key)).toString() == ourName)
            return true;
    }
    return false;
}

} // namespace

bool OtherAudio::othersPlaying(const QByteArray &sinkInputsJson, qint64 ourPid,
                               const QString &ourName)
{
    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(sinkInputsJson, &error);
    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        qCWarning(lcAudio) << "cannot read what pactl said:" << error.errorString();
        return false;
    }

    const QJsonArray streams = document.array();
    for (const QJsonValue &value : streams) {
        const QJsonObject stream = value.toObject();

        // A paused player keeps its stream open and corked. That is not
        // somebody playing something.
        if (stream.value(QLatin1StringView("corked")).toBool())
            continue;

        // A stream we cannot place is somebody else's by default: better to
        // quieten the wallpaper than to talk over whatever it is.
        if (isOurs(stream.value(QLatin1StringView("properties")).toObject(), ourPid, ourName))
            continue;

        return true;
    }

    return false;
}

OtherAudio::OtherAudio(QObject *parent)
    : QObject(parent)
{
    m_settle.setSingleShot(true);
    m_settle.setInterval(150);
    connect(&m_settle, &QTimer::timeout, this, &OtherAudio::look);

    connect(&m_query, &QProcess::finished, this, &OtherAudio::readAnswer);

    // Every line pactl prints is a change to something; the ones about sink
    // inputs are the ones that mean a program started or stopped playing.
    connect(&m_subscription, &QProcess::readyReadStandardOutput, this, [this]() {
        const QByteArray events = m_subscription.readAllStandardOutput();
        if (events.contains("sink-input"))
            m_settle.start();
    });

    // ponytail: the subscription is not restarted if pactl dies. Nothing
    // kills it in practice, and a wallpaper that stops quietening itself is
    // a nuisance rather than a fault. Restart it here if that ever changes.
    connect(&m_subscription, &QProcess::errorOccurred, this, [](QProcess::ProcessError error) {
        qCWarning(lcAudio) << "pactl subscribe will not run, so the wallpaper will not"
                           << "quieten itself for other programs:" << error;
    });

    m_subscription.start(QLatin1StringView(PactlCommand), {QStringLiteral("subscribe")});

    // What is already playing, before anything changes.
    look();
}

void OtherAudio::look()
{
    // One at a time: a burst that arrives while pactl is still answering is
    // waited out rather than piled on top of it.
    if (m_query.state() != QProcess::NotRunning) {
        m_settle.start();
        return;
    }

    m_query.start(QLatin1StringView(PactlCommand),
                  {QStringLiteral("--format=json"), QStringLiteral("list"),
                   QStringLiteral("sink-inputs")});
}

void OtherAudio::readAnswer()
{
    if (m_query.exitStatus() != QProcess::NormalExit || m_query.exitCode() != 0) {
        qCWarning(lcAudio) << "pactl could not list what is playing:"
                           << m_query.readAllStandardError().trimmed();
        return;
    }

    const bool playing = othersPlaying(m_query.readAllStandardOutput(),
                                       QCoreApplication::applicationPid(),
                                       QCoreApplication::applicationName());
    if (playing == m_playing)
        return;

    m_playing = playing;
    qCDebug(lcAudio) << (playing ? "something else is playing" : "nothing else is playing");
    emit playingChanged();
}
