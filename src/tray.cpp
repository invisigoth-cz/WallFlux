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
    m_items = items;
    // clear() takes the actions and leaves the submenus they opened, which
    // are children of the menu: those go too, or every change keeps one.
    m_menu.clear();
    qDeleteAll(m_menu.findChildren<QMenu *>(Qt::FindDirectChildrenOnly));
    fill(&m_menu, items);
    emit itemsChanged();
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

        if (one.contains(QStringLiteral("submenu"))) {
            QMenu *below = menu->addMenu(icon, text);
            below->setEnabled(enabled);
            fill(below, one.value(QStringLiteral("submenu")).toList());
            continue;
        }
        QAction *action = menu->addAction(icon, text);
        action->setEnabled(enabled);
        const QString id = one.value(QStringLiteral("id")).toString();
        connect(action, &QAction::triggered, this, [this, id] { emit triggered(id); });
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
