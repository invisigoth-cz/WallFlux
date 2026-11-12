// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QProcess>
#include <QQmlEngine>
#include <QVariantMap>

/// Tries the whole library out, unseen and unheard, and says how each
/// wallpaper fared -- the same grades the window gives one that has been
/// played: "perfect", "minor" where the renderer left something out, and
/// "broken" where it would not play, drew nothing, or brought the trial down.
///
/// The trying is done by this same program started again as `wallflux
/// --grade`, one wallpaper after another: a scene drawn off screen for three
/// seconds, a video opened, a page loaded. A program of its own, so that a
/// wallpaper that crashes the renderer crashes nothing the user is looking
/// at; that one is marked broken and the rest go on in a new one.
class Compatibility : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Compatibility)
    QML_SINGLETON

    Q_PROPERTY(bool running READ running NOTIFY progress)
    Q_PROPERTY(int done READ done NOTIFY progress)
    Q_PROPERTY(int total READ total NOTIFY progress)
    /// The id of the wallpaper being tried, or empty.
    Q_PROPERTY(QString current READ current NOTIFY progress)

public:
    explicit Compatibility(QObject *parent = nullptr);
    ~Compatibility() override;

    bool running() const { return m_child != nullptr; }
    int done() const { return m_done; }
    int total() const { return m_total; }
    QString current() const { return m_current; }

    /// Tries each of `wallpapers`, each `{ id, kind, source, values }`:
    /// `kind` "scene", "video" or "web", `source` the URL the window plays,
    /// `values` the JSON of its settings as the renderer takes them.
    Q_INVOKABLE void test(const QVariantList &wallpapers);
    Q_INVOKABLE void stop();

    /// The other end: what `wallflux --grade` runs. Reads one wallpaper a
    /// line on standard input and writes what it found on standard output.
    static int grade(int argc, char **argv);

signals:
    void progress();
    void graded(const QString &id, const QString &grade, const QStringList &why);

private:
    void launch();
    void read();
    void ended();

    QProcess *m_child = nullptr;
    /// What is still to be tried, in order; the first is the one running.
    QList<QVariantMap> m_left;
    QByteArray m_pending;
    QString m_current;
    int m_done = 0;
    int m_total = 0;
};
