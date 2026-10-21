# Changelog

Everything worth knowing about each release of WallFlux. The shape of this
file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] — 2026-10-21

### Added

- **A panel beside the grid saying what a wallpaper is.** A click opens it,
  in either tab: the picture, the name, and what is known -- the kind, the
  picture size, what it takes up on disk, the age rating, whether it
  answers to sound, the tags, the workshop's own number for it. What
  cannot be answered is left out rather than shown as a dash.
- **Downloading works again**, which it had not since the hover panel was
  taken out: a green Download at the foot of the panel, with the progress
  on it, turning itself over to "Installed" the moment the folder watcher
  notices the wallpaper arrive.
- **A wallpaper can be given back.** A red Remove, with a line under it
  saying what will happen. It is two halves: Steam is told to unsubscribe,
  and the folder is deleted -- because Steam does not do the second half.
  Measured: unsubscribing answers `k_EResultOK` and the item leaves the
  subscribed list, and the folder then stays where it is, both while
  WallFlux runs and a minute after it has quit. Nothing is deleted that is
  not the workshop folder named after that id under a root this program
  scanned, and there is a test for the refusal.
- **An asset or a preset says what it is.** They carry no type of their
  own -- an asset is a part to build a scene out of, a preset is somebody
  else's wallpaper plus settings -- so the library has nothing to list them
  as and does not. The panel now says so under the Download button rather
  than letting one download quietly into nothing.

### Changed

- **One click picks a wallpaper out, two put it on the desktop.** One click
  used to do both, which with a panel to fill meant reading about a
  wallpaper changed what was on the screen.
- **Nothing is drawn over a preview on hover.** What that panel carried is
  what the panel beside the grid now says, which was the point of taking it
  out in 0.5.0.

## [0.5.0] — 2026-10-21

### Changed

- **The Steam tab is quick to reach.** It was built when the tab was
  switched to and thrown away when it was left, so every switch rebuilt the
  rows and fetched every preview over the network again. It is built the
  first time it is looked at and kept from then on.
- **Previews are kept on the disk.** They were fetched again every time
  anything was looked at twice. Steam says a preview is good for a week
  (`Cache-Control: max-age=604800`, read off the response), so a page
  already seen now costs no network at all and survives a restart.
- **Previews are taken as Steam stores them, and are no longer resized.**
  Asking the CDN for a smaller copy looks like the obvious saving and is a
  trap: most of the workshop's previews are animated, and a size nobody has
  asked for before is re-encoded on the spot. Measured over six of them,
  0.7 to 7.9 seconds each against 0.03 to 0.09 for the file as stored --
  and the resized copy was not even smaller, one coming back as 1 099 kB
  where the stored file is 924 kB. This is what made a page take tens of
  seconds. With it gone, and measured with an empty cache: a page of the
  Workshop fills in 0.86 s, every Discover row in 0.40 s, a section in
  0.20 s and the next page in 0.28 s.
- **A question somebody is waiting for goes first.** Discover asks one
  question per row and Steam answers them one at a time, measured at 0.08
  to 0.6 seconds each -- so a search typed on arriving at the tab waited out
  every row still queued. Searching, paging and sorting now go to the front:
  measured, a search went from last of six to second, answered in 0.4 s.
  The same question is never asked twice either, so three clicks on Next
  mean page four rather than pages two, three and four in turn.
- **Both filters stand beside the grid** rather than popping up over it, in
  the same place whether the window is its usual size or maximised, with the
  grid reflowing into what is left and a second click on Filters putting the
  panel away. They were popups that did not fit: the workshop's fifty-nine
  chips are taller than most windows, and the bottom of the panel ran off
  the edge taking Clear with it.
- **Discover fills the page.** Four rows left a maximised window half empty,
  and with nothing hanging over the bottom edge there was nothing to scroll
  -- so the rest were never asked for at all. How many rows there are now
  comes from the height of the window, topped up on every resize, and
  scrolling adds to it as before.
- **"See more" opens that row whole.** It used to be a trip to the Workshop
  with the row's tag ticked, which left somebody who had asked for Trending
  looking at a page of filters and a sort button. It is its own page now:
  the section's name with the way back beside it, the wallpapers under it,
  and paging through all of them.
- **Discover offers nothing to filter or sort by.** Neither does a section
  opened out of it. They are pages somebody else picked, and the Workshop
  is the one place in this window where a question is put together by hand
  -- which is how Wallpaper Engine's own has it.
- **Nothing is drawn over a preview on hover any more.** The panel that
  said what the file was, how big its picture was, what it took up on disk
  and how loud it played -- and, in the workshop, offered Download -- is
  gone from both grids. It is to be said somewhere else and the tile is to
  carry something else; until then the only sound control is the one in the
  footer, and nothing in the window starts a download. `Steamworks` can
  still do it and nothing calls it.
- **A page of the workshop stops at a thousand.** It offered 64 929 of
  them, which is three and a quarter million wallpapers divided by fifty
  and not an offer at all: measured, Steam answers page 1001 and refuses
  page 1024 outright. A thousand is what Wallpaper Engine stops at and is
  inside what Steam will serve.
- **A Discover row shows twenty wallpapers, not fifty.** Steam hands over
  fifty in one answer whatever is asked; a row is a taste of a section and
  Wallpaper Engine's own stops at twenty. "See more" is where the rest are.
- **The pages are pages now**, not Previous and Next: the first, the last,
  the five around the one being looked at, and arrows that are not there
  when there is nothing that way, including a jump to either end.
- **The buttons have their corners taken off**, and room around what is
  written on them. Fusion draws a near-square box with the text against the
  edge, which was the only sharp thing left in a window of rounded previews
  and round chips.
- **The filters are one value to a row.** They were chips side by side,
  which puts a different number on every line and ends every line somewhere
  else; five sections of that is a panel with no straight edge in it. Each
  value keeps the shape of a chip and takes the width of the panel, and the
  headings over them are bigger and heavier than what they head.
- **The library's filter offers Audio responsive** whether anything
  installed carries it or not -- measured, none of the 55 here does, they
  carry genres and nothing else. It is the one somebody looks for before
  they have the wallpaper that answers it, and the day one is downloaded
  the chip already means something.
- **The scrollbars can be seen, and are out of the way of the pictures.**
  Fusion's own fades out whenever it is not being touched, so a grid that
  ran over the bottom edge said nothing about it. Every scrollbar in the
  picker is now one that stays, is wide enough to aim at, and lives in a
  strip of its own out by the window's edge rather than drawn over the last
  column of previews.

### Fixed

- **The sort menus close on a second click.** They were closing on the
  press, and the click that followed opened them again -- so the menu
  flickered and stayed on the screen, and the only way to put it away was to
  click somewhere that meant something else.
- **A Discover row's scrollbar is under its cards** rather than drawn across
  the bottom of every preview in the row.
- **The grid starts at the top of a new answer.** Page two opened where page
  one had been scrolled to, and so did a section opened out of Discover.
- **The Czech catalogue has the new words in it.** Three strings had been
  added to the window without being added to it.
- **The search fields have a cross to clear them**, and stop at 50
  characters. Clearing the workshop's search asks Steam again without the
  word, since what is on the screen is otherwise still the answer to
  something that is no longer in the field.
- **Something on the screen says Steam has been asked.** The grid went on
  showing the last answer at full strength while a new one was on its way,
  which made searching look like nothing happening at all. It dims while
  Steam is being asked, with a spinner over it.

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

[Unreleased]: https://github.com/invisigoth-cz/WallFlux/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.2...v0.5.0
[0.3.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/invisigoth-cz/WallFlux/releases/tag/v0.1.0
