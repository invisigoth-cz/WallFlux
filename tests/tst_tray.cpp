// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "tray.h"

#include <QAction>
#include <QMenu>
#include <QSignalSpy>
#include <QTest>

/// The tray's menu is read by the panel over D-Bus, item by item. Throwing
/// it away and building it again while the panel has it open crashed
/// plasmashell (24 November, QMenu::timerEvent on a submenu that was gone),
/// and with a menu per monitor it changed at every turn of a playlist. So a
/// change of words keeps every item, and a change of shape waits for the
/// menu to close.
class TestTray : public QObject
{
    Q_OBJECT

    static QVariantList monitorMenu(const QString &title, const QStringList &recent)
    {
        QVariantList recentItems;
        for (const QString &id : recent)
            recentItems << QVariantMap{{QStringLiteral("id"), QStringLiteral("recent@eDP-1:") + id},
                                       {QStringLiteral("text"), id}};
        return {
            QVariantMap{{QStringLiteral("id"), QStringLiteral("choose")},
                        {QStringLiteral("text"), QStringLiteral("Choose")}},
            QVariantMap{{QStringLiteral("separator"), true}},
            QVariantMap{{QStringLiteral("text"), QStringLiteral("Monitor 1 — ") + title},
                        {QStringLiteral("submenu"), QVariantList{
                            QVariantMap{{QStringLiteral("id"), QStringLiteral("pause@eDP-1")},
                                        {QStringLiteral("text"), QStringLiteral("Pause")}},
                            QVariantMap{{QStringLiteral("text"), QStringLiteral("Recent")},
                                        {QStringLiteral("submenu"), recentItems}}}}},
            QVariantMap{{QStringLiteral("id"), QStringLiteral("quit")},
                        {QStringLiteral("text"), QStringLiteral("Quit")}},
        };
    }

private slots:
    /// New words in the same shape: the same actions and submenus, renamed.
    void changesWordsWithoutRebuilding()
    {
        Tray tray;
        tray.setItems(monitorMenu(QStringLiteral("Dark Core"), {QStringLiteral("a"), QStringLiteral("b")}));
        const QList<QAction *> before = tray.menu()->actions();
        QMenu *monitor = before.at(2)->menu();
        QVERIFY(monitor);
        QAction *recentFirst = monitor->actions().at(1)->menu()->actions().at(0);

        tray.setItems(monitorMenu(QStringLiteral("DOOM"), {QStringLiteral("c"), QStringLiteral("a")}));
        QCOMPARE(tray.menu()->actions(), before);
        QCOMPARE(before.at(2)->menu(), monitor);
        QCOMPARE(monitor->title(), QStringLiteral("Monitor 1 — DOOM"));
        QCOMPARE(monitor->actions().at(1)->menu()->actions().at(0), recentFirst);
        QCOMPARE(recentFirst->text(), QStringLiteral("c"));

        // And the renamed item says its new id when chosen.
        QSignalSpy chosen(&tray, &Tray::triggered);
        recentFirst->trigger();
        QCOMPARE(chosen.takeFirst().at(0).toString(), QStringLiteral("recent@eDP-1:c"));
    }

    /// Another shape is built afresh -- a moment later, never at once: the
    /// panel may still be closing the menu the change was chosen in.
    void reshapesAMomentLater()
    {
        Tray tray;
        tray.setItems(monitorMenu(QStringLiteral("Dark Core"), {QStringLiteral("a")}));
        QMenu *monitor = tray.menu()->actions().at(2)->menu();

        tray.setItems(monitorMenu(QStringLiteral("Dark Core"), {QStringLiteral("a"), QStringLiteral("b")}));
        QCOMPARE(tray.menu()->actions().at(2)->menu(), monitor);
        QCOMPARE(monitor->actions().at(1)->menu()->actions().size(), 1);

        // A change of words meanwhile is not lost either.
        tray.setItems(monitorMenu(QStringLiteral("DOOM"), {QStringLiteral("a"), QStringLiteral("b")}));

        QTRY_COMPARE(tray.menu()->actions().at(2)->menu()->actions().at(1)->menu()->actions().size(), 2);
        QCOMPARE(tray.menu()->actions().at(2)->menu()->title(), QStringLiteral("Monitor 1 — DOOM"));
    }

    /// An item hidden and shown again is the same item: the shape holds.
    void hidesWithoutReshaping()
    {
        Tray tray;
        QVariantList items = monitorMenu(QStringLiteral("Dark Core"), {QStringLiteral("a")});
        tray.setItems(items);
        QAction *quit = tray.menu()->actions().at(3);

        QVariantMap last = items.at(3).toMap();
        last.insert(QStringLiteral("visible"), false);
        items[3] = last;
        tray.setItems(items);
        QCOMPARE(tray.menu()->actions().at(3), quit);
        QVERIFY(!quit->isVisible());
    }
};

QTEST_MAIN(TestTray)
#include "tst_tray.moc"
