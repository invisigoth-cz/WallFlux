// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QMenu>
#include <QObject>
#include <QQmlEngine>
#include <QSystemTrayIcon>
#include <QTimer>
#include <QVariantList>

/// The icon in the panel and its menu, built with Qt Widgets.
///
/// Qt.labs.platform used to draw it, and hands the panel a submenu as a
/// bare title with nothing under it -- measured over D-Bus, a static one
/// too -- so Recent wallpapers never opened. A QMenu's submenu arrives
/// whole. The menu is described from QML as a list and built again
/// whenever the list changes, so its words stay QML's and translate.
class Tray : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(bool visible READ visible WRITE setVisible NOTIFY visibleChanged)
    Q_PROPERTY(QString tooltip READ tooltip WRITE setTooltip NOTIFY tooltipChanged)
    /// The icon as SVG text; empty for the installed one.
    Q_PROPERTY(QString svg READ svg WRITE setSvg NOTIFY svgChanged)
    /// Each `{ id, text, icon, enabled, visible, separator, submenu }`: `icon`
    /// a theme name, `enabled` and `visible` true unless said, `submenu` a
    /// list of the same. Hide an item rather than leave it out where it comes
    /// and goes: a menu of the same shape is changed in place.
    Q_PROPERTY(QVariantList items READ items WRITE setItems NOTIFY itemsChanged)

public:
    explicit Tray(QObject *parent = nullptr);
    ~Tray() override;

    bool visible() const { return m_icon.isVisible(); }
    void setVisible(bool visible);
    QString tooltip() const { return m_icon.toolTip(); }
    void setTooltip(const QString &tooltip);
    QString svg() const { return m_svg; }
    void setSvg(const QString &svg);
    QVariantList items() const { return m_items; }
    void setItems(const QVariantList &items);

    /// The menu the panel is handed, for a test to read.
    QMenu *menu() { return &m_menu; }

    /// The picture SVG text makes, at the sizes a panel asks for.
    static QIcon drawn(const QString &svg);

signals:
    void visibleChanged();
    void tooltipChanged();
    void svgChanged();
    void itemsChanged();
    /// A click on the icon itself.
    void clicked();
    /// A menu item chosen, by its id.
    void triggered(const QString &id);

private:
    void fill(QMenu *menu, const QVariantList &items);
    /// Puts new words, icons and ids on the items already there, for a list
    /// of the same shape. Nothing is taken away, so the panel reading the
    /// menu over D-Bus has nothing pulled out from under it.
    void refresh(QMenu *menu, const QVariantList &items);
    /// Throws the menu away and builds it again, for a list of another shape.
    void rebuild();
    static bool sameShape(const QVariantList &a, const QVariantList &b);

    QSystemTrayIcon m_icon;
    QMenu m_menu;
    QString m_svg;
    QVariantList m_items;
    /// A change of shape, made a moment later; see `setItems`.
    QTimer m_reshape;
};
