# WallFlux

Live wallpapers for Linux. Plays the Wallpaper Engine wallpapers you already
own, on KDE Plasma 6 under Wayland.

A copy of Wallpaper Engine on Steam is required: WallFlux reads the wallpapers
from your Steam workshop folder and ships no assets of its own.

## Status

Early, and not yet a usable program. It builds and opens an empty full-screen
window. Finding the wallpapers in the Steam workshop folder is the next step,
and nothing plays yet.

## How it works

A Wallpaper Engine wallpaper of the web kind is HTML, CSS and JavaScript, and
one of the video kind is an ordinary video file. A desktop already knows how
to draw both, so WallFlux does not draw them itself: it hands web wallpapers
to QtWebEngine, which is Chromium, and will hand video wallpapers to
QtMultimedia. Web wallpapers come first, video after.

That is a deliberate change of direction. WallFlux began as a renderer of its
own, written against OpenGL ES, that read Wallpaper Engine's packages and
translated each wallpaper's shaders. It worked, but it meant owning a graphics
engine to maintain forever. That work is kept on the branch
`zaloha-puvodniho-pokusu-o-vlastni-engine` rather than thrown away.

## Requirements

- KDE Plasma 6 on Wayland — developed on Arch (CachyOS)
- Qt 6.5 or newer, including QtWebEngine
- layer-shell-qt, for its `org.kde.layershell` QML module. Needed at run
  time, not at build time: without it the window still opens, but as an
  ordinary one rather than as a wallpaper.
- zlib and lz4
- CMake 3.21 or newer, and a C++17 compiler

## Build

    cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
    cmake --build build
    ./build/wallflux

And the tests, which need Qt's Test module:

    ctest --test-dir build --output-on-failure

Qt prints its messages to the terminal when there is one and sends them to
the journal when there is not, so a run started from a desktop file is read
with `journalctl --user -t wallflux`. A wallpaper's own JavaScript console is
silenced by default; `QT_LOGGING_RULES=js=true` brings it back.

## Thanks

WallFlux is inspired by [linux-wallpaperengine][lwe] by Almamu, which has been
playing Wallpaper Engine wallpapers on Linux for years and is the reason this
seemed worth attempting at all. It is a separate project, under the same
licence as this one, and nothing here is taken from it so far.

Wallpaper Engine is a product of Wallpaper Engine Team and is not affiliated
with this project.

[lwe]: https://github.com/Almamu/linux-wallpaperengine

## License

GPL-3.0-or-later. WallFlux is free software: use it, change it and pass it on,
as long as what you pass on stays free on the same terms. See [LICENSE](LICENSE).
