// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "tray.h"

#include <QBuffer>
#include <QImageReader>

Tray::Tray(QObject *parent)
    : QObject(parent)
{
    m_icon.setIcon(drawn({}));
    m_icon.setContextMenu(&m_menu);
    // A menu of another shape is built a moment later rather than at once:
    // plasmashell holds the submenus it made out of ours, and one pulled
    // apart under a menu it was closing -- a profile chosen in it had
    // changed the layout -- crashed it (24 November, QMenu::timerEvent).
    // When it is open or closing cannot be asked: over D-Bus Qt says it is
    // about to show once and never that it hid. So the menu keeps its shape
    // for everything that changes while somebody may be looking at it --
    // see `refresh` -- and what does change its shape waits.
    m_reshape.setSingleShot(true);
    m_reshape.setInterval(1500);
    connect(&m_reshape, &QTimer::timeout, this, &Tray::rebuild);
    connect(&m_icon, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason why) {
        if (why == QSystemTrayIcon::Trigger)
            emit clicked();
    });
}

Tray::~Tray()
{
    // The icon goes before the menu it points at.
    m_icon.setContextMenu(nullptr);
}

void Tray::setVisible(bool visible)
{
    if (visible == m_icon.isVisible())
        return;
    m_icon.setVisible(visible);
    emit visibleChanged();
}

void Tray::setTooltip(const QString &tooltip)
{
    if (tooltip == m_icon.toolTip())
        return;
    m_icon.setToolTip(tooltip);
    emit tooltipChanged();
}

void Tray::setSvg(const QString &svg)
{
    if (svg == m_svg)
        return;
    m_svg = svg;
    m_icon.setIcon(drawn(svg));
    emit svgChanged();
}

void Tray::setItems(const QVariantList &items)
{
    if (items == m_items)
        return;
    // The first one is built at once: there is nothing for the panel to
    // hold yet.
    const bool first = m_items.isEmpty() && m_menu.actions().isEmpty();
    const bool same = !m_reshape.isActive() && sameShape(m_items, items);
    m_items = items;
    if (first)
        rebuild();
    else if (same)
        refresh(&m_menu, items);
    else
        m_reshape.start();
    emit itemsChanged();
}

void Tray::rebuild()
{
    m_reshape.stop();
    // clear() takes the actions and leaves the submenus they opened, which
    // are children of the menu: those go too, or every change keeps one.
    m_menu.clear();
    qDeleteAll(m_menu.findChildren<QMenu *>(Qt::FindDirectChildrenOnly));
    fill(&m_menu, m_items);
}

bool Tray::sameShape(const QVariantList &a, const QVariantList &b)
{
    if (a.size() != b.size())
        return false;
    for (qsizetype i = 0; i < a.size(); ++i) {
        const QVariantMap x = a.at(i).toMap();
        const QVariantMap y = b.at(i).toMap();
        if (x.value(QStringLiteral("separator")).toBool() != y.value(QStringLiteral("separator")).toBool()
            || x.contains(QStringLiteral("submenu")) != y.contains(QStringLiteral("submenu")))
            return false;
        if (x.contains(QStringLiteral("submenu"))
            && !sameShape(x.value(QStringLiteral("submenu")).toList(),
                          y.value(QStringLiteral("submenu")).toList()))
            return false;
    }
    return true;
}

void Tray::refresh(QMenu *menu, const QVariantList &items)
{
    const QList<QAction *> actions = menu->actions();
    for (qsizetype i = 0; i < items.size() && i < actions.size(); ++i) {
        const QVariantMap one = items.at(i).toMap();
        QAction *action = actions.at(i);
        if (one.value(QStringLiteral("separator")).toBool())
            continue;
        const QString text = one.value(QStringLiteral("text")).toString();
        const QIcon icon = QIcon::fromTheme(one.value(QStringLiteral("icon")).toString());
        const bool enabled = one.value(QStringLiteral("enabled"), true).toBool();
        action->setVisible(one.value(QStringLiteral("visible"), true).toBool());
        if (QMenu *below = action->menu()) {
            below->setTitle(text);
            below->setIcon(icon);
            below->setEnabled(enabled);
            refresh(below, one.value(QStringLiteral("submenu")).toList());
            continue;
        }
        if (action->text() != text)
            action->setText(text);
        action->setIcon(icon);
        action->setEnabled(enabled);
        action->setData(one.value(QStringLiteral("id")).toString());
    }
}

void Tray::fill(QMenu *menu, const QVariantList &items)
{
    for (const QVariant &entry : items) {
        const QVariantMap one = entry.toMap();
        if (one.value(QStringLiteral("separator")).toBool()) {
            menu->addSeparator();
            continue;
        }
        const QString text = one.value(QStringLiteral("text")).toString();
        const QIcon icon = QIcon::fromTheme(one.value(QStringLiteral("icon")).toString());
        const bool enabled = one.value(QStringLiteral("enabled"), true).toBool();

        const bool visible = one.value(QStringLiteral("visible"), true).toBool();
        if (one.contains(QStringLiteral("submenu"))) {
            QMenu *below = menu->addMenu(icon, text);
            below->setEnabled(enabled);
            below->menuAction()->setVisible(visible);
            fill(below, one.value(QStringLiteral("submenu")).toList());
            continue;
        }
        QAction *action = menu->addAction(icon, text);
        action->setEnabled(enabled);
        action->setVisible(visible);
        // The id rides on the action, so that `refresh` can change it.
        action->setData(one.value(QStringLiteral("id")).toString());
        connect(action, &QAction::triggered, this, [this, action] { emit triggered(action->data().toString()); });
    }
}

QIcon Tray::drawn(const QString &svg)
{
    if (svg.isEmpty())
        return QIcon::fromTheme(QStringLiteral("wallflux"), QIcon(QStringLiteral(":/icons/wallflux.svg")));
    QIcon icon;
    const QByteArray bytes = svg.toUtf8();
    for (const int side : {16, 22, 24, 32, 48, 64}) {
        QBuffer buffer;
        buffer.setData(bytes);
        QImageReader reader(&buffer, "svg");
        reader.setScaledSize(QSize(side, side));
        const QImage picture = reader.read();
        if (!picture.isNull())
            icon.addPixmap(QPixmap::fromImage(picture));
    }
    return icon.isNull() ? drawn({}) : icon;
}
