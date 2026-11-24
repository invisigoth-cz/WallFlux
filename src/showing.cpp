// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "showing.h"

Showing::Showing(QObject *parent)
    : QObject(parent)
{
}

void Showing::setLibrary(WallpaperCore *library)
{
    if (library == m_library)
        return;
    if (m_library)
        disconnect(m_library, nullptr, this, nullptr);
    m_library = library;
    if (m_library)
        connect(m_library, &WallpaperCore::wallpapersChanged, this, &Showing::reread);
    emit libraryChanged();
}

bool Showing::select(const QString &id)
{
    if (!m_library)
        return false;
    const Wallpaper found = m_library->find(id);
    if (found.id.isEmpty())
        return false;

    // Saying nothing when nothing has changed is not an optimisation here,
    // it is the difference between a wallpaper that plays and one that does
    // not. Clicking the wallpaper that is already playing used to report a
    // change; QML then handed the video player the same file again, which
    // stops it and reports no change of its own, so nothing was left to
    // start it. The screen went black until something else was chosen.
    //
    // The same wallpaper whose file has moved is a change, and is
    // announced as one.
    if (m_current.id == found.id && m_current.source == found.source)
        return true;

    m_current = found;
    emit currentChanged();
    m_library->played(found.id);
    return true;
}

void Showing::clear()
{
    if (m_current.id.isEmpty())
        return;
    m_current = Wallpaper{};
    emit currentChanged();
}

void Showing::reread()
{
    if (m_current.id.isEmpty())
        return;
    const Wallpaper found = m_library->find(m_current.id);
    if (found.id.isEmpty()) {
        clear();
        return;
    }
    if (found.source != m_current.source) {
        select(found.id);
        return;
    }
    // What ffprobe has learnt since -- its size, whether it has a sound --
    // taken in quietly: announcing it would reload a player that is playing.
    m_current = found;
}
