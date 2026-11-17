# WallFlux

Live wallpapers for Linux. Plays the Wallpaper Engine wallpapers you already
own, on KDE Plasma 6 under Wayland. Wayland only: WallFlux does not run on
X11.

A purchased copy of Wallpaper Engine on Steam is required: WallFlux reads the
wallpapers from your Steam workshop folder and ships no assets of its own.

## What it does

WallFlux sits in the panel as a tray icon. The icon opens a window with:

- **Your library** -- every installed wallpaper, with search, filters,
  favourites and a panel saying what each one is, and playlists along the
  bottom: wallpapers that take turns, with fades between them.
- **The Steam Workshop** -- browse, search and download without leaving
  the window.
- **Displays** -- one wallpaper on every screen you tick.
- **Settings** -- language (English, Czech), quality presets, sound,
  themes, starting at login.

All three kinds of Wallpaper Engine wallpaper play: **web**, **video** and
**scene**, scenes with their music, particles, scripts and the mouse. The
wallpaper knows when it is not being looked at -- a window over it, another
program with the keyboard or playing sound, the screens off or locked -- and
you choose in Settings whether it then keeps playing, mutes, pauses or stops.

## System requirements

- **Wallpaper Engine**, purchased on Steam, and the **Steam client** to
  download it and its wallpapers. Steam has to be running only for the Steam
  Workshop tab; wallpapers already downloaded play without it.
- **Arch Linux** or a system built on it -- CachyOS, EndeavourOS, Manjaro
  and the like. WallFlux is a pacman package.
- **KDE Plasma 6 on Wayland.** Not X11; other desktops are not supported.
- **A 64-bit x86 processor with SSE4.2** (x86-64-v2). Without it WallFlux
  does not start at all.
- **A graphics card and driver with OpenGL 3.3.** Video and web wallpapers
  are light; **scene** wallpapers are drawn by WallFlux itself and ask far
  more of the machine. They are known to play on a current desktop graphics
  card, and on an older laptop (AMD A8-6410, 2014) they do not play yet.
- **PipeWire** with its PulseAudio server, which is what Plasma uses by
  default -- for the wallpapers' sound and for the ones that follow your
  music.
- **An internet connection** for the Steam Workshop tab and for updates.

Everything else is installed along with WallFlux: Qt 6 (with QtWebEngine
and QtMultimedia), layer-shell-qt, KWayland, FFmpeg, the Breeze icons and
Valve's Steamworks SDK.

## Install

For **Arch Linux users** and Arch-based systems. Open a terminal and paste
this line:

    curl -fsSL https://raw.githubusercontent.com/invisigoth-cz/WallFlux/main/install.sh | bash

**Or read the script before running it**, which is wise with anything off
the internet:

    curl -fsSLO https://raw.githubusercontent.com/invisigoth-cz/WallFlux/main/install.sh
    less install.sh
    bash install.sh

The script ([install.sh](install.sh)) installs Valve's Steamworks SDK from
the AUR if it is missing -- the Steam Workshop tab needs it, and no AUR
account or helper is needed for it -- and then builds the newest release of
WallFlux on your machine and installs it as an ordinary pacman package. The
build takes a few minutes. It asks for your password, and for a `Y` before
installing anything.

WallFlux is then in the application menu. Start it and its icon appears in
the panel; click the icon to choose a wallpaper.

## Update

Settings says when a new version is out, and shows the same line with a
button that copies it: run it again and it builds the newest release in
place of the old one. Then quit WallFlux from its tray icon and start it
again.

Prefer to do it by hand? The script does nothing more than this, from a
clone of this repository:

    paru -S steamworks-sdk        # or yay -S, once
    git clone https://github.com/invisigoth-cz/WallFlux.git
    cd WallFlux/packaging && makepkg -si

## Uninstall

One line removes WallFlux, whatever it needed that nothing else does, its
settings, its playlists and its cache:

    sudo pacman -Rns wallflux; rm -rf ~/.config/WallFlux ~/.cache/WallFlux ~/.config/autostart/wallflux.desktop

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
