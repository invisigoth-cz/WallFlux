// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QProcess>
#include <QQmlEngine>
#include <QTimer>

/// Says whether anything other than this program is playing sound.
///
/// Wallpaper Engine quietens its wallpaper when something else wants the
/// speakers, and this is what the interface asks to do the same. It only
/// reports; what to do about it is decided in QML.
///
/// `pactl subscribe` says when something changed and `pw-dump` says what is
/// playing, because only PipeWire knows whether a stream is really running:
/// pactl calls a stream PipeWire has paused for idling uncorked. Where
/// either is missing, this reports silence for ever and nothing else goes
/// wrong.
class OtherAudio : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(OtherAudio)
    QML_SINGLETON

    Q_PROPERTY(bool playing READ isPlaying NOTIFY playingChanged)

public:
    explicit OtherAudio(QObject *parent = nullptr);

    bool isPlaying() const { return m_playing; }

    /// Whether `pw-dump` describes an output stream that belongs to somebody
    /// else and is running.
    ///
    /// Takes the name this program runs under as well as its pid, because
    /// not every stream says which process it came from -- see isOurs() in
    /// the implementation.
    ///
    /// Static and handed its own text so it can be tested without a sound
    /// card, a desktop session or anything playing.
    static bool othersPlaying(const QByteArray &pwDumpJson, qint64 ourPid,
                              const QString &ourName);

signals:
    void playingChanged();

private:
    void look();
    void readAnswer();

    /// Runs for as long as the program does, printing a line per change.
    QProcess m_subscription;
    QProcess m_query;
    /// Volume changes arrive in bursts, and each one would otherwise start
    /// its own pw-dump.
    QTimer m_settle;
    bool m_playing = false;
};
