# WallFlux

Plays the Wallpaper Engine wallpapers somebody already owns, on Linux and
Wayland, without Windows and without Wallpaper Engine running. C++20, Qt 6,
GPL-3.0-or-later, eighteen files and about two thousand lines. Target
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
- **Sound that gets out of the way by itself.** The wallpaper is quiet while
  another program is using the speakers, and quiet and stopped while a window
  covers the screen; both come back on their own. Neither is a setting. The
  one switch there is turns the wallpaper's own sound off altogether.
- **A tray icon** that opens a grid of every installed wallpaper and plays
  the one that is clicked.
- **Noticing what Steam downloads.** The workshop folders are watched, so a
  wallpaper subscribed to in Steam turns up in the grid on its own, a couple
  of seconds after the writing stops.
- **English and Czech**, switched in the window without a restart.
- **A picker with its sections across the top**: Installed, Steam Workshop,
  Playlists, Displays, Settings, centred. Only Installed is built; the four
  others say
  "Coming soon" and are there because naming them is what decides where the
  next piece of work goes.
- **A picker that looks like something**: dark and light themes switched
  without a restart, a search that narrows the grid as it is typed, name
  order either way round, a band around each preview saying whether it can
  be played, and a panel over the preview with what the file is, how big its
  picture is and what it takes up on disk -- with the volume for a video on
  the card itself.
- **An Arch package** (`packaging/PKGBUILD`), installed, with an autostart
  entry that has been verified through a real logout and login.

Of the 51 wallpapers found on the development machine, 10 play: 3 web and
7 video. The other 41 are Wallpaper Engine's own packed **scene** format,
which nothing here can read. They are listed in the picker, dimmed, and a
click does nothing.

## Layout

```
src/main.cpp          WebEngine init, the single-copy lock, layer-shell setup
src/wallpapercore.h   WallpaperKind::Kind, Wallpaper (Q_GADGET), WallpaperCore
src/wallpapercore.cpp scanning Steam's folders, reading project.json
src/language.h        Language: the EN/CZ switch, and what it has to call
src/language.cpp      installs a QTranslator and retranslates the engine
src/otheraudio.h      OtherAudio: is another program using the speakers
src/otheraudio.cpp    reading `pactl --format=json list sink-inputs`
src/steamworks.h      Steamworks: can the workshop be asked, and the way in
src/steamworks.cpp    dlopen of libsteam_api.so, by name, nothing linked
src/screencover.h     ScreenCover: is a window covering the wallpaper
src/screencover.cpp   a script loaded into KWin, which calls back over D-Bus
qml/Main.qml          the wallpaper window: WebEngineView, Video, tray icon
qml/Picker.qml        the picker window: header, search, grid, footer
qml/WallpaperCard.qml one preview, its band, its name and its hover panel
qml/FilterPanel.qml   the filter popup: chips in sections, and what is on
qml/PreviewTile.qml   a preview, a name and a hover panel -- the shape of a
                      card, shared by the library and the workshop
qml/WorkshopView.qml  the Steam Workshop tab: rows of what nobody owns yet
tests/tst_wallpapercore.cpp   QtTest against a fake Steam tree in a temp dir
tests/tst_otheraudio.cpp      reading pactl's answer, no sound card needed
tests/tst_screencover.cpp     the states, with the session bus pointed nowhere
i18n/wallflux_cs.ts   the Czech catalogue, compiled into the binary
packaging/PKGBUILD    builds from this checkout's git history
wallflux.desktop      installed to applications/ and to /etc/xdg/autostart/
```

`WallpaperCore` is a QML singleton named `Wallpapers`, declared with
`QML_NAMED_ELEMENT` + `QML_SINGLETON` (**not** `setContextProperty` or
`qmlRegisterType`, which qmllint cannot see):

- properties: `wallpapers` (QList<Wallpaper>), `current`
- methods: `rescan()`, `rescan(roots)`, `select(id)`, `clear()`,
  `details(id)` (`{ size, resolution }`, read off the disk when the picker
  shows one card and kept afterwards)
- static: `scan(roots)`, `steamRoots()`
- `Wallpaper`: `id`, `title`, `kind`, `source`, `preview`, `tags`, `rating`,
  `resolution`, `sizeOnDisk`, `added`, `bytes`, `hasFile`, `playable`
- also on the singleton: `recent` (ids chosen in this run, newest first)

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

- **Qt draws the wallpapers. There is no renderer of our own.** An earlier
  attempt wrote one in Rust with OpenGL; it is parked on the branch
  `zaloha-puvodniho-pokusu-o-vlastni-engine` and only matters if scene
  wallpapers are ever taken on.
- **Nothing plays at startup.** The program comes up as a tray icon and the
  desktop keeps the wallpaper it already had until somebody picks one. The
  choice is not remembered between runs, by design.
- **The tray icon is the whole interface.** Not a CLI, not a Plasma plugin;
  both were considered and turned down.
- **Layer-shell is set up in C++**, from `main()`, before the window reaches
  the compositor — which is why `Main.qml` has `visible: false` and `main()`
  calls `show()`.

## Traps, each one found the expensive way

- **`LayerBottom`, never `LayerBackground`.** Plasma draws its own desktop
  into a background-layer surface, and clicking the desktop raises it above
  anything else in that layer — the wallpaper vanished and the system one
  came back. Layers are a fixed order; bottom is still below ordinary windows.
- **Qt's own messages do not reach a redirected stderr.** Pipe the output
  anywhere and `qCInfo`/`console.log` go to the journal instead, leaving only
  raw ffmpeg noise in the file. Use `QT_FORCE_STDERR_LOGGING=1` (the older
  `QT_LOGGING_TO_CONSOLE` still works but warns) or `journalctl --user -f`.
  **Nothing can be debugged without knowing this.**
- **`Video.autoPlay` starts the first file and no other.** Hand it a second
  source and it loads, reports nothing and sits there stopped. Measured: 24
  switches gave 1 start. `onSourceChanged: play()` is why switching works.
- **`select()` must stay silent when the choice has not changed.** Emitting
  `currentChanged` anyway makes QML hand the player the same URL, which stops
  it for good — a black screen with no error. Regression test:
  `staysQuietWhenTheChoiceIsTheSame()`.
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
- **`qmllint` is not in `PATH`** on this machine; it is
  `/usr/lib/qt6/bin/qmllint`.
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
- **A checkable Controls item cannot both be told what to be and asked what
  it is.** Bind `checked` to a property and set that property from
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
```

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

1. **Download.** The button is in the hover panel of every workshop card and
   is switched off. What it needs is `SteamAPI_ISteamUGC_SubscribeItem` and
   `DownloadItem`, the progress out of `GetItemDownloadInfo`, and then
   nothing: the folder watcher already notices what Steam puts on the disk,
   so the wallpaper turns up in the library by itself. The reading of
   Steam's answers is written and measured, so this is the small half of
   what the browsing was.

   To try it, `libsteam_api.so` has to be on the machine -- see the comment
   at the top of `src/steamworks.cpp` for why WallFlux does not ship one.
   Either install `steamworks-sdk` from the AUR, or take the SDK once:

   ```sh
   base=https://github.com/calendulish/Overlays/raw/master/dev-util/steamworks-sdk
   curl -L -o sdk.zip "$base/files/steamworks_sdk_158.zip"
   sha256sum sdk.zip
   # 4142982451d25120ddf35abf2b55499154b56132a16a7ccdae4806aaed5c99d0
   bsdtar -xf sdk.zip sdk/redistributable_bin/linux64/libsteam_api.so
   ```

   and point `WALLFLUX_STEAM_API` at it. The hash is the one the AUR package
   has been building against since 2023; it was checked against that, and
   the library's own exports were read before any of it was called.

2. **Searching and filtering the workshop.** The field and the two buttons
   above the rows do nothing yet. `AddRequiredTag` takes the same tag
   vocabulary the library filters already use, and `SetSearchText` the
   field. One more order -- most subscribed -- is already in the enum.

3. **A list of wallpapers that take turns.** The tab is there and says
   "Coming soon". Asked for: a person's own selection out of the grid, each
   one on the desktop for a quarter of an hour, swapped on a schedule while
   the program sits in the panel. Nothing in the sound or cover behaviour
   stands in the way -- neither of them cares which wallpaper is playing.

4. **Displays and Settings**, the two remaining tabs. Nothing has been asked
   for in either of them yet.

5. **The tray icon itself.** Today it is `preferences-desktop-wallpaper`
   from the icon theme. A drawn one has been asked for, without any hurry.

6. **Scene wallpapers**, all 41 of them. A direction to decide, not an
   afternoon's work.
