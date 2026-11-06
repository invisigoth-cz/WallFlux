// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#pragma once

#include <QMetaObject>
#include <QPointer>
#include <QQmlEngine>
#include <QQuickItem>

#include <memory>

/// What one screen draws, shown again on another: Wallpaper Engine's "Clone
/// single wallpaper". The wallpaper is worked out once, on the first screen,
/// and every other screen shows that picture -- not a second simulation, a
/// second video decoder and a second web page, each costing as much as the
/// first and none of them in step with it.
///
/// `source` is an item in another window with `layer.enabled`, so that it is
/// a texture. The windows draw on threads of their own, with contexts that
/// share their textures (QtWebEngine asks for that, and so does this): after
/// each frame of the source a fence is set, and the mirror's window waits on
/// it before drawing. The picture is cropped to fill, as a wallpaper is.
class Mirror : public QQuickItem
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QQuickItem *source READ source WRITE setSource NOTIFY sourceChanged)

public:
    explicit Mirror(QQuickItem *parent = nullptr);
    ~Mirror() override;

    QQuickItem *source() const { return m_source; }
    void setSource(QQuickItem *source);

    struct Frame;

signals:
    void sourceChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *old, UpdatePaintNodeData *) override;
    void itemChange(ItemChange change, const ItemChangeData &value) override;

private:
    void follow();
    void forget();

    QPointer<QQuickItem> m_source;
    std::shared_ptr<Frame> m_frame;
    QMetaObject::Connection m_sourceWindow;
    QMetaObject::Connection m_drawn;
    QMetaObject::Connection m_drawing;
};
