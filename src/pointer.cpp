// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "pointer.h"

#include <QDBusConnection>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcPointer, "wallflux.pointer")

namespace {

constexpr auto ObjectPath = "/Pointer";

/// The script KWin runs while the mouse is watched. `workspace.cursorPos`
/// and its signal were measured on KWin 6 before this was written, and
/// `callDBus` was measured sending the two numbers as int32.
QString scriptFor(const QString &busName)
{
    return QStringLiteral(R"JS(
var wallflux = "%1";
var path = "%2";
var iface = "%3";
var last = 0;

// A mouse reports hundreds of times a second and a frame is a sixtieth.
// ponytail: the last move inside a sixtieth is not sent, so the scene can
// rest a few pixels short of where the mouse stopped. A trailing send needs
// a timer, which KWin's scripts were not looked at for.
function tell() {
    var now = Date.now();
    if (now - last < 16)
        return;
    last = now;
    var at = workspace.cursorPos;
    callDBus(wallflux, path, iface, "setPosition", at.x, at.y);
}

workspace.cursorPosChanged.connect(tell);
// Where it is before it moves.
tell();
)JS")
        .arg(busName, QLatin1StringView(ObjectPath),
             QLatin1StringView("cz.invisigoth.WallFlux.Pointer"));
}

} // namespace

Pointer::Pointer(QObject *parent)
    : QObject(parent)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    m_registered = bus.isConnected()
                   && bus.registerObject(QLatin1StringView(ObjectPath), this,
                                         QDBusConnection::ExportAllSlots);
    if (!m_registered)
        qCWarning(lcPointer) << "cannot answer on the session bus, so no scene will follow the"
                             << "mouse";
}

void Pointer::setWatching(bool watching)
{
    if (m_watching == watching)
        return;
    m_watching = watching;
    emit watchingChanged();

    if (watching && m_registered) {
        if (m_script.load(scriptFor(QDBusConnection::sessionBus().baseService())))
            qCInfo(lcPointer) << "following the mouse";
    } else if (!watching) {
        m_script.unload();
        // What it said last is no longer where the mouse is.
        if (m_known) {
            m_known = false;
            emit positionChanged();
        }
    }
}

void Pointer::setPosition(int x, int y)
{
    if (!fromKWin(*this))
        return;
    // A call already on the bus when the script was unloaded.
    if (!m_watching)
        return;
    const QPointF at(x, y);
    if (m_known && at == m_position)
        return;
    m_known = true;
    m_position = at;
    emit positionChanged();
}
