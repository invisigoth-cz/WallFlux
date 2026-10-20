# Changelog

Everything worth knowing about each release of WallFlux. The shape of this
file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] — 2026-10-20

### Fixed

- **The Steam tab says why it is empty.** With Steam not running it showed
  rows that sat there saying "asking…" about a question nobody had asked,
  and the real reason in small grey type under the controls -- it looked
  broken rather than unavailable. The tab now shows nothing but the reason,
  in the middle of the page and in full, since the two reasons it can be --
  no Steamworks library on the machine, or Steam not running -- need
  different things done about them.
- **Being told to start Steam is now advice worth taking.** The Steamworks
  library was opened once, when the workshop was first wanted, so starting
  Steam afterwards changed nothing until WallFlux was restarted. There is a
  Try again button, and coming back to the tab asks again by itself. The
  rows and the grid fill themselves the moment Steam becomes reachable.
- **The reason is shown in the language the window is in.** It was built in
  C++ when the workshop failed to open, which fixed it in whatever language
  was installed at that moment: the heading changed with the language and
  the sentence under it did not.

## [0.3.1] — 2026-10-20

### Added

- **The workshop filter gained a ŠTÍTKY section**, the eleven tags that say
  what a wallpaper does rather than what it is of: 3D, HDR, Audio
  responsive, Customizable, Approved and the rest. The library's filter
  already had one; now both have the same five.

### Changed

- **A wallpaper's age rating is kept in Steam's own word** -- Everyone,
  Questionable, Mature -- rather than turned into G, PG13 and R. The old
  chips worked, but a rating read one way off the disk and another out of a
  workshop query, so no single chip could serve both filters. These three
  are the only tag names WallFlux carries itself, in any language, because
  they are the one place Wallpaper Engine's catalogues are no help: it
  files age under "G - All ages" and the like, which is a different
  vocabulary from what Steam tags with.
- **The library and the workshop say the same words**: DRUH, PŘÍSTUPNOST,
  ROZLIŠENÍ, ŽÁNR and ŠTÍTKY in both, where the workshop used to say TYP,
  VĚKOVÉ OMEZENÍ and ZNAČKY, and called the genres by the word the library
  uses for everything else.
- **The picture sizes offered are the ones a monitor has.** The four
  portrait sizes and the portrait standard definition are for telephones;
  WallFlux draws on a desktop, so they were five chips that could only ever
  disappoint. Fourteen left, ultrawide and multi-monitor among them.

### Fixed

- **A wallpaper already on the disk is no longer offered for download.**
  The card reads the library's own list of ids, so a download that finishes
  turns its own button over the moment the folder watcher notices.

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

[Unreleased]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/invisigoth-cz/WallFlux/releases/tag/v0.1.0
