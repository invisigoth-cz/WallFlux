// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QDBusConnection>
#include <QObject>
#include <QQmlEngine>
#include <QTemporaryFile>

/// Says whether an ordinary window is covering the screen.
///
/// A wallpaper nobody can see has no business making a noise, and that is
/// what this notices: a browser, a game or a file manager pulled up to fill
/// the screen.
///
/// Wayland tells a window nothing about the ones in front of it, so the
/// answer has to come from the compositor. On KWin a script is loaded into
/// it over D-Bus and calls back here. Elsewhere this reports an uncovered
/// screen for ever and nothing else goes wrong.
class ScreenCover : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(ScreenCover)
    QML_SINGLETON
    // Exported under a name of our own rather than the one Qt derives from
    // the class, because the script on the other side has to spell it out.
    Q_CLASSINFO("D-Bus Interface", "cz.invisigoth.WallFlux.ScreenCover")

    Q_PROPERTY(bool covered READ isCovered NOTIFY coveredChanged)

public:
    explicit ScreenCover(QObject *parent = nullptr);
    ~ScreenCover() override;

    bool isCovered() const { return m_covered; }

public slots:
    /// Called by the KWin script over D-Bus. Not meant for QML, which reads
    /// `covered` instead.
    void setCovered(bool covered);

    /// Called by KWin itself, which announces this one.
    ///
    /// "Show desktop" puts every window out of the way without minimising
    /// any of them or moving it an inch, so the script sees no change and
    /// goes on reporting a covered screen -- while the wallpaper is in fact
    /// the thing somebody asked to look at. Measured: both the state and
    /// the geometry of a maximised window stay exactly as they were.
    void setShowingDesktop(bool showing);

signals:
    void coveredChanged();

private:
    void loadIntoKWin();
    void watchShowDesktop(QDBusConnection &bus);
    /// Works out what QML is told, and says nothing where that has not
    /// changed: handing the players a change that is not one stops them.
    void settle();

    /// KWin loads a script from a path, so the script has to be a real file.
    /// It lives as long as this object does.
    QTemporaryFile m_script;
    /// What the two sources say, and the answer the two of them make.
    bool m_windowCovers = false;
    bool m_showingDesktop = false;
    bool m_covered = false;
};
