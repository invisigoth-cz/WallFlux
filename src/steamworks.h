// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QString>
#include <QList>
#include <QPair>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

/// Whether this machine can be asked about the Steam Workshop, and the way in.
///
/// Nothing of Valve's is shipped with WallFlux and nothing is linked against
/// at build time. The library is looked for when it is first wanted and
/// opened with dlopen; where it is not there, this reports as much and the
/// interface falls back to sending somebody to the Steam client by hand.
///
/// Why not ship it: the licence that travels with the Steamworks SDK permits
/// redistributing it only in its entirety, unmodified, not integrated with
/// another program, and with its own agreement put in front of the end user
/// -- and the agreement that actually governs the SDK is behind a partner
/// login and cannot be read. Somebody who has the SDK accepted those terms
/// themselves; WallFlux stays out of it by never being the one to hand it on.
///
/// The GPL exception in LICENSE-EXCEPTION is what makes even this much
/// allowed from our side.
class Steamworks : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Steamworks)
    QML_SINGLETON

    /// Whether Steam answered and the workshop interface is there to ask.
    Q_PROPERTY(bool available READ isAvailable CONSTANT)

    /// Why not, in words an interface can show. Empty while all is well.
    Q_PROPERTY(QString trouble READ trouble CONSTANT)

    /// What each question came back with: one map per wallpaper, with `id`,
    /// `title`, `preview`, `tags`, `bytes` and `votesUp`.
    ///
    /// One list per order rather than one shared one, because the window
    /// shows two rows at once and filling both from a single question would
    /// mean the second row showing the tail of the first -- which is not
    /// what "New" says.
    Q_PROPERTY(QVariantList trending READ trending NOTIFY foundChanged)
    Q_PROPERTY(QVariantList newest READ newest NOTIFY foundChanged)

    /// Whether a question is outstanding.
    Q_PROPERTY(bool busy READ isBusy NOTIFY busyChanged)

    /// How far along each download is, keyed by published file id: 0 to 100
    /// while it runs, 100 once Steam says it is there, and -1 where Steam
    /// would not do it. An id that is absent was never asked for.
    ///
    /// One map rather than a signal per wallpaper, because what wants to
    /// know is a button on a card and a binding is how a card asks.
    Q_PROPERTY(QVariantMap downloads READ downloads NOTIFY downloadsChanged)

public:
    /// What to put at the top, in the order the picker's own menu has them.
    enum Order {
        Trending,     ///< What the workshop is making a fuss of now.
        Newest,       ///< By the date it was published.
        MostSubscribed, ///< By how many people took it.
    };
    Q_ENUM(Order)

    explicit Steamworks(QObject *parent = nullptr);
    ~Steamworks() override;

    bool isAvailable() const { return m_ugc != nullptr; }
    QString trouble() const { return m_trouble; }
    QVariantList trending() const { return m_trending; }
    QVariantList newest() const { return m_newest; }
    bool isBusy() const { return m_call != 0; }
    QVariantMap downloads() const { return m_downloads; }

    /// Asks the workshop for one page of wallpapers, fifty at a time, which
    /// is what Steam gives. The answer arrives in the list for that order a
    /// moment later. Asking again while one is outstanding is allowed: the
    /// questions are asked one after another, since a query handle has to be
    /// let go of before the next is made.
    Q_INVOKABLE void browse(Steamworks::Order order, int page = 1);

    /// Subscribes to one wallpaper and asks Steam to fetch it now. Nothing
    /// is fetched here: Steam downloads it into the folder it already owns,
    /// and the watcher on that folder is what puts it in the library -- so
    /// there is nothing to do once this has been called but watch
    /// `downloads` and wait.
    Q_INVOKABLE void download(const QString &id);

signals:
    void foundChanged();
    void busyChanged();
    void downloadsChanged();

private:
    /// Opens libsteam_api.so from wherever it is, or says where it looked.
    bool open();

    /// Steam has no thread of its own here: its answers are collected by
    /// asking, which is what this does while a question is outstanding.
    void pump();

    /// Reads one finished query out of Steam and into `found`.
    void collect(quint64 call, int callbackId, int paramSize);

    /// Asks Steam how far each outstanding download has got.
    void measure();

    /// Whether the pump still has anything to pump for.
    bool waiting() const { return m_call != 0 || !m_downloading.isEmpty(); }

    void *m_library = nullptr;
    /// The ISteamUGC pointer, which is what everything about the workshop is
    /// asked through. Null until open() has worked.
    void *m_ugc = nullptr;
    QString m_trouble;
    QVariantList m_trending;
    QVariantList m_newest;

    /// What was asked, so the answer lands in the right list, and what is
    /// still waiting to be asked.
    Order m_asking = Trending;
    QList<QPair<Order, int>> m_queued;

    /// What is on its way down, and how far each has got. The list is the
    /// ids to ask about; the map is what the interface reads.
    QList<quint64> m_downloading;
    QVariantMap m_downloads;

    /// The pipe callbacks come down, and what is outstanding on it.
    qint32 m_pipe = 0;
    quint64 m_query = 0;
    quint64 m_call = 0;
    QTimer m_pump;

    // The library's own entry points, found by name. Spelled out rather
    // than taken from Valve's headers, which are deliberately not here.
    void (*m_shutdown)() = nullptr;
    void (*m_runFrame)(qint32) = nullptr;
    bool (*m_nextCallback)(qint32, void *) = nullptr;
    void (*m_freeCallback)(qint32) = nullptr;
    bool (*m_callResult)(qint32, quint64, void *, int, int, bool *) = nullptr;
    quint64 (*m_createQuery)(void *, int, int, quint32, quint32, quint32) = nullptr;
    quint64 (*m_sendQuery)(void *, quint64) = nullptr;
    bool (*m_queryResult)(void *, quint64, quint32, void *) = nullptr;
    bool (*m_previewUrl)(void *, quint64, quint32, char *, quint32) = nullptr;
    bool (*m_releaseQuery)(void *, quint64) = nullptr;
    quint64 (*m_subscribe)(void *, quint64) = nullptr;
    bool (*m_downloadItem)(void *, quint64, bool) = nullptr;
    bool (*m_downloadInfo)(void *, quint64, quint64 *, quint64 *) = nullptr;
};
