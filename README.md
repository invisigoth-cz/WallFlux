# WallFlux

Live wallpapers for Linux. Plays the Wallpaper Engine wallpapers you already
own, on KDE Plasma 6 under Wayland.

A copy of Wallpaper Engine on Steam is required: WallFlux reads the wallpapers
from your Steam workshop folder and ships no assets of its own.

## What it does

WallFlux sits in the panel as a tray icon. The icon opens a window with:

- **Your library** -- every installed wallpaper, with search, filters,
  favourites and a panel saying what each one is.
- **The Steam Workshop** -- browse, search and download without leaving
  the window.
- **Playlists** -- wallpapers that take turns, with fades between them.
- **Displays** -- one wallpaper on every screen you tick.
- **Settings** -- language (English, Czech), quality presets, sound,
  themes, starting at login.

All three kinds of Wallpaper Engine wallpaper play: **web**, **video** and
**scene**, scenes with their music, particles, scripts and the mouse. The
sound gets out of the way by itself when another program plays, and the
wallpaper pauses under a maximised window.

## Install

**Before you start**, you need:

1. **Wallpaper Engine** bought on Steam and installed -- WallFlux plays the
   wallpapers Steam has downloaded for it, and ships none of its own.
2. **KDE Plasma 6 on Wayland**, on an Arch-based system: Arch Linux,
   CachyOS, EndeavourOS, Manjaro and the like.
3. **An AUR helper** -- `paru` (CachyOS has it already) or `yay`.

Then open a terminal and paste these two lines, one after the other:

    paru -S steamworks-sdk
    sudo pacman -U https://github.com/invisigoth-cz/WallFlux/releases/latest/download/wallflux-x86_64.pkg.tar.zst

The first installs Valve's Steamworks SDK, which only the AUR carries and
which the Steam Workshop tab needs. The second installs WallFlux itself.
Both ask for your password and then for a `Y` to go ahead.

For example, on a fresh CachyOS:

    $ paru -S steamworks-sdk
    :: Proceed with installation? [Y/n] y
    $ sudo pacman -U https://github.com/invisigoth-cz/WallFlux/releases/latest/download/wallflux-x86_64.pkg.tar.zst
    :: Proceed with installation? [Y/n] y

WallFlux is then in the application menu. Start it and its icon appears in
the panel; click the icon to choose a wallpaper. New versions are offered
in Settings and install themselves.

## Uninstall

One line removes WallFlux, its settings, its playlists and its cache.
The Steamworks SDK and everything else it needed stay installed:

    sudo pacman -R wallflux; rm -rf ~/.config/WallFlux ~/.cache/WallFlux ~/.config/autostart/wallflux.desktop

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

## Building from source

You need Qt 6.5 or newer with QtWebEngine and QtMultimedia, layer-shell-qt,
CMake 3.21 or newer, a C++20 compiler and Rust with `cargo`.

    cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
    cmake --build build
    ./build/wallflux --show

The tests: `ctest --test-dir build --output-on-failure`. An Arch package is
built with `cd packaging && makepkg -si`.

Qt prints its messages to the terminal when there is one and sends them to
the journal when there is not, so a run started from the menu is read with
`journalctl --user -t wallflux`.

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
