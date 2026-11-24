// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include "kwinscript.h"

#include <QDBusConnection>
#include <QObject>
#include <QQmlEngine>
#include <QStringList>

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
class ScreenCover : public QObject, protected QDBusContext
{
    Q_OBJECT
    QML_NAMED_ELEMENT(ScreenCover)
    QML_SINGLETON
    // Exported under a name of our own rather than the one Qt derives from
    // the class, because the script on the other side has to spell it out.
    Q_CLASSINFO("D-Bus Interface", "cz.invisigoth.WallFlux.ScreenCover")

    /// The screens a window is in front of, by connector name -- none while
    /// the desktop is being shown. Each monitor's wallpaper reads its own
    /// name in it; which screens a wallpaper is on is QML's to know.
    Q_PROPERTY(QStringList coveredScreens READ coveredScreens NOTIFY coveredScreensChanged)
    /// Whether another program's window has the keyboard, however small it
    /// is -- Wallpaper Engine's "other application focused".
    Q_PROPERTY(bool focused READ isFocused NOTIFY focusedChanged)
    /// The screen that window is on, by connector name, or empty -- so that
    /// only that monitor's wallpaper need stand still, as Wallpaper Engine's
    /// "pause per monitor". Empty too while the desktop is shown.
    Q_PROPERTY(QString focusedScreen READ focusedScreen NOTIFY focusedScreenChanged)
    /// Whether KWin is showing the desktop -- "Peek at Desktop" in the panel,
    /// or the Show Desktop shortcut.
    Q_PROPERTY(bool showingDesktop READ isShowingDesktop NOTIFY showingDesktopChanged)

public:
    explicit ScreenCover(QObject *parent = nullptr);
    ~ScreenCover() override;

    QStringList coveredScreens() const { return m_covered; }
    bool isFocused() const { return m_focused; }
    QString focusedScreen() const { return m_focusedScreen; }
    bool isShowingDesktop() const { return m_showingDesktop; }

    /// Minimises this program's own windows, or brings them back, through
    /// KWin. Peeking at the desktop leaves them where they are: KWin keeps
    /// a window that belongs to the same program as a desktop, and the
    /// wallpaper's surface is one to it. And a Wayland program can minimise
    /// itself but not bring itself back, so KWin does both.
    Q_INVOKABLE void tuckAway(bool away);

public slots:
    /// Called by the KWin script over D-Bus with the names of the screens a
    /// window covers, joined by commas. Not meant for QML, which reads
    /// `coveredScreens` instead.
    void setCoveredScreens(const QString &names);
    /// Called by the script too, when the window with the keyboard changes.
    void setFocused(bool focused);
    /// Called by the script too, with the screen the window with the
    /// keyboard is on, or nothing where no other program has it.
    void setFocusedScreen(const QString &name);

    /// Called by KWin itself, which announces this one.
    ///
    /// "Show desktop" puts every window out of the way without minimising
    /// any of them or moving it an inch, so the script sees no change and
    /// goes on reporting a covered screen -- while the wallpaper is in fact
    /// the thing somebody asked to look at. Measured: both the state and
    /// the geometry of a maximised window stay exactly as they were.
    void setShowingDesktop(bool showing);

signals:
    void coveredScreensChanged();
    void focusedChanged();
    void focusedScreenChanged();
    void showingDesktopChanged();

private:
    void watchShowDesktop(QDBusConnection &bus);
    /// Works out what QML is told, and says nothing where that has not
    /// changed: handing the players a change that is not one stops them.
    void settle();
    void settleFocus();

    /// The script that watches the windows, for as long as this lives.
    KWinScript m_script{QStringLiteral("wallflux-screencover")};
    KWinScript m_tuck{QStringLiteral("wallflux-tuck")};
    /// What the two sources say, and the answer the two of them make.
    QStringList m_coveredScreens;
    bool m_showingDesktop = false;
    QStringList m_covered;
    bool m_otherHasFocus = false;
    bool m_focused = false;
    QString m_otherScreen;
    QString m_focusedScreen;
};
