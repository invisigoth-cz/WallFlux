// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "screencover.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcCover, "wallflux.cover")

namespace {

constexpr auto ObjectPath = "/ScreenCover";

/// The script KWin runs on our behalf.
///
/// It is built here rather than installed as a file of its own because the
/// address it answers to is this process's own name on the bus, which is
/// only known once the program is running.
///
/// `var` and plain functions throughout: that is what was measured to work
/// in KWin's engine, and a wallpaper is not the place to find out which
/// newer spelling it has since learned.
QString scriptFor(const QString &busName)
{
    return QStringLiteral(R"JS(
var wallflux = "%1";
var path = "%2";
var iface = "%3";
var ours = %4;

// A window covers the screen when it fills the room a maximised window gets
// -- which is the screen minus the panels, not the whole of it. Comparing
// against the full screen instead would miss every maximised window, and
// comparing geometry rather than asking for a maximised flag is deliberate:
// KWin 6 has no such flag to ask for, and this way a window dragged out to
// fill the screen by hand counts too.
function covers(w) {
    if (!w.normalWindow || w.minimized)
        return false;
    if (!w.onAllDesktops && w.desktops.indexOf(workspace.currentDesktop) < 0)
        return false;
    var g = w.frameGeometry;
    var room = workspace.clientArea(KWin.MaximizeArea, w.output, workspace.currentDesktop);
    return g.width >= room.width && g.height >= room.height;
}

var told = null;

function look() {
    var list = workspace.windowList();
    var covered = false;
    for (var i = 0; i < list.length; ++i) {
        if (covers(list[i])) {
            covered = true;
            break;
        }
    }
    // Dragging a window emits a geometry change per frame, and all but the
    // one that crosses the edge say the same thing as the last.
    if (covered === told)
        return;
    told = covered;
    callDBus(wallflux, path, iface, "setCovered", covered);
}

function watch(w) {
    w.frameGeometryChanged.connect(look);
    w.minimizedChanged.connect(look);
    w.fullScreenChanged.connect(look);
    w.maximizedChanged.connect(look);
    w.desktopsChanged.connect(look);
    look();
}

// Another program has the keyboard: an ordinary window that is not one of
// ours. Not the desktop or a panel, which are not normal windows, and not
// the picker, or choosing a wallpaper would stop the one being chosen.
var hadFocus = null;
function focus() {
    var w = workspace.activeWindow;
    var other = !!w && w.normalWindow && w.pid !== ours;
    if (other === hadFocus)
        return;
    hadFocus = other;
    callDBus(wallflux, path, iface, "setFocused", other);
}
workspace.windowActivated.connect(focus);
focus();

workspace.windowAdded.connect(watch);
workspace.windowRemoved.connect(look);
workspace.currentDesktopChanged.connect(look);

var open = workspace.windowList();
for (var i = 0; i < open.length; ++i)
    watch(open[i]);

// What is on the screen before anything moves.
look();
)JS")
        .arg(busName, QLatin1StringView(ObjectPath),
             QLatin1StringView("cz.invisigoth.WallFlux.ScreenCover"),
             QString::number(QCoreApplication::applicationPid()));
}

} // namespace

ScreenCover::ScreenCover(QObject *parent)
    : QObject(parent)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        qCWarning(lcCover) << "no session bus, so the wallpaper will not quieten itself for"
                           << "windows that cover it";
        return;
    }

    // No service name of our own is asked for: the script is handed this
    // connection's unique name, which saves owning a name on the bus and
    // cannot collide with anything.
    if (!bus.registerObject(QLatin1StringView(ObjectPath), this,
                            QDBusConnection::ExportAllSlots)) {
        qCWarning(lcCover) << "cannot answer on the session bus:" << bus.lastError().message();
        return;
    }

    if (m_script.load(scriptFor(bus.baseService())))
        qCInfo(lcCover) << "watching for windows that cover the wallpaper";
    watchShowDesktop(bus);
}

void ScreenCover::watchShowDesktop(QDBusConnection &bus)
{
    if (!bus.connect(QStringLiteral("org.kde.KWin"), QStringLiteral("/KWin"),
                     QStringLiteral("org.kde.KWin"), QStringLiteral("showingDesktopChanged"),
                     this, SLOT(setShowingDesktop(bool)))) {
        qCWarning(lcCover) << "cannot hear KWin announce \"show desktop\", so the wallpaper will"
                           << "stay quiet while somebody is looking at it";
        return;
    }

    // Whatever it is now, before anything is announced.
    QDBusInterface kwin(QStringLiteral("org.kde.KWin"), QStringLiteral("/KWin"),
                        QStringLiteral("org.kde.KWin"), bus);
    const QVariant showing = kwin.property("showingDesktop");
    if (showing.isValid())
        setShowingDesktop(showing.toBool());
}

ScreenCover::~ScreenCover() = default;

void ScreenCover::setCovered(bool covered)
{
    if (!fromKWin(*this))
        return;
    m_windowCovers = covered;
    settle();
}

void ScreenCover::setFocused(bool focused)
{
    if (!fromKWin(*this))
        return;
    m_otherHasFocus = focused;
    settle();
}

void ScreenCover::setShowingDesktop(bool showing)
{
    if (!fromKWin(*this))
        return;
    m_showingDesktop = showing;
    settle();
}

void ScreenCover::settle()
{
    settleFocus();

    // A window in front of the wallpaper only counts while the windows are
    // where they were put.
    const bool covered = m_windowCovers && !m_showingDesktop;

    // Silent when the answer has not changed. The same trap as select() in
    // WallpaperCore: handing QML a change that is not one makes it reload a
    // player that was playing perfectly well.
    if (covered == m_covered)
        return;

    m_covered = covered;
    qCDebug(lcCover) << (covered ? "a window is covering the wallpaper"
                                 : "the wallpaper is in view");
    emit coveredChanged();
}

void ScreenCover::settleFocus()
{
    // "Show desktop" hands the keyboard to the desktop, which is no program.
    const bool focused = m_otherHasFocus && !m_showingDesktop;
    if (focused == m_focused)
        return;
    m_focused = focused;
    qCDebug(lcCover) << (focused ? "another program has the keyboard"
                                 : "no other program has the keyboard");
    emit focusedChanged();
}
