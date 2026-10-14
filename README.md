# WallFlux

Live wallpapers for Linux. Plays Wallpaper Engine wallpapers on Wayland
compositors that support `wlr-layer-shell` (KWin, Hyprland, sway, niri, labwc).

A copy of Wallpaper Engine on Steam is required: WallFlux reads the wallpapers
you already own and ships no assets of its own.

## Status

Early, and not yet a usable program. Reads Wallpaper Engine's package and
texture containers, translates each wallpaper's own GLSL shaders and draws its
scene. Particle systems and video wallpapers come next.

## Build

    cargo build --release

## License

MIT
