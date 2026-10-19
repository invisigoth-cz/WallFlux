// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "language.h"

#include <QCoreApplication>
#include <QLoggingCategory>
#include <QQmlEngine>

Q_LOGGING_CATEGORY(lcLanguage, "wallflux.language")

namespace {
/// Where qt_add_translations puts the compiled catalogue inside the binary.
constexpr auto CzechCatalogue = ":/i18n/wallflux_cs.qm";
}

Language::Language(QObject *parent)
    : QObject(parent)
{
    if (!m_czechTranslator.load(QLatin1StringView(CzechCatalogue))) {
        qCWarning(lcLanguage) << "no Czech in this build, so the language switch will"
                              << "change nothing";
    }
}

void Language::setCzech(bool czech)
{
    // Silent when the answer has not changed, as everything here is: a
    // retranslate throws away and rebuilds every binding in the interface.
    if (czech == m_czech)
        return;

    if (czech)
        QCoreApplication::installTranslator(&m_czechTranslator);
    else
        QCoreApplication::removeTranslator(&m_czechTranslator);

    m_czech = czech;

    // Every qsTr() in the QML is a binding, and this is what runs them all
    // again. Without it the language changes for whatever is built next and
    // for nothing that is already on screen.
    if (QQmlEngine *engine = qmlEngine(this))
        engine->retranslate();

    emit czechChanged();
}
