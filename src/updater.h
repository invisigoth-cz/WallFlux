// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QNetworkAccessManager>
#include <QObject>
#include <QQmlEngine>
#include <QUrl>

/// Whether a newer WallFlux has been released.
///
/// Asked of GitHub's releases, without a key: the newest release's tag.
/// Installing it is `install.sh`'s, the same line WallFlux was installed
/// with, run in a terminal by whoever wants it; WallFlux only says so.
///
/// Says what is happening as a `State` and never as a sentence: see the
/// trap about a sentence built in C++ being built once.
class Updater : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Updater)
    QML_SINGLETON

public:
    enum State { Idle, Checking, Current, Available, Failed };
    Q_ENUM(State)

    Q_PROPERTY(State state READ state NOTIFY changed)
    /// The newest release's number, without the `v`.
    Q_PROPERTY(QString latest READ latest NOTIFY changed)
    /// What the release says about itself, in Markdown.
    Q_PROPERTY(QString notes READ notes NOTIFY changed)
    Q_PROPERTY(QUrl page READ page NOTIFY changed)
    /// What went wrong, as the network said it.
    Q_PROPERTY(QString detail READ detail NOTIFY changed)

    explicit Updater(QObject *parent = nullptr);

    State state() const { return m_state; }
    QString latest() const { return m_latest; }
    QString notes() const { return m_notes; }
    QUrl page() const { return m_page; }
    QString detail() const { return m_detail; }

    Q_INVOKABLE void check();

    /// Whether `candidate` is a later version than `current`, number by
    /// number: 0.10.0 is later than 0.9.0, which as text it is not.
    static bool newer(const QString &candidate, const QString &current);

signals:
    void changed();

private:
    void set(State state, const QString &detail = {});

    QNetworkAccessManager m_network;
    State m_state = Idle;
    QString m_latest;
    QString m_notes;
    QUrl m_page;
    QString m_detail;
};
