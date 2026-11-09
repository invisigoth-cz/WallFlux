// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QJsonObject>
#include <QNetworkAccessManager>
#include <QObject>
#include <QProcess>
#include <QQmlEngine>
#include <QUrl>

/// Whether a newer WallFlux has been released, and putting it in place.
///
/// Asked of GitHub's releases, without a key: the newest release's tag and
/// the Arch package attached to it. Installing is pacman's, through pkexec,
/// which puts up the desktop's own password window; then the program starts
/// itself again. Only an installed copy -- `/usr/bin/wallflux` -- installs;
/// one run out of a build tree can only say a release is out.
///
/// Says what is happening as a `State` and never as a sentence: see the
/// trap about a sentence built in C++ being built once.
class Updater : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Updater)
    QML_SINGLETON

public:
    enum State { Idle, Checking, Current, Available, Downloading, Installing, Failed };
    Q_ENUM(State)

    Q_PROPERTY(State state READ state NOTIFY changed)
    /// The newest release's number, without the `v`.
    Q_PROPERTY(QString latest READ latest NOTIFY changed)
    /// What the release says about itself, in Markdown.
    Q_PROPERTY(QString notes READ notes NOTIFY changed)
    Q_PROPERTY(QUrl page READ page NOTIFY changed)
    /// Per cent of the package fetched.
    Q_PROPERTY(int progress READ progress NOTIFY changed)
    /// What went wrong, as the network or pacman said it.
    Q_PROPERTY(QString detail READ detail NOTIFY changed)
    Q_PROPERTY(bool canInstall READ canInstall CONSTANT)

    explicit Updater(QObject *parent = nullptr);

    State state() const { return m_state; }
    QString latest() const { return m_latest; }
    QString notes() const { return m_notes; }
    QUrl page() const { return m_page; }
    int progress() const { return m_progress; }
    QString detail() const { return m_detail; }
    bool canInstall() const;

    Q_INVOKABLE void check();
    Q_INVOKABLE void install();

    /// Whether `candidate` is a later version than `current`, number by
    /// number: 0.10.0 is later than 0.9.0, which as text it is not.
    static bool newer(const QString &candidate, const QString &current);
    /// The Arch package among a release's assets, or nothing.
    static QUrl packageIn(const QJsonObject &release);

signals:
    void changed();

private:
    void set(State state, const QString &detail = {});
    void installFrom(const QString &file);

    QNetworkAccessManager m_network;
    QProcess m_pacman;
    State m_state = Idle;
    QString m_latest;
    QString m_notes;
    QUrl m_page;
    QUrl m_package;
    int m_progress = 0;
    QString m_detail;
};
