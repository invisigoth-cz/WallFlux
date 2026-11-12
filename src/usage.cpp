// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "usage.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QSet>
#include <QThread>

#include <unistd.h>

namespace {

QByteArray slurp(const QString &path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

} // namespace

Usage::Usage(QObject *parent)
    : QObject(parent)
{
    m_timer.setInterval(1000);
    connect(&m_timer, &QTimer::timeout, this, &Usage::sample);
}

void Usage::setWatching(bool watching)
{
    if (watching == m_timer.isActive())
        return;
    if (watching) {
        // A fresh start: the first reading only sets the counters going.
        m_lastTicks = -1;
        m_lastEngines.clear();
        m_clock.start();
        m_timer.start();
        sample();
    } else {
        m_timer.stop();
    }
    emit watchingChanged();
}

bool Usage::readStat(const QByteArray &stat, qint64 *parent, qint64 *ticks)
{
    const int close = stat.lastIndexOf(')');
    if (close < 0)
        return false;
    // After the name: state, ppid, ... utime is the 14th field of the whole
    // line and stime the 15th, so the 12th and 13th after the bracket.
    const QList<QByteArray> fields = stat.mid(close + 1).simplified().split(' ');
    if (fields.size() < 13)
        return false;
    bool ok1 = false, ok2 = false, ok3 = false;
    *parent = fields.at(1).toLongLong(&ok1);
    *ticks = fields.at(11).toLongLong(&ok2) + fields.at(12).toLongLong(&ok3);
    return ok1 && ok2 && ok3;
}

namespace {

/// The number after `key` on its own line, as /proc writes its kilobytes.
qint64 kilobytes(const QByteArray &text, QByteArrayView key)
{
    for (const QByteArray &line : text.split('\n')) {
        if (line.startsWith(key))
            return line.mid(key.size()).simplified().split(' ').value(0).toLongLong();
    }
    return 0;
}

} // namespace

qint64 Usage::residentKb(const QByteArray &status)
{
    return kilobytes(status, "VmRSS:");
}

qint64 Usage::availableKb(const QByteArray &meminfo)
{
    return kilobytes(meminfo, "MemAvailable:");
}

QString Usage::drmClient(const QByteArray &fdinfo, QHash<QString, qint64> *engines)
{
    QString client;
    for (const QByteArray &line : fdinfo.split('\n')) {
        if (line.startsWith("drm-client-id:")) {
            client = QString::fromLatin1(line.mid(14).trimmed());
        } else if (line.startsWith("drm-engine-") && line.endsWith(" ns")) {
            const int colon = line.indexOf(':');
            const QString engine = QString::fromLatin1(line.mid(11, colon - 11));
            const qint64 ns = line.mid(colon + 1, line.size() - colon - 4).trimmed().toLongLong();
            engines->insert(engine, ns);
        }
    }
    return client;
}

void Usage::sample()
{
    // Every process on the machine once, to find this one's descendants.
    const qint64 self = QCoreApplication::applicationPid();
    QHash<qint64, qint64> parents;
    QHash<qint64, qint64> ticksOf;
    const QStringList pids = QDir(QStringLiteral("/proc")).entryList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QString &name : pids) {
        bool number = false;
        const qint64 pid = name.toLongLong(&number);
        if (!number)
            continue;
        qint64 parent = 0, ticks = 0;
        if (readStat(slurp(QStringLiteral("/proc/%1/stat").arg(pid)), &parent, &ticks)) {
            parents.insert(pid, parent);
            ticksOf.insert(pid, ticks);
        }
    }
    QSet<qint64> ours{self};
    for (bool grew = true; grew;) {
        grew = false;
        for (auto it = parents.cbegin(); it != parents.cend(); ++it) {
            if (!ours.contains(it.key()) && ours.contains(it.value())) {
                ours.insert(it.key());
                grew = true;
            }
        }
    }

    qint64 ticks = 0;
    qint64 kb = 0;
    // Each DRM client once, however many files it is open through.
    QHash<QString, QHash<QString, qint64>> clients;
    for (const qint64 pid : std::as_const(ours)) {
        ticks += ticksOf.value(pid);
        kb += residentKb(slurp(QStringLiteral("/proc/%1/status").arg(pid)));
        const QString fds = QStringLiteral("/proc/%1/fdinfo").arg(pid);
        for (const QString &fd : QDir(fds).entryList(QDir::Files)) {
            QHash<QString, qint64> engines;
            const QString client = drmClient(slurp(fds + u'/' + fd), &engines);
            if (!client.isEmpty())
                clients.insert(QStringLiteral("%1/%2").arg(pid).arg(client), engines);
        }
    }
    QHash<QString, qint64> engines;
    for (const auto &one : std::as_const(clients)) {
        for (auto it = one.cbegin(); it != one.cend(); ++it)
            engines[it.key()] += it.value();
    }

    const double seconds = m_clock.restart() / 1000.0;
    if (m_lastTicks >= 0 && seconds > 0) {
        const double hz = double(sysconf(_SC_CLK_TCK));
        const double cores = double(std::max(1, QThread::idealThreadCount()));
        m_cpu = std::max(0.0, (ticks - m_lastTicks) / hz / seconds / cores * 100.0);
        if (engines.isEmpty()) {
            m_gpu = -1;
        } else {
            double busiest = 0;
            for (auto it = engines.cbegin(); it != engines.cend(); ++it) {
                const qint64 before = m_lastEngines.value(it.key(), it.value());
                busiest = std::max(busiest, (it.value() - before) / 1e9 / seconds * 100.0);
            }
            m_gpu = std::min(100.0, busiest);
        }
    }
    m_lastTicks = ticks;
    m_lastEngines = engines;
    m_ram = kb / 1024.0;
    m_free = availableKb(slurp(QStringLiteral("/proc/meminfo"))) / 1024.0;
    emit changed();
}
