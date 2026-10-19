# WallFlux

Plays the Wallpaper Engine wallpapers somebody already owns, on Linux and
Wayland, without Steam or Windows. C++20, Qt 6, GPL-3.0-or-later, ten files
and about eight hundred lines. Target machine is CachyOS with KDE Plasma 6.7
on Wayland, and that is where everything below was measured.

## What works today

- **Web wallpapers** (`index.html` + JavaScript) drawn by QtWebEngine.
- **Video wallpapers** drawn by QtMultimedia, looping, with their sound on.
- **A tray icon** that opens a grid of every installed wallpaper and plays
  the one that is clicked.
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
qml/Main.qml          the wallpaper window: WebEngineView, Video, tray icon
qml/Picker.qml        a window of preview thumbnails, click to play
tests/tst_wallpapercore.cpp   QtTest against a fake Steam tree in a temp dir
packaging/PKGBUILD    builds from this checkout's git history
wallflux.desktop      installed to applications/ and to /etc/xdg/autostart/
```

`WallpaperCore` is a QML singleton named `Wallpapers`, declared with
`QML_NAMED_ELEMENT` + `QML_SINGLETON` (**not** `setContextProperty` or
`qmlRegisterType`, which qmllint cannot see):

- properties: `wallpapers` (QList<Wallpaper>), `current`
- methods: `rescan()`, `rescan(roots)`, `select(id)`, `clear()`
- static: `scan(roots)`, `steamRoots()`
- `Wallpaper`: `id`, `title`, `kind`, `source`, `preview`, `hasFile`, `playable`

## Decisions that are settled — do not reopen without being asked

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
qmllint -I build/ qml/Main.qml qml/Picker.qml
```

Anything about stacking or what is on screen has to be measured, not
reasoned about:

- **Stacking order**, without touching the mouse: write a KWin script that
  prints `workspace.stackingOrder`, load it with
  `qdbus6 org.kde.KWin /Scripting loadScript <file.js> <unique-name>`, run it
  with `... /Scripting start`, read the output with `journalctl --user`.
  Setting `workspace.activeWindow` to the plasmashell desktop window is what
  a click on the desktop does.
- **A picture of the desktop** with the windows out of the way:
  `qdbus6 org.kde.kglobalaccel /component/kwin invokeShortcut "Show Desktop"`
  (again to undo), then `spectacle -b -n -f -o <file.png>`.
  `workspace.showingDesktop = true` from a script does **not** work.

## Open, in the order they are likely to be taken

1. **A sound switch.** Video wallpapers play their audio and the only way to
   silence one is to turn the wallpaper off. A menu item, a few lines.
2. **The tray icon itself.** Today it is `preferences-desktop-wallpaper` from
   the icon theme. A drawn one has been asked for, without any hurry.
3. **Scene wallpapers**, all 41 of them. A direction to decide, not an
   afternoon's work.
