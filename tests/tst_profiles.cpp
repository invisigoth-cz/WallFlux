// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

#include "profiles.h"

#include <QSignalSpy>
#include <QStandardPaths>
#include <QTest>

/// What a profile remembers: the whole arrangement of the monitors.
///
/// The configuration directory is redirected into a temporary one, so this
/// never reads or writes the profiles somebody actually has.
class TestProfiles : public QObject
{
    Q_OBJECT

    static QVariantMap twoMonitors()
    {
        return {
            {QStringLiteral("layout"), QStringLiteral("perDisplay")},
            {QStringLiteral("displays"), QVariantMap{
                {QStringLiteral("eDP-1"), QVariantMap{
                    {QStringLiteral("wallpaper"), QStringLiteral("111")},
                    {QStringLiteral("playlist"), QString()},
                    {QStringLiteral("muted"), true},
                    {QStringLiteral("properties"), QVariantMap{{QStringLiteral("colour"), QStringLiteral("1 0 0")}}}}},
                {QStringLiteral("DP-9"), QVariantMap{
                    {QStringLiteral("wallpaper"), QStringLiteral("222")},
                    {QStringLiteral("playlist"), QStringLiteral("evening")},
                    {QStringLiteral("muted"), false}}}}},
        };
    }

private slots:
    void initTestCase() { QStandardPaths::setTestModeEnabled(true); }

    void init()
    {
        Profiles first;
        const QVariantList all = first.all();
        for (const QVariant &one : all)
            first.forget(one.toMap().value(QStringLiteral("name")).toString());
    }

    void remembersAnArrangement()
    {
        Profiles{}.save(QStringLiteral("desk"), twoMonitors());

        const QVariantMap back = Profiles{}.named(QStringLiteral("desk"));
        QCOMPARE(back.value(QStringLiteral("name")).toString(), QStringLiteral("desk"));
        QCOMPARE(back.value(QStringLiteral("layout")).toString(), QStringLiteral("perDisplay"));
        const QVariantMap edp = back.value(QStringLiteral("displays")).toMap()
                                    .value(QStringLiteral("eDP-1")).toMap();
        QCOMPARE(edp.value(QStringLiteral("wallpaper")).toString(), QStringLiteral("111"));
        QCOMPARE(edp.value(QStringLiteral("muted")).toBool(), true);
        QCOMPARE(edp.value(QStringLiteral("properties")).toMap().value(QStringLiteral("colour")).toString(),
                 QStringLiteral("1 0 0"));
    }

    /// A monitor that is not plugged in now is kept as it was: nothing
    /// here knows which monitors are connected.
    void keepsMonitorsThatAreAway()
    {
        Profiles{}.save(QStringLiteral("desk"), twoMonitors());
        const QVariantMap away = Profiles{}.named(QStringLiteral("desk"))
                                     .value(QStringLiteral("displays")).toMap()
                                     .value(QStringLiteral("DP-9")).toMap();
        QCOMPARE(away.value(QStringLiteral("playlist")).toString(), QStringLiteral("evening"));
    }

    void replacesOneOfTheSameName()
    {
        Profiles p;
        p.save(QStringLiteral("desk"), twoMonitors());
        p.save(QStringLiteral("desk"), {{QStringLiteral("layout"), QStringLiteral("clone")}});
        QCOMPARE(p.all().size(), 1);
        QCOMPARE(p.named(QStringLiteral("desk")).value(QStringLiteral("layout")).toString(),
                 QStringLiteral("clone"));
    }

    void renamesWithoutTakingAnotherName()
    {
        Profiles p;
        p.save(QStringLiteral("a"), twoMonitors());
        p.save(QStringLiteral("b"), twoMonitors());

        QVERIFY(!p.rename(QStringLiteral("a"), QStringLiteral("b")));
        QVERIFY(!p.rename(QStringLiteral("a"), QStringLiteral("  ")));
        QVERIFY(!p.rename(QStringLiteral("nobody"), QStringLiteral("c")));
        QVERIFY(p.rename(QStringLiteral("a"), QStringLiteral(" c ")));
        QVERIFY(p.named(QStringLiteral("a")).isEmpty());
        QCOMPARE(Profiles{}.named(QStringLiteral("c")).value(QStringLiteral("layout")).toString(),
                 QStringLiteral("perDisplay"));
    }

    void refusesNoName()
    {
        Profiles p;
        QSignalSpy changes(&p, &Profiles::allChanged);
        p.save(QStringLiteral("   "), twoMonitors());
        QVERIFY(p.all().isEmpty());
        QCOMPARE(changes.count(), 0);
    }

    void forgets()
    {
        Profiles p;
        p.save(QStringLiteral("desk"), twoMonitors());
        p.forget(QStringLiteral("desk"));
        QVERIFY(Profiles{}.all().isEmpty());
    }
};

QTEST_MAIN(TestProfiles)
#include "tst_profiles.moc"
