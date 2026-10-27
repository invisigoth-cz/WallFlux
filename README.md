# WallFlux

Live wallpapers for Linux. Plays the Wallpaper Engine wallpapers you already
own, on KDE Plasma 6 under Wayland.

A copy of Wallpaper Engine on Steam is required: WallFlux reads the wallpapers
from your Steam workshop folder and ships no assets of its own.

## Status

Usable, and used daily on the machine it is written on. It sits in the
panel as a tray icon; the icon opens a picker with every installed
wallpaper, the Steam Workshop to browse and download from, playlists and a
switch per screen. All three kinds of Wallpaper Engine wallpaper play --
web, video and scene -- though scenes do not yet answer to music or to the
mouse.

## How it works

A Wallpaper Engine wallpaper of the web kind is HTML, CSS and JavaScript, and
one of the video kind is an ordinary video file. A desktop already knows how
to draw both, so WallFlux hands web wallpapers to QtWebEngine, which is
Chromium, and video wallpapers to QtMultimedia.

The scene kind is Wallpaper Engine's own: a package of images, particle
systems, effects and shaders written in its own dialect. Nothing else draws
those, so WallFlux has a renderer of its own for them, in Rust and OpenGL,
under `crates/`. It reads the packages, translates the shaders and draws
into the same window as the other two kinds, linked into the program rather
than run beside it.

## Requirements

- KDE Plasma 6 on Wayland — developed on Arch (CachyOS)
- Qt 6.5 or newer, including QtWebEngine
- layer-shell-qt, for its `org.kde.layershell` QML module. Needed at run
  time, not at build time: without it the window still opens, but as an
  ordinary one rather than as a wallpaper.
- CMake 3.21 or newer, a C++20 compiler, and Rust with `cargo` to build
  the scene renderer
- For the Steam Workshop tab, Valve's Steamworks SDK -- on Arch,
  `steamworks-sdk` from the AUR

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
licence as this one. Its code is read where Wallpaper Engine's own
behaviour is not on the disk to be read -- the way a scene moves with the
mouse follows the formula it arrived at -- and none of it is copied.

Wallpaper Engine is a product of Wallpaper Engine Team and is not affiliated
with this project.

[lwe]: https://github.com/Almamu/linux-wallpaperengine

## License

GPL-3.0-or-later, with one added permission. WallFlux is free software: use
it, change it and pass it on, as long as what you pass on stays free on the
same terms. See [LICENSE](LICENSE).

The added permission is for the Steamworks SDK, and for nothing else: it
allows WallFlux to be linked against it and distributed that way. A workshop
item for a paid application can be fetched only by an account that owns that
application, and the Steamworks SDK is the only interface Valve offers for
doing so. See [LICENSE-EXCEPTION](LICENSE-EXCEPTION).
