// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

#pragma once

#include <QObject>
#include <QQmlEngine>
#include <QTranslator>

/// Which language the interface is in.
///
/// English is what the interface is written in, so it needs no catalogue of
/// its own: switching back is this translator coming off again.
///
/// Starts in English rather than following the desktop's own language. That
/// is a deliberate default and not an oversight -- it is one line here if it
/// should follow the system instead.
class Language : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Language)
    QML_SINGLETON

    Q_PROPERTY(bool czech READ isCzech WRITE setCzech NOTIFY czechChanged)

public:
    explicit Language(QObject *parent = nullptr);

    bool isCzech() const { return m_czech; }
    void setCzech(bool czech);

signals:
    void czechChanged();

private:
    QTranslator m_czechTranslator;
    bool m_czech = false;
};
