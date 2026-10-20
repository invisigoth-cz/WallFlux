# Changelog

Everything worth knowing about each release of WallFlux. The shape of this
file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-10-20

### Added

- **The Steam Workshop can be browsed, and wallpapers fetched from it.** The
  Steam tab has two ways in. **Discover** is rows picked for you -- Trending,
  New and one per genre -- built a few at a time as the page is scrolled,
  because each row is its own question to Steam. **Workshop** is one grid
  with search, filters, sort and paging, fifty at a time out of three and a
  quarter million. Everything comes from Steam itself: no API key, no web
  page scraped.
- **Download.** The button on every workshop card subscribes to the
  wallpaper and asks Steam to fetch it now, showing how far along it is.
  Nothing else is needed afterwards: the folder watcher notices what Steam
  puts on the disk and the wallpaper appears in Installed by itself.
- **Filters over the workshop's whole vocabulary**: type, age rating,
  picture size and genre. All four are tags as far as Steam is concerned,
  so a genre row and a ticked chip are the same mechanism. Everything
  ticked has to match, which the panel says, because it is the opposite of
  what a row of chips usually means.
- **Tags are called what Valve calls them, in 36 languages.** Wallpaper
  Engine's own install carries the names, so the Czech interface says
  "Abstraktní" and "Dopravní Prostředky" where it used to say the raw Steam
  tag. No tag name is written into WallFlux in any language. Where Valve
  has no name -- the three age ratings and a few tags newer than the
  installed copy -- the tag itself is shown.
- **WallFlux says when the workshop cannot be reached** rather than looking
  empty, and carries on with everything else.

### Changed

- **The Steamworks SDK is now a dependency rather than a suggestion.**
  Browsing and downloading need `libsteam_api.so`, and there is no second
  source for it: Steam ships none on Linux, and Wallpaper Engine carries
  only the Windows DLL. Installing WallFlux from the AUR now brings
  `steamworks-sdk` with it, so nothing is asked of anybody. The cost, said
  plainly: Valve's SDK lands on the disk of everyone who installs WallFlux,
  whether they open the Steam tab or not.
- **The Steam Workshop tab is called Steam.**
- **The licence gained one added permission**, for the Steamworks SDK and
  nothing else. Fetching a workshop item needs an account that owns the
  application it belongs to, and the Steamworks SDK is the only interface
  Valve offers for that; the GPL alone would forbid shipping WallFlux linked
  against it. See [LICENSE-EXCEPTION](LICENSE-EXCEPTION). Every other library
  still has to be GPL-compatible.

## [0.2.0] — 2026-10-19

A different program from 0.1.0 under the same name. That one was a renderer
of its own, written in Rust, that drew Wallpaper Engine's packed scene
format; this one lets Qt draw and takes on the two formats Qt can read. The
Rust engine is parked on the branch
`zaloha-puvodniho-pokusu-o-vlastni-engine` and only matters if scene
wallpapers are ever taken on again.

### Added

- **Web wallpapers** (`index.html` and JavaScript), drawn by QtWebEngine.
- **Video wallpapers**, drawn by QtMultimedia, looping, with their sound.
- **A tray icon** that opens a grid of everything installed. Nothing plays
  at startup: the desktop keeps the wallpaper it had until somebody picks
  one, and the choice is not remembered between runs.
- **A picker** with its five sections across the top — Installed, Steam
  Workshop, Playlists, Displays, Settings, of which only the first is built
  — dark and light themes switched without a restart, a search that narrows
  the grid as it is typed, ordering by name, date added, size or what was
  played last either way up, and filters by tag, age rating, resolution and
  kind. A band around each preview says whether it can be played, and
  hovering a playable one shows what it is, how big its picture is, what it
  takes up on disk, and the volume for a video.
- **Sound that gets out of the way.** The wallpaper goes quiet while another
  program is using the speakers, and quiet and stopped while a window covers
  the screen; both come back on their own. Neither is a setting.
- **English and Czech**, switched in the window without a restart.
- **An Arch package** (`packaging/PKGBUILD`) with an autostart entry,
  verified through a real logout and login.

### Known limitations

- **Scene wallpapers are not drawn.** 41 of the 51 found on the development
  machine are Wallpaper Engine's own packed format, which nothing here
  reads. They are listed in the picker, dimmed, and a click does nothing.
- **On Plasma the wallpaper covers the desktop's icons and widgets** while
  it plays. Measured twice: on Plasma the wallpaper and the icons are one
  opaque surface and there is no "between" to slip into. The only real fix
  is a Plasma wallpaper plugin. On wlroots compositors — Hyprland, sway,
  niri, labwc — layer-shell alone is enough and the problem does not exist.
- **Resolution is only known for videos**, read with ffprobe shortly after
  each scan. Nothing in `project.json` carries one — checked across all 53
  — and a scene or web wallpaper has no single answer to give.

## [0.1.0] — 2026-10-14

The first attempt: a renderer of its own in Rust and OpenGL, reading
Wallpaper Engine's `.tex` and scene formats and drawing the scenes directly.
Tagged for the record, and not continued — see 0.2.0.

[Unreleased]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/invisigoth-cz/WallFlux/releases/tag/v0.1.0
