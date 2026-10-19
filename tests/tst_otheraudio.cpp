// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "otheraudio.h"

#include <QCoreApplication>
#include <QProcess>
#include <QTest>

/// Only the reading of what pactl says is tested here. Starting pactl and
/// waiting for it to say something needs a sound server, which a test has no
/// business requiring; the part that can get the answer wrong is this one.
class TestOtherAudio : public QObject
{
    Q_OBJECT

private:
    /// What pactl prints for one running stream belonging to this process.
    ///
    /// Built with arg() rather than by gluing raw strings together: a raw
    /// string whose text ends in a quote closes early as far as moc is
    /// concerned, and the whole test class then vanishes from the build.
    static QByteArray streamFrom(qint64 pid)
    {
        return QStringLiteral(
                   R"([{"corked":false,"properties":{"application.process.id":"%1"}}])")
            .arg(pid)
            .toUtf8();
    }

    /// What pactl prints for a stream with no pid at all, named as the one
    /// who made it. This is what Qt's own video playback looks like.
    static QByteArray namedStream(const QString &name)
    {
        return QStringLiteral(R"([{"corked":false,"properties":{"node.name":"%1"}}])")
            .arg(name)
            .toUtf8();
    }

private slots:
    void saysNothingIsPlayingWhenNothingIs()
    {
        QVERIFY(!OtherAudio::othersPlaying("[]", QCoreApplication::applicationPid(),
                                           QStringLiteral("wallflux")));
    }

    void hearsAnotherProgram()
    {
        // pid 1 is init, which this process is certainly not a child of.
        const QByteArray playing =
            R"([{"corked":false,"properties":{"application.process.id":"1"}}])";
        QVERIFY(OtherAudio::othersPlaying(playing, QCoreApplication::applicationPid(),
                                          QStringLiteral("wallflux")));
    }

    void ignoresAPausedProgram()
    {
        const QByteArray paused =
            R"([{"corked":true,"properties":{"application.process.id":"1"}}])";
        QVERIFY2(!OtherAudio::othersPlaying(paused, QCoreApplication::applicationPid(),
                                            QStringLiteral("wallflux")),
                 "a paused player was heard as playing");
    }

    void ignoresItself()
    {
        const qint64 us = QCoreApplication::applicationPid();
        QVERIFY2(!OtherAudio::othersPlaying(streamFrom(us), us, QStringLiteral("wallflux")),
                 "the wallpaper heard its own sound and would have muted itself");
    }

    void ignoresItsOwnChildren()
    {
        // A web wallpaper's sound comes out of a QtWebEngine process, which
        // is a child of ours rather than ourselves -- so this needs a real
        // child, not a made-up number: the walk up /proc is the part that
        // can be wrong.
        QProcess child;
        child.start(QStringLiteral("sleep"), {QStringLiteral("30")});
        QVERIFY2(child.waitForStarted(), "could not start a child process to test with");

        const bool heard = OtherAudio::othersPlaying(streamFrom(child.processId()),
                                                     QCoreApplication::applicationPid(),
                                                     QStringLiteral("wallflux"));
        child.kill();
        child.waitForFinished();

        QVERIFY2(!heard, "a wallpaper's own web engine process was heard as somebody else");
    }

    void ignoresItsOwnVideoWhichCarriesNoPid()
    {
        // Measured on PipeWire: Qt plays a video wallpaper as a native
        // client, and the stream has no application.process.id to read --
        // only a name. Without this the wallpaper quietens itself the moment
        // it makes a sound, and stays that way.
        QVERIFY2(!OtherAudio::othersPlaying(namedStream(QStringLiteral("wallflux")),
                                            QCoreApplication::applicationPid(),
                                            QStringLiteral("wallflux")),
                 "the wallpaper heard its own video and would have muted itself for good");
    }

    void hearsANamelessProgramThatCarriesNoPid()
    {
        QVERIFY2(OtherAudio::othersPlaying(namedStream(QStringLiteral("vlc")),
                                           QCoreApplication::applicationPid(),
                                           QStringLiteral("wallflux")),
                 "another program's sound was taken for the wallpaper's own");
    }

    void treatsAStreamWithNoPidAsSomebodyElse()
    {
        QVERIFY(OtherAudio::othersPlaying(R"([{"corked":false,"properties":{}}])",
                                          QCoreApplication::applicationPid(),
                                          QStringLiteral("wallflux")));
    }

    void staysQuietWhenPactlSaysSomethingUnreadable()
    {
        QVERIFY(!OtherAudio::othersPlaying("not json at all",
                                           QCoreApplication::applicationPid(),
                                           QStringLiteral("wallflux")));
    }
};

QTEST_MAIN(TestOtherAudio)
#include "tst_otheraudio.moc"
