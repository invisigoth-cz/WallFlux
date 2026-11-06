// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "otheraudio.h"

#include <QCoreApplication>
#include <QProcess>
#include <QTest>

/// Only the reading of what pw-dump says is tested here. Starting it and
/// waiting for something to play needs a sound server, which a test has no
/// business requiring; the part that can get the answer wrong is this one.
class TestOtherAudio : public QObject
{
    Q_OBJECT

private:
    /// What pw-dump prints for one output stream in the given state.
    ///
    /// Built with arg() rather than by gluing raw strings together: a raw
    /// string whose text ends in a quote closes early as far as moc is
    /// concerned, and the whole test class then vanishes from the build.
    static QByteArray stream(const QString &state, const QString &properties)
    {
        return QStringLiteral(R"([{"id":90,"type":"PipeWire:Interface:Node","info":)"
                              R"({"state":"%1","props":{"media.class":"Stream/Output/Audio",%2}}}])")
            .arg(state, properties)
            .toUtf8();
    }

    /// A running stream belonging to the given process.
    static QByteArray streamFrom(qint64 pid)
    {
        return stream(QStringLiteral("running"),
                      QStringLiteral(R"("application.process.id":"%1")").arg(pid));
    }

    /// A running stream with no pid at all, named as the one who made it.
    /// This is what Qt's own video playback looks like.
    static QByteArray namedStream(const QString &name)
    {
        return stream(QStringLiteral("running"), QStringLiteral(R"("node.name":"%1")").arg(name));
    }

    static bool heard(const QByteArray &json)
    {
        return OtherAudio::othersPlaying(json, QCoreApplication::applicationPid(),
                                         QStringLiteral("wallflux"));
    }

private slots:
    void saysNothingIsPlayingWhenNothingIs() { QVERIFY(!heard("[]")); }

    void hearsAnotherProgram()
    {
        // pid 1 is init, which this process is certainly not a child of.
        QVERIFY(heard(streamFrom(1)));
    }

    void ignoresAPausedProgram()
    {
        QVERIFY2(!heard(stream(QStringLiteral("idle"), QStringLiteral(R"("application.process.id":"1")"))),
                 "a paused player was heard as playing");
    }

    void ignoresAStreamPipeWireHasSuspended()
    {
        // speech-dispatcher keeps a stream open for as long as it runs, and
        // pactl calls it uncorked while PipeWire has it suspended -- measured,
        // the wallpaper kept itself quiet all afternoon for nothing.
        const QByteArray speech = stream(
            QStringLiteral("suspended"),
            QStringLiteral(R"("application.process.id":"1","application.name":"speech-dispatcher-dummy")"));
        QVERIFY2(!heard(speech), "a stream that plays nothing was heard as playing");
    }

    void ignoresWhatIsNotAnOutputStream()
    {
        // A recording, a device or a sink is running too while a wallpaper
        // listens or plays; none of them is another program's sound.
        QVERIFY(!heard(R"([{"type":"PipeWire:Interface:Node","info":{"state":"running",)"
                       R"("props":{"media.class":"Audio/Sink","application.process.id":"1"}}},)"
                       R"({"type":"PipeWire:Interface:Client","info":{"props":{}}}])"));
    }

    void ignoresItself()
    {
        QVERIFY2(!heard(streamFrom(QCoreApplication::applicationPid())),
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

        const bool itsOwn = !heard(streamFrom(child.processId()));
        child.kill();
        child.waitForFinished();

        QVERIFY2(itsOwn, "a wallpaper's own web engine process was heard as somebody else");
    }

    void ignoresItsOwnVideoWhichCarriesNoPid()
    {
        // Measured on PipeWire: Qt plays a video wallpaper as a native
        // client, and the stream has no application.process.id to read --
        // only a name. Without this the wallpaper quietens itself the moment
        // it makes a sound, and stays that way.
        QVERIFY2(!heard(namedStream(QStringLiteral("wallflux"))),
                 "the wallpaper heard its own video and would have muted itself for good");
    }

    void hearsANamelessProgramThatCarriesNoPid()
    {
        QVERIFY2(heard(namedStream(QStringLiteral("vlc"))),
                 "another program's sound was taken for the wallpaper's own");
    }

    void treatsAStreamWithNoPidAsSomebodyElse()
    {
        QVERIFY(heard(stream(QStringLiteral("running"), QStringLiteral(R"("node.name":"x")"))));
    }

    void staysQuietWhenItSaysSomethingUnreadable() { QVERIFY(!heard("not json at all")); }
};

QTEST_MAIN(TestOtherAudio)
#include "tst_otheraudio.moc"
