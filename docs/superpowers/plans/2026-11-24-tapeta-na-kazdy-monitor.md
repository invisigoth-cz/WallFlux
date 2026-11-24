# A Wallpaper on Every Monitor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wallpaper Engine's "Wallpaper per display" beside today's "Clone single wallpaper": each monitor its own wallpaper, playlist, properties, mute and pause, with a tray submenu per monitor and saved profiles.

**Architecture:** What plays today — one wallpaper, one running playlist, pause, transition, scene music — becomes `qml/Display.qml`, one per player. Per-display mode has one `Display` per screen name (kept for the whole run, so an unplugged monitor keeps its state); clone mode has one `Display` drawing on every non-excluded screen, the first drawing and the rest mirroring, which is today's program. Two small C++ types carry the logic a test must reach: `Showing` (one display's wallpaper, quiet when unchanged) and `PlaylistRun` (one display's running playlist), split out of `WallpaperCore` and `Playlists`. `Profiles` is a C++ singleton storing arrangements.

**Tech Stack:** C++20, Qt 6 (Quick, Controls Fusion, QtCore Settings, Multimedia, WebEngine), QtTest, CMake.

**Spec:** `docs/superpowers/specs/2026-11-24-tapeta-na-kazdy-monitor-design.md` (Czech). Read it before any task; it is the authority on behaviour. CLAUDE.md is the authority on traps.

## Global Constraints

- Nothing plays at program start. Profiles play only when loaded by the user. What plays on which monitor is not remembered between runs.
- Remembered (QtCore `Settings` in `Picker.qml`'s `remembered`): layout (`"perDisplay"` default, or `"clone"`), per-monitor mute (by connector name), clone exclusions (connector names), last selected monitor, `chooserOnOpen` (default true), per-monitor properties.
- One monitor connected, or layout `"clone"`: the library, footer and tray look exactly as today — no "Monitor 1", no chooser, no per-monitor submenus.
- Clone must keep today's behaviour exactly: one wallpaper worked out once, others `Mirror` it, one sound, covered only when all its screens are covered.
- Monitors are identified by `QScreen.name` (connector). Numbered left to right, then top to bottom, by `virtualX`/`virtualY`. Label: `qsTr("Monitor %1 · %2").arg(n).arg(model or name)` where model is `manufacturer + " " + model` trimmed if non-empty.
- Pause and mute each have two independent flags: the monitor's own and "all". Resuming/unmuting all never clears a monitor's own flag.
- No checkable menu items in the tray; words say the state. Never `pkill -x wallflux`. On the laptop build with `-j2`.
- Every user-visible string through `qsTr`, Czech added to `i18n/wallflux_cs.ts` (`cmake --build build --target update_translations`, then fill the new entries). No translated model on a ComboBox (trap in CLAUDE.md).
- Checks for every task: `cmake --build build && ctest --test-dir build --output-on-failure` and `/usr/lib/qt6/bin/qmllint -I build/ qml/*.qml` (one expected `Look` warning). Commit messages say why, end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A monitor unplugged while its playlist runs and plugged back — the playlist must be held while away and carry on, not fire `plays` into a display with no surface. (Task 5: `holdingThePlaylist` includes "no screen connected".)
2. Choosing the wallpaper a display already shows must not reload the player (the `select()` trap, now per display). (Task 3 test `staysQuietWhenTheChoiceIsTheSame`, moved.)
3. A playlist renamed or deleted while it runs on two monitors at once. (Task 2 tests `renameReachesEveryRun`, `forgettingStopsEveryRun`.)
4. Switching layout while a transition is running on a display — the old display's `rescue` timer must not fire into a destroyed object. (Task 6: switching layout calls `swapNow()` on displays with `arriving !== ""` first.)
5. A profile naming a monitor that is not connected, and later connected. (Task 4 test `keepsMonitorsThatAreAway`; Task 8 applies it when the display is created.)

---

### Task 1: ScreenCover says which screens are covered

**Files:**
- Modify: `src/screencover.h`, `src/screencover.cpp`
- Test: `tests/tst_screencover.cpp`

**Interfaces:**
- Produces: `ScreenCover.coveredScreens` (`QStringList`, NOTIFY `coveredScreensChanged`) — names of screens a window covers, **empty while the desktop is shown**. `shownOn` and `covered` are removed (no caller is left after Task 5; Task 5 computes coverage per display).

- [ ] **Step 1: Write failing test** `listsTheCoveredScreensAndNoneWhileTheDesktopIsShown` in `tst_screencover.cpp`, using the file's existing way of calling the slots as if from KWin: after `setCoveredScreens("eDP-1,HDMI-A-1")`, `coveredScreens() == {"eDP-1","HDMI-A-1"}` and the spy on `coveredScreensChanged` counted 1; the same call again counts no more; `setShowingDesktop(true)` makes it `{}`; `setShowingDesktop(false)` brings both back.
- [ ] **Step 2: Run** `ctest --test-dir build -R screencover --output-on-failure` — FAIL (no member).
- [ ] **Step 3: Implement** the property in `settle()` (compute `m_showingDesktop ? QStringList{} : m_coveredScreens`, emit only on change); remove `shownOn`, `setShownOn`, `covered`, `m_covered`, `m_shownOn` and the tests that used them; keep `focused`, `showingDesktop`, `tuckAway`.
- [ ] **Step 4: Run** the test — PASS. `Main.qml` still binds `shownOn`; replace that `Binding` and `ScreenCover.covered` in `reasons` with a temporary `ScreenCover.coveredScreens.length > 0 && picker.screensWanted.every(n => ScreenCover.coveredScreens.indexOf(n) >= 0)` so the program behaves as before until Task 5.
- [ ] **Step 5: Commit** "ScreenCover names the covered screens, so each monitor can pause alone".

### Task 2: A running playlist is its own object

**Files:**
- Create: `src/playlistrun.h`, `src/playlistrun.cpp`
- Modify: `src/playlists.h`, `src/playlists.cpp`, `CMakeLists.txt` (module sources; `tst_playlists` sources), `qml/Main.qml`, `qml/Picker.qml`, `qml/WallpaperSurface.qml`
- Test: `tests/tst_playlists.cpp`

**Interfaces:**
- Produces `PlaylistRun` (`QML_ELEMENT`, not a singleton), moved verbatim from `Playlists` with these changes:
  - `Q_PROPERTY(QString playing …)`, `changesOnVideoEnd`, `changesWhilePaused`, `transition`, `transitionMs` (NOTIFY `playingChanged`); signal `plays(const QString &id)`.
  - `Q_INVOKABLE void start(const QVariantMap &one)` — takes the playlist itself (`Playlists.named(name)`), ignores an empty map or empty `ids` with a warning.
  - `stop()`, `next()`, `previous()`, `hold(bool)` unchanged.
  - `Q_INVOKABLE void renamed(const QString &from, const QString &to)` — if `playing == from`, `playing = to`, emit.
  - `Q_INVOKABLE void forgotten(const QString &name)` — if `playing == name`, `stop()`.
- `Playlists` keeps `all`, `save`, `forget`, `rename`, `named`, `staysFor`, `asClock` and the file; loses the run members, `playing` and the run properties; gains signals `renamed(const QString &from, const QString &to)` and `forgotten(const QString &name)`.
- QML wiring (until Task 5): one `PlaylistRun { id: run }` in `Main.qml` with a `Connections { target: Playlists; onRenamed: run.renamed(from, to); onForgotten: run.forgotten(name) }`; every `Playlists.playing/start/stop/next/previous/hold/changes*/transition*` read in `Main.qml`, `Picker.qml`, `WallpaperSurface.qml` goes to `run` (Picker gets `required property PlaylistRun run` or a plain `property var run` set by Main; `start(name)` becomes `run.start(Playlists.named(name))`).

- [ ] **Step 1: Move the run tests** (`addsTheThreeUpAndKeepsAFloor`, `alwaysFirstSurvivesTheShuffle`, `shufflingWithoutItMovesTheFirstOne`, `holdingKeepsWhatWasLeft`, `nextMovesOnAndRewindsTheClock`, `previousGoesBackRound`, `saysWhatTheRunningPlaylistWasTold`) to a `PlaylistRun run; run.start(p.named("…"))` form, keeping their assertions. Rewrite `renamesWithoutTakingAnotherName` to drop the `playing()` check. Add:
  - `renameReachesEveryRun`: two runs started on `"a"`, connected to `Playlists::renamed`; `p.rename("a","c")` → both `playing() == "c"`.
  - `forgettingStopsEveryRun`: two runs on `"a"`; `p.forget("a")` → both `playing().isEmpty()`, each emitted `playingChanged` once.
- [ ] **Step 2: Run** `ctest --test-dir build -R playlists` — FAIL (no `PlaylistRun`).
- [ ] **Step 3: Implement** the split; `forget` emits `forgotten`, `rename` emits `renamed` on success; wire QML as in Interfaces.
- [ ] **Step 4: Run** all tests and qmllint — PASS; start the program, start a playlist from the library and watch it turn (`QT_FORCE_STDERR_LOGGING=1 QT_LOGGING_RULES="wallflux.*=true"`, look for `turning to`).
- [ ] **Step 5: Commit** "A running playlist is its own object, so each monitor can run one".

### Task 3: A display's wallpaper is its own object

**Files:**
- Create: `src/showing.h`, `src/showing.cpp`
- Modify: `src/wallpapercore.h`, `src/wallpapercore.cpp`, `CMakeLists.txt` (module; add `src/showing.cpp` to `tst_wallpapercore`), `qml/Main.qml`, `qml/Picker.qml`, `qml/WallpaperSurface.qml`, `qml/DisplaysView.qml` (comment only)
- Test: `tests/tst_wallpapercore.cpp`

**Interfaces:**
- `WallpaperCore` loses `current`, `select`, `clear` and the reselect in `rescan`; gains
  - `Q_INVOKABLE Wallpaper find(const QString &id) const` (empty `Wallpaper` when absent),
  - `Q_INVOKABLE void played(const QString &id)` (moves `id` to the front of `recent`, emits `recentChanged`; ignores an id not installed).
- Produces `Showing` (`QML_ELEMENT`):
  - `Q_PROPERTY(WallpaperCore *library READ library WRITE setLibrary NOTIFY libraryChanged)` — in QML `library: Wallpapers`.
  - `Q_PROPERTY(Wallpaper current READ current NOTIFY currentChanged)`.
  - `Q_INVOKABLE bool select(const QString &id)` — the body of today's `WallpaperCore::select` (quiet when id and source are unchanged; calls `library->played(id)` when it changes).
  - `Q_INVOKABLE void clear()`.
  - On `library->wallpapersChanged`: reselect the current id, or `clear()` if it is gone (today's rescan rule).
- QML (until Task 5): one `Showing { id: showing; library: Wallpapers }` in `Main.qml`; every `Wallpapers.current` / `Wallpapers.select` / `Wallpapers.clear` in `Main.qml`, `Picker.qml`, `WallpaperSurface.qml` goes to it (Picker and the surfaces are handed it as a property, as `run` in Task 2).

- [ ] **Step 1: Rewrite** `staysQuietWhenTheChoiceIsTheSame` and `remembersWhatWasPlayedInThisRun` against `Showing s; s.setLibrary(&core);` with the same assertions (spy on `Showing::currentChanged`; `core.recent()` order). Add `forgetsAWallpaperThatLeftTheDisk`: select "888", delete its folder, `core.rescan({steam})` → `s.current().id.isEmpty()`.
- [ ] **Step 2: Run** `ctest --test-dir build -R wallpapercore` — FAIL.
- [ ] **Step 3: Implement**; wire QML.
- [ ] **Step 4: Run** all tests, qmllint; play a video, double click the same card again — it keeps playing (the trap).
- [ ] **Step 5: Commit** "What a display shows is its own object, quiet when nothing changes".

### Task 4: Profiles

**Files:**
- Create: `src/profiles.h`, `src/profiles.cpp`, `tests/tst_profiles.cpp`
- Modify: `CMakeLists.txt` (module source, `tst_profiles` target + `add_test(NAME profiles …)` like `tst_playlists`)

**Interfaces:**
- Produces `Profiles` (`QML_NAMED_ELEMENT(Profiles)`, `QML_SINGLETON`), file `profiles.json` beside `playlists.json` (same `QStandardPaths::AppConfigLocation` directory; read Playlists' `wherePlaylistsLive` and do the same):
  - `Q_PROPERTY(QVariantList all READ all NOTIFY allChanged)` — each `{ name, layout, displays }`, `layout` `"perDisplay"`/`"clone"`, `displays` a map from connector name to `{ wallpaper, playlist, muted, properties }` (`properties` the same shape as `picker.propertiesOf` holds for one wallpaper). Clone stores its one display under the key `"clone"` and the exclusions under `excluded` (list of names).
  - `Q_INVOKABLE void save(const QString &name, const QVariantMap &arrangement)` — replaces one of that name; empty name refused.
  - `Q_INVOKABLE void forget(const QString &name)`, `Q_INVOKABLE bool rename(const QString &from, const QString &to)` (refuses empty or taken, as Playlists), `Q_INVOKABLE QVariantMap named(const QString &name) const`.

- [ ] **Step 1: Write tests**: `remembersAnArrangement` (save, new `Profiles`, `named` gives the same `displays` back); `renamesWithoutTakingAnotherName` (as in tst_playlists); `keepsMonitorsThatAreAway` (a profile with `"DP-9"` survives save/load untouched — nothing filters by connected screens); `refusesNoName`.
- [ ] **Step 2: Run** `ctest --test-dir build -R profiles` — FAIL.
- [ ] **Step 3: Implement** (JSON via `QJsonDocument`, written atomically with `QSaveFile`).
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** "Profiles: the whole arrangement of monitors under a name".

### Task 5: Display.qml — clone unchanged, built from one object

Pure restructuring: after it, the program behaves exactly as before.

**Files:**
- Create: `qml/Display.qml`
- Modify: `qml/Main.qml`, `qml/WallpaperSurface.qml`, `CMakeLists.txt` (QML file list)

**Interfaces:**
- Produces `Display` (root `Item`, no visual):
  - `required property var screens` — screen objects it draws on (first draws, others mirror).
  - `property string key` — connector name, or `"clone"`.
  - `readonly property alias showing`, `readonly property alias run` — its `Showing` and `PlaylistRun`.
  - `property bool pausedHere`, `property bool mutedHere`.
  - `property bool pausedAll`, `property bool mutedAll` — bound by Main.
  - `property var coverModes` — Main hands `{ window: picker.underWindow, focus: …, sound: …, asleep: … }`.
  - `readonly property bool connected: screens.length > 0`.
  - `readonly property bool covered` — today's `covered` rules, with the window reason true when `screens` are non-empty and every one of them is in `ScreenCover.coveredScreens` (mode `"pause"`/`"stop"`), or when **any** in-use screen of the whole program is covered (mode `"pauseall"`/`"stopall"`, Main passes `anyCovered`).
  - `readonly property bool silent`, `readonly property bool stopped` — today's rules plus `mutedHere || mutedAll` and `pausedHere || pausedAll`.
  - `function step(by)`, `function changeTo(id)`, `function swapNow()` — moved from Main; `step` reads `picker.shown` through a `property var order` Main binds.
  - Inside: `Showing`, `PlaylistRun` with the `Connections` to `Playlists.renamed/forgotten`, `holdingThePlaylist` (today's, plus `!connected`), the `Instantiator` of `WallpaperSurface` over `screens` (today's delegate, reading `showing.current` and `run`), the `SceneSound` `Repeater`, `arriving`/`fading`/`waiting`/`rescue`.
- `WallpaperSurface` gets `property var wallpaper` and `property PlaylistRun run` instead of reading the globals.
- `Main.qml` (this task): one `Display { key: "clone"; screens: <ticked screens as today> }`; Pointer `watching` = any display showing a scene and not covered; tray and Picker read that display.

- [ ] **Step 1: Baseline** in a virtual KWin with two outputs (CLAUDE.md, "Checked in `kwin_wayland --virtual --output-count 2` under `dbus-run-session`"): play one video and one scene, grab both outputs with `spectacle -b -n -f`, save as `before-*.png`.
- [ ] **Step 2: Implement** `Display.qml` and move the code.
- [ ] **Step 3: Verify**: all tests, qmllint over every file, the same two grabs `after-*.png` — both outputs identical to each other to the pixel for the scene's first frames as before; video plays with sound (`parec` peak > 0 on the default sink monitor); a playlist with Fade turns (`turning to` in the log and a mid-fade grab shows two pictures); Show Desktop shortcut (`invokeShortcut "Show Desktop"`) covers/uncovers.
- [ ] **Step 4: Commit** "What plays is a Display, so there can be one per monitor".

### Task 6: Per-display layout

**Files:**
- Modify: `qml/Main.qml`, `qml/Picker.qml` (settings: `layout`, `excludedFromClone`, `mutedMonitors`; migration), `qml/Display.qml`

**Interfaces:**
- Consumes: `Display` (Task 5), `ScreenCover.coveredScreens` (Task 1).
- Produces on `Main.qml` (`root`), for Tasks 7–9:
  - `readonly property var displays` — the `Display` objects in monitor order (left to right, top to bottom; displays whose monitor is away come last).
  - `function display(key)` → `Display` or null. Picker gets `root` as `property var stage`.
  - `property bool pausedAll`, `property bool mutedAll` (the latter is `picker.muted`, the footer button).
  - `function setLayout(layout)`, `function copyTo(fromKey, toKey)` (wallpaper and its properties), `function swap(a, b)` (wallpaper, playlist with its position restarted, properties, mute), `function removeFrom(key)` (clone: exclude that monitor; the source or the footer's Off: clear the clone), `function applyProfile(map)`, `function arrangement()` → map for `Profiles.save`.
- Settings on `remembered`: `layout` (default `"perDisplay"`), `excludedFromClone` (list), `mutedMonitors` (list). Migration once: if `remembered.screens` is set and `excludedFromClone` was never written, `excludedFromClone = connected names not in remembered.screens`; then `screens` is no longer written.
- Display set: per-display — an `Instantiator` over `seenNames` (every connector name seen in this run, appended on `Application.screens` change, never removed); each `Display.screens` = the connected screen of that name or `[]`. Clone — one `Display` with `screens` = connected screens not in `excludedFromClone`, ordered so the clone source (`cloneSource` setting, default first by position) is first.
- Layout switch: first `swapNow()` on any display with `arriving !== ""`; per→clone gives the clone the selected display's wallpaper and running playlist; clone→per gives every display the clone's wallpaper and properties, the playlist to the first.
- Cover modes: `underWindow` gains `"pauseall"` and `"stopall"`; `"mute"` mutes only the covered display in per-display layout.

- [ ] **Step 1: Implement**.
- [ ] **Step 2: Verify** in the two-output virtual KWin: per-display, two different wallpapers on the two outputs (grabs differ, each matches its wallpaper); both sounds audible (two `wallflux` sink-inputs in `pactl list sink-inputs`); clone still identical to Task 5's grabs; a window maximised on output 1 only (KWin script moving a `konsole` there and maximising) pauses display 1 only with `"pause"` and both with `"pauseall"`; unplugging an output is `kscreen-doctor output.<name>.disable` inside the virtual session — the display keeps its wallpaper id and its playlist is held (`held with … ms left` in the log), re-enabling resumes it; old settings with one screen unticked come up in clone with that screen excluded.
- [ ] **Step 3: Commit** "A wallpaper per monitor, as Wallpaper Engine's default layout".

### Task 7: The library follows the selected monitor

**Files:**
- Create: `qml/DisplayMap.qml` (the monitors drawn to scale, extracted from `DisplaysView.qml`: a `Repeater` of rectangles from `virtualX/Y`, each with the display's preview, its number, a speaker button, a selected ring; signals `chosen(string key)`, `contextAsked(string key, point at)`)
- Modify: `qml/Picker.qml`, `qml/DisplaysView.qml` (uses `DisplayMap`), `qml/PlaylistBar.qml`, `qml/PropertiesView.qml` if it reads `propertiesOf` directly, `CMakeLists.txt`

**Interfaces:**
- Consumes: `stage.displays`, `stage.display(key)` (Task 6).
- Produces on `picker`: `property string selectedKey` (remembered as `lastMonitor`; falls back to the first display when absent), `readonly property var selected` (`stage.display(selectedKey)`), `readonly property bool manyMonitors` (layout `"perDisplay"` and more than one connected screen).
- `propertiesOf` keys become `key + "|" + id`; `propertiesFor(one)`/`valuesFor(one)` take `(one, key)` and fall back to the plain `id` entry (today's saved values) — **every caller passes the key** so bindings refresh (function-binding trap).
- Everything reading the shown wallpaper or the run (tint, ring on the playing card, footer title and Off, Playlists button, playlist strip, details panel's properties) reads `picker.selected.showing` / `.run`.
- Above the details panel, when `manyMonitors`: a `Label` "Monitor N · name" and `RoundedButton` `qsTr("Choose monitor")` opening a `Popup` with `DisplayMap` (closes on choosing) and a `CheckBox` `qsTr("Show when the window opens")` bound to `remembered.chooserOnOpen`. The popup opens by itself in `onVisibleChanged` when `manyMonitors && chooserOnOpen`. Double click / Enter applies to `picker.selected`.

- [ ] **Step 1: Implement**.
- [ ] **Step 2: Verify** in the two-output virtual KWin with a grab of the window (CLAUDE.md, "A picture of the picker"): with one output — no label, no button, no popup; with two — popup on open, choosing monitor 2 then double clicking a card changes only output 2; the strip shows monitor 2's playlist; a property changed on monitor 2 leaves monitor 1's alone; qmllint clean.
- [ ] **Step 3: Commit** "The library works on the monitor that is chosen".

### Task 8: The Monitors tab

**Files:**
- Modify: `qml/DisplaysView.qml`, `qml/Picker.qml` (profile dialogs, reusing the playlist Load/Save popups' pattern)

**Interfaces:**
- Consumes: Tasks 4, 6, 7.
- Layout: a `RoundedCombo` with the model `["perDisplay", "clone"]` and translated `displayText`/delegate (`qsTr("Wallpaper per monitor")`, `qsTr("Clone one wallpaper")`), calling `stage.setLayout`. The ticks and their switches are removed. `DisplayMap` with previews; a click selects (`picker.selectedKey`); the speaker toggles `mutedHere` (per-display) or the clone's (shown on the clone source only).
- Right-click `Menu` on a monitor: `qsTr("Change wallpaper")` (select it, switch to the library tab), `qsTr("Remove wallpaper")`, `qsTr("Copy to")` ▸ other monitors, `qsTr("Swap with")` ▸ other monitors, `qsTr("Mute")`/`qsTr("Unmute")`; in clone also `qsTr("Make it the clone's source")` and `qsTr("Flip")`/`qsTr("Unflip")` (not on the source; flipped names remembered as `flippedMonitors`; `WallpaperSurface` mirrors with a `Scale { xScale: -1; origin.x: width / 2 }` on its `Mirror`). A `Menu` gets `implicitWidth` (trap).
- Profiles: `RoundedButton`s `qsTr("Save profile")` (name field, replaces same name) and `qsTr("Load profile")` (list with Rename, Delete-with-confirmation, Load) → `Profiles.save(name, stage.arrangement())`, `stage.applyProfile(Profiles.named(name))`.

- [ ] **Step 1: Implement**.
- [ ] **Step 2: Verify** in the virtual KWin: each menu item does what its words say on the outputs (grabs); a saved profile loads back after quitting and restarting the program (and nothing played at start before loading); a profile naming an absent output applies to it when it is enabled; qmllint clean.
- [ ] **Step 3: Commit** "The Monitors tab: layout, each monitor's menu, profiles".

### Task 9: Tray, Settings and words

**Files:**
- Modify: `qml/Main.qml` (tray items), `qml/SettingsView.qml` (cover modes; `Guide` cards), `i18n/wallflux_cs.ts`, `CLAUDE.md`

**Interfaces:**
- Consumes: Tasks 6–8.
- Tray, when `manyMonitors`: the structure in the spec's "Tray" section. Item ids carry the key: `"pause@<key>"`, `"previous@<key>"`, `"next@<key>"`, `"mute@<key>"`, `"recent@<key>:<id>"`, `"playlist@<key>:<name>"`, `"remove@<key>"`; top level `"pauseall"`, `"muteall"`, `"profile:<name>"`. Monitor item text: `qsTr("%1 — %2").arg(label).arg(title or qsTr("nothing"))`. A running playlist's row reads `"▶  " + name`, as the library's menu. In a monitor's Recent, the wallpaper that monitor shows is greyed out (`enabled: false`), as today. Tooltip lists every display's line. Otherwise today's menu.
- Settings › Performance, "A window covers the wallpaper": the combo's words for `"pause"`/`"stop"` become `qsTr("Pause that monitor")`/`qsTr("Stop that monitor")` and `"pauseall"`/`"stopall"` are added (`qsTr("Pause all")`, `qsTr("Stop all")`) only while `manyMonitors`; in clone or with one screen the four of today.
- `Guide` cards in Controls: the chooser, the Monitors tab's menu, profiles, the tray per monitor — both languages.
- CLAUDE.md: "What works today" entry, Layout entries for the new files, the Displays paragraph ("Worked out once") updated; the "Next, chosen by the user on 23 November" paragraph removed.

- [ ] **Step 1: Implement** tray and Settings.
- [ ] **Step 2: Verify the tray over D-Bus** (CLAUDE.md trap: `com.canonical.dbusmenu GetLayout` on the program's StatusNotifierItem menu): two monitors show two submenus each with seven children and a nested Recent; one monitor shows today's menu. Trigger `pause@<key>` then `pauseall` then `pauseall` again (via `Event` on the dbusmenu) — the first monitor is still paused, the second plays.
- [ ] **Step 3: Translations**: `cmake --build build --target update_translations`, fill every new Czech entry, rebuild, switch the window to Czech and look at the tray and the Monitors tab.
- [ ] **Step 4: Measure on the laptop** (A8-6410 + monitor): two scenes at once, Medium and Low — CPU/GPU from the status bar and frames per second from `QT_LOGGING_RULES="wallflux.*=true"`; report the numbers to the user, no remedy built.
- [ ] **Step 5: Commit** "Tray per monitor, cover modes per monitor, and the words for all of it".
