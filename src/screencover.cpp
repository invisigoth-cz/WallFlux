// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#include "screencover.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDir>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcCover, "wallflux.cover")

namespace {

constexpr auto ObjectPath = "/ScreenCover";
/// What KWin files the script under. Ours alone, and the same every run.
constexpr auto PluginName = "wallflux-screencover";

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
             QLatin1StringView("cz.invisigoth.WallFlux.ScreenCover"));
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

    m_script.setFileTemplate(QDir::tempPath() + QStringLiteral("/wallflux-cover-XXXXXX.js"));
    if (!m_script.open()) {
        qCWarning(lcCover) << "cannot write the compositor script:" << m_script.errorString();
        return;
    }
    m_script.write(scriptFor(bus.baseService()).toUtf8());
    // KWin reads the file when it is asked to load it, so what is written has
    // to be on disk by then rather than sitting in a buffer. The file stays
    // open so it is not swept away under the script that is running from it.
    m_script.flush();

    loadIntoKWin();
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

void ScreenCover::loadIntoKWin()
{
    QDBusInterface scripting(QStringLiteral("org.kde.KWin"), QStringLiteral("/Scripting"),
                             QStringLiteral("org.kde.kwin.Scripting"),
                             QDBusConnection::sessionBus());
    if (!scripting.isValid()) {
        qCInfo(lcCover) << "no KWin to ask, so nothing will be said about windows covering the"
                        << "wallpaper (on wlroots compositors this is expected)";
        return;
    }

    // Unloaded first, every time. Asked to load a script it already has under
    // this name, KWin hands back the one that is loaded and reads nothing --
    // and after a restart of this program that would be a script still
    // calling the bus name of the copy that is gone.
    scripting.call(QStringLiteral("unloadScript"), QLatin1StringView(PluginName));

    const QDBusReply<int> loaded = scripting.call(QStringLiteral("loadScript"),
                                                  m_script.fileName(),
                                                  QLatin1StringView(PluginName));
    if (!loaded.isValid()) {
        qCWarning(lcCover) << "KWin would not take the script:" << loaded.error().message();
        return;
    }

    // ponytail: the script is not reloaded if KWin restarts. On Wayland a
    // KWin that goes away takes the session with it, so there is nothing to
    // come back to. Watch the service here if that ever stops being true.
    const QDBusReply<void> started = scripting.call(QStringLiteral("start"));
    if (!started.isValid())
        qCWarning(lcCover) << "KWin would not start the script:" << started.error().message();
    else
        qCInfo(lcCover) << "watching for windows that cover the wallpaper";
}

ScreenCover::~ScreenCover()
{
    QDBusInterface scripting(QStringLiteral("org.kde.KWin"), QStringLiteral("/Scripting"),
                             QStringLiteral("org.kde.kwin.Scripting"),
                             QDBusConnection::sessionBus());
    if (scripting.isValid())
        scripting.call(QStringLiteral("unloadScript"), QLatin1StringView(PluginName));
}

void ScreenCover::setCovered(bool covered)
{
    m_windowCovers = covered;
    settle();
}

void ScreenCover::setShowingDesktop(bool showing)
{
    m_showingDesktop = showing;
    settle();
}

void ScreenCover::settle()
{
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
