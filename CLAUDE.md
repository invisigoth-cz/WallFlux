# WallFlux

Plays the Wallpaper Engine wallpapers somebody already owns, on Linux and
Wayland, without Windows and without Wallpaper Engine running. C++20, Qt 6,
GPL-3.0-or-later -- plus, under `crates/`, the scene renderer in Rust,
linked into the binary.
Target
machine is CachyOS with KDE Plasma 6.7 on Wayland, and that is where
everything below was measured.

**Steam is how the wallpapers get here.** Wallpaper Engine is bought and
installed through it, the workshop is where wallpapers come from, and Steam
is the only thing that can fetch one: a workshop item for a paid app is
downloadable only by an account that owns the app. So Steam is a dependency
of getting wallpapers, not of playing them -- what is read is the folder
Steam has already filled, and nothing of Steam's has to be running for that.

## What works today

- **Web wallpapers** (`index.html` + JavaScript) drawn by QtWebEngine.
- **Video wallpapers** drawn by QtMultimedia, looping, with their sound on.
- **Scene wallpapers** -- Wallpaper Engine's own packed format -- drawn by
  the renderer under `crates/`, into a `SceneView` on the same surface as
  the other two: transitions, pausing under a cover and every screen work
  the same. A scene's own music plays -- unpacked from its `scene.pkg`
  into the cache by the renderer and played by `SceneSound`, once for the
  program and not once per screen, in the scene's own `loop`, `random` or
  `single` manner. Its particle systems bring their children -- running
  alongside, bursting where a particle is born or dies, or riding along
  with one -- and are set up as the scene says, not as their preset does.
  A scene that answers to the music hears what the speakers are playing
  -- 7 of the 40 here do -- read off the default output's monitor by the
  renderer itself, once for every screen. A scene's JavaScript runs --
  82 of the 93 scripts in 7 packages, under QuickJS: what turns a clock's
  hands, swells a ring with the bass and keeps a particle system's rate on
  the beat. The other eleven write text, which is not drawn yet, or switch
  an effect or a particle override other than the rate. And a scene
  follows the mouse -- layers that lean away from it, particles pushed
  aside -- with the position asked of KWin, since the wallpaper's surfaces
  take no input and must not.
- **Sound that gets out of the way by itself.** The wallpaper is quiet while
  another program is using the speakers, and quiet and stopped while a window
  covers the screen; both come back on their own. Neither is a setting. The
  one switch there is turns the wallpaper's own sound off altogether.
- **A tray icon** that opens a grid of every installed wallpaper and plays
  the one that is clicked.
- **Noticing what Steam downloads.** The workshop folders are watched, so a
  wallpaper subscribed to in Steam turns up in the grid on its own, a couple
  of seconds after the writing stops.
- **The Steam Workshop, browsed.** Two ways in. *Discover* is rows picked
  for you -- Trending, New and one per genre, twenty wallpapers to a row --
  as many rows as the window is tall, with more as it is scrolled, since
  each row is its own question. Nothing in Discover is filtered or sorted
  by hand; "See more" opens one row whole, as a page with the section's
  name and the way back. *Workshop* is one grid with search, filters, sort
  and paging, fifty at a time and a thousand pages deep.
  Clicking a wallpaper anywhere in this tab opens a panel down the right
  with its picture, what is known about it and a green Download; the
  library's grid has the same panel with a red Remove instead. Out here
  the click is the only thing there is -- it opens the panel and closes it
  again, and a double click does nothing whatever, because nothing in this
  tab is ours to play.

  **Leaving a tab shuts its panel**, in either direction and for every
  tab. The panel is a question about one wallpaper in one grid, so walking
  away from the grid ends it; coming back to a tab and finding the panel
  as it was left is the window remembering something nobody asked it to.
  The click still being counted is forgotten with it, or it would open a
  panel in the tab that had just been left, a few tenths of a second after
  leaving it.
- **A filter panel that folds.** Each section's heading is a switch: the
  arrow turns, the chips slide out from under it, and a number beside the
  heading says how many of them are ticked while they are away. The short
  sections start open and the long ones folded -- half a dozen is the
  line, which leaves the workshop's six kinds and three age ratings out
  and folds its nineteen picture sizes and twenty-five genres. Sixty-four
  chips, and the panel opens shorter than a window.
- **One vocabulary for both filters.** Type, age rating, picture size,
  genre and the rest are all tags as far as Steam is concerned, so a genre
  row and a ticked chip are the same required tag. Both panels read DRUH,
  PŘÍSTUPNOST, ROZLIŠENÍ, ŽÁNR, ŠTÍTKY, and every tag is called what Valve
  calls it in whichever of its 36 languages the window is in.
- **An empty Steam tab that says why**, with a Try again that works: the
  Steamworks library is opened again rather than once at startup, so
  starting Steam afterwards is advice worth taking.
- **English and Czech**, switched in the window without a restart.
- **Wallpapers that fade into one another.** A playlist can cross from one
  wallpaper to the next, go out through black, or hold the last frame over
  the gap, over nought to three seconds.
- **Wallpapers that take turns.** A strip along the bottom of the library
  from the moment the window opens, empty or not -- the place a playlist is
  made has to be visible before there is anything in it. Along its top:
  Playlists, then Load, Save, Configure and Add wallpaper; under them the
  chosen wallpapers, twenty to a row. Hovering a card shows a tick box --
  white with a black edge, blue when it is ticked, fixed colours rather
  than the palette's because the box sits on a picture and not on the
  window -- and Add wallpaper holds every box open so several can be
  ticked without hunting for each one. The room for the previews comes
  with the first one that is ticked and goes with the last; the words and
  the buttons are there from the start. Load is the saved playlists, to open one back onto
  the strip -- which also starts it -- or throw it away, which the button
  asks about first: forgetting a playlist cannot be undone and there is
  nowhere to undo it from; Save asks what to call it and replaces one of
  that name; Configure is how it runs. A saved playlist is started from the
  Playlists button beside the filters, and stopped by clicking it again;
  that button always says Playlists and the tick in its menu says which
  one is on, as the sort menu does.
  There is no Playlists tab: a playlist is made out of the grid, so it is
  made in the grid.

  **Configure is laid out as Wallpaper Engine's own.** Five of its
  switches do what they say: *change when the video ends*, *allow changing
  while paused*, *always begin with the first wallpaper*, the transition
  and how long it takes. Nothing in the popup is a placeholder any
  more: "the first wallpaper plays only at startup" was the last one and
  it has been taken out, since WallFlux is not started with the session in
  a way that would give it a meaning. The transition drop down opens with
  an empty row
  chosen, as Wallpaper Engine's does; the interval is hours, minutes and
  seconds, typed rather than clicked at.

  **There was a "change wallpaper" list and there is not any more.** It
  offered a timer, login, time of day, day of week and never; four of
  those were never going to be written, and a list of five with one of
  them meant is a promise rather than a plan. A playlist turns on its
  clock, and on a video ending where it was told to.

  **How the three are wired.** None of the decisions are `Playlists`':
  it knows nothing about what a wallpaper is or whether a window is in the
  way, so `Main.qml` works out whether the clock should be standing still
  and says so with `hold()` -- a video is in charge and has not finished,
  or a window is covering the wallpaper and this playlist was not told to
  carry on regardless. `hold()` keeps what was left rather than starting
  the turn again, which is why the timer is single-shot and wound at every
  turn: a repeating one cannot be stopped and picked up. Measured end to
  end with a real cover: held with 8 515 ms left, and it turned 8.5 s after
  the screen came back. "Always begin with the first" is the one that is
  entirely `Playlists`' own -- it holds index nought out of the shuffle,
  and means nothing without shuffling, since an unshuffled playlist starts
  with its first wallpaper anyway.

  **What those four mean in Wallpaper Engine**, read off its own guides and
  developer replies, so that copying the behaviour is copying rather than
  inventing:

  - *Change wallpaper when the video ends* overrides the timer and applies
    to videos only -- the developers say it cannot be done for any other
    kind.
  - *Allow changing while paused* decides whether the playlist goes on
    turning while playback is paused, which happens when another window is
    fullscreen or when it is paused from the tray. With it off the timer
    pauses too, which is the usual reason a playlist looks stuck.
  - *Always begin with the first wallpaper* is what it says; paired with
    the video-end option it is how somebody builds an intro video followed
    by a second wallpaper.
  - A playlist does not cycle at all while Wallpaper Engine's own window is
    open.
- **A picker with its sections across the top**: Installed, Steam,
  Displays, Settings, centred. The first two are built; the other two say
  "Coming soon" and are there because naming them is what decides where
  the next piece of work goes.
- **A wallpaper on every screen that is ticked for it.** The Displays tab
  draws the monitors as they are arranged -- read off their positions on
  the shared desktop, so it is the arrangement somebody set up in their own
  display settings -- and gives each a switch. One wallpaper, drawn on the
  ticked ones; not a wallpaper per screen.

  **The screens are Qt's own**, `Qt.application.screens`. No `xrandr`, no
  `wlr-randr`, no `kscreen-doctor`, nobody's output parsed: Qt names a
  screen after its connector, which is the same name KDE's display
  settings show and the same one under `/sys/class/drm`. Checked against
  all three on this machine.

- **A panel beside the grid saying what a wallpaper is.** A click opens
  it and a click on the same card puts it away again, in either tab --
  which is why there is no cross on the picture any more: the card is the
  switch. It slides rather than appearing, two tenths of a second of
  width, with its contents held at their full width and cut off by the
  edge so that nothing inside is squashed on the way. In the library both
  halves wait out the same 400 ms as the double click, because opening
  and closing each move every card and a card that moves out from under
  the pointer cannot be clicked a second time. Inside: the picture, the
  name, and
  under that a button saying who made it -- their avatar, their name and a
  magnifier -- which turns the workshop grid into everything else they
  have published, with a blue chip beside the filters to come back from.
  It is there from the moment the panel opens, with the name filling
  itself in a moment later. Then what is known, a
  row to a line with an icon and a hairline under it: the kind, the picture
  size, what it takes up, the age rating, whether it answers to sound. The
  tags are chips under their own heading rather than a line of commas.
  Under all of it the one thing to do about it: green Download in the
  workshop, red Remove in the library, each with a line under it saying
  what will happen. One click picks a wallpaper out; two put it on the
  desktop.

  **The library has the same row**, although project.json does not say who
  wrote a wallpaper -- checked across every one of them. It takes two
  keyless questions instead of one, both in `qml/steamprofile.js`: the
  workshop is asked about this id -- which answers with the maker and with
  the full list of tags, see the trap -- and then who that account is. The
  magnifier there opens the Steam tab and asks the question in the grid
  that can answer it. **Coming back is the Workshop button**, which clears
  the author and asks again: clearing alone does nothing, because the grid
  is already in the Workshop's own mode and `onModeChanged` never fires.
- **A grid the arrow keys walk.** The window hands the keyboard to the
  grid when it opens: arrows turn one card's band blue, Enter puts that one
  on the desktop, Space ticks it into the playlist. A click moves
  the band as well, so the mouse and the keyboard do not each keep a place
  of their own. The panel follows it only once it is open -- opening
  one because somebody pressed Right would be the window deciding what the
  question was.

- **A picker that looks like something**: dark and light themes switched
  without a restart, a search that narrows the grid as it is typed and a
  cross to empty it, name order either way round, a band around each
  preview saying whether it can be played -- lit up while the pointer is
  on it -- a preview that leans in under the pointer, the whole window
  tinted by whatever the panel is showing, rounded buttons with room around their words, filters as
  one value to a row beside the grid, and scrollbars that stay visible in
  a strip of their own rather than fading out over the pictures. In the
  corner of the footer an `i`, which says the version when it is hovered
  and does nothing when it is clicked -- `Application.version`, out of
  `WALLFLUX_VERSION` in CMakeLists.txt, which is `project()`'s own number
  rather than a third place to keep it.
- **An Arch package** (`packaging/PKGBUILD`), installed, with an autostart
  entry that has been verified through a real logout and login.

Of the wallpapers found on the development machine -- 53 at the last count,
and the number grows as the workshop is used -- 12 are web and video and
41 are **scene**, and all of them play but one: Diablo IV Lilith, whose
only picture is a 3D model, which the renderer turns down with a reason.
A scene whose `scene.pkg` is missing is listed, dimmed, and a click does
nothing.

## Layout

```
src/main.cpp          WebEngine init and the single-copy lock
src/desktop.h         Desktop: turning a window into a wallpaper
src/desktop.cpp       the layer-shell settings, asked for per window
src/sceneview.h       SceneView: a scene wallpaper as a QML item, and
                      SceneSounds: the music it carries, unpacked
src/sceneview.cpp     the renderer driven from Qt's render thread
src/wallpapercore.h   WallpaperKind::Kind, Wallpaper (Q_GADGET), WallpaperCore
src/wallpapercore.cpp scanning Steam's folders, reading project.json
src/language.h        Language: the EN/CZ switch, and what it has to call
src/language.cpp      installs a QTranslator and retranslates the engine
src/playlists.h       Playlists: wallpapers that take turns, and the file
src/playlists.cpp     reading and writing ~/.config/WallFlux/wallflux/
src/otheraudio.h      OtherAudio: is another program using the speakers
src/otheraudio.cpp    reading `pactl --format=json list sink-inputs`
src/steamworks.h      Steamworks: can the workshop be asked, and the way in
src/steamworks.cpp    dlopen of libsteam_api.so, by name, nothing linked
src/tags.h            Tags: what a workshop tag is called, and the genres
src/tags.cpp          reads Wallpaper Engine's locale/, 36 languages
src/screencover.h     ScreenCover: is a window covering the wallpaper
src/screencover.cpp   a script loaded into KWin, which calls back over D-Bus
src/pointer.h         Pointer: where the mouse is, for the scenes
src/pointer.cpp       another KWin script, loaded only while it is watched
src/kwinscript.h      KWinScript: a script run inside KWin, shared by both
src/kwinscript.cpp    written to a file, unloaded first, loaded, started
qml/Main.qml          the program: the tray icon, the picker, one surface
                      per screen, and the transitions they share
qml/SceneSound.qml    one sound object of a scene, played as it asks
qml/WallpaperSurface.qml  one screen's wallpaper: the layer-shell window,
                      the web view, the player, and what a transition
                      holds over them
qml/DisplaysView.qml  the Displays tab: the screens drawn to scale and a
                      switch for each
qml/Picker.qml        the picker window: header, search, grid, footer
qml/WallpaperCard.qml one preview, its band and its name
qml/Details.qml       what is known about one wallpaper, beside the grid --
                      shared, with the one button handed in as a child.
                      `facts` is `{ label, value, icon }`, `tags` is its
                      own list drawn as chips, `author` is
                      `{ name, avatar }` or null
qml/PlaylistBar.qml   the playlist being put together, along the bottom
qml/FilterPanel.qml   the filter popup: chips in sections, and what is on
qml/PreviewTile.qml   a preview and a name -- the shape of a card, shared
                      by the library and the workshop
qml/WorkshopView.qml  the Steam tab: Discover rows and the Workshop grid
qml/WorkshopFilters.qml  the workshop's filter popup, the whole vocabulary
qml/Chip.qml          one value, on or off -- shared by both filters
qml/ChipSection.qml   a heading and the values under it, one to a row
qml/Scroller.qml      a scrollbar that stays visible, used by every view
                      -- except the details panel, which passes
                      `policy: ScrollBar.AsNeeded`
qml/steamprofile.js   who made a workshop item, and who that account is:
                      two keyless questions, shared by both tabs
qml/RoundedButton.qml a button with its corners off -- every button here
qml/RoundedTool.qml   the same corners for an icon on its own
tests/tst_wallpapercore.cpp   QtTest against a fake Steam tree in a temp dir
tests/tst_otheraudio.cpp      reading pactl's answer, no sound card needed
tests/tst_screencover.cpp     the states, with the session bus pointed nowhere
tests/tst_pointer.cpp         known only while watched, the same way
tests/tst_playlists.cpp       what a playlist remembers and how long a
                              wallpaper stays -- five seconds of suite time
                              on purpose, see below
tests/tst_tags.cpp            tag names against Wallpaper Engine's catalogue
tests/tst_sceneview.cpp       a scene drawn through the item, checked against
                              the same scene drawn directly -- and the right
                              way up
i18n/wallflux_cs.ts   the Czech catalogue, compiled into the binary
packaging/PKGBUILD    builds from this checkout's git history
wallflux.desktop      installed to applications/ and to /etc/xdg/autostart/
Cargo.toml            the Rust workspace: the two crates below
crates/wpe-format/    reads the scene format -- .pkg, .tex, scene.json,
                      materials, effects, particles, and the shader dialect
crates/wallflux/src/script.rs  a scene's JavaScript, run with rquickjs, and
                      `script/host.js`, the globals a script finds -- ours,
                      written from what the library's scripts use
crates/wallflux/      draws a scene with OpenGL; `examples/shot.rs` renders
                      one wallpaper to a PNG, which is how it is checked.
                      `src/ffi.rs` and `include/wallflux_scene.h` are the
                      four calls the program makes; `src/main.rs` is the
                      old standalone viewer and is not part of the program
tools/sweep.sh        `shot` over every scene wallpaper on the machine,
                      pictures and one log into the directory it is given
```

`WallpaperCore` is a QML singleton named `Wallpapers`, declared with
`QML_NAMED_ELEMENT` + `QML_SINGLETON` (**not** `setContextProperty` or
`qmlRegisterType`, which qmllint cannot see):

- properties: `wallpapers` (QList<Wallpaper>), `current`
- methods: `rescan()`, `rescan(roots)`, `select(id)`, `clear()`,
  `discard(id)` (deletes the workshop folder -- the other half of giving a
  wallpaper back, see the trap), `details(id)` (`{ size, resolution }`,
  read off the disk when the picker shows one card and kept afterwards)
- static: `scan(roots)`, `steamRoots()`
- `Wallpaper`: `id`, `title`, `kind`, `source`, `preview`, `tags`, `rating`,
  `resolution`, `sizeOnDisk`, `added`, `bytes`, `soundKnown`, `hasSound`,
  `hasFile`, `playable`
- also on the singleton: `recent` (ids chosen in this run, newest first)

`Steamworks` and `Tags` are singletons of the same module:

- `Steamworks`: `available`, `trouble` (a `Trouble` enum, **not** a
  sentence -- see the trap), `busy`, `found` and `totals` (keyed by the
  name the asker gave the question), `downloads` (id -> 0..100, -1 refused);
  `browse(key, order, page, tags, search, urgent, creator)`, `download(id)`,
  `remove(id)` (unsubscribes, and nothing else -- the folder is
  `Wallpapers.discard()`'s), `retry()`. An answer's entries carry `owner`,
  the maker's steamID64 **as a string** -- it does not fit a double and QML
  would round it. `creator` narrows the question to one account's published
  work, which is a different call of Steam's with its own orders, so
  `order` says nothing while it is set.
- `Tags`: `genres` (25), `types`, `ratings`, `resolutions`, `properties`;
  `name(tag, locale)`, `labelled(tags, locale)`

`Playlists` is another singleton of that module, and the only thing in
this program that remembers anything between runs:

- `all` (each `{ name, ids, hours, minutes, seconds, shuffle, videoEnds,
  whilePaused, alwaysFirst, transition, transitionMs }`), `playing` (a
  name or empty), `changesOnVideoEnd`, `changesWhilePaused`, `transition`
  and `transitionMs` (what the *running* one was told, empty or false
  while none is); `save(name, ids, how)` -- `how` being those
  settings by name rather than nine positional arguments --
  `forget(name)`, `named(name)`, `start(name)`, `stop()`, `next()` (on to
  the next one now and wind the clock again), `hold(bool)` (stop the clock
  without stopping the playlist, and pick it up where it left off), and
  the public static `staysFor(one)` -- the three numbers added up in seconds, never
  less than five, worked out in one place because that is the sum a test
  can check; and `asClock(seconds)` -- the same rule read backwards,
  floor and ceiling and all, which is what the settings popup hands its
  three typed boxes to. An exact inverse, so the boxes can never show an
  interval that is not the one that will run: three noughts typed in read
  back as five seconds, which is what the playlist will actually wait
- it plays nothing itself: it says `plays(id)` and `Main.qml` hands that to
  `Wallpapers.select()`, the same place a click on a card goes
- the file is `playlists.json` under `QStandardPaths::AppConfigLocation`,
  which on this machine is `~/.config/WallFlux/wallflux/`

`Pointer` is one more: `position` in the desktop's coordinates, `known`,
and `watching`, which `Main.qml` binds to a scene being on show and in
view. Each screen's `SceneView` is handed `pointer` -- the position less
`Screen.virtualX/Y`, or undefined -- and passes it on as a fraction.

`OtherAudio` and `ScreenCover` are singletons of the same module, each with
one property -- `playing` and `covered` -- and no business of their own in
what the answer is used for. `Main.qml` puts the three together in one line:

```qml
readonly property bool silent: picker.muted || OtherAudio.playing || ScreenCover.covered
```

## Decisions that are settled — do not reopen without being asked

- **GPL-3.0-or-later, with an added permission for the Steamworks SDK** and
  for nothing else, written out in LICENSE-EXCEPTION. The licence itself is
  not up for discussion; the permission is there so the Steam part can be
  written at all, since a workshop item for a paid application can only be
  fetched through Valve's proprietary SDK. Every other library WallFlux
  links must still be GPL-compatible.

- **`steamworks-sdk` from the AUR is a dependency, not a suggestion.**
  Nothing else on a Linux machine carries `libsteam_api.so`: Steam ships
  none, and Wallpaper Engine carries only the Windows DLL -- both measured.
  That package puts the SDK under `/opt` in its entirety, unmodified, with
  Valve's licence beside it, which is what the SDK's terms require and what
  carrying it inside this package would break. So it is depended on and
  `dlopen`ed, never shipped and never linked. An AUR helper resolves it;
  plain `makepkg -si` will want it built first. The price, which was said
  out loud before it was chosen: everyone who installs WallFlux gets
  Valve's SDK on disk whether they open the Steam tab or not.

- **Qt draws web and video wallpapers; scene wallpapers are our own
  renderer.** It is Rust with OpenGL, under `crates/`, and it was parked on
  the branch `zaloha-puvodniho-pokusu-o-vlastni-engine` from the day main
  was cleared for Qt until 27 October, when it came back into main by a
  merge -- so `git log crates/` carries its whole history.

  **It is linked into the program, not run beside it.** Cargo builds it
  into a static library inside the CMake build tree, and `SceneView` -- a
  `QQuickFramebufferObject` -- drives it on Qt's render thread with Qt's
  own OpenGL context. Chosen over a separate process because a scene in
  an item is one more thing on the one surface: the transitions grab it
  the way they grab a video, and there are not two layer-shell surfaces
  from two programs in the same layer. The price, said before it was
  chosen: a renderer that crashes takes the tray icon and the picker with
  it, which is why every call into it is behind `catch_unwind` and a scene
  that panics is not drawn again. Building needs `cargo`; running needs
  nothing more than before.
- **Nothing plays at startup.** The program comes up as a tray icon and the
  desktop keeps the wallpaper it already had until somebody picks one. The
  choice is not remembered between runs, by design.

  **Playlists are the one exception, and were asked for as one.** A saved
  playlist is written to a file and is there at the next start -- but it is
  not started at the next start, which keeps the rule above intact.
- **The tray icon is the whole interface.** Not a CLI, not a Plasma plugin;
  both were considered and turned down.
- **Layer-shell is set up in C++**, before the window reaches the
  compositor — which is why a surface is built `visible: false` and shown
  by hand afterwards. It used to be `main()` doing that to the one window
  there was; a wallpaper belongs to one screen, so there is one window per
  screen now and they come and go with the monitors, which only QML is in
  a position to notice. `Desktop::asWallpaper(window)` is what `main()`
  used to write out.

## Traps, each one found the expensive way

- **A `QQuickFramebufferObject` drawn with OpenGL comes out upside down.**
  OpenGL draws from the bottom up and Qt 6 shows the framebuffer as it
  is; `setMirrorVertically(true)` is what turns it over. Nothing about it
  looks broken in code, and a scene upside down is still a scene, so
  `tst_sceneview` draws the same wallpaper straight through the C
  interface and checks the item against it: 0.86 apart the right way up,
  47 apart the wrong way.

- **A renderer that asks to be drawn again is drawn without being
  synchronized.** `Renderer::update()` re-renders the framebuffer and
  never calls `synchronize()`, so anything left there for the item to
  pick up waits for somebody to change the item -- for a wallpaper that
  failed to load, never. The reason goes to the item as a queued call.

- **The scene graph has to be OpenGL, and is told so.** `main()` sets it
  before any window exists. It is OpenGL on this machine without being
  asked, which is exactly why it has to be said: elsewhere Qt is free to
  pick Vulkan, and every scene would draw nothing with no error at all.

- **A scene's bound values are written twice, and the scene's copy is
  stale.** A value bound to a user property sits in `scene.json` beside
  the binding and again in `project.json`, where the property is declared;
  Wallpaper Engine plays the second. 205 of the 365 in this library
  disagree -- one wallpaper's scene tints everything red at full
  strength, and its project has the tint off.

- **A scene's `instanceoverride` multiplies the particle system it
  places, and stays with that system.** 462 particle objects carry one
  -- alpha, size, lifetime, rate, speed, count, and a colour written
  either as `colorn` (0-1) or, in older scenes, `color` (0-255), never
  both. Unread, every smoke kept its preset's colours, which is where the
  pastel blobs in half the library came from. **A child system does not
  inherit it:** handed down, Star Citizen's embers -- five times the rate
  and the size -- took their glows with them and burned the sky white;
  kept to the parent, the picture matches the wallpaper's preview. Both
  were rendered before deciding.

- **A sweep at six seconds misses what is brief or late.** A lightning
  flash lives a tenth of a second and the Star Wars sparks start at ten,
  so a change to bursts moved one picture out of forty. Check those at
  the moment they happen -- the particle count `shot` prints is the
  quickest way to find it: the Star Destroyer jumps by a thousand at
  15.1 s.

- **`_rt_imageLayerComposite_<id>_a` means two things, depending on who
  says it.** Written by layer `<id>` itself, in a slot its effect already
  binds `previous` to, it is only a name for what that bind hands over --
  turning the effect down over it lost Arkham Knight's blur and godrays
  and the motion blur in eight more wallpapers. Written by another layer,
  it is that layer's *finished* picture, which the renderer keeps for any
  layer somebody reads. Almamu's renderer reads whichever of the pair a
  chain ended in, and its own TODO records a wallpaper getting the wrong
  picture that way.

  **And `g_TextureReductionScale` has to be set.** `blend` divides an
  offset in the scene's pixels by it; left at nought, every blend placed
  with `TRANSFORMUV` was thrown off to infinity and drew nothing. It is how
  many scene pixels one pixel of the picture stands for -- `skew` says so
  outright -- which for our buffers, made as big as the object is *on
  screen*, is the object's size over the buffer's.

  **The Space Marine's gun is steel, not brass.** It looked right gold
  for days, and that was the texture drawn raw because its whole chain
  had been dropped; the tint that greys it is the wallpaper's own, and the
  warm light comes from the muzzle, through layer 151. The wallpaper's
  `preview.gif` is what settled it. **When a picture moves, look at the
  preview before deciding which of the two was right.**

- **Plasma calls listening to the speakers using the microphone.** A
  record stream on a sink's monitor puts `plasmashell_microphone` -- the
  microphone icon -- in the tray for as long as it is open. Measured by
  reading `RegisteredStatusNotifierItems` with and without one. The only
  streams it hides are four volume mixers' meters, matched by
  `application.id` in `libKF6PulseAudioQt` -- passing for one of them would
  be lying. So the renderer listens only while it is asked: `Scene::listens()`
  says at load whether a shader reads `g_AudioSpectrum16*` or a particle
  system's rate, field or swirl follows the sound, only such a scene asks,
  and `audio.rs` closes the stream once the bands have gone a second
  unread. A wallpaper under a window is not drawn and so does not ask, and
  the icon goes with it.

- **A scene's scripts are ES modules inside scene.json, on the value they
  drive**, and every object is a layer to them whether it is drawn or not:
  Ender Spartan turns an invisible `SharedNoise` with one script and reads
  its angles from ten others. So `script/host.js` keeps a layer for every
  object in the file, and the layers are where place, turn, size, alpha and
  visibility live while a scene with scripts runs -- `render.rs` copies
  them onto the objects scene.json placed itself (`from_scene`), never onto
  the children a particle system brought.

  **Angles are degrees to a script and radians in the file.** Dark Core's
  clock turns its hands with `* WEMath.rad2deg`; handed radians, its hour
  hand read -122 and spun.

  **A picture drawn at six seconds has to have run six seconds of
  scripts.** Most of them ease towards the music by `engine.frametime`, and
  one frame at t=6 is one call with a frametime of nought -- Dark Core's
  ring stayed dark however loud `shot` was told the room was. The first
  frame catches up in thirtieths of a second; every later frame is one step,
  told at most a tenth of a second went by, because 300 steps of Dark Core
  is a quarter of a second of render thread after a window uncovers it.

  **And a scene that reads the clock is never the same picture twice.**
  Eleven of Dark Core's scripts call `new Date()`. In a sweep, a scripted
  clock that moved is not a regression.

  **Every call has a deadline and a script that misses it is stopped.**
  A script runs on Qt's render thread; one that loops for ever would stop
  the desktop being drawn. QuickJS's interrupt handler ends any call past
  20 ms, a script that throws or is interrupted is not run again, and the
  value it drove stays where it was -- tested for both in `script.rs`.

  **Almamu's renderer has no `engine.registerAudioBuffers`**, which 39 of
  the 93 call: his scenes stay at the quiet end of what they say. Ours
  hands the sixteen bands it measures, each spread over the 32 or 64
  entries a script may ask for. Asking for them is what makes a scene
  listen, so the microphone rule above holds for scripts too.

- **The mouse comes from KWin, and the surfaces never take input.** A
  click on the desktop belongs to Plasma's desktop underneath ours; a
  wallpaper with an input region would swallow it. `workspace.cursorPos`
  and `cursorPosChanged` were measured on KWin 6 first, and so was what
  `callDBus` makes of two JS numbers -- **`int32`**, read off
  `dbus-monitor`. A slot taking doubles would never have been called, and
  D-Bus says nothing about a call whose signature matches no slot. The
  script sends at most every 16 ms and is loaded only while a scene is
  showing and uncovered; asked to stop, it is unloaded.

  **And a mouse that changes only the item is a mouse the scene never
  hears of** -- the framebuffer trap above, again: `setPointer` calls
  `update()`. `tst_sceneview` lays three Blue Dawns side by side with the
  mouse left, right and left, and fails with the `update()` taken out
  (measured: 0.006 apart either way) while the outer two agree and the
  middle one leans. Blue Dawn because Warhammer, the first choice, does
  not draw the same picture twice -- 900 apart with the mouse in one place
  -- and its "follows the mouse" had been that noise all along.

- **`LayerBottom`, never `LayerBackground`.** Plasma draws its own desktop
  into a background-layer surface, and clicking the desktop raises it above
  anything else in that layer — the wallpaper vanished and the system one
  came back. Layers are a fixed order; bottom is still below ordinary windows.
- **Qt's own messages do not reach a redirected stderr.** Pipe the output
  anywhere and `qCInfo`/`console.log` go to the journal instead, leaving only
  raw ffmpeg noise in the file. Use `QT_FORCE_STDERR_LOGGING=1` (the older
  `QT_LOGGING_TO_CONSOLE` still works but warns) or `journalctl --user -f`.
  **Nothing can be debugged without knowing this.**
- **`Video` is a convenience item that hides `mediaStatus`**, which is the
  one property a playlist turning on the end of a clip needs. Its `stopped`
  signal is not a stand-in: it fires when the source is swapped as well as
  when a clip ends, so a playlist turning from one video to another would
  have turned twice. `Main.qml` uses `MediaPlayer` + `VideoOutput` +
  `AudioOutput` instead, which is what `Video` wraps anyway -- same traps,
  same `onSourceChanged: play()`.

  **And a video told to loop for ever never reports an end.** `loops` has
  to drop to 1 while a playlist is waiting on it, or `EndOfMedia` never
  arrives and nothing turns.

- **`Video.autoPlay` starts the first file and no other.** Hand it a second
  source and it loads, reports nothing and sits there stopped. Measured: 24
  switches gave 1 start. `onSourceChanged: play()` is why switching works.
- **`select()` must stay silent when the choice has not changed.** Emitting
  `currentChanged` anyway makes QML hand the player the same URL, which stops
  it for good — a black screen with no error. Regression test:
  `staysQuietWhenTheChoiceIsTheSame()`.
- **project.json's tags are the author's genres and nothing else.**
  Measured against the workshop's own list for the same wallpaper:
  `["Sci-Fi"]` on the disk against `Wallpaper, Scene, Sci-Fi, Audio
  responsive, 1920 x 1080, Everyone` in the answer. Steam adds the asset
  type, the kind, the resolution, the age rating and whether it answers to
  sound, and none of those is written into the folder. So a wallpaper
  looked at in the library used to show a third of the tags it showed in
  the Steam tab. The keyless details call that finds the author brings the
  full list back with it, so it costs nothing extra -- and **anything
  asking "does it carry this tag" has to read that list**, or the Audio
  responsive row says "not stated" about a wallpaper whose own chips
  underneath say otherwise.

- **The library's filter is built from the workshop's tags, asked for in
  one question.** `GetPublishedFileDetails` takes every id in a single
  POST -- all 56 on this machine, answered in one go, no key -- so the
  window asks about the whole library the first time it is opened, and
  about anything that turns up later. project.json carries the genres its
  author ticked and nothing else: ten distinct tags on the disk against
  twenty-nine in the answer. The TAGS section went from one chip, which
  was put there by hand so the section would exist at all, to five that
  are really carried; Audio responsive ticked finds ten wallpapers where
  it used to find none, and Mature finds one where project.json calls
  every wallpaper Everyone.

  **Both lists together, never one instead of the other.** Measured the
  other way round: one genre here is in project.json and *not* in the
  workshop's answer, because an author can change what is published
  without the downloaded copy hearing of it -- replacing the disk's list
  cost that genre its chip, and the count of genres fell from ten to nine.

  **And whatever the chips are built from, the grid has to match on.**
  The filter offering a tag that the predicate cannot see is a chip that
  finds nothing, which looks exactly like a library with nothing in it.

- **"Audio responsive" is not "has a sound".** The tag means the
  wallpaper follows the music somebody else is playing -- right for all
  seven scenes here whose renderer says they listen -- and it was read as
  whether the wallpaper makes a sound at all: "not stated" over a
  wallpaper playing its own soundtrack. Valve's Czech for the tag,
  "Reaguje na zvuk", invites exactly that. So the panel has two rows,
  *Follows the music* off the tag and *Sound of its own* off the
  wallpaper: a scene's scene.json, read during the scan by
  `wallflux_scene_has_sound` -- the package's table and that one entry,
  45 ms for all 41 packages where reading them whole is hundreds of
  megabytes -- and a video's streams from the ffprobe that already asks
  its size. Six of ten videos and 19 of 41 scenes carry one. A web page
  is left without the row: it plays what it likes.

- **Who made an installed wallpaper takes two questions, not one.**
  project.json has no author in it at all -- the keys are `title`, `type`,
  `file`, `preview`, `tags`, `description`, `contentrating`, `version`,
  `visibility`, `workshopid`, `workshopurl` and `general`, counted across
  all of them. The folder's name is the workshop id, and
  `ISteamRemoteStorage/GetPublishedFileDetails/v1/` answers `creator` for
  it without a key; the profile page below turns that into a name and a
  face. Both live in `qml/steamprofile.js` as a `.pragma library`, so the
  library and the workshop ask once between them. **`XMLHttpRequest` works
  inside a `.pragma library`** -- checked with a throwaway `qml` run before
  the file was written, since a shared script cannot see the engine.

- **Steam's profile page served as XML needs no key**, which is the only
  keyless way to put a name and a face to the steamID64 a UGC answer
  carries. `GET steamcommunity.com/profiles/<id64>?xml=1` gives
  `<steamID>` (the persona name) and `<avatarIcon>` in CDATA; the Web API's
  `ISteamUser/GetPlayerSummaries` wants a key and is not used. Asked from
  QML with `XMLHttpRequest`, once per account, and the answers kept in a
  map -- a grid of fifty cards is often a dozen authors.

  **Putting the answer back has to be a new object.** Assigning the same
  map back to the property changes nothing anybody is bound to, so the
  name arrives and the panel goes on showing nothing. `Object.assign({},
  ...)` is why it appears.

- **One author's work is `CreateQueryUserUGCRequest`, not a search.**
  Steam's text search matches the title and the description and has no
  idea who wrote anything. The user query takes an `AccountID_t`, which is
  the bottom half of a steamID64, and `k_EUserUGCList_Published` with
  `k_EUGCMatchingUGCType_All`. Those numbers were read out of
  `/opt/steamworks-sdk/public/steam/isteamugc.h` rather than guessed, the
  same as every other number in `steamworks.cpp`, and
  `SteamAPI_ISteamUGC_CreateQueryUserUGCRequest` is exported by
  `libsteam_api.so` -- checked with `nm -D`. Measured against a running
  Steam: four pages of one author's wallpapers came back.

- **A theme icon on its own, with no button round it, is `IconImage` from
  `QtQuick.Controls.impl`.** There is no public type for it and no
  `image://theme` provider, and it wants `color` for the same reason the
  tab icons do -- untinted it keeps whatever the icon theme drew it and
  goes invisible in one of the two themes. The panel's rows use it; a
  `ToolButton` would have brought a hover and a click nobody wants.

- **The picker needs `transientParent: null`.** The wallpaper window is a
  background layer-shell surface that takes no input; a window belonging to
  it gets dragged down there and cannot be clicked.
- **`GNUInstallDirs` keeps sysconfdir under the prefix**, so with `/usr` the
  autostart file would land in `/usr/etc/xdg/autostart`, silently. Hence the
  `if(CMAKE_INSTALL_PREFIX STREQUAL "/usr")` in CMakeLists.txt.
- **The PKGBUILD must not sit in the repository root.** `makepkg` works in
  `src/` and `pkg/` beside itself, and this project has a `src/` with the
  program in it: `makepkg -C` from the root would empty it.
- **`makepkg` builds from git, not from the working tree.** Commit first,
  then `makepkg -f`, then `pacman -U`, or the package carries old code.
- **The wallpaper's own sound stream carries no pid.** Qt plays a video as a
  native PipeWire client, and the stream has no `application.process.id` at
  all -- only `client.id` and a `node.name` of `wallflux`. `othersPlaying()`
  counts a stream it cannot place as somebody else's, so the wallpaper heard
  itself, quietened itself a fraction of a second after starting, and stayed
  that way: software muting leaves the stream open and uncorked, so the
  reason never goes away. Matching on the application name is why the sound
  works at all. A web wallpaper is the other case and does carry a pid -- of
  a QtWebEngine child process, hence the walk up `/proc`.
- **`Video.muted` mutes in software, and `pactl` reports `mute: no` the whole
  time.** Do not read that flag to decide whether muting works. Record the
  sink instead: `parec --format=s16le --rate=16000 --channels=1 -d
  "$(pactl get-default-sink).monitor"` and look at the peak -- 0 is silence.
  Measured: the flag says one thing and the speakers another.
- **KWin 6 scripting has no `maximizedMode`**; it reads as `undefined`.
  Compare the window's `frameGeometry` with
  `workspace.clientArea(KWin.MaximizeArea, w.output, workspace.currentDesktop)`,
  which is the screen minus the panels -- 2560x1377 of a 2560x1413 screen
  here. Comparing against the whole screen misses every maximised window.
- **"Show desktop" changes neither a window's state nor its geometry.** The
  script sees nothing happen and goes on reporting a covered screen while the
  wallpaper is the thing somebody asked to look at. `workspace.showingDesktop`
  is `undefined` in the scripting API; KWin announces it on D-Bus instead, as
  `org.kde.KWin.showingDesktopChanged`, which is what `ScreenCover` listens to.
- **`loadScript` with a name KWin already has reads nothing.** It hands back
  the script that is loaded, so after a restart the compositor is left running
  one that calls a bus name nobody answers to any more. Unload first, always.
- **Chromium will not freeze a page that is on screen.** `lifecycleState`
  follows the view's visibility rather than the cover itself, which is why
  `onVisibleChanged` sets it.
- **A `Popup` already has a `locale`, and it is FINAL.** Declaring one on a
  component that derives from Popup does not shadow it or warn in passing:
  the file fails to load, and with it everything that uses the file, so the
  application comes up with no window at all and says only "the QML did not
  give us a window to show". The real reason is three lines further up --
  `Cannot override FINAL property`. `qmllint` passes it. Found by running
  the program, which is the only thing that showed it.
- **A window-wide `Shortcut` takes Escape before any popup does.** The
  picker used to hide itself on Escape, which meant that with a popup open
  Escape put the whole window away instead of closing the popup -- every
  one of them declares `Popup.CloseOnEscape` and never got the chance to
  use it. There is no Escape handler in this program now, on purpose: the
  key belongs to whatever is open, and the window is put away with the
  buttons in its corner.

  **And `CloseOnEscape` only works where the popup has the keyboard.**
  Setting the policy is not enough: a Popup does not take focus by being
  opened, so the key goes to whatever had it -- here the grid behind it.
  Measured with `activeFocus` on all three of the playlist popups: Save
  closed on Escape and the other two did not, because Save puts the cursor
  in its name field on the way up and they have nothing to put it in.
  `focus: true` on the Popup is what the other two were missing.

- **Do not translate a drop down's model.** It is a binding, so
  `retranslate()` rebuilds the list and the box drops back to its first
  row -- taking the choice with it. Measured: Order set to Random and the
  transition set to Fade to black both read blank after a language switch,
  while the three number boxes beside them survived, and a playlist saved
  then was saved in order with no transition and nobody told. The model is
  the words as they go into the file, which never change; `displayText`
  and the delegate are what translate, out of a function that takes the
  language it never reads. **Holding the chosen word in a property of your
  own and binding `currentIndex` to it does not work** -- a ComboBox
  assigns `currentIndex` itself when somebody picks a row, which breaks
  that binding for good.

- **Replacing a `Menu`'s background takes its width with it.** A Menu has
  no width of its own: it comes from the background it is given, and a
  plain `Rectangle` has none. Rounding the corners of the playlists menu
  that way left it twelve pixels wide -- its padding and nothing else --
  with its items sized to nothing and no text in sight, over two saved
  playlists that were plainly there. `contentWidth` reads -1 when this has
  happened. Say `implicitWidth` out loud, as the sort menu beside it
  already did, which is why that one never broke.

- **A `GridLayout` says nothing when a row loses a cell.** Taking the
  "change wallpaper" drop down out left its label behind, so every row
  under it sat one column over -- words in the right-hand column, drop
  downs and tick boxes in the left, and the whole panel looking shuffled
  rather than broken. It reads as four separate complaints about
  alignment, and it is one odd cell. Count the cells.

- **A `Label` does not dim itself when it is switched off.** `enabled`
  reaches the mouse and not the colour, so a line beside a greyed-out tick
  box sat there in full black. It needs `opacity` said out loud.

- **A transition needs the old wallpaper after it has stopped being the
  wallpaper, and there is no second player to keep it in.** One video and
  one web view is all `Main.qml` has. What is held instead is a *picture*
  of the old one, grabbed the instant before the switch and drawn over the
  new one while it fades -- `grabToImage` catches a playing video frame,
  checked before any of this was built. So: `leaving` is that picture and
  `veil` is a black rectangle, both over everything and both nothing at
  all until a transition runs.

  - *Fade to black* needs no picture: the veil goes up, the wallpaper
    changes behind it, the veil goes down. Measured at the halfway point:
    the screen grabs as rgb 0, pure black.
  - *Fade* is the picture at full opacity fading to nothing over the new
    wallpaper. Measured at the halfway point: `leaving.opacity` 0.51 and
    the grab is plainly two wallpapers at once.
  - *None (reduce flicker)* is the same picture held for 300 ms and then
    dropped, which covers the black blink while the next video finds its
    first frame. No fade -- it is the transition that is meant to look
    like none.
  - *Random* was offered and is not any more, but a playlist written down
    while it was still says `random` and the window still plays it --
    rolled once per change and held for the length of it, so the end of a
    transition matches its start. Nothing had to be migrated, which is the
    whole reason a transition is a word and not a row number.

  **The grab never comes back when the window is not being drawn**, which
  is exactly what a covered screen is -- so there is a 400 ms rescue timer
  that changes the wallpaper without a transition. Without it a playlist
  behind a maximised window would simply stop changing. Measured: covered
  the whole time, the wallpaper still turned.

  **A transition is written down as a word**, not as the number of a row
  in a drop down: `""`, `soft`, `none`, `random`, `fade`, `black`. The
  rows have been reordered twice already and a number in a file would have
  meant something else each time.

- **A `Popup` that does not fit does not scroll, it overflows.** The
  workshop's filter holds fifty-nine chips, which is taller than the window
  on anything but a very tall screen: the bottom section and the footer ran
  off the edge and Clear could not be reached at all. It takes a height it
  is told there is room for and a Flickable inside. Told, because a Popup
  is not an Item and has no `Window` attached -- asking gives undefined and
  the height comes out as nothing, which collapses the popup to its footer
  floating over the page with no background.
- **A sentence built in C++ with `tr()` is built once**, in whatever
  language was installed at that moment, and stays that way.
  `QQmlEngine::retranslate()` refreshes bindings that translate, and a
  string handed over from C++ is not one. The heading changed with the
  language and the reason under it did not. So C++ reports *which* reason
  and the window chooses the words. **And a binding that gets its words out
  of a JavaScript function is not refreshed either** -- the function has to
  take the language as an argument it never reads, so that reading it is
  what makes the binding rebuild. It looks like a pointless parameter and
  it is not.
- **A `MouseArea` takes hover away from the one under it**, so anything
  shown *because* the pointer is there and holding a mouse area of its own
  flickers several times a second: the tick box on a card appeared, took
  the hover, which hid it, which gave the hover back. A `HoverHandler` on
  the card answers "is the pointer anywhere on this" without taking it
  from anybody.
- **`Row` is a QtQuick type.** An inline `component Row:` shadows it
  silently and whatever used the real one breaks somewhere else entirely.
  Named `DiscoverRow` for that reason.
- **`QStringLiteral` is UTF-16 and a compiled `.qm` is compressed**, so
  plain `strings` finds neither. Checking what actually went into a built
  binary needs `strings -a -e l` for the first and nothing at all for the
  second -- look for `:/i18n/wallflux_cs.qm` instead. Both of these were
  read as "the package is missing my change" before the method was
  suspected.
- **`qmllint` is not in `PATH`** on this machine; it is
  `/usr/lib/qt6/bin/qmllint`.
- **A `GridView` puts `currentIndex` back to nought whenever its model
  changes**, and this grid's model is a fresh array every time anything
  about any wallpaper changes -- a video answering with its size, Steam
  finishing a download, a card being looked at. So a row number is no
  answer to "which card is the ring on": it was handed back to the first
  card a moment after every click, which is why only the first wallpaper
  could be picked and why the arrow keys went nowhere. The id is the ring
  now (`picker.ringOn`), the grid's own `keyNavigationEnabled` is off and
  the four arrows are answered by `grid.walk()`, which scrolls to what it
  lands on. Putting the row back after each rebuild was tried first and is
  a race: the grid resets itself when it gets round to it, which is
  sometimes after the repair has run -- measured, with the ring landing on
  a card nobody had pressed an arrow towards.

  **And looking at a card rebuilt the whole grid.** The details answer was
  assigned into `workshopTags` even for an id already in it, and `shown`
  is built out of that map -- so one click destroyed every delegate and
  put the view back to its first row. Only what is new goes in now.
  Measured: the ring ten cards down, and the grid stayed where it was.

- **The grid read "what was played when" whether or not it sorted by it.**
  `shown` is a binding, so it depends on everything it reads -- and
  putting a wallpaper on the desktop changes `Wallpapers.recent`, which
  rebuilt the list and with it every card. A rebuild puts the view back to
  its first row, so a double click somewhere down the grid started the
  wallpaper and threw the window up to the top of the library. `recent` is
  read inside its own `case` now, where sorting by it happens. Measured:
  the view stayed where it was, 3 757 pixels down.

- **A view that was scrolled when the window changed size is left past the
  end of the grid, and in that state it draws nothing and will not be
  moved.** Measured on maximising with the grid scrolled down: 2 588 down
  a content height of 1 491, no cards drawn at all, and only a scroll of
  somebody's own brought them back. `positionViewAtIndex`,
  `returnToBounds`, `forceLayout` and writing `contentY` by hand were each
  measured doing nothing there -- and this is not something the ring
  introduced: it reproduces with the repair switched off, so it has been
  waiting in there for a grid that was scrolled when something resized it.
  `grid.positionViewAtBeginning()` **first**, and then
  `positionViewAtIndex` -- rewinding gives the view somewhere to count
  from, after which it goes where it is asked. Both in the one function,
  so nothing is drawn in between.

  **So one place does all of it.** `grid.keepTheRingInView()` is what the
  arrows, the panel opening, every model rebuild and every resize go
  through, and the grid asks for it on `onModelChanged`, `onWidthChanged`
  and `onHeightChanged` -- through a 150 ms timer, which is what puts the
  three of them together when a window is maximised and they all arrive at
  once.

  **And the rewind only when the view is really stranded.** Rewinding
  destroys every delegate on screen, and what comes back are new `Image`s
  loading asynchronously -- so a repair one case in many needs was the
  whole grid visibly blinking on every resize, which is what maximising a
  window and putting it back looks like. In bounds, `Contain` scrolls the
  least it can and does nothing at all when the card is on screen already.
  Measured on open, on maximising and on coming back: contentY 0 against a
  content height of 4 176 and then 1 491, and not one rewind.

- **A double click cannot be counted by the card it lands on.** A rebuild
  between the two presses destroys the delegate, so the second press
  reaches a different object which sees a first click -- and with the
  model rebuilding as often as it did, a double click on anything but the
  first card was two single clicks. `Picker.qml` counts it instead, over
  a 400 ms timer, which also waits out the reflow: opening the panel takes
  the grid from five columns to four, and a card that moves out from under
  the pointer cannot be clicked a second time.

  **And a double click never reaches that counting.** Qt delivers the
  second press as `doubleClicked`, not as a second `clicked`, so it goes
  straight to `apply()` and `chooseOrApply` is none the wiser -- the click
  it was counting is still running, and four tenths of a second after the
  wallpaper started, the panel did what that first click had decided. With
  the panel open on the card being double clicked, that decision was
  "shut", so playing a wallpaper closed the panel by itself. `apply()`
  stops the timer, which is what makes a double click mean one thing only.
  Both paths have to exist: where a model rebuild destroys the delegate
  between the presses, the second press lands on a new object which sees a
  first click and `doubleClicked` never comes at all.

  **Which is why a double click can only be ignored by waiting.** The
  workshop has nothing to play, so a double click there is to do nothing
  at all -- and since the first `clicked` arrives either way, the only way
  to take it back is not to have acted on it yet. A click out there waits
  the four tenths out and `doubleClicked` cancels it. Measured with
  `QtTest`, which can deliver what a mouse cannot: one click gives one
  toggle of the panel, a double click gives none, and two clicks 380 ms
  apart give one -- Qt no longer calls that a double click, and the second
  click rewinds the wait rather than being counted. The wait cannot lose
  the race, either: this machine's double click interval is 400 ms
  measured from the first *press*, and the timer starts at the first
  *release*, so the cancel is always that much ahead of it.

- **A ring round a card is a second frame, and it takes the name in with
  it.** The preview already has a band, so an arrow-key highlight drawn as
  a rectangle over the whole card put two frames round one picture and a
  box round the words under it. The band changes colour instead. Nothing
  is lost by borrowing it: a wallpaper that cannot be played is drawn at
  four tenths and says so that way too.

- **A card in the grid is a square, and the whole preview is sixteen by
  nine.** Both, in different places. The grid crops to a square and puts
  the name inside the picture over a gradient, which is what Wallpaper
  Engine's own picker draws and what fits five cards across a window
  rather than three; the details panel and the playlist strip keep the
  wallpaper's own shape, because the panel is the one place a preview is
  looked at whole and a square there would crop a third of it off for
  nothing. The price was said out loud before it was chosen: a square
  throws away the sides of every preview in the grid.

  **A `PreviewTile` takes the shape it is given.** It used to ask for
  nine sixteenths of its *own* width whatever height it was handed, so
  `Layout.preferredHeight` did not shorten the picture -- it let the name
  underneath be drawn over the bottom of it, with no warning anywhere.
  The frame fills the tile now and the name is inside the picture, so
  there is nothing left to overflow and both of the panel's two numbers
  are read.

- **A child's scale goes into the texture an effect reads; the source
  item's own does not.** The preview leans in a twentieth under the
  pointer, and the picture is drawn by a `MultiEffect` that rounds its
  corners -- so scaling the thing handed to `source` would have been
  thrown away with the rest of that item's transform. The Image sits
  inside a box with `clip: true` and the box is the source: the zoom is a
  child's, the crop is the box's, and the card itself does not move.
  Checked by drawing one at 1.0 and one at 1.3 side by side and looking at
  the grab, which is the only way this one shows itself.

- **A gradient stop of `"transparent"` is a transparent *black*.** The
  name's gradient fades the card's own base colour up out of the picture,
  and ending it at `"transparent"` interpolates through black on the way
  -- a dark bruise across the bottom of every card in the light theme.
  Each stop names the base colour and changes only the alpha.

- **The details panel scrolls from the author's row down.** The picture
  and the name stay put; everything under them is in the Flickable. Two
  wrong answers were tried first: a floor under the list
  (`Layout.minimumHeight`) overflows the column and cuts the button off the
  bottom, and scrolling the picture too means a hundred and sixty pixels of
  photograph in the content, so there is always more than fits and the bar
  never goes away. With the picture outside it, a maximised window has
  nothing to scroll and says so.

- **Setting `policy: ScrollBar.AsNeeded` on `Scroller` does nothing on its
  own.** Fusion does "as needed" by fading the `contentItem` *it* draws,
  and Scroller replaces that contentItem -- so the policy is set, the
  fading is gone with the thing that did it, and a full-length bar is drawn
  down the edge of content that fits. Scroller does the hiding itself now.
  Found in a maximised window, where there was plainly nothing to scroll
  and a bar saying there was.

- **A `ToolButton` with nothing round it is a button nobody sees.** Fusion
  draws no background until it is pointed at, so a magnifier beside the
  author's name read as decoration and was reported missing. The whole row
  is a `Button` with an edge on it now.

- **A picture four tenths there over a dark window is a pale grey room.**
  The window is tinted by whatever the panel is showing -- the preview,
  blurred past recognition, half the colour taken out, faded in over six
  tenths of a second. At the opacity the effect is drawn with and nothing
  else, a bright preview lifted the whole window and the small print under
  the heading went with it. The window's own colour goes back over it, three
  tenths of it, which is what the program this was copied from does too.
  Found by drawing the three strengths side by side and reading the words
  on each.

- **A property here can shadow a caller's id.** `PreviewTile` grew a
  `card` property for the colour its name fades into, and a
  `WallpaperCard`'s own id is `card` -- so every binding written inside
  that tile in `WallpaperCard.qml` read the property instead of the card.
  `qmllint` says `id-shadows-member` and names both; nothing at runtime
  does. It is `backing` now. Run `qmllint` over **every** qml file, not
  the two or three a change was in: this one went in with the squares and
  was found a day later, in a file that had not been touched.

- **A `MultiEffect` mask draws nothing unless the mask has
  `layer.enabled: true`.** Not a dimmer picture or a square-cornered one:
  nothing, every card blank. A `Rectangle` is not a texture until it is a
  layer, and the effect quietly has nothing to work with. Looked at side by
  side, with and without, which is the only way this one shows itself.
- **Wallpaper Engine's `locale/` is a translation table, not the list of
  tags.** The install -- `steamapps/common/wallpaper_engine` -- carries
  `locale/` in **36 languages**, `ui_cs-cz.json` among them, each with 103
  keys under `ui_workshop_tags_*`. Matching `ui_en-us.json` against another
  of them by key gives English name -> translated name for nothing, and the
  names are Valve's own: `Cartoon` is "Skica" and `Vehicle` is "Dopravní
  Prostředky". **So no tag name is ever typed into the source or into
  `wallflux_cs.ts`, in any language.**

  **Look a tag up by its key, not by its English value.** The key is
  `ui_workshop_tags_` followed by the tag lowercased with every run of
  non-alphanumerics turned into one `_`: `Sci-Fi` -> `..._sci_fi`,
  `MMD (Miku-Miku Dance)` -> `..._mmd_miku_miku_dance`. Measured over the
  25 tags the installed wallpapers and a live query actually carry, the key
  finds 19 and the value only 17, and what the key adds is the two that
  matter: `Unspecified` is keyed `..._unspecified` but *valued*
  "Unspecified genre", so only the key reaches "Nespecifikovaný žánr", and
  `3840 x 2160` is valued "3840 x 2160 - 4K". Nothing the value finds is
  missed by the key. **Do not use the rule backwards** -- value to key
  fails for 38 of the 103, because `asset_type_*` and `dual_*` keys carry a
  prefix their value does not and the resolutions carry a `- 4K` their key
  does not.

  **What the file is not is the vocabulary.** An earlier note here said the
  whole workshop vocabulary was on the disk; it is not. Six of those 25 are
  absent however they are looked up:

  | what Steam really sends | what `locale/` has |
  | --- | --- |
  | `Everyone`, `Questionable`, `Mature` | `G - All ages`, `PG13 - Parental guidance`, `R - Mature` |
  | `Media Integration`, `Puppet Warp`, `Video Texture` | nothing at all |

  Age rating is a different vocabulary outright and three tags are newer
  than the snapshot. So the list of tags comes from the data --
  `project.json` for what is installed, the query's answer for the rest --
  and `locale/` is only laid over it, falling back to the raw tag where
  Valve has nothing.

- **Which tags are a genre is nowhere on the disk.** Resolutions can be told
  from the key (`_1920_x_1080`) and `asset_type_*` and `visibility_*` have
  prefixes, but the 25 genres are not grouped in `locale/` or anywhere under
  `ui/` -- both were searched. That one list is written out in the source,
  and it is the only tag list that is.
- **What project.json does and does not carry**, counted across all 53:
  `tags` and `preview` in every one, `type` and `file` in 51, `contentrating`
  in 49 and always "Everyone", and **no resolution anywhere**. So the tag,
  rating and kind filters are free, and the resolution one is not.
- **Steam's answers have to be fetched, not waited for.** There is no thread
  of Steam's in this process: a query is sent, and the result sits in a queue
  until `SteamAPI_ManualDispatch_RunFrame` and `GetNextCallback` are called
  -- here on a 100 ms timer while a question is outstanding. A finished call
  arrives as callback 703, whose payload says which call and what the real
  callback number is; 3401 is a finished UGC query, read with
  `GetAPICallResult`. Every callback must be freed or the queue stops.
- **The shape of Steam's structs is measured, not guessed.** Valve's headers
  are not in this repository, so `steamworks.cpp` writes out what it needs --
  and every offset in it came from compiling a throwaway program against
  those headers once and printing `offsetof`. The `static_assert`s in that
  file are that measurement: `SteamUGCDetails_t` is 9764 bytes with the title
  at 24 and the tags at 8183, a callback envelope is 20 bytes with the
  payload pointer at 8, and the structs are packed to 4. Get one wrong and
  the build stops rather than reading somebody else's bytes.
- **`SteamAPI_Init` is not an exported symbol** -- it is inline in Valve's
  header and calls `SteamInternal_SteamAPI_Init`. Through dlopen what there
  is to call is `SteamAPI_InitSafe`, with `SteamAPI_GetHSteamUser` and
  `SteamInternal_FindOrCreateUserInterface` to reach ISteamUGC, and
  `SteamAppId` in the environment before any of it or the library asks Steam
  to relaunch the program. Measured against a running Steam: the library
  opens, init succeeds and the UGC interface answers.
- **The workshop answers without an API key, up to a point.** Measured:
  `ISteamRemoteStorage/GetPublishedFileDetails/v1/` is a POST that takes
  published file ids and needs no key at all -- it gives the title, the tags,
  the file size, the subscriber count and a direct `preview_url`. What it
  cannot do is *find* anything. `IPublishedFileService/QueryFiles`, which
  can, answers 403 without a key. The public browse page at
  `steamcommunity.com/workshop/browse/?appid=431960` gives 30 ids a page with
  no key and takes `browsesort` and `requiredtags[]`, so ids out of the page
  and details out of the keyless endpoint is a route that keeps no secret
  anywhere.
- **Subscribing and downloading from outside Steam is possible, and the
  only route to it is the Steamworks SDK.** Not through the URL handler: the
  client knows 28 `steam://` verbs -- read out of `steamui/sp.js` -- and not
  one subscribes or downloads, and `steam://url/` has 17 targets and none
  either. Through `ISteamUGC`, which is what `SubscribeItem` in
  `linux64/steamclient.so` is. A process has to declare itself as app 431960
  to use it.

  **It is done on this machine already.** `linux-wallpaper-engine-ux`
  (jagrat7/linux-wallpaper-engine, MIT, Electron) ships `steamworks.js` with
  Valve's `libsteam_api.so` inside its `app.asar` and calls
  `workshop.subscribe`, `workshop.download`, `workshop.downloadInfo`,
  `workshop.installInfo` and `workshop.getAllItems` -- the last with
  `rankedByTrendDays`, so **browsing the whole workshop goes the same way**,
  with no web API key and no page to scrape either.

  **What stops WallFlux doing the same is the licence, not the mechanism.**
  GPL-3.0 code cannot link a proprietary library without an exception written
  for it. That
  project is MIT and can; this one would need either a linking exception for
  the Steamworks SDK, a different licence, or the Steam part in a separate
  program of its own. Until one of those is decided: ids off the public
  browse page, details off the keyless endpoint, and Download opens the
  item's page for somebody to press Subscribe once -- after which the folder
  watcher makes the rest invisible.
- **Steam unsubscribes and then leaves the folder exactly where it is.**
  `UnsubscribeItem` works and says so -- the call answers callback 1315
  with `k_EResultOK`, `GetNumSubscribedItems` drops by one and
  `GetSubscribedItems` no longer carries the id -- and the wallpaper is
  still on the disk, still in the grid, and a Refresh finds it again.
  Measured with the program running and again a minute after it had quit;
  Steam cleans up in its own time, if at all. So giving a wallpaper back is
  two halves: Steam lets go of it, and `WallpaperCore::discard()` takes the
  folder. Deleting without unsubscribing would be worse than useless -- the
  next thing Steam does is fetch it again.

  **`GetItemState` is not the way to ask whether something is subscribed
  here.** It answered 4 (installed) and nothing else for an item the acf
  and `GetSubscribedItems` both called subscribed, before and after the
  call. The subscribed list is what to read.

- **Asking Steam's CDN to resize a preview is a trap**, and it looks like
  the obvious saving. A preview is a few hundred kilobytes and a card draws
  it 240 across, and Steam's own pages put
  `?imw=...&imh=...&ima=fit&impolicy=Letterbox` on the very same URLs. For
  a photograph it is real -- 296 kB becomes 11 kB. But most of the
  workshop's previews are animated, and for those the picture is
  **re-encoded when it is asked for**: a size nobody has asked for before
  is made on the spot. Measured over six previews, 0.7 to 7.9 seconds each,
  where the file as stored came back in 0.03 to 0.09 -- and the resized
  copy was not even smaller, one of them 1 099 kB against 924 kB stored.
  Sixteen stored previews, eight at a time: 8 MB in 0.19 s. So the bytes
  were never the problem on a connection like this one; the re-encoding
  was, and it is what made a page take tens of seconds. Take the URL as
  Steam gives it. On a slow line the sums would come out differently for
  the photographs, and that is a comment and a URL away.
- **Steam will not page for ever.** Measured against Trending: page 1001
  comes back with its fifty, page 1024 is refused outright with result 25,
  and so are 2000 and 64 929. The total it reports -- three and a quarter
  million -- divided by the fifty it answers with is 64 929 pages that do
  not exist. The grid stops at a thousand, which is where Wallpaper Engine
  stops too.
- **A Popup closes on the press, not the click, so a button cannot toggle
  one** without being told to. The press lands outside the popup, which
  closes it, and the click that follows opens it again -- so the menu
  flickers and stays. `Popup.CloseOnPressOutsideParent` keeps a press on
  the button that owns it from counting as outside, and then
  `opened ? close() : open()` does what everybody expects.
- **Asking ffprobe during the scan is a trap.** It is 53 milliseconds a
  video, which sounds free until the constructor of WallpaperCore scans the
  real machine on every test that builds one: the suite went from 0.03 to
  3.2 seconds. It is asked in the background after each scan instead, and
  each answer fills itself in and says so.
- **Killing a QProcess inside `qDeleteAll` of the list it is in is a
  segfault.** Destroying a running one waits for the program to go and
  delivers `finished` on the way, and the handler for that edits the list
  being deleted. Take them out of the list and disconnect them first.
- **Every wallpaper here carries the same date.** All 53 folders were
  downloaded by Steam in one go, and their creation time and last change are
  the same minute -- so ordering by "date added" barely reorders anything on
  this machine. It is not broken; there is nothing to sort by.
- **A `static` member declared after `signals:` is a signal as far as moc
  is concerned.** It generates a call through `this` for it and the build
  stops with "invalid use of 'this' outside of a non-static member
  function", pointing at generated code rather than at the header. The
  access specifier is what decides, not the keyword: put it under `public:`.

- **Say it with the control, not with a line of small print under it.**
  The three number boxes had a sentence beneath them explaining that five
  seconds was the least however short they were set -- because nought was
  typeable and the floor was invisible. Putting the floor into
  `asClock()`, which the boxes are written back from, made the sentence
  pointless: the boxes read five by themselves. One line of C++ for one
  fewer line of apology.

- **Clamp the time, not one of its units.** The settings popup's three
  boxes take two digits each and carry what is over -- sixty seconds is a
  minute -- and the first way of doing it carried unit by unit, capping
  the hours at 23 on the way. Typing ninety seconds at 23:59 then gave
  23:00:30: *less* than before, because capping the hours threw away the
  minutes under them. Added up as one number of seconds, clamped once and
  handed back as a clock, it comes out 23:59:59. The arithmetic lives in
  `Playlists::asClock()` rather than in QML, because a rule in QML is a
  rule no test can reach.

- **Fusion paints the strip a SpinBox's arrows sit in whether or not there
  are any.** `up.indicator: null` and `down.indicator: null` take the
  arrows away and leave a grey block hanging off the end of the number,
  lit up whenever the box has the keyboard, with nothing to click. The box
  needs a background of its own.

- **A two-character field that is full refuses the next keystroke**, and
  the number boxes are two characters wide: a box reading 15 could not be
  typed over at all, only selected by hand first. They pick their whole
  number out when they are reached, so typing replaces it. **Through
  `Qt.callLater`**, because a click moves the focus and *then* sets the
  cursor, which would undo a selection made on the spot. A second click
  in a box that already has the keyboard puts the cursor where it was
  asked for, which is how a field is edited rather than replaced.

- **A `CheckBox` with no words still leaves room for them.** Its padding
  and the gap before the label are both still there, so one aligned to the
  right of a layout hangs over the edge it was aligned to. `padding: 0`
  and `spacing: 0`.

- **The playlist timer is tested by letting it run.** `tst_playlists`
  waits five real seconds for a turn, which is most of the suite's time
  and is there on purpose: the one way to get this wrong is the
  multiplier, seconds read as minutes is a playlist sixty times too slow,
  and no unit test of the arithmetic catches that.

  The playlists menu is the same lesson from the other side: which
  playlist is playing is `Playlists`' own business, so its items are not
  checkable at all -- the tick is a Label reading the singleton, and there
  is no control state to break.

- **A Controls item cannot both be told what to be and asked what it is.**
  Caught twice now. The sort menu was the first; the second was the
  playlist's `SpinBox`, written as `value: picker.playlistMinutes` with
  `onValueModified: picker.playlistMinutes = value`. The box read 1 and
  the property was still 15, so a playlist set to a minute was saved as a
  quarter of an hour and sat there changing nothing. The control holds the
  number and the window reads it off when it saves. Bind `checked` to a property and set that property from
  `onToggled`, and the click breaks the binding it was reading. The sort menu
  holds its own state instead and the window reads it off the items; the
  filter chips are a rectangle and a mouse area, which have no state of their
  own to fight with.
- **A default that lives on a control is a default that a restyle can
  lose.** Rewriting the sort menu dropped `checked: true` from Ascending and
  turned the whole grid back to front -- silently, since an order is still an
  order. Found by looking at the picture, not by any test.
- **`autoExclusive` takes in every checkable sibling.** The sort menu's
  direction shares a parent with its four keys and would have joined their
  group; a `ButtonGroup` naming its members is what keeps them apart.
- **Switching language needs `QQmlEngine::retranslate()`.** Installing the
  translator alone changes what is built next and nothing already on screen.
  `qt_add_translations` compiles the catalogue into the binary under `/i18n`,
  and `qt6-tools` has to be in the package's makedepends for it.
- **`icon.color` tints whatever it is given, glyph or picture.** The four
  monochrome tab icons need it, or they stay whatever colour the system icon
  theme drew them and go invisible in one of the two themes -- this machine's
  theme draws them light, so the light theme is the one that loses them.
  Steam's own logo must *not* have it: tinted, it comes out as a plain white
  disc. It is also the one icon here that comes from hicolor rather than
  Breeze, installed by Steam itself.
- **A `TabBar` told to fill the width shares that width out between its
  tabs**, which turns five icons into five wide slabs. Left to its own width
  inside a rectangle that fills instead, the tabs stay the size they are --
  and `implicitWidth` is worth setting on an icon-only tab, which otherwise
  comes out two dozen pixels wide.
- **Fusion leaves a `ToolButton`'s `icon.color` unset, so its icon is drawn
  in nothing at all.** A `Button` sets it and shows up; a `ToolButton` does
  not, with or without `flat`, `display` or an explicit `icon.width`. Set
  `icon.color` on every one of them. Measured one variant per row.
- **`weather-clear` and `weather-clear-night` are 48-pixel applet icons.**
  At 16 all that survives is a white disc. `brightness-high` and
  `brightness-low` are the action-sized pair and read properly.
- **project.json carries no resolution.** Checked across all 53 of them: the
  keys are `file`, `preview`, `title`, `type`, `tags` and housekeeping.
  `ffprobe` reads it off the video instead, in 53 milliseconds.
- **`qt6-wayland` does not exist on CachyOS**; the Wayland plugin lives in
  `qt6-base`.

## Known limitation, deliberately accepted

While a wallpaper is playing, our window covers whatever plasmashell draws
on the desktop — icons and widgets. Measured twice (13 and 15 October): on
Plasma the wallpaper and the icons are one opaque surface, and there is no
"between" to slip into. The only real fix is a Plasma wallpaper plugin.
**Do not go back to KWin stacking tricks**; they were measured and lead
nowhere. On wlroots compositors (Hyprland, sway, niri, labwc) layer-shell
alone is enough and this problem does not exist.

## How to check a change

```sh
cmake --build build && ctest --test-dir build --output-on-failure
/usr/lib/qt6/bin/qmllint -I build/ qml/Main.qml qml/Picker.qml
cargo test --offline     # the scene renderer, when crates/ was touched
```

A change to the renderer is checked against the whole library, not one
wallpaper: `tools/sweep.sh <dir>` before and after, then `cmp` the two
directories picture by picture. Every picture that moved has to be one
the change meant to move -- and looked at, because "different" is not
"better". Fixing a shader once turned a Space Marine's armour white.
The sweeps so far are in `~/WallFlux-ukazky/<date>/`.

Anything about stacking or what is on screen has to be measured, not
reasoned about:

- **Stacking order**, without touching the mouse: write a KWin script that
  prints `workspace.stackingOrder`, load it with
  `qdbus6 org.kde.KWin /Scripting loadScript <file.js> <unique-name>`, run it
  with `... /Scripting start`, read the output with `journalctl --user`.
  Setting `workspace.activeWindow` to the plasmashell desktop window is what
  a click on the desktop does.
- **Whether anything is actually coming out of the speakers**: record the
  default sink's monitor with `parec` and look at the peak, as above. Ears
  and `pactl`'s mute flag both lie here, in opposite directions.
- **Driving the sound behaviour without touching the mouse**: `qdbus6
  org.kde.kglobalaccel /component/kwin invokeShortcut "Show Desktop"` takes
  every window off the wallpaper and puts it back, which is a covered screen
  and an uncovered one; `paplay` on anything in `/usr/share/sounds` is
  another program using the speakers. Both states can be read back from
  `QT_FORCE_STDERR_LOGGING=1 QT_LOGGING_RULES="wallflux.*=true"`.
- **A picture of the picker**, without touching the mouse and without the
  window ending up behind something: open it with
  `busctl --user call <name> /StatusNotifierItem org.kde.StatusNotifierItem
  Activate ii 100 100` (the name is the one of this program's two bus names
  whose `busctl --user tree` shows `/StatusNotifierItem`), bring it to the
  front with a KWin script that sets `workspace.activeWindow` to the window
  whose caption contains "choose a wallpaper" -- which also prints its
  geometry, so the screenshot can be cropped to exactly the window --
  then `spectacle -b -n -f -o <file.png>`.
- **A picture of the picker without raising it**: give the window
  `onVisibleChanged: if (visible) grab.restart()` and a timer that calls
  `picker.contentItem.grabToImage(r => r.saveToFile(...))`. The timer has to
  be started from the window becoming visible: at component creation there is
  nothing on the screen to grab and the callback never comes.

  **It needs the window to be drawing.** `grabToImage` returns true and then
  the callback simply never arrives if the window is fully covered -- by
  another window, or by the lock screen -- because the compositor stops
  asking it for frames. Measured, after half an hour of wondering why a file
  never appeared: the session had locked. So this is the polite recipe, not
  the reliable one; when it goes quiet, check what is in front of the window
  before suspecting the code.
- **A picture of one piece of QML**, without the application at all:
  `item.grabToImage(function (r) { r.saveToFile("<file.png>") })` in a file
  run with `/usr/lib/qt6/bin/qml`. Several variants of the same thing side
  by side in one window is how both of the Qt traps above were found; both
  look like working code and neither says anything when it does not work.
- **A picture of the desktop** with the windows out of the way:
  `qdbus6 org.kde.kglobalaccel /component/kwin invokeShortcut "Show Desktop"`
  (again to undo), then `spectacle -b -n -f -o <file.png>`.
  `workspace.showingDesktop = true` from a script does **not** work.

## How a release is made

The number lives in two places and both have to say the same thing:
`project(WallFlux VERSION ...)` in CMakeLists.txt and `pkgver` in
packaging/PKGBUILD. Then, in this order:

1. Write the entry in CHANGELOG.md under a heading for the new version,
   moving anything from Unreleased, and fix the compare links at the bottom.
2. Commit the number and the entry together.
3. `git tag -a v<version>` with a message that says what the release is.
   **With the `v`** -- v0.1.0 is tagged that way and two conventions in one
   repository is one too many.
4. `git push origin main && git push origin v<version>`.
5. `gh release create v<version> --notes-file <the changelog entry>`, with
   the built package attached if there is one.
6. `cd packaging && makepkg -f`, which builds from the tag now that it is
   committed, then `sudo pacman -U wallflux-<version>-1-x86_64.pkg.tar.zst`.

0.2.0 was tagged before CHANGELOG.md existed, so its tag does not contain
the file. Everything after it should.

## Open, in the order they are likely to be taken

1. **Displays and Settings**, the two remaining tabs. Nothing has been
   asked for in either of them yet.

2. **The tray icon itself.** Today it is `preferences-desktop-wallpaper`
   from the icon theme. A drawn one has been asked for, without any hurry.

3. **Scene wallpapers, in the order agreed on 27 October:** the music the
   scene answers to, its JavaScript and the mouse are all done. Next are
   the user properties the Settings tab is for -- their starting values
   are already read from `project.json`, and a script declares more of
   them with `createScriptProperties`.

### Small things seen and left

- What a scene's scripts do not reach yet: text (11 scripts -- the
  clocks in Future Clock, Time Reactor and Dark Core; text is not drawn at
  all), an effect switched on and off, a particle override other than the
  rate, an object hidden in scene.json that a script would show -- it is
  never loaded -- and the children of a particle system whose parent a
  script moves, which were placed once from where the parent started.

- A saved playlist still cannot be renamed. Saving under a name that is
  taken replaces it, which covers changing one; Load is where one is thrown
  away.
- Wallpaper Engine's Load and Save dialogs each carry a keyboard shortcut
  field. There is nothing here that could listen for one, so neither is
  drawn, and keyboard shortcuts are not going to be taken on.
- `Clear selection` empties what is ticked and nothing else. Checked by
  counting the chips of every section before picking, after picking and
  after clearing: ten genres, one tag, four picture sizes, unchanged
  throughout. The lists come from the whole library and from `Tags`, never
  from what is left after filtering, so there is nothing there to lose.
- The volume for a video went with the panel that used to be drawn over a
  preview: the only sound control now is the one in the footer, which is on
  or off. Nobody has asked for it back.
- The COMPATIBILITY section in the library's filter is a placeholder,
  shown and switched off. It is for when a wallpaper fetched from the
  workshop has been tried and found to work or not, and nothing records
  that yet.
- `SteamAppId=431960` in the environment makes the window's resource class
  `steam_app_431960`, which is confusing when reading `workspace.windowList()`.
  Harmless; worth knowing before chasing it.
