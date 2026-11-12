// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QElapsedTimer>
#include <QHash>
#include <QObject>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

/// What WallFlux costs the machine, for the status bar: its share of the
/// processor, of the graphics card, and the memory it holds -- its own and
/// its helpers' together, since a web wallpaper runs in a QtWebEngine
/// process of its own.
///
/// Read off /proc once a second while somebody is `watching`: a process's
/// `stat` for its processor time, its `status` for what it holds in memory,
/// and its open files' `fdinfo` for the graphics card, where the kernel's
/// DRM drivers (amdgpu, i915, xe, nouveau) count the time each client kept
/// an engine busy. Nothing is asked of anybody else.
class Usage : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Usage)
    QML_SINGLETON

    Q_PROPERTY(bool watching READ watching WRITE setWatching NOTIFY watchingChanged)
    /// Per cent of the whole processor, every core counted.
    Q_PROPERTY(double cpu READ cpu NOTIFY changed)
    /// Per cent of the graphics card's busiest engine; -1 where the driver
    /// does not say.
    Q_PROPERTY(double gpu READ gpu NOTIFY changed)
    /// Megabytes held in memory.
    Q_PROPERTY(double ram READ ram NOTIFY changed)
    /// Megabytes the whole machine still has to give, as the kernel counts
    /// what is available: free, and cache it can drop.
    Q_PROPERTY(double free READ free NOTIFY changed)

public:
    explicit Usage(QObject *parent = nullptr);

    bool watching() const { return m_timer.isActive(); }
    void setWatching(bool watching);
    double cpu() const { return m_cpu; }
    double gpu() const { return m_gpu; }
    double ram() const { return m_ram; }
    double free() const { return m_free; }

    /// A process's parent and its processor time in clock ticks, out of
    /// its `stat` -- whose name field may hold spaces and brackets, so the
    /// fields are counted from the last `)`.
    static bool readStat(const QByteArray &stat, qint64 *parent, qint64 *ticks);
    /// The kilobytes `VmRSS` says, out of a process's `status`.
    static qint64 residentKb(const QByteArray &status);
    /// The kilobytes `MemAvailable` says, out of `/proc/meminfo`.
    static qint64 availableKb(const QByteArray &meminfo);
    /// One open file's `fdinfo`: the DRM client it belongs to, or empty,
    /// and the nanoseconds each engine was busy for it.
    static QString drmClient(const QByteArray &fdinfo, QHash<QString, qint64> *engines);

signals:
    void watchingChanged();
    void changed();

private:
    void sample();

    QTimer m_timer;
    QElapsedTimer m_clock;
    qint64 m_lastTicks = -1;
    QHash<QString, qint64> m_lastEngines;
    double m_cpu = 0;
    double m_gpu = -1;
    double m_ram = 0;
    double m_free = 0;
};
