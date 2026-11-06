// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "mirror.h"

#include <QCoreApplication>
#include <QMutex>
#include <QOpenGLContext>
#include <QOpenGLExtraFunctions>
#include <QQuickWindow>
#include <QSGSimpleTextureNode>
#include <QSGTextureProvider>

/// The source's last finished picture, handed from its render thread to the
/// mirror's.
struct Mirror::Frame
{
    QMutex lock;
    GLuint texture = 0;
    QSize size;
    /// Set after the source drew it; the mirror's context waits on it.
    GLsync fence = nullptr;
};

Mirror::Mirror(QQuickItem *parent)
    : QQuickItem(parent)
    , m_frame(std::make_shared<Frame>())
{
    setFlag(ItemHasContents);
}

Mirror::~Mirror()
{
    forget();
}

void Mirror::setSource(QQuickItem *source)
{
    if (m_source == source)
        return;
    m_source = source;
    emit sourceChanged();
    follow();
}

void Mirror::itemChange(ItemChange change, const ItemChangeData &value)
{
    if (change == ItemSceneChange)
        follow();
    QQuickItem::itemChange(change, value);
}

void Mirror::forget()
{
    disconnect(m_sourceWindow);
    disconnect(m_drawn);
    disconnect(m_drawing);
}

void Mirror::follow()
{
    forget();
    {
        QMutexLocker locked(&m_frame->lock);
        m_frame->texture = 0;
    }
    update();
    if (!m_source)
        return;

    // The source may not be in its window yet when it is handed over.
    m_sourceWindow = connect(m_source, &QQuickItem::windowChanged, this, &Mirror::follow);
    QQuickWindow *from = m_source->window();
    QQuickWindow *to = window();
    if (!from || !to)
        return;

    // On the source's render thread, once its frame is drawn.
    m_drawn = connect(
        from, &QQuickWindow::afterRendering, from,
        [frame = m_frame, source = QPointer(m_source), me = QPointer(this)] {
            QSGTextureProvider *provider = source ? source->textureProvider() : nullptr;
            QSGTexture *texture = provider ? provider->texture() : nullptr;
            auto *native = texture ? texture->nativeInterface<QNativeInterface::QSGOpenGLTexture>()
                                   : nullptr;
            QOpenGLContext *context = QOpenGLContext::currentContext();
            if (!native || !context)
                return;
            QOpenGLExtraFunctions *gl = context->extraFunctions();
            GLsync fence = gl->glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
            gl->glFlush();
            {
                QMutexLocker locked(&frame->lock);
                if (frame->fence)
                    gl->glDeleteSync(frame->fence);
                frame->fence = fence;
                frame->texture = native->nativeTexture();
                frame->size = texture->textureSize();
            }
            QMetaObject::invokeMethod(
                qApp, [me] { if (me) me->update(); }, Qt::QueuedConnection);
        },
        Qt::DirectConnection);

    // On the mirror's render thread, before it samples the picture: the
    // source's commands have to have finished first. A wait on the server,
    // inside the lock so that the fence cannot be deleted under it.
    m_drawing = connect(
        to, &QQuickWindow::beforeRendering, to,
        [frame = m_frame] {
            QOpenGLContext *context = QOpenGLContext::currentContext();
            QMutexLocker locked(&frame->lock);
            if (frame->fence && context)
                context->extraFunctions()->glWaitSync(frame->fence, 0, GL_TIMEOUT_IGNORED);
        },
        Qt::DirectConnection);
}

QSGNode *Mirror::updatePaintNode(QSGNode *old, UpdatePaintNodeData *)
{
    auto *node = static_cast<QSGSimpleTextureNode *>(old);
    GLuint id;
    QSize size;
    {
        QMutexLocker locked(&m_frame->lock);
        id = m_frame->texture;
        size = m_frame->size;
    }
    if (!id || size.isEmpty() || width() <= 0 || height() <= 0) {
        delete node;
        return nullptr;
    }

    if (!node) {
        node = new QSGSimpleTextureNode;
        node->setOwnsTexture(true);
    }
    // ponytail: the source draws its next frame into the same texture while
    // this one may still be reading it. At fifteen frames a second and one
    // quad to draw here, a torn frame has not been seen; a copy per frame
    // would close it if one is.
    QSGTexture *shown = node->texture();
    auto *native = shown ? shown->nativeInterface<QNativeInterface::QSGOpenGLTexture>() : nullptr;
    if (!native || native->nativeTexture() != id || shown->textureSize() != size)
        node->setTexture(QNativeInterface::QSGOpenGLTexture::fromNative(
            id, window(), size, QQuickWindow::TextureHasAlphaChannel));

    // Cropped to fill, as a wallpaper is on a screen of another shape.
    const qreal wide = width() / height();
    QSizeF kept(size);
    if (kept.width() / kept.height() > wide)
        kept.setWidth(kept.height() * wide);
    else
        kept.setHeight(kept.width() / wide);
    node->setSourceRect(QRectF(QPointF((size.width() - kept.width()) / 2,
                                       (size.height() - kept.height()) / 2),
                               kept));
    node->setRect(boundingRect());
    node->markDirty(QSGNode::DirtyMaterial);
    return node;
}
