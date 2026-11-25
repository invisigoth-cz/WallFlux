# WallFlux

Plays the Wallpaper Engine wallpapers somebody already owns, on Linux and
Wayland, without Windows and without Wallpaper Engine running. C++20, Qt 6,
GPL-3.0-or-later -- plus, under `crates/`, the scene renderer in Rust,
linked into the binary.
Target
machine is CachyOS with KDE Plasma 6.7 on Wayland, and that is where
everything below was measured.

**Steam is how the wallpapers get here.** Wallpaper Engine is bought and
installed through it, the workshop is where wallpapers come from, and Steam
is the only thing that can fetch one: a workshop item for a paid app is
downloadable only by an account that owns the app. So Steam is a dependency
of getting wallpapers, not of playing them -- what is read is the folder
Steam has already filled, and nothing of Steam's has to be running for that.

## What works today

- **Text in a scene**: its own font or Valve's, at the size and place the
  file says, through the object's effects, rewritten when a script changes it.
  A font it cannot read falls back to Noto Sans; Windows' `systemfont_` names
  and colour emoji do not exist here.
- **Web wallpapers** (`index.html` + JavaScript) drawn by QtWebEngine.
- **Video wallpapers** drawn by QtMultimedia, looping, with their sound on.
- **Scene wallpapers** -- Wallpaper Engine's own packed format -- drawn by
  the renderer under `crates/`, into a `SceneView` on the same surface as
  the other two: transitions, pausing under a cover and every screen work
  the same. A scene's own music plays -- unpacked from its `scene.pkg`
  into the cache by the renderer and played by `SceneSound`, once for the
  program and not once per screen, in the scene's own `loop`, `random` or
  `single` manner. Its particle systems bring their children -- running
  alongside, bursting where a particle is born or dies, or riding along
  with one -- and are set up as the scene says, not as their preset does.
  A scene that answers to the music hears what the speakers are playing
  -- 7 of the 40 here do -- read off the default output's monitor by the
  renderer itself, once for every screen. A scene's JavaScript runs --
  all 93 scripts in 7 packages, under QuickJS: what writes a clock's digits, turns
  its hands, swells a ring with the bass, keeps a particle system's rate, alpha
  and speed on the beat, shakes the camera and switches an effect on and off
  (16 November). And a scene
  follows the mouse -- layers that lean away from it, particles pushed
  aside -- with the position asked of KWin, since the wallpaper's surfaces
  take no input and must not.
- **Sound that gets out of the way by itself.** The wallpaper is quiet while
  another program is using the speakers, and quiet and stopped while a window
  covers the screen; both come back on their own. Both became settings on 9
  November, at the user's asking, on by default; with the second off the
  wallpaper goes on playing -- and drawing -- behind the window.
- **A tray icon** that opens a grid of every installed wallpaper and plays
  the one that is clicked. Its menu (10 November) also pauses -- the
  wallpaper stands still and quiet as under a window, and a playlist is held
  unless told to change while paused, Wallpaper Engine's meaning -- goes on
  to the next wallpaper of a running playlist, mutes, and offers the last
  five wallpapers chosen in this run. No item is checkable: the words say
  the state, since a checkable item ticks itself and breaks its binding.
- **Noticing what Steam downloads.** The workshop folders are watched, so a
  wallpaper subscribed to in Steam turns up in the grid on its own, a couple
  of seconds after the writing stops.
- **The Steam Workshop, browsed.** Two ways in. *Discover* is rows picked
  for you -- Recent approved (Wallpaper Engine's own first row: `most_recent`,
  tag Approved, the five portrait sizes left out), Trending, New and one per
  genre, twenty wallpapers to a row --
  as many rows as the window is tall, with more as it is scrolled, since
  each row is its own question. Nothing in Discover is filtered or sorted
  by hand; "See more" opens one row whole, as a page with the section's
  name and the way back. *Workshop* is one grid with search, filters, sort
  and paging, fifty at a time and a thousand pages deep.
  Clicking a wallpaper anywhere in this tab opens a panel down the right
  with its picture, what is known about it and a green Download; the
  library's grid has the same panel with a red Remove instead. Out here
  the click is the only thing there is -- it opens the panel and closes it
  again, and a double click does nothing whatever, because nothing in this
  tab is ours to play.

  **Anime can be hidden** (10 November), Wallpaper Engine's Show Anime /
  Hide Anime (`filterConfig.Anime` in its `scripts.js`): a switch in
  Settings › General above the reset (the user's choice of place), kept on
  the picker as `showAnime` and handed to the Steam tab by a `Binding`. Hidden, every
  question but the account's own favourites carries `-Anime` --
  `Steamworks.browse` reads a tag with a `-` in front as
  `AddExcludedTag` -- the genre's row goes, and the rows' keys get
  `|-Anime` so that switching asks them all again. The Workshop filter's
  Anime chip goes while it is hidden, and is unticked if it was (the user's
  choice: Wallpaper Engine's genres are "include" boxes, where the setting
  only unticks Anime, but ours are "require" chips, which cannot copy that). Measured against Steam: 0 of 50 carry the tag, and the
  totals with and without add up to the whole workshop exactly.

  **The library's panel stays open when the tab is left**, and so does the
  tint: the user's word of 9 November, reversing an earlier rule that shut
  it. **The Steam tab's is shut on leaving** (`steam.item.shut()`), the
  user's word of 10 November -- what was looked at there has usually just
  been downloaded, and is played from the library. What
  was picked last, in either grid, tints every tab -- Displays and
  Settings too -- until another is picked or the same card is clicked
  again (`lastPicked` in `Picker.qml`). Only the click still being counted
  is forgotten on leaving, or it would open a panel in the tab that had
  just been left, a few tenths of a second later.
- **A filter panel that folds, one section at a time.** Each section's
  heading is a switch: the arrow turns, the chips slide out from under it,
  and a number beside the heading says how many of them are ticked while
  they are away. Opening one folds whichever was open, and every section is
  folded each time the panel is shown (the user's word of 10 November, both
  tabs). It used to be the short ones open and the long ones folded.
- **One vocabulary for both filters.** Type, age rating, picture size,
  genre and the rest are all tags as far as Steam is concerned, so a genre
  row and a ticked chip are the same required tag. Both panels read DRUH,
  PŘÍSTUPNOST, ROZLIŠENÍ, ŽÁNR, ŠTÍTKY in that order (the library's had its
  own until 16 November), the library's OBLÍBENÉ and KOMPATIBILITA after
  them, and every tag is called what Valve
  calls it in whichever of its 36 languages the window is in.
- **An empty Steam tab that says why**, with a Try again that works: the
  Steamworks library is opened again rather than once at startup, so
  starting Steam afterwards is advice worth taking.
- **English and Czech**, switched in the window without a restart.
- **Wallpapers that fade into one another.** A playlist can cross from one
  wallpaper to the next, go out through black, or hold the last frame over
  the gap, over nought to three seconds.
- **Wallpapers that take turns.** A strip along the bottom of the library
  from the moment the window opens, empty or not -- the place a playlist is
  made has to be visible before there is anything in it. Along its top:
  Playlists, then Load, Save, Configure and Add wallpaper (a line beside
  Configure saying what it was set to was tried on 16 November and taken
  out again, the user's word; Load's rows say each saved one's settings in
  `playlistSummary`'s words, and the Playlists menu says nothing but the
  names); under them the
  chosen wallpapers, twenty to a row. Hovering a card shows a tick box --
  white with a black edge, blue when it is ticked, fixed colours rather
  than the palette's because the box sits on a picture and not on the
  window -- and Add wallpaper holds every box open so several can be
  ticked without hunting for each one. The room for the previews comes
  with the first one that is ticked and goes with the last; the words and
  the buttons are there from the start. Load is the saved playlists, to open one back onto
  the strip -- which also starts it -- or throw it away, which the button
  asks about first: forgetting a playlist cannot be undone and there is
  nowhere to undo it from; Save asks what to call it and replaces one of
  that name; Configure is how it runs. A saved playlist is started from the
  Playlists button beside the filters, and stopped by clicking it again;
  that button always says Playlists and the tick in its menu says which
  one is on, as the sort menu does. **Whatever starts a playlist puts it on
  the strip** (16 November, `onPlayingChanged` in `Picker.qml`), so the
  strip and its line are always about the one playing; a rename of the one
  playing keeps the strip as it is.
  There is no Playlists tab: a playlist is made out of the grid, so it is
  made in the grid.

  **Configure is laid out as Wallpaper Engine's own.** Five of its
  switches do what they say: *change when the video ends*, *allow changing
  while paused*, *always begin with the first wallpaper*, the transition
  and how long it takes. Nothing in the popup is a placeholder any
  more: "the first wallpaper plays only at startup" was the last one and
  it has been taken out, since WallFlux is not started with the session in
  a way that would give it a meaning. The transition drop down opens with
  an empty row
  chosen, as Wallpaper Engine's does; the interval is hours, minutes and
  seconds, typed rather than clicked at.

  **There was a "change wallpaper" list and there is not any more.** It
  offered a timer, login, time of day, day of week and never; four of
  those were never going to be written, and a list of five with one of
  them meant is a promise rather than a plan. A playlist turns on its
  clock, and on a video ending where it was told to.

  **How the three are wired.** None of the decisions are `Playlists`':
  it knows nothing about what a wallpaper is or whether a window is in the
  way, so `Main.qml` works out whether the clock should be standing still
  and says so with `hold()` -- a video is in charge and has not finished,
  or a window is covering the wallpaper and this playlist was not told to
  carry on regardless. `hold()` keeps what was left rather than starting
  the turn again, which is why the timer is single-shot and wound at every
  turn: a repeating one cannot be stopped and picked up. Measured end to
  end with a real cover: held with 8 515 ms left, and it turned 8.5 s after
  the screen came back. "Always begin with the first" is the one that is
  entirely `Playlists`' own -- it holds index nought out of the shuffle,
  and means nothing without shuffling, since an unshuffled playlist starts
  with its first wallpaper anyway.

  **What those four mean in Wallpaper Engine**, read off its own guides and
  developer replies, so that copying the behaviour is copying rather than
  inventing:

  - *Change wallpaper when the video ends* overrides the timer and applies
    to videos only -- the developers say it cannot be done for any other
    kind.
  - *Allow changing while paused* decides whether the playlist goes on
    turning while playback is paused, which happens when another window is
    fullscreen or when it is paused from the tray. With it off the timer
    pauses too, which is the usual reason a playlist looks stuck.
  - *Always begin with the first wallpaper* is what it says; paired with
    the video-end option it is how somebody builds an intro video followed
    by a second wallpaper.
  - A playlist does not cycle at all while Wallpaper Engine's own window is
    open.
- **A picker with its sections across the top**: Installed, Steam,
  Displays, Settings, all four built. Each an equal share of the width
  with its icon in the middle and its name as a tooltip, no line under the
  open one -- a tile behind it slides from one to the next, placed from
  the index (the user's design of 10 November; `currentItem` lags the
  index and a tile bound to it stayed behind).
- **Controls**, the fifth section of Settings (`Guide` cards in
  `SettingsView.qml`): how each part is used, in both languages, found by
  the settings search. Keep it true when behaviour changes.
- **Name order is Latin letters, then digits, then the rest**
  (`WallpaperCore::readsBefore`), each group in the locale's order.
- **What the wallpaper does when it is not wanted is Wallpaper Engine's
  four** (12 November; Settings › Audio for another program's sound, with
  following the music and its sensitivity, and Performance for the rest):
  keep playing, mute, pause or stop, for a covering window, another program with the keyboard, another
  program's sound (no stop) and the screens off or locked. `reasons` in
  `Main.qml`; Stop takes the first screen's `showing*` away, so the scene,
  video or page is freed and loaded again from the start. They replaced
  three switches, whose answers are read as the nearest of the four until
  one is chosen (`underWindow` and the rest in `Picker.qml`). Off or locked
  is `Asleep`: KWin's `org_kde_kwin_dpms` through KWayland (every screen
  off), and the screen saver's ActiveChanged. **Sensitivity to music** is
  Wallpaper Engine's recording volume, 100 % its default
  (`wallflux_audio_gain`), for scenes and pages alike.
- **The window's typeface is a setting** (12 November, Settings ›
  Appearance): the desktop's own (Noto Sans here, the default) or one of
  seven carried in the binary -- Inter, Geist, Manrope, Plus Jakarta Sans,
  Figtree, Onest, DM Sans, all OFL with the whole of Czech, licences in
  `fonts/` and installed beside the package's. `picker.typeface`, set as
  `font.family` on the window.
- **WallFlux's own icon** (12 November, the user's pick of three): a W on a
  rounded tile, `icons/wallflux.svg` for the menu, and in the tray the same
  shape in the window's theme and accent (`qml/emblem.js`), so it follows a
  theme, a custom accent and the desktop's own colours under System.
- **Next and Previous in the tray** (12 November) go through the running
  playlist, or through the library in the grid's order when none runs --
  counted from the one on show or, after Off, from the one played last.
  **Recent wallpapers** is a real submenu since `Tray` (0.37.1); nothing of
  it is kept for the next start, the user's word.
- **The workshop's search takes a number or a link** (12 November):
  Steam's text search reads titles and descriptions only, so an item's
  number or its page's address is asked for with
  `CreateQueryUGCDetailsRequest` instead (`askedFor` in `steamworks.cpp`).
- **A stranger's wallpaper runs nothing in the window** (12 November): a
  property's `condition` is read by `WallpaperCore::holds`, not run -- it was
  `new Function` inside the picker's engine, where it could reach every
  singleton -- and a web wallpaper may fetch `file://` only under the
  wallpapers' folders (`WallpaperFilesOnly`, `src/webguard.h`); measured, a
  page read `/etc/hostname` before. Workshop text is `Text.PlainText`.
  **Hardened further on 12 November, at the user's asking** ("somebody who
  downloads it and tries"): the release page opens only on github.com (and
  since 16 November WallFlux installs nothing itself, see Updates); ffmpeg and ffprobe are told `-protocol_whitelist file`
  and a video `-format_whitelist`, and a video wallpaper whose file does not
  begin as a container does is not played (`looksLikeVideo`), since Qt's
  player cannot be told the same; the KWin callbacks on the session bus
  answer KWin's connection only (`fromKWin`, measured: a spoofed
  `setCovered` from busctl refused); the single-copy socket is the user's
  only; a page may navigate to file, http(s), data, blob and about and no
  other scheme. Measured as already safe: today's ffmpeg refuses an HLS or
  concat list disguised as `.mp4` by itself; Chromium launches no external
  scheme without a click, and a wallpaper gets none; the web renderer runs
  under seccomp; package and texture sizes are capped. Still standing:
  ffmpeg, QuickJS and Chromium are as safe as their own code.
- **A Settings tab, laid out as a desktop's settings are now** (the user's
  sections of 9 November, after linux-wallpaper-engine-ux): a search and the
  sections down the left, the section's name large over one card to a
  setting; a search shows what matches in every section, each card saying
  where it is from, accents and case ignored. *General*: language, starting
  at login (`Autostart`, an entry of the user's own in `~/.config/autostart`,
  `Hidden=true` when off), the tray icon, starting minimised, minimising on
  close (off, closing quits), and a reset. *Audio*: volume, mute (the status
  bar's own), quiet while another program plays, quiet under a maximised
  window. *Performance / Display*: Wallpaper Engine's four quality presets
  (Low 10 fps, no MSAA, no glow; Medium 15; High 25 and MSAA 2x; Ultra 30),
  which set the next three at once and light none once those are changed by
  hand -- frame rate (10/15/25/30), smooth edges (MSAA off/2/4/8, the
  `SceneView`'s framebuffer, made again when it changes), the glow (see the
  post-processing trap) -- then texture resolution (half drops the top
  mipmap; a picture stored without mipmaps stays whole), default scaling
  (default and fill crop, fit adds bars, stretch -- `transform::set_scaling`
  for scenes, the `VideoOutput`'s fill mode for videos; a mirrored screen
  still crops), the music, the mouse, parallax, particles, and the usage
  -- CPU, GPU and RAM of WallFlux and its child processes in the status bar,
  read off `/proc` by `Usage` once a second while the window is on show
  (the GPU from each DRM client's `fdinfo`, counted once per client).
  *Appearance*:
  six themes as tiles with a picture of each (Light and Dark are Breeze's,
  System is the desktop's own `SystemPalette`, Steam its client's, Hard Light
  and Hard Dark high contrast), an accent colour, the tint and the status
  bar. Colours ease from one theme to the next (`shades` in `Picker.qml`,
  which the window's palette reads); a section slides in. Switches are
  pills, the volume's handle round. Every card works; nothing is a
  placeholder. **Remembered** in `~/.config/WallFlux/wallflux.conf` through
  QtCore's `Settings`, the ticked screens too -- not the wallpaper.

- **Updates of WallFlux itself** (`Updater`): `releases/latest` of
  invisigoth-cz/WallFlux, asked without a key when the window first opens
  and `checkUpdates` allows, and **only said**: Settings › General names the
  new version and shows the line that installs it, with Copy beside it, and
  opens the release page on a click. **WallFlux is installed and updated by
  `install.sh`** (17 November, the user's choice, since the AUR's new
  accounts stayed closed): `curl -fsSL
  https://raw.githubusercontent.com/invisigoth-cz/WallFlux/main/install.sh |
  bash`, or downloaded and read first, as the README shows. It builds
  `steamworks-sdk` from the AUR (`--asdeps`, no account or helper needed) if
  it is missing, and then `packaging/PKGBUILD` of the newest release's tag
  with `makepkg -si`; the same version installed already, it says so and
  stops. Piped, its stdin is the script, so pacman's questions are read from
  `/dev/tty`; `| bash -s -- --noconfirm` asks nothing, which is how it is
  tested in a clean `archlinux` container. It asks one question itself and
  then tells pacman and makepkg `--noconfirm`; keeps sudo's password alive
  while it builds (`sudo -v` and a loop); and if a
  `wallflux` of the user's is running, ends it and starts the new one with
  `systemd-run --user`, so it has the session's environment -- its own
  language, whatever country it is in -- and nothing of the terminal's: a
  WallFlux restarted from a terminal with `LC_ALL=C` in it sorted by code
  point (17 November). `setsid -f` where there is no user manager.
  **Update** in Settings is `Updater::install()`: that line in
  `konsole -e bash -c` under `systemd-run --user --scope`, so it outlives
  the WallFlux it ends, and held open at the end for a key. **Not detached
  alone**: a WallFlux started by an update is a service, a child of it is
  in that service's cgroup, and systemd ends the cgroup with the service --
  0.41.1 to 0.41.2 died at the swap and started nothing (17 November,
  reproduced with a dummy). The swap itself -- quit, wait, `exec wallflux`
  -- is one `systemd-run --user bash -c` for the same reason, and survives
  the script dying, which an older WallFlux's terminal still will; `Updater.command` is the line,
  in one place. Released 0.40.1 and earlier have only the line and Copy.
  **On CachyOS the password is asked for twice**, after the question and
  again before the package is installed: the system's own check on a
  package from outside its repositories and the AUR. The user's word of
  17 November: it stays, nothing is to get round it, and nothing is to
  promise the password once.
  **The trap's variables are global**: it runs after `main` has returned,
  and with `local` ones 0.40.1's script died on `work: unbound variable`
  and left the build behind. Before 17 November the advice was
  `paru -Syu`, and before 16 November WallFlux fetched the package and ran
  `pkexec pacman -U` itself. **The repository has to be public for any of
  it**: private, GitHub answers 404 to a request without a login, measured
  on 9 November.

- **Updates from the workshop.** `installedAt` and `latestAt` on each
  `Wallpaper` are `timeupdated` and `latest_timeupdated` out of Steam's
  `steamapps/workshop/appworkshop_431960.acf`; the keyless details answer
  adds `time_updated`, kept in the picker's `workshopUpdated`. Newer than
  `installedAt` by either is `picker.outdated`: a blue Update on the card,
  an Update button in the panel with `Steamworks.downloads`' progress, and
  Update all beside Refresh. `Steamworks.download()` is the update --
  `DownloadItem` fetches a newer version of an installed item -- and five
  seconds after one finishes the library is read again, since Steam writes
  its record after the files. Steam fetches updates by itself while it
  runs: on 9 November none of 58 was behind, so this is for when it did not.

- **A wallpaper's own properties**, in the library's panel under the
  author's row (`PropertiesView.qml`): what project.json declares, in its
  `order`, each shown while its `condition` -- the author's JavaScript,
  run with every setting as `{ value }` under its own name -- holds. Bool,
  slider (sent when let go), colour (`ColorDialog`, "r g b" from nought to
  one), combo, text input, text, file and folder. What is set is the
  picker's `propertiesOf`, by id and name, remembered, and laid over the
  declared values (preset values included) by `propertiesFor` and
  `valuesFor`. A page is told what changed through `applyUserProperties`;
  a scene is loaded again with `wallflux_scene_reload_with`, which lays the
  values into project.json before the bindings read it -- and **carries on
  from the scene that was running** (10 November): its clock, the particles
  alive (born colours and sizes kept, so a new colour spreads as they turn
  over), each system's run ahead, the canvases an effect gathers frames in,
  the videos' decoders and frame, and the scripts, which are not started
  again but told (`Scripts::retell`): the layer values the settings moved,
  their own `scriptProperties` bound to a setting, and `applyUserProperties`
  with only what changed. Objects are matched by id, whether the scene placed
  them and how many of that id came before, since a setting that shows or
  hides an object changes which are loaded. `carriesOnUnderNewSettings`
  draws Gears of War both ways: the same to the pixel. Rebuilding the whole
  scene rather than keeping bindings live was chosen because it covers every
  bound value there is, not just the ones this library uses; the price is
  the load itself, a moment of the render thread -- **119 ms for DOOM, 130
  for Dark Core, 19 for Gears** since the same day, from 964, 591 and 110:
  the running scene's textures are taken over (`texture::Uploads`, which
  also uploads a texture once per scene however many passes name it -- DOOM
  named 167 times), the package is opened rather than read whole
  (`Package::open`, the table and then each entry when asked: 105 ms of
  DOOM's 290 MB), and the video players are taken over whole, decoder and
  texture, so nothing is written out or asked of ffprobe again (85 of Dark
  Core's ms). What is left is parsing the textures and building the
  particle systems. Videos have only Windows' scheme colour and show none.

- **A wallpaper's compatibility, measured** (10 November): the grade it got
  the last time it was played, kept by id in `wallflux.conf` (`tried` in
  `Picker.qml`) -- *Works*, *Minor issues* where the renderer left something
  out (`SceneView.loaded(skipped)`, off `wallflux_scene_skipped`, named in
  the panel's row), *Broken* where it did not play or cannot be played at
  all, *Not tried yet* otherwise. A video is graded by `LoadedMedia` or an
  error, a page by how its load ended. No "major issues": nothing can tell
  them from minor ones without somebody looking. The filter's COMPATIBILITY
  section filters by it, and the grid reads `tried` only while it does.
  Kept and not removed, by the user's choice of 10 November, because the
  renderer does not draw everything: 3D models, applications and 15 of
  Wallpaper Engine's 19 own scenes, whatever the workshop sends next.

  **And tested for the whole library at once** (12 November, Settings ›
  Performance, the user's place for it): `Compatibility` starts this same
  binary as `wallflux --grade`, hands it every playable wallpaper as a JSON
  line on stdin and reads `wallflux-trial {...}` lines back -- a scene loaded
  with its settings and drawn 46 frames at 640x360 off screen (all black is
  Broken), a video opened by a `QMediaPlayer` with no output, a page by a
  muted `QWebEnginePage`: the window's own rules. A separate process so that a
  wallpaper that crashes the renderer crashes nothing on show; the one it was
  trying is marked Broken and a new process takes the rest. 63 in about 20
  s, `nice` 10, no sound, no microphone (`wallflux_options` with no music).
  Checked against the grades the library had from being played: all 25 the
  same, plus SCP's `chromatic_aberration` found as Minor issues by itself.

- **Favourites, as Wallpaper Engine's heart** (10 November): the workshop's
  own favourite on the account (`AddItemToFavorites`), so the same on every
  machine -- read out of `scripts.js`, where `setFavorited` is the whole of
  it. A heart on every card and on the panel's picture, red while it is on;
  ONLY FAVOURITES in the library's filter; *Your favourites* and *Friends'
  favourites* as Discover rows and as orders of the Workshop grid
  (`k_EUserUGCList_Favorited` for the account, found by
  `ISteamUser::GetSteamID`; `k_EUGCQuery_FavoritedByFriendsRankedByPublicationDate`).
  A personal row that finds nothing is not drawn. Nothing in Wallpaper
  Engine's code says a favourite changes what the workshop recommends.

- **What a scene made on the GPU goes with it** (10 November,
  `Scene::free`, called when the C interface drops it): every texture,
  program, buffer and framebuffer, each once, less what a scene loaded
  again took over (`handed_over`). Until then nothing was given back, and
  loading DOOM twelve times took VRAM from 10.7 to 15.8 GB; now it stays
  put. **And every offscreen picture is cleared when it is made** -- see the
  trap about memory given back.

- **A wallpaper on each monitor, or one cloned onto all** (24 November,
  Wallpaper Engine's "Wallpaper per display" and "Clone single wallpaper",
  copied whole at the user's word but for stretching and splitting, read
  off its `ui/dist/scripts/scripts.js` and `locale/`; the design is
  `docs/superpowers/specs/2026-11-24-tapeta-na-kazdy-monitor-design.md`).
  What plays is a `Display` (`qml/Display.qml`): the wallpaper (`Showing`),
  the playlist running on it (`PlaylistRun`), its pause and mute, its
  transitions, the scene's music and a surface on each of its screens.
  Per monitor there is one `Display` for every connector name seen in this
  run, kept while the monitor is unplugged -- its playlist held -- and
  picked up when it comes back; cloned there is one, on every monitor not
  left out of it (`excludedFromClone`). `picker.layout` is `perDisplay` (the
  default) or `clone`; the old ticks were read once as monitors left out
  (`screensCarried`).

  **Every monitor's sound plays at once**, each muted on its own
  (`mutedMonitors`, by key) apart from Mute all (`picker.muted`); **pause is
  the same, two things** -- a monitor's own (`pausedHere`) and everything's
  (`root.paused`), and Resume all leaves a monitor paused on its own
  standing (the user's word). The library works on the chosen monitor
  (`picker.selectedKey`, remembered; Choose monitor beside Filters, open
  with the window while there are several, `chooserOnOpen`): a double click,
  the strip, the panel's settings -- `propertiesOf` keyed `key|id`, the old
  bare-id values standing in -- and the footer. The Monitors tab (Displays)
  draws them with `DisplayMap`, and a right click changes, removes, copies
  to, swaps with and mutes, and while cloning makes a monitor the source
  (`cloneSource`) or flips its copy (`flippedMonitors`). **Profiles**
  (`Profiles`, `profiles.json`) are what the monitors ticked when saving
  show, under a name -- the chosen one ticked to begin with -- loaded from
  the tab or the tray onto those monitors alone, each the one it came from,
  the others left as they are (the user's word of 25 November: a profile
  of one monitor is not to start anything on the others; until 0.45.0 it
  kept every monitor, as Wallpaper Engine's does); cloned, the one picture.
  Nothing plays at start still. The tray
  has a menu per monitor with two or more and the layout per monitor, and
  today's menu otherwise. A window covering a monitor pauses that one, or
  every one with "Pause all"/"Stop all" (`whenCovered` `pauseall`/`stopall`);
  so does another program with the keyboard (`whenFocused`, the same six,
  `ScreenCover.focusedScreen` saying which monitor it is on -- the user's
  word of 24 November, after dragging VS Code to the other monitor paused
  both).
  Measured on this laptop (A8-6410) at 1920x1200 on two virtual outputs:
  one cloned light scene 9 % of a core, two light scenes 21 %, DOOM alone
  124 %, DOOM and Space Visualizer 161 % -- the four cores saturate there,
  and a heavy scene simply draws fewer frames, as in Wallpaper Engine; no
  brake was built (the user's choice).

  **Worked out once while cloned.** The first screen of the clone plays the
  wallpaper with its picture as a layer, and every other screen is a
  `Mirror` showing that texture, cropped to fill. Until 6 November each screen
  ran a simulation, a decoder and a page of its own: twice the CPU on two
  screens (Space Visualizer 13 % of a core against 7 now, 6 on one), a video's
  sound twice over, and the screens not in step. Checked in `kwin_wayland
  --virtual --output-count 2` under `dbus-run-session`, so that the real
  KWin's scripts are left alone: the two screens' grabs identical to the pixel.

  **A window covers a display only where it covers every one of its
  screens** (23 November, found on the laptop with a monitor beside it):
  the KWin script names the outputs a window fills, `ScreenCover.coveredScreens`
  lists them, and each `Display` checks its own. Until then a maximised VS
  Code on the laptop paused the wallpaper on the monitor too.

  **Testing it without touching the desktop**: a `kwin_wayland --virtual
  --output-count 2` under `dbus-run-session --config-file` with a bus whose
  only `servicedir` is empty, so nothing is activated (no portal, no
  ksecretd crashing into DrKonqi), a bare `StatusNotifierWatcher` in Python
  for the tray, `XDG_RUNTIME_DIR` and `XDG_CONFIG_HOME` of its own (the
  single-copy lock and the settings) and `PIPEWIRE_RUNTIME_DIR` the real
  one for sound. Outputs are unplugged with `kscreen-doctor
  output.Virtual-1.disable` **with that session's environment** -- without
  it the command reaches the real desktop. Grabs with `spectacle -b -n -f`
  inside it; the tray read over `com.canonical.dbusmenu GetLayout` and
  clicked with `Event`.

  **The screens are Qt's own**, `Qt.application.screens`. No `xrandr`, no
  `wlr-randr`, no `kscreen-doctor`, nobody's output parsed: Qt names a
  screen after its connector, which is the same name KDE's display
  settings show and the same one under `/sys/class/drm`. Checked against
  all three on this machine.

- **A panel sliding out does not reflow the grid** (17 November, the user's
  choice after trying a panel over the grid -- "the wallpapers must always
  be seen" -- and one standing beside it for good -- an empty panel at
  start). Both panels take their room beside the grid, and `gridArea` in
  `Picker.qml` lays the grid out once, at `held`: the width it will have
  when the panels have stopped, worked out from which are `open` rather
  than read off the layout, which reads a width a frame stale when the
  animation ends and laid the grid out twice. Meanwhile the grid is
  `scale`d to the room there is, the scrollbar's 30 pixels left out of the
  ratio so that the cards land where the new layout puts them, its height
  divided by the scale so the rows below come up, and `cacheBuffer` keeps
  those rows' pictures loaded. `columns` is counted off the width with no
  panel out, so a panel shrinks the cards rather than moving them to other
  rows -- under 170 across a column goes, and the cards glide (`gliding`,
  a `Behavior` on x and y enabled for 300 ms, and only on a card the grid
  has placed once, or a new one flies in from the corner). `contentY` is
  scaled with the cell so the same rows stay in view. Measured in a
  virtual KWin: opening the details panel laid the grid out 11 times
  (5, 4, 3, 2 columns), now once at 5. The Steam tab's Workshop grid
  (`gridArea` in `WorkshopView.qml`) is the same, measured the same.
- **A panel beside the grid saying what a wallpaper is.** A click opens
  it and a click on the same card puts it away again, in either tab; so
  does the cross in the picture's top right corner, back since 10 November
  at the user's asking, with the heart in the top left (`closeAsked`). It slides rather than appearing, two tenths of a second of
  width, with its contents held at their full width and cut off by the
  edge so that nothing inside is squashed on the way. In the library both
  halves wait out the same 400 ms as the double click, because opening
  and closing each move every card and a card that moves out from under
  the pointer cannot be clicked a second time. Inside: the picture, the
  name, and
  under that a button saying who made it -- their avatar, their name and a
  magnifier -- which turns the workshop grid into everything else they
  have published, with a blue chip beside the filters to come back from.
  It is there from the moment the panel opens, with the name filling
  itself in a moment later. Then what is known, a
  row to a line with an icon and a hairline under it: the kind, the picture
  size, what it takes up, the age rating, whether it answers to sound. The
  tags are chips under their own heading rather than a line of commas.
  Under all of it the one thing to do about it: green Download in the
  workshop, red Remove in the library, each with a line under it saying
  what will happen. One click picks a wallpaper out; two put it on the
  desktop.

  **The library has the same row**, although project.json does not say who
  wrote a wallpaper -- checked across every one of them. It takes two
  keyless questions instead of one, both in `qml/steamprofile.js`: the
  workshop is asked about this id -- which answers with the maker and with
  the full list of tags, see the trap -- and then who that account is. The
  magnifier there opens the Steam tab and asks the question in the grid
  that can answer it. **Coming back is the Workshop button**, which clears
  the author and asks again: clearing alone does nothing, because the grid
  is already in the Workshop's own mode and `onModeChanged` never fires.
- **Several wallpapers chosen and removed at once** (24 November, the
  user's word, as Wallpaper Engine chooses several): Ctrl and a click,
  Shift and a click, a band dragged over the grid, Ctrl and Space, Shift
  and an arrow; Escape lets go. `picker.selection`, the logic in
  `qml/selection.js` (`tests/tst_selection.qml`, run by qmltestrunner). A
  bar above the grid says how many and removes them after one question,
  which says which cannot go without Steam. The grid takes no mouse drag
  (`acceptedButtons: Qt.NoButton`, Qt 6.9 and later) -- the drag is the
  band -- and keeps its own wheel. Made not interactive and scrolled by a
  `WheelHandler` instead, it scrolled for a test's wheel and not for a real
  one on the desktop. **A new .js or
  .qml file wants its `QT_RESOURCE_ALIAS` line in CMakeLists.txt**, or it
  lands under `qml/` in the resources and an import of it fails at start
  -- the build says nothing.

- **A grid the arrow keys walk.** The window hands the keyboard to the
  grid when it opens: arrows turn one card's band blue, Enter puts that one
  on the desktop, Space ticks it into the playlist. A click moves
  the band as well, so the mouse and the keyboard do not each keep a place
  of their own. The panel follows it only once it is open -- opening
  one because somebody pressed Right would be the window deciding what the
  question was.

- **A picker that looks like something**: dark and light themes switched
  without a restart, a search that narrows the grid as it is typed and a
  cross to empty it, name order either way round, a band around each
  preview saying whether it can be played -- lit up while the pointer is
  on it -- a preview that leans in under the pointer, the whole window
  tinted by whatever the panel is showing, rounded buttons with room around their words, filters as
  one value to a row beside the grid, and scrollbars that stay visible in
  a strip of their own rather than fading out over the pictures. In the
  corner of the footer an `i`, which says the version when it is hovered
  and does nothing when it is clicked -- unless a newer WallFlux is out:
  then it is yellow (`Look.notice`, at least 3.5 to 1 against the window
  in all six themes, System's by the desktop's brightness) and a click
  opens Settings › General, where Update is (17 November; it replaced a
  "WallFlux x is out" button beside it that was all but invisible in the
  dark theme) -- `Application.version`, out of
  `WALLFLUX_VERSION` in CMakeLists.txt, which is `project()`'s own number
  rather than a third place to keep it.
- **An Arch package** (`packaging/PKGBUILD`, `wallflux`, built from the
  release's tag on GitHub by `install.sh`, ready for the AUR), with an autostart entry that has been
  verified through a real logout and login. Its `check()` leaves out
  `sceneview` and `mirror`, which want a window, a GPU and this machine's
  wallpapers.

Of the wallpapers found on the development machine -- 53 at the last count,
and the number grows as the workshop is used -- 12 are web and video and
41 are **scene**, and all of them play. Diablo IV Lilith was long taken
for the one 3D model among them and turned down; its one image is a video,
and it plays since scenes' videos do.
A scene whose `scene.pkg` is missing is listed, dimmed, and a click does
nothing.

## Layout

```
src/main.cpp          WebEngine init, the single-copy lock, and the socket a
                      second start knocks on to show the window
src/desktop.h         Desktop: turning a window into a wallpaper
src/desktop.cpp       the layer-shell settings, asked for per window
src/sceneview.h       SceneView: a scene wallpaper as a QML item, and
                      SceneSounds: the music it carries, unpacked
src/sceneview.cpp     the renderer driven from Qt's render thread
src/wallpapercore.h   WallpaperKind::Kind, Wallpaper (Q_GADGET), WallpaperCore
src/wallpapercore.cpp scanning Steam's folders, reading project.json
src/language.h        Language: the EN/CZ switch, and what it has to call
src/language.cpp      installs a QTranslator and retranslates the engine
src/playlists.h       Playlists: wallpapers that take turns, and the file
src/playlists.cpp     reading and writing ~/.config/WallFlux/wallflux/
src/playlistrun.h     PlaylistRun: one playlist running, one per display
src/playlistrun.cpp   the clock, the shuffle, holding and turning
src/showing.h         Showing: the wallpaper one display shows
src/showing.cpp       quiet when nothing changes, reread on a rescan
src/profiles.h        Profiles: the arrangement of the monitors under a name
src/profiles.cpp      profiles.json beside the playlists
src/otheraudio.h      OtherAudio: is another program using the speakers
src/otheraudio.cpp    `pactl subscribe` for when, `pw-dump` for what is running
src/steamworks.h      Steamworks: can the workshop be asked, and the way in
src/steamworks.cpp    dlopen of libsteam_api.so, by name, nothing linked
src/tags.h            Tags: what a workshop tag is called, and the genres
src/tags.cpp          reads Wallpaper Engine's locale/, 36 languages
src/screencover.h     ScreenCover: is a window covering the wallpaper
src/screencover.cpp   a script loaded into KWin, which calls back over D-Bus
src/mirror.h          Mirror: the first screen's picture on another screen
src/mirror.cpp        a texture shared between two windows' contexts, fenced
src/pointer.h         Pointer: where the mouse is, for the scenes
src/pointer.cpp       another KWin script, loaded only while it is watched
src/kwinscript.h      KWinScript: a script run inside KWin, shared by both
src/kwinscript.cpp    written to a file, unloaded first, loaded, started
qml/Main.qml          the program: the tray icon, the picker, a Display per
                      monitor or one cloned, and what is done across them
qml/SceneSound.qml    one sound object of a scene, played as it asks
qml/WallpaperSurface.qml  one screen's wallpaper: the layer-shell window,
                      the web view, the player, and what a transition
                      holds over them
qml/SettingsView.qml  the Settings tab: sections down the left, a row to a
                      setting; the values live on the picker
qml/Toggle.qml        a switch as they are drawn now, told and answering
qml/Heart.qml         the favourite's heart drawn on a picture, told and
                      asking
qml/PropertiesView.qml  a wallpaper's own settings, in the library's panel
src/autostart.h       Autostart: starting at login, by the user's own entry
src/autostart.cpp     ~/.config/autostart/wallflux.desktop, Hidden or not
src/power.h           Power: on a battery or not, and how full, off UPower
src/power.cpp         the system bus, OnBattery and the display device
src/updater.h         Updater: whether a newer WallFlux is on GitHub
src/updater.cpp       releases/latest, its tag against this version
src/usage.h           Usage: what WallFlux costs, for the status bar
src/usage.cpp         /proc: stat, status and fdinfo of it and its children
src/asleep.h          Asleep: every screen off (DPMS) or the session locked
src/asleep.cpp        KWayland's Dpms per output, the screen saver's signal
src/compatibility.h   Compatibility: the library tried out, and `--grade`
src/compatibility.cpp the trial process and the window's end of it
src/webguard.h        WallpaperFilesOnly: a page's file:// kept to wallpapers
qml/emblem.js         the tray's icon in the window's colours, as SVG text
src/tray.h            Tray: the panel's icon and its menu, built with Qt Widgets
src/tray.cpp          the menu built from QML's list, the icon drawn per size
icons/wallflux.svg    the application's icon, installed to hicolor
fonts/                the seven typefaces and their OFL licences
qml/DisplaysView.qml  the Displays tab: the layout, the monitors and each
                      one's menu, profiles
qml/DisplayMap.qml    the monitors drawn to scale with what they show, their
                      numbers and speakers -- the tab's and the chooser's
qml/Display.qml       what plays on one monitor, or cloned on all: the
                      wallpaper, its playlist, pause, transitions, music
qml/Picker.qml        the picker window: header, search, grid, footer
qml/WallpaperCard.qml one preview, its band and its name
qml/Details.qml       what is known about one wallpaper, beside the grid --
                      shared, with the one button handed in as a child.
                      `facts` is `{ label, value, icon }`, `tags` is its
                      own list drawn as chips, `author` is
                      `{ name, avatar }` or null
qml/PlaylistBar.qml   the playlist being put together, along the bottom
qml/FilterPanel.qml   the filter popup: chips in sections, and what is on
qml/PreviewTile.qml   a preview and a name -- the shape of a card, shared
                      by the library and the workshop
qml/WorkshopView.qml  the Steam tab: Discover rows and the Workshop grid
qml/WorkshopFilters.qml  the workshop's filter popup, the whole vocabulary
qml/Chip.qml          one value, on or off -- shared by both filters
qml/ChipSection.qml   a heading and the values under it, one to a row
qml/Scroller.qml      a scrollbar that stays visible, used by every view
                      -- except the details panel, which passes
                      `policy: ScrollBar.AsNeeded`
qml/steamprofile.js   who made a workshop item, and who that account is:
                      two keyless questions, shared by both tabs
qml/RoundedButton.qml a button with its corners off -- every button here
qml/RoundedTool.qml   the same corners for an icon on its own
qml/RoundedCombo.qml  the same corners for a drop down -- every one here
qml/SearchField.qml   the one search field: a pill, a magnifier, a cross
qml/RoundSlider.qml   the one slider: a thin track and a round handle
qml/Look.qml          singleton: what a theme is besides its palette --
                      positive/negative buttons, card bands, tags and how
                      the chosen card is marked (`ring`); the picker hands
                      it the theme and the accent
tests/tst_wallpapercore.cpp   QtTest against a fake Steam tree in a temp dir
tests/tst_otheraudio.cpp      reading pw-dump's answer, no sound card needed
tests/tst_screencover.cpp     the states, with the session bus pointed nowhere
tests/tst_pointer.cpp         known only while watched, the same way
tests/tst_profiles.cpp        a profile kept whole, monitors away included
tests/tst_playlists.cpp       what a playlist remembers and how long a
                              wallpaper stays -- fourteen seconds of suite
                              time on purpose, see below
tests/tst_tags.cpp            tag names against Wallpaper Engine's catalogue
tests/tst_autostart.cpp       the user's entry written hidden and not
tests/tst_usage.cpp           /proc's lines read, and this process read
tests/tst_power.cpp           mains for ever without UPower
tests/tst_updater.cpp         versions as numbers
tests/tst_mirror.cpp          one window's picture in another, the right way
                              up and cropped, and kept up to date
tests/tst_sceneview.cpp       a scene drawn through the item, checked against
                              the same scene drawn directly -- and the right
                              way up
i18n/wallflux_cs.ts   the Czech catalogue, compiled into the binary
packaging/PKGBUILD    builds from this checkout's git history
install.sh            installs and updates: the SDK out of the AUR, then
                      the newest release's PKGBUILD through makepkg
wallflux.desktop      installed to applications/ and to /etc/xdg/autostart/
Cargo.toml            the Rust workspace: the two crates below
crates/wpe-format/    reads the scene format -- .pkg, .tex, scene.json,
                      materials, effects, particles, puppets (.mdl), and the
                      shader dialect
crates/wallflux/src/video.rs  a video a scene uses as a texture, decoded by
                      ffmpeg run as a program, one player per clip
crates/wallflux/src/text.rs  a scene's text, laid out and drawn into a picture
                      with ab_glyph, which the engine's font shader shows
crates/wallflux/src/script.rs  a scene's JavaScript, run with rquickjs, and
                      `script/host.js`, the globals a script finds -- ours,
                      written from what the library's scripts use
crates/wallflux/      draws a scene with OpenGL; `examples/shot.rs` renders
                      one wallpaper to a PNG, which is how it is checked.
                      `src/ffi.rs` and `include/wallflux_scene.h` are the
                      four calls the program makes; `src/main.rs` is the
                      old standalone viewer and is not part of the program
tools/sweep.sh        `shot` over every scene wallpaper on the machine,
                      pictures and one log into the directory it is given
```

`WallpaperCore` is a QML singleton named `Wallpapers`, declared with
`QML_NAMED_ELEMENT` + `QML_SINGLETON` (**not** `setContextProperty` or
`qmlRegisterType`, which qmllint cannot see):

- properties: `wallpapers` (QList<Wallpaper>)
- methods: `rescan()`, `rescan(roots)`, `find(id)`, `played(id)` (first in
  `recent`),
  `discard(id)` (deletes the workshop folder -- the other half of giving a
  wallpaper back, see the trap), `details(id)` (`{ size, resolution }`,
  read off the disk when the picker shows one card and kept afterwards)
- static: `scan(roots)`, `steamRoots()` (the usual Steam installs and every
  library their `steamapps/libraryfolders.vdf` lists; `Assets::locate()` in
  `wpe-format` reads the same file for the renderer), `readable(text)` -- the text
  without Chinese, Japanese and Korean (the user's word of 10 November),
  tidied, or whole where nothing readable would be left; applied to a
  wallpaper's title at the scan and a workshop answer's title -- **not** to
  a property's label, where the user found it ugly and confusing
- `Wallpaper`: `id`, `title`, `kind`, `source`, `preview`, `tags`, `rating`,
  `resolution`, `sizeOnDisk`, `added`, `bytes`, `soundKnown`, `hasSound`,
  `hasFile`, `playable`
- also on the singleton: `recent` (ids chosen in this run, newest first)

What a display shows is a `Showing` (`QML_ELEMENT`, `src/showing.h`):
`library` (`Wallpapers`), `current`, `select(id)` -- **silent when the
choice has not changed**, the trap below -- and `clear()`; it rereads the
library on a rescan and lets go of a wallpaper that left the disk.

`Steamworks` and `Tags` are singletons of the same module:

- `Steamworks`: `available`, `trouble` (a `Trouble` enum, **not** a
  sentence -- see the trap), `busy`, `found` and `totals` (keyed by the
  name the asker gave the question), `downloads` (id -> 0..100, -1 refused);
  `browse(key, order, page, tags, search, urgent, creator)` -- a tag
  starting with `-` is left out rather than required --, `download(id)`,
  `favorites` (ids) and `favorite(id, on)`, the orders `MyFavorites` and
  `FriendsFavorites` beside the three rankings,
  `remove(id)` (unsubscribes, and nothing else -- the folder is
  `Wallpapers.discard()`'s), `retry()`. An answer's entries carry `owner`,
  the maker's steamID64 **as a string** -- it does not fit a double and QML
  would round it. `creator` narrows the question to one account's published
  work, which is a different call of Steam's with its own orders, so
  `order` says nothing while it is set.
- `Tags`: `genres` (25), `types`, `ratings`, `resolutions`, `properties`;
  `name(tag, locale)`, `labelled(tags, locale)`

`Playlists` is another singleton of that module, and, with the settings
`Picker.qml` keeps in `wallflux.conf`, the only thing in this program that
remembers anything between runs:

- `all` (each `{ name, ids, hours, minutes, seconds, shuffle, videoEnds,
  whilePaused, alwaysFirst, transition, transitionMs }`); `save(name, ids,
  how)` -- `how` being those settings by name rather than nine positional
  arguments -- `forget(name)` and `rename(from, to)`, which say
  `forgotten`/`renamed` so every run of it follows, `named(name)`, and
  the public static `staysFor(one)` -- the three numbers added up in seconds, never
  less than five, worked out in one place because that is the sum a test
  can check; and `asClock(seconds)` -- the same rule read backwards,
  floor and ceiling and all, which is what the settings popup hands its
  three typed boxes to. An exact inverse, so the boxes can never show an
  interval that is not the one that will run: three noughts typed in read
  back as five seconds, which is what the playlist will actually wait
- running one is a `PlaylistRun` (`QML_ELEMENT`, one per `Display`):
  `playing`, `changesOnVideoEnd`, `changesWhilePaused`, `transition` and
  `transitionMs` (what the *running* one was told, empty or false while
  none is), `start(Playlists.named(name))`, `stop()`, `next()` (on to the
  next one now and wind the clock again), `previous()`, `hold(bool)` (stop
  the clock without stopping the playlist, and pick it up where it left
  off). It plays nothing itself: it says `plays(id)` and its `Display`
  puts that on show, the same place a click on a card goes
- `Profiles` is the same shape for whole arrangements (`profiles.json`
  beside it): `all`, `save(name, arrangement)`, `forget`, `rename`, `named`
- the file is `playlists.json` under `QStandardPaths::AppConfigLocation`,
  which on this machine is `~/.config/WallFlux/wallflux/`

`Pointer` is one more: `position` in the desktop's coordinates, `known`,
and `watching`, which `Main.qml` binds to a scene being on show and in
view. Each screen's `SceneView` is handed `pointer` -- the position less
`Screen.virtualX/Y`, or undefined -- and passes it on as a fraction.

`OtherAudio` and `ScreenCover` are singletons of the same module -- `playing`,
and `coveredScreens` (by connector name, none while the desktop is shown)
with `focused` (another program's normal window has the
keyboard; the KWin script is handed our pid, so the picker does not count) --
with no business of their own in what the answer is used for. So is `Power`
(`hasBattery`, `onBattery`, `percent`). A `Display`'s `covered` is a window in
front where `quietUnderWindows` asks, `focused` where `pauseOnFocus` asks, or
a battery under `batteryFloor`; `drawnFor` is Low on a battery where
`batteryLow` asks. It is put together with the rest in one line:

```qml
readonly property bool silent: display.mutedHere || display.mutedAll || display.covered
    || display.reasons.some(r => r.on && r.mode !== "keep")
```

## Decisions that are settled — do not reopen without being asked

- **GPL-3.0-or-later, with an added permission for the Steamworks SDK** and
  for nothing else, written out in LICENSE-EXCEPTION. The licence itself is
  not up for discussion; the permission is there so the Steam part can be
  written at all, since a workshop item for a paid application can only be
  fetched through Valve's proprietary SDK. Every other library WallFlux
  links must still be GPL-compatible.

- **`steamworks-sdk` from the AUR is a dependency, not a suggestion.**
  Nothing else on a Linux machine carries `libsteam_api.so`: Steam ships
  none, and Wallpaper Engine carries only the Windows DLL -- both measured.
  That package puts the SDK under `/opt` in its entirety, unmodified, with
  Valve's licence beside it, which is what the SDK's terms require and what
  carrying it inside this package would break. So it is depended on and
  `dlopen`ed, never shipped and never linked. An AUR helper resolves it;
  plain `makepkg -si` will want it built first. The price, which was said
  out loud before it was chosen: everyone who installs WallFlux gets
  Valve's SDK on disk whether they open the Steam tab or not.

- **Qt draws web and video wallpapers; scene wallpapers are our own
  renderer.** It is Rust with OpenGL, under `crates/`, and it was parked on
  the branch `zaloha-puvodniho-pokusu-o-vlastni-engine` from the day main
  was cleared for Qt until 27 October, when it came back into main by a
  merge -- so `git log crates/` carries its whole history.

  **It is linked into the program, not run beside it.** Cargo builds it
  into a static library inside the CMake build tree, and `SceneView` -- a
  `QQuickFramebufferObject` -- drives it on Qt's render thread with Qt's
  own OpenGL context. Chosen over a separate process because a scene in
  an item is one more thing on the one surface: the transitions grab it
  the way they grab a video, and there are not two layer-shell surfaces
  from two programs in the same layer. The price, said before it was
  chosen: a renderer that crashes takes the tray icon and the picker with
  it, which is why every call into it is behind `catch_unwind` and a scene
  that panics is not drawn again. Building needs `cargo`; running needs
  nothing more than before.
- **Nothing plays at startup.** The program comes up as a tray icon and the
  desktop keeps the wallpaper it already had until somebody picks one. The
  choice is not remembered between runs, by design.

  **Playlists are the one exception, and were asked for as one.** A saved
  playlist is written to a file and is there at the next start -- but it is
  not started at the next start, which keeps the rule above intact.
- **The tray icon is the whole interface.** Not a CLI, not a Plasma plugin;
  both were considered and turned down. **It can be switched off** (the
  user's word, 9 November): then starting WallFlux again from the menu shows
  the window of the copy already running -- `main()` listens on
  `wallflux.socket` beside its lock, and a second start knocks and leaves.
- **Layer-shell is set up in C++**, before the window reaches the
  compositor — which is why a surface is built `visible: false` and shown
  by hand afterwards. It used to be `main()` doing that to the one window
  there was; a wallpaper belongs to one screen, so there is one window per
  screen now and they come and go with the monitors, which only QML is in
  a position to notice. `Desktop::asWallpaper(window)` is what `main()`
  used to write out.

## Traps, each one found the expensive way

- **A texture made with no pixels holds what the memory held before.**
  `tex_image_2d` with no data leaves whatever was there, which is nought
  only while the GPU hands out memory nobody used. Once scenes gave theirs
  back, a buffer read before its first write -- a motion blur's last frame
  on the first frame -- showed what the scene before had left, and
  `drawsASceneIntoTheItem` wandered from 2 to 19 apart from run to run.
  `make_target` clears what it makes; the same rows now read 0, where they
  had read 6.7, 1.4 and 3.7 apart before any of this, which was the same
  thing in fresh memory.

- **A `QQuickFramebufferObject` drawn with OpenGL comes out upside down.**
  OpenGL draws from the bottom up and Qt 6 shows the framebuffer as it
  is; `setMirrorVertically(true)` is what turns it over. Nothing about it
  looks broken in code, and a scene upside down is still a scene, so
  `tst_sceneview` draws the same wallpaper straight through the C
  interface and checks the item against it: 0.86 apart the right way up,
  47 apart the wrong way.

- **A renderer that asks to be drawn again is drawn without being
  synchronized.** `Renderer::update()` re-renders the framebuffer and
  never calls `synchronize()`, so anything left there for the item to
  pick up waits for somebody to change the item -- for a wallpaper that
  failed to load, never. The reason goes to the item as a queued call.

- **Qt putting its state back took the indices off the particles.**
  `QQuickOpenGLUtils::resetOpenGLState()`, called after every frame,
  binds no element buffer *before* it unbinds the vertex array -- and the
  element buffer belongs to whichever array is bound, which was the
  particles'. From the second frame on every particle system was drawn
  without indices: no smoke anywhere on the desktop, while `shot`, which
  resets nothing, drew all of it -- so every sweep looked right. `Scene::draw`
  leaves no array bound; `survivesQtPuttingItsStateBack` fails without that
  (10.4 apart). **A picture from `shot` says nothing about the desktop
  where state between frames is concerned.**

- **Two windows read each other's textures only if `main()` asks for it.**
  `Qt::AA_ShareOpenGLContexts`, before the application is built. QtWebEngine
  used to set it and in Qt 6.12 does not: measured, the two screens' contexts
  were in different share groups, the mirror's texture number named some other
  texture of its own, and the second screen came out black with no error.
  `tst_mirror` sets it as `main()` does, so it cannot see this; the two-screen
  check in a virtual KWin can.

- **The scene graph has to be OpenGL, and is told so.** `main()` sets it
  before any window exists. It is OpenGL on this machine without being
  asked, which is exactly why it has to be said: elsewhere Qt is free to
  pick Vulkan, and every scene would draw nothing with no error at all.

- **A scene's bound values are written twice, and the scene's copy is
  stale.** A value bound to a user property sits in `scene.json` beside
  the binding and again in `project.json`, where the property is declared;
  Wallpaper Engine plays the second. 205 of the 365 in this library
  disagree -- one wallpaper's scene tints everything red at full
  strength, and its project has the tint off.

- **A colour bound inside `instanceoverride` is the project's too, like every
  other bound value.** Dark Core's scene.json has its tubes blue, purple and
  orange and its project orange for all of them, and on Windows, untouched, all
  of them burn orange -- the user's word and a recording with music. An earlier
  rule kept the scene's colours, read off a recording in silence where the
  tubes hardly burn at all: faint glints in a dark tube are no evidence of a
  colour. Look at a particle system's colour while it is lit.

- **A scene's `instanceoverride` multiplies the particle system it
  places, and its children with it.** 462 particle objects carry one
  -- alpha, size, lifetime, rate, speed, count, and a colour written
  either as `colorn` (0-1) or, in older scenes, `color` (0-255), never
  both. Unread, every smoke kept its preset's colours, which is where the
  pastel blobs in half the library came from. **A child takes the whole of
  it**, found on 9 November: a child Wallpaper Engine spawns is bound with
  its own bindings and the same object's values, exactly as the parent
  was. `rate` is not a binding at all except to `turbulence`'s
  `timescale`: the object multiplies the system's step by it, and the
  children run in that time. Until then children
  took only alpha, rate and colour -- a rule read off Star Citizen's embers
  burning the sky white, which other mistakes of the time explain -- and
  Space Visualizer's lava spark trails, children of systems at a size of
  0.2, were thick bright plumes where Windows has fans of hair-thin red
  lines. With the size: Star Citizen 7.62 to 6.94 levels off its recording,
  Terminator Resistance 3.73 to 3.22, Blue Dawn and the rest unmoved.

  **Except the colour, which reaches a child as far as it moved the
  parent** (25 November): the child's colours are moved by the scene's
  colour less the *parent's* own, and not multiplied in again for an old
  scene. Measured from its own and multiplied, Star Citizen's
  `emberglow_small` -- a warm-white halo under embers set to orange --
  burned orange over half the ground; against the recording, eight moments
  averaged, warmth R-B beside the drill +13 and +11 became +7 and +5,
  Windows +4 and +2. A sweep moved Star Citizen, Star Destroyer (its fire's
  mean colour nearer the recording) and Blue Dawn a hair; 12 scenes here
  have a recoloured system with children. The user could not tell by eye.

  **And a child that bursts at its parent's events is a whole system per
  event**, up to the child entry's `maxcount` of them at once (missing, it
  reads as nought and nothing stops it): Wallpaper Engine starts one for
  each particle born. One shared pool let a new burst have only what the last ones
  left; `burst_per_event` gives each burst the file's `maxcount`. Space
  Visualizer's sparks against Windows, the brightest half-percent of what
  they add: 42/72/22 against 49/75/25 for the white ones, 48/25/17 against
  64/30/15 for the red (94/31/18 before the size went down).

- **A scene's y runs up, and its angles are radians.** Origin in the
  bottom-left corner: rain falls at a y of -10 000, snow at -50 to -90, a
  torch's embers rise on +10, and Space Marine II's aliens -- cut out of
  the picture behind them -- sit back in their holes only that way round.
  Read from the top, which this renderer did until 29 October, every layer
  that was not centred was mirrored to the other half of the screen and
  every particle moved upside down; a scene that is mostly a centred
  background looks almost right while it is wrong, which is how it lasted.
  An image is turned over on its own quad (`-size[1]`) so its rows, which
  run down, still stand up; a particle system is not. And `angles` is in
  radians -- 308 objects in 28 wallpapers, the largest 3.142 -- where
  `transform::model` read degrees and turned every one of them a
  fifty-seventh of the way.

- **A scene is opaque, and blending leaves rubbish in the alpha channel.**
  The colours come out right; the alpha is each layer's alpha squared and
  means nothing, and whatever shows the picture reads it -- Pyramid
  Spaceships came out a quarter transparent where its smoke was, which in a
  montage over white looked like white smoke. Wallpaper Engine and Almamu
  mask alpha on the last pass; `Scene::draw` sets it to one at the end.

- **A layer's `colorBlendMode` is a pass of its own, after its effects.**
  `materials/util/effectpassthrough.json` with `BLENDMODE` set, reading
  the scene so far at unit four -- `ApplyBlending`'s numbering in
  `common_blending.h`, 31 being plain addition. Drawn as an ordinary layer,
  Time Reactor's gold lines were grey and Augmentation's iris another
  colour. **Bloom** is likewise the engine's own four shaders
  (`downsample_quarter_bloom`, `blur_h_bloom`, `downsample_eighth_blur_v`,
  `combine`) over the finished scene, `g_TexelSize` a texel of the scene.

- **Almamu's renderer is the quickest second opinion, and it can be run
  where nobody sees it**: inside `kwin_wayland --virtual --width 1920
  --height 1080 --socket <name> "/usr/bin/linux-wallpaperengine --window
  0x0x1920x1080 --silent --disable-mouse --screenshot <png>
  --screenshot-delay 180 --assets-dir <WPE assets> <wallpaper dir>"`. It
  does not quit after the screenshot. Preview, Almamu and ours side by side
  is how the two traps above were found. **It does not draw a particle
  system's children at all**, so where those matter the preview is the
  only reference.

- **A scene's video is an image whose `.tex` holds an MP4**, flagged as
  video and carried whole as its one surface -- not a 3D model and not
  something only a script asks for: Diablo IV Lilith is one such image and
  nothing else, and Dark Core shows one clip on four layers. It is written
  into the program's cache beside the music
  (`~/.cache/WallFlux/wallflux/scenes/<id>/`), since an MP4 cannot be read
  through a pipe, and `ffmpeg` decodes it to RGBA down one, a frame at most
  1920 across, looping; a jump of more than two seconds starts it again
  where the scene is. A full pipe is what stops a decoder that is ahead.
  **The plexus panel in Dark Core shows blue in silence and Almamu's shows
  an equaliser**: its alpha is a script on the music, and Almamu's scripts
  have no `registerAudioBuffers` -- the same reason his blue ring is lit
  and ours is dark until something plays.

- **A particle operator's missing values are Wallpaper Engine's own, and
  they are known.** Wallpaper Engine fills in what the file leaves out for
  each operator, and those values are what is used here. Guessed instead, smoke was the worst of it. `alphafade`'s two times
  are **fractions of the particle's life**, in until the first and out from
  the second, 0.5 each when missing -- they were read as seconds with no
  fade out, so smoke held its full alpha all its life and vanished at once.
  `sizechange` and `alphachange` run from 1 to **0**, `colorchange` from
  "1 1 1" to **"0 0 0"** (Almamu's renderer has 1 1 1 there, and is wrong),
  the `turbulence` operator's mask is "1 1 0" at 500 to 1000. The data
  agrees: 510 fades in the library and the engine's presets, lifetimes up
  to twenty seconds, and no value above one. Smoke that is white is often
  meant to be: `genericparticle` multiplies by `g_Overbright`, and Pyramid
  Spaceships' smoke asks for 5.

- **An instance's `rate` is how fast the system's time runs, not how many it lets
  out.** Warhammer's charges say 0.07. Read as a fourteenth of the particles, each
  muzzle flickered with one or two; on Windows it burns steady and hardly moves.
  Between frames a tenth of a second apart, Windows / a fourteenth / a slow clock:
  1.44 / 11.72 / 1.98, 1.56 / 7.30 / 1.64, 0.66 / 4.19 / 0.57, and the muzzles'
  mean brightness 223, 161, 172, 167 against 219, 163, 172, 174 where it was 176,
  143, 156, 108. As many particles are alive either way; each lives fourteen times
  as long. Authors of scripted ones call the setting `firespeed` and
  `smokespeed`. **Almamu's renderer multiplies the emission**, and Star Citizen,
  at 1.8 to 5, cannot tell the two apart.

- **`rotationrandom` is any turn unless it says otherwise**: `min` "0 0 0", `max`
  "0 0 6.28318530717", and `angularvelocityrandom` "0 0 -5" to "0 0 5", both out
  of Wallpaper Engine. 112 steps state neither end and 32 only the top. With a
  stated end standing for both, as it does for colours, all of Warhammer's rays
  lay at one angle: a single beam across the picture, where Wallpaper Engine has a
  fan -- the "star-shaped flares" that were open for two days.

- **A perspective camera stands where the plane at nought fills the scene**:
  `(height / 2) / tan(fov / 2)`, not Almamu's thousand, and its matrix goes up
  column by column like every other here. Written row by row it put Star wars'
  two layers of falling snow off the screen, and no test or message said so.

- **In these recordings `ffmpeg -ss` before `-i` hands back the same frame for
  times close together.** Consecutive frames came out identical, which looked like
  particles standing still. Decode a stretch with `-vf fps=10` instead.

- **On a Czech desktop `strtod("0.5")` is nought, and QuickJS's `JSON.parse`
  goes through `strtod`.** `QCoreApplication` calls `setlocale(LC_ALL, "")`, so
  every object's scale and alpha, carried into a scene's scripts as JSON, lost
  everything after the point and was written back each frame: a scene with any
  script lost its layers at half size and every alpha under one. Future Clock's
  red went grey and its scrolling code went away, Time Reactor's dials went
  away. `SceneView`'s constructor puts `LC_NUMERIC` back to C;
  `readsItsNumbersWhateverTheLocale` fails without it (reddest by 4). **`shot`
  never calls `setlocale`, so a sweep cannot see this** -- which is the second
  time the desktop and `shot` have disagreed. Found by the user's eyes: when
  they do disagree, draw through `SceneView` in `kwin_wayland --virtual`.

- **An object turns about its origin, not its middle.** `alignment` puts the
  origin on an edge, and the way from there to the middle turns with the
  object (`middle_of`). Moved straight, Dark Core's hands stood at twelve
  whatever the time, and its second hand started from the wrong place.

- **A `pointsize` is points at 300 dots an inch.** Read off the sizes the
  editor wrote beside the text, over four fonts: the width less any padding is
  the advance times the point size times 4.16. And `ab_glyph`'s `pt_to_px_scale`
  assumes 96, which made everything a third too large and ran Future Clock's
  digits off their tiles.

- **A line's gap goes above it, and the baseline is a twentieth of an em lower
  still.** Both measured against screenshots from Windows, on Future Clock, Time
  Reactor and Dark Core: the same 0.05 em on three unrelated fonts, whose cause
  was not found. Without them Future Clock's digits sit above the middle of the
  lamps on their tiles.

- **`padding` is one number in older scenes and "x y" in newer ones.**
  Rain Window with clock writes `"32.00000 32.00000"`, and read as one number
  the whole scene was refused ("did not match any variant of untagged enum
  Either"). `Vector<2>` takes both, one number standing for both axes.
- **WEColor and WEVector are Wallpaper Engine's own script modules**, in
  `assets/scripts/jsmodules/` beside its `wemath.js`; WEMath is written out in
  `script.rs`, the other two are read from the install at load, as its shaders
  are -- its code, not ours to carry.
- **Text's `padding` is *effect padding*.** Only the objects with effects have
  it inside their stored `size`, and the box is what those effects are stretched
  over: Future Clock draws one clock four times, each behind a mask with one
  digit's window in it, so a picture any other size looks in the wrong place.

- **An alignment is about the origin, not the box.** `right` ends the text on
  `origin`, `left` starts it there, only `center` straddles it. Worked out from
  Future Clock's digits, which land within five scene pixels of their tiles that
  way and hundreds off centred in the box; Dark Core's date stops running under
  the panel beside it.

- **The order a new particle draws its random numbers in is part of every
  picture.** Moving `size` ahead of `rotation` changed nothing about how a
  particle looks and moved 34 of 41 pictures in a sweep, which then says nothing
  about what a change did. Back in order: 37 identical, and the four that moved
  were three that gained text and **Ender Spartan, which calls `Math.random` nine
  times** and is not the same picture twice from the same binary.

- **A particle with `REFRACT` is multiplied by the scene behind it.**
  `genericparticle.frag` does `color.rgb *= g_Texture3`, whose default is
  `_rt_FullFrameBuffer`: fire burns on a lit hull and all but vanishes against
  a dark sky. 13 packages here ask for it. Unbound, the sampler read the
  particle's own texture -- the yellow blobs hanging over DOOM. Any sampler
  defaulting to that target and surviving compilation is handed the scene as
  it stands (`Pass::screen_slots`), copied before each such system.

- **An emitter that listens and says no more hears the two lowest bands,
  squared, from 0.8 to 1** -- `audioprocessingfrequencystart` 0, `...end` 1,
  `audioprocessingexponent` 2 and `audioprocessingbounds` "0.8 1.0", filled in by
  Wallpaper Engine. They were taken as the whole spectrum,
  unsquared, from 0 to 1.5. DOOM, Space Visualizer, Snow Lights and Augmentation
  leave some of them out; in DOOM it moved the picture by a quarter of a level.

- **`audioprocessingbounds` is the part of the loudness a step hears, not a
  range of strengths.** `CreateAudioResponse` in the stock shaders:
  `smoothstep(bounds.x, bounds.y, level)`, then the exponent. So a step that
  follows the sound stands still in silence. DOOM's video is silent (-64 dB)
  and its torches and 15 000 sparks a second are not there; Star Destroyer's
  Wildfire has no audio mode and burns. Read the other way round, DOOM ran
  flat out in a silent room.

- **A step hears the loudest of its bands, not their average.** Wallpaper
  Engine reads the sound the same way for every emitter and operator: mode 1 the left ear, 2 the right, 3 the two
  added band by band and halved, then the largest over the range, then the
  bounds, `smoothstep` and `pow` by the exponent. The stock *effect* shaders
  average a range for themselves, which is where the average came from. Space
  Visualizer listens across eleven bands and more, and answered the music half
  as much; DOOM, on two, moved by tenths of a level, towards its recording.

- **A particle system can be turned about x and y, and is.** Six here are:
  Space Visualizer's and Snow Lights' stars, which fly along z -- towards the
  camera -- and are turned a quarter about y to fly across the screen instead,
  the "Stars (left to right)" in the first one's settings. Read about z alone,
  the red streaks Windows throws across Space Visualizer's sky were not there
  at all. **The order is z, then x the other way round, then y** -- read
  off the one system turned about two axes: Space Visualizer's
  `starfield2` (-0.785 about x, 1.571 about y) is lit only over the bright
  sun, being `REFRACT`, and on Windows draws a band from the top left down
  through it; Almamu's `Rx * Ry * Rz` flew it level. Snow Lights' star field,
  turned about x alone, did not move by a fifth of a level either way.
  Wallpaper Engine's own order was not found. The shader is handed the
  screen's axes and the eye with the turn about x and y undone, or a turned
  sprite would stand edge on -- **and not the turn about z**, which stays on
  the sprites as it always has: undone too, it moved 27 pictures of a
  sweep. No image here is turned about anything but z.

- **The bottom bit of a system's `flags` is *Worldspace*** -- the editor's own
  word for it. A particle is placed by the object's transform once, where it is
  born, and after that lives in the scene's axes: drawn with no model matrix,
  its gravity the scene's. Its position goes through the whole transform and
  its velocity through **the turn alone, not the scale**. Space Visualizer's
  sparks are the evidence: in their object's frame (0.6 by 0.3, turned -43
  degrees) they came out as short white dashes all leaning one way, turned
  and scaled at birth as fans a quarter the size; on Windows each is a fan of
  red lines about a hundred pixels long, which is their speed over their drag
  as the file has it, bending from the object's turn towards their gravity.
  Twelve packages here carry the bit (rain, fireworks, vapour, trails); in a
  sweep Rainbow Six, StarCraft, Gears, Snow Lights and Space Marine II moved
  a little and none looked different. **Measured from the object's origin,
  not from the scene's corner**: the control points, a vortex's middle and the
  mouse are handed over from there, and with the particles counted from the
  corner Space Visualizer's sky swirled round the bottom left of the screen
  and Gears' turbulence read another part of its noise. Those points are not
  turned or scaled with the object yet.

  **Its gravity is felt at the object's size and not at its turn.** Space
  Visualizer's spark trails say "500 1500" on objects 0.6 by 0.3 turned -43
  degrees. In the scene's own axes they climbed for seconds in tall columns
  and threw rays across the planet; turned as well, each burst lay down at 25
  degrees; scaled only, it leans at about 56 and stays low, as every burst in
  the recording does. Pixels brighter than Windows' in the brightest moment of
  a minute fell from 3.9 % to 3.2 %. DOOM, Snow Lights, Rainbow Six and STAR
  WARS carry such systems too: DOOM measured the same against its recording
  within noise, the others did not move. Read off recordings.

  **A `ropetrail`'s `length` is not multiplied by its `subdivision`**, though
  that reading drew Space Visualizer's bursts as straight lines from where each
  spark was born, as Windows does: Augmentation's vortex (0.5, 16 and 16) then
  kept eight seconds of path and filled the iris with green and white tangles
  where Windows has one thin ring. Taken out again. Wallpaper Engine fills a
  missing `segments` with 4 (here 8), `subdivision` with 1 and `length` with 1,
  and leaves `fadealpha` and `fadesize` off.

  **And every renderer in the list draws.** `sparktrails` in Space Visualizer
  -- the only system here with two -- says `spritetrail` and `ropetrail`, and
  on Windows its lines start where each spark was born and grow for half a
  second, which a sprite trail cannot do. The second renderer is a twin
  system seeded alike, so it lets out the same particles. A `ropetrail`'s
  `length` is the whole path in seconds, as read before: Blue Moon's shooting
  stars trail about 0.35 s of their flight on Windows against 0.2 here, where
  a length per segment would be 1.4.

- **A thinned trail is counted in pieces and never let out more than the file
  allows.** A `ropetrail`'s buffer is its particles times its gaps, so thinning it
  to the ceiling and taking that as the particle count gave Space Visualizer's
  sparktrails -- 100 of 16 points, thinned to 703 -- room for 703 sparks, and the
  sprite twin beside it 100. Twins are held to the smaller of the two after
  thinning, or they part. **That scene asks for 167 917 at once and is given
  48 000** (24 000 until 0.24.0, a ceiling of 703 a system: its five sky systems
  kept 2.8 % of 25 000 and its white stars 17 %, which was most of what its sky
  lacked against Windows). See the budget trap below for what it costs.

- **A `spritetrail` that says no more is 0.05 long and capped at 10**,
  with no floor. The cap is in sizes of the particle:
  `ComputeParticleTrailTangents` draws it `size x min(speed x length,
  maxlength)`. Read as uncapped, Space Visualizer's lava sparks -- 3 to 10
  across, hundreds a second, `length` 3.03 -- drew lines across the whole
  planet where Windows throws fans of short red ones; with the hearing above
  they fired often enough to show it.

- **`sphererandom` is an even disc times `directions`, not a direction and a
  distance.** `directions` multiplies the place drawn; normalised after the
  mask, "1 0.07 0" sent some of Star Destroyer's fire well off the hull it
  lies along. Almamu's reading, and the video agrees. Wallpaper Engine's
  defaults for a flat scene are "1 1 0" and a radius of 256; a box
  draws min to max per axis with a random sign.

- **An instance's `count` is how many are let out, not how many may live.**
  Wallpaper Engine binds the instance's values to the system's fields, and
  the only bindings reading `count` are each emitter's `rate` and
  `instantaneous` (the latter truncated), the steps of
  `mapsequencebetweencontrolpoints` (`1 / (count x steps - 1)`) and those of
  `mapsequencearoundcontrolpoint`.
  Nothing binds it to `maxcount`. Scaling the pool instead left Gears of War's
  ripple -- `magic_pulse`, one a second, a pool of 16, count 0.03 -- with a pool
  of nought: never drawn, where Windows throws one every 33 s (30.5 and 63.5 in
  the recording). The user's "bubble" and "a stone thrown into water" were it.

- **A perspective particle system is seen through `perspectiveoverridefov`**, a
  key of `general` beside `fov`, where the scene has one -- 95 in Snow Lights,
  DOOM, Augmentation, Space Visualizer and Terminator, 30 in four others, absent
  in the rest; `fov` itself is 50 in all of them. Through 50 the camera stood
  1158 back instead of 495 and Snow Lights' nearest flakes, which pass the
  camera as large stars in the recording, were dots like the rest. The key is
  one of Wallpaper Engine's scene properties beside `fov`; where it is read
  for the camera was not found, so the meaning is the recording's.
  Windows' largest stars are still two or three times ours.

- **`shot` cannot show motion blur, and a muzzle flash leans on it.** Dark
  Angels' flash in one `shot` frame is a small blob and a cold scene; in a
  `clip` its layer 151, accumulated over frames and blended back by 534, 516,
  604 and 197, is the big yellow flame and the warm light over everything, as
  on Windows. Judge such a wallpaper in `clip`.

- **`starttime` is how far ahead a system begins, not a delay.**
  `magic_charge_rays` says 15 and its rays are there from the first frame.

- **The program does not thin any more, as Wallpaper Engine never does** (6
  November): `DEFAULT_BUDGET` is `usize::MAX`, and the thinning below is only
  what `shot` and `clip` do when handed a budget. Unthinned at 2560x1440 with
  loud music, 600 frames: Space Visualizer 30.3 ms a frame with 137 253 alive
  (17.5 at 48 000), DOOM 25.8 with 167 422 (15.9) -- inside the 66 ms a frame
  has at fifteen a second. The two passages after this one are history.

- **Thinning takes from the largest systems, not from all of them.** Scaled
  alike to fit the budget, Gears of War's 75 620 particles left its eyes' two
  torches 13 flames of 41: white over 3.0 % of the area round the eyes, against
  8.7 % in the recording and 7.5 % unthinned. `most_for_each` is a ceiling only
  the giants reach. Four wallpapers here are over the budget: Rainbow Six (50 201),
  Gears (75 681), Space Visualizer (167 917) and DOOM (177 307).

- **The budget is what keeps 60 frames a second, and it is the CPU's.** Measured on
  the real machine (Ryzen 9 5900X, RX 6900 XT) at 2560x1440: a particle costs a
  third of a microsecond to step, and drawing it is cheap beside that. Stepped one
  system after another, Space Visualizer unthinned took 62 ms a frame and DOOM 67;
  **the systems with no parent now step in parallel** (`advance_to`, one thread per
  ten thousand alive), which halves that and changes no picture -- a sweep came out
  identical but for the four scenes that read the clock or `Math.random`, and those
  differ between two runs of the old binary too. At 48 000 Space Visualizer and DOOM
  take 15.9 and 15.8 ms with loud music; at 76 000 Space Visualizer took 18.6.
  **Keep a root system from reading any other system while it steps**: that is what
  lets them run at once, and the one thing that would make parallel pictures differ.
  Children still step after, in order, since they read their parent's step.

- **The recordings from Windows run at fifteen pictures a second.** Counted
  as frames that differ from the one before: Space Visualizer 16, DOOM 19,
  Romulus 16, Snow Lights 15, Gears 16, in 60 fps video. Wallpaper Engine's
  `config.json` says `"fps": 15` for this user. That is how it keeps 170 000
  particles going -- four times the time a frame that 60 has -- and it is why
  motion in those recordings steps.

  **This program draws fifteen a second, scenes and web pages alike, as
  Wallpaper Engine does by default.** Its welcome dialog sets the Medium preset
  (`qualityPreset="medium"` in `ui/dist/scripts/scripts.js`), and
  `getQualityPreset` makes Low 10, Medium 15, High 25 and Ultra 30 frames a
  second; the settings warn above 30. **No preset touches particles** -- they
  set MSAA, post-processing, reflections, shadows and volumetrics -- and
  Wallpaper Engine reads `fps` with the rest of the general settings and
  nothing about particles. Its limiter, after a frame, `Sleep` for what is left
  of the second shared out over the frames still owed in it, and no
  limit at all at 0. So Wallpaper Engine
  never thins a scene: a heavy one simply gets fewer frames. Thinning more at a
  higher rate is ours, decided on 6 November so a heavy scene stays smooth and
  cheap. The Settings tab is to offer the four rates.

  A scene is paced by a timer on `SceneView`, not by the renderer asking again
  as soon as it has drawn, which ran at the screen's 144;
  `drawsFifteenTimesASecond` counted 288 frames in two seconds that way. A
  page's `requestAnimationFrame` is wrapped in `WallpaperSurface.qml` to pass
  over frames that come sooner than a fifteenth after the last. **Not with a
  timer**: Chromium runs a frame's callbacks about 12 ms after the time it
  hands them, and a timer counted from that time ran at sixty when thirty was
  asked for. CSS animations and video are not held. Measured: 15.3 a second in
  a page.

  **Stepping one system's particles on several threads was tried and taken
  out.** Moving the loop into a closure to share it out made it slower on one
  thread -- Space Visualizer at 48 000 from 16.4 to 23.5 ms a frame -- and the
  threads only won that back on the giants (29 to 26.5 unthinned). What is
  left of a frame there is drawing and writing vertices, not stepping.

- **`shot` turns the sound on only for the frames it times.** It settles to the
  start in silence, so a scene that answers to music is still filling up during
  the measured frames and a median of a hundred of them is the ramp, not the
  scene: Space Visualizer's simulation went from 0.1 to 16 ms over 90 frames.
  Time 600 frames or more before believing a number.

- **Whether a texture repeats is bit 2 of its flags, in the file.** It agrees with
  `clampuvs` in all 243 `.tex-json` that state one, and a wallpaper carries the
  flag and not the sidecar. Read off the sidecar alone, Gears of War's ripple map
  clamped, and past its first tile smeared one column into fine horizontal
  stripes: the "ripple too strong" that was never about strength.

- **A post-processing layer's buffer is the screen.** `fullscreenlayer` still
  stores a size, 1248x702 in Gears' 1920x1080, and taking it as the buffer's made
  a blur reach twice as far from a smaller picture. `rgba_backbuffer` is what it
  says, so it is told the output's own size.

- **`shot` runs a scripted rate wrong when it starts late.** `settle` catches up
  at the rate in the file, before any script has said otherwise, so Dark Core's
  fires, which a script holds at a tenth in silence, come out lit from the first
  frame. For a scene like that draw from nought: `shot <dir> <png> w h 0 48000
  1500` is twenty-five seconds as the desktop has them. It clears every frame now
  too, as the program does; without that Dark Core's tubes went white by frame six.

- **A mirrored animation that starts off rest holds its first stretch.** Three of
  Alien Isolation's start with a bone out of place: the jaw is written out of the
  mouth at frame 0 and comes in by frame 120. Wallpaper Engine leaves it in, and it
  first comes out at the far end of the mirror, forty seconds on: seen by the user
  on Windows, not explained. `first_settled_frame` takes the first frame furthest
  back along the way to rest, because a curve may stand still at either end.

- **`g_TextureReductionScale` goes with the size `g_Texture0Resolution` claims.**
  A buffer claims the scene's size, so the scale is about one. Counted in the
  buffer's real pixels, `blend`'s offset fell short by the factor the screen is
  smaller than the scene, and Dark Angels' light missed its cathedral. Only that
  wallpaper moved in a sweep.

- **The sound is read as Wallpaper Engine reads it**, not as a visualiser
  would: a DFT of 1920 samples, no window, bins 1 to 639, each bin's power
  weighted by `0.501 - cos(pi x) 0.499` -- the bass a five-hundredth of the top,
  which levels music out -- its root put into one of 64 entries by `x^0.25`, the
  loudest bin winning, and the lot times 127 x 0.001 x 640/960 x the recording
  volume (`audioinputvolume` x 0.02, so 50 is one). Every reading starts from
  nought: the meter holds and smooths nothing. `audio.rs` does the same, bin for
  bin: a transform of the newest 1920 samples (fifteen of 128, put together).

  **At 48 kHz, as Windows mixes and PipeWire runs** -- the Windows recordings'
  sound is 48 kHz, so its bins are 25 Hz apart. Read at 44.1 kHz off an FFT of
  2048, the nearest bin standing in for each, DOOM's 35 Hz drone filled band 0
  and lit the D and the M through it, where on Windows they stay dark and light
  on the kicks: the D followed Windows at 0.63, the M at 0.64. A replica of the
  meter in numpy put it down to the rate (0.68 for an exact 1920 at 44.1 kHz,
  0.76 at 48), and drawn the D follows at 0.74, the M 0.75, the Os 0.79 and
  0.82 (were 0.75 and 0.81). Dark Core, Snow Lights, Augmentation and Space
  Visualizer against their recordings: the same within a hundredth.

  **Compare a letter a frame late.** The Windows recordings run one frame of
  fifteen behind a `clip` of the same sound; unshifted, the D followed at 0.37
  for a picture that follows at 0.62. Ours on the desktop is late by about as
  much, the monitor's 0.12 s.

  **And then the scene evens the music out before anything sees it** --
  `wpe_format::particle::Response`, as the scene's render loop does it,
  once a frame and by the frame's own time. Each ear's 64 in eight groups of
  eight, each group divided by an envelope that goes towards the louder of the
  group's loudest and a third of the loudest of all -- up at one a second, down
  at half that -- and starts again from one after silence; then eased at twenty a
  second and moved at most forty a second. Without it the meter is right for the
  first second of music and four to nine times too weak after: measured on the
  white lines under Dark Core's STANDBY (each one is 380 scene pixels times one
  entry), against a recording with the system's sound in it, the lines went
  from a fifth of Windows' length to the same within a few hundredths.

  The meter it replaced was a visualiser's -- a Hann window, sixteen bands, rising
  at once and falling slowly, square-rooted and capped at one -- with a `TOP_LIFT`
  fitted to DOOM. On music its values sat near the top and stayed there, and a
  script that smooths for itself then stood still: Dark Core's red triangle, three
  times band 4 smoothed over a second, was lit all the time where Windows blinks,
  and its green bars under STANDBY hardly moved. Simulated over a recording's music:
  the triangle's alpha 2.5 to 3.0 before, 0.25 to 0.94 now.

  **32 and 16 are the loudest of each pair, twice over**, in the same loop -- and
  the averaged ear is the two ears' entries averaged first and the loudest of
  those taken after. Dark Core's green bars, which read 16, 32 and 64 side by
  side, agree: an average of each group would have them at two thirds of their
  length. Shaders here read only `g_AudioSpectrum16*`; scripts get the 64 and
  ask for what they like.

- **A system whose rate a script drives starts from nought, like any other.** A
  script holds Dark Core's and Time Reactor's fires at a tenth of their speed in
  silence, so they take tens of seconds to fill their tubes, and on Windows both
  do -- the user's word for both. They are built alike (`wildfire.json`, the same
  script), so no rule can start one ahead and not the other. A 3.2 s head start
  fitted to a Dark Core recording was taken out on 2 November: that recording had
  simply started late. Only the preset's own `starttime` is run ahead.

  **Time Reactor's smoke has its alpha on a script as well**, a fifth in silence
  and up to whole with the music. Unrun, it burned five times as strong, and its
  `wildfire.json` is not Dark Core's: born up to 600 out, as fast as 500 and with
  no fade in, so the second of `starttime` run ahead at full speed already
  reaches a third of the way along its tubes. That second is the preset's and
  stays -- Space Visualizer's rays, at a rate of 0.01, are only there from the
  first frame because `starttime` is not slowed by the rate.

- **Pushes are felt over less time at fifteen.** Wallpaper Engine runs a
  frame's particle operators once, or twice at half the time when its frame
  rate is set to 1-20. Each run is handed the step and
  a second time, the step times `min(0.025 / frame, 1)^0.7` with the frame
  as the scene measured it: gravity, the move and a spin take the
  first, and drag, `turbulence`, `vortex` and `controlpointattract` the
  second. `movement` is gravity, then the move, then `velocity x= 1 -
  min(drag x felt, 0.99999988)`. At 15 a second that is half the push our
  `1 / (1 + drag dt)` and plain `dt` gave; at 40 and over it is the same.
  `drag_left` and `felt` in `particles.rs` spread it over our sixtieths so a
  frame comes out as Wallpaper Engine's; the frame is capped at a fifteenth,
  so a `settle` is a run of desktop frames. Against recordings, 9 November:
  Gears 5.49 to 5.41 levels off, its swing 1.00 to 0.88 off; Star wars 5.87
  to 5.68, 4.25 to 4.01; DOOM, Snow Lights and Augmentation the same within
  a few hundredths; Pyramid's YouTube recording is at 60 and agrees with
  the change drawn at 60 (8.72 against 8.70). Space Visualizer's swarm lost
  its narrow plume round the small planet's right edge and streams wide, as
  on Windows. Drag alone, without the pushes, made Gears worse (5.73).

- **A scene moves a system's control points, and a point can be in the
  scene's own coordinates.** `instanceoverride` carries `controlpoint0` to `7`
  (and `controlpointangle0` to `7`, not read here, and a `brightness` that
  nothing here reads); Wallpaper Engine puts each in place of the file's
  offset. A point whose `flags` has
  bit 1 is given in the scene: `discharge.json`'s far end, which the scene
  drags to where the bolt strikes. For a system in its own space the point
  goes through the object's inverse, for one in the scene's axes it is
  measured from the object's origin, as everything else there is. Before 9
  November both were read as offsets from the system: Augmentation's bolts
  struck elsewhere (its swing against the recording 0.94 to 0.91 off). Blue
  Dawn's lens flare and Snow Lights' bolts carry them as well; Snow Lights'
  never strike within a minute.

- **`oscillate*` frequencies are radians a second, and the two scales are two ends.**
  An ember's "frequencymax 20, scalemin 0.7" is alpha swinging between 0.7 and 1 up
  to three times a second. Read as cycles, with the missing end taken as the stated
  one, it was twenty blinks a second down to 0.3: the flicker the user saw on the
  flying lights of four wallpapers. Wallpaper Engine's defaults: frequency 1
  to 10 (position 1 to 5), alpha 0 to 1, size 0.8 to 1.2, phase 0 to 2 pi.

- **An fbo marked `unique` is the effect's own, whatever its name.** Motion blur keeps
  last frame in `_rt_FullCompoBuffer1` and `blurprecise` works in a buffer of that
  name. Shared, the blur overwrote the memory each frame, so accumulation mixed with
  an unscrolled picture: Snow Lights' deer came out 0.64 of itself and pale.
  `var.py`-style unpacked copies of a package go straight into `shot`, which is how
  this was found: switch one effect off in scene.json and look.

- **The `turbulence` operator pushes round the hills of a noise, with its clock in
  scene pixels.** Almamu's reading: position plus time times `timescale`, then the
  scale doubled, then a curl (flattened here). Rainbow Six's -50 was added after the
  scale, fifty cells a second, a new direction every frame: fog that shivered instead
  of rolling. 21 wallpapers have the operator; only 5 systems state a `timescale`.

- **Every `turbulentvelocityrandom` adds its push.** Time Reactor's fire has one up
  its tube and one down; only the last counted, so half its tubes stayed empty.

- **A `right` along `forward` is a straight jet.** Dark Angels' Torch says "0 1 0"
  and its video runs down the barrel; let wander, it lit a column as far as the top of
  composition layer 197, the straight edge. One preset and one video; the meaning of
  `right` beyond that is a guess that fits.

- **A vortex turns the other way from its axis, and says 500, 650 and 2500 when it
  says nothing.** Space Visualizer's stars fly left to right turned about y, and
  their vortex about z leans them: on Windows they rise to the right by about 12
  degrees, here they fell by 20 -- the half of them pushed down is the half nearer
  the camera, so no turn of the object explains it, only the swirl's own sense.
  Reversed, DOOM and Snow Lights moved by less than a fifth of a level over half a
  minute with their recordings' sound, and Augmentation's rings are rings either
  way. The defaults are Wallpaper Engine's, for a flat scene: `distanceinner` 500, `distanceouter` 650, `speedinner`
  2500, `speedouter` nought, axis "0 0 1"; 30 vortices here leave some out, and
  read as nought they swirled at their inner speed everywhere.

  **How it pushes is Wallpaper Engine's own step**: the way from the
  middle -- control point plus `offset` -- less the part along the axis **only
  where bit 0 of `flags` is set**, made of length one; the distance is that way's
  length; the speed is the inner one plus `clamp((d - inner) / (outer - inner))`
  times the difference, both speeds times the sound and the step's time; and the
  velocity gains `normal x axis x speed`, **not made of length one again**, with
  the axis normalised when the file is read. So a particle out along the axis is
  hardly pushed, and "1 1 1" -- twenty of the library's 31 vortices -- is a third
  of a turn about each axis, not a stronger one. Until 6 November ours measured
  from the axis always and pushed at full speed whatever the angle: Space
  Visualizer's stars, which fly along their axis, rose at about 20 degrees where
  Windows has 15, and 18 now. Snow Lights, Augmentation and DOOM against their
  recordings: the same to the hundredth, frames 0.16 to 0.44 of a level apart.
  No vortex here sets `flags`, `offset` or `controlpoint`, so those two are not
  read yet.

- **A vortex is a sideways push, and `controlpointattract` a constant pull within half
  its `threshold`.** Almamu's reading of both. Augmentation's 8000 as degrees a second
  was a turn and a half a frame and drew straight spikes; as a push held back by its
  drag of 20 it swirls on the beat as in the video.

- **A sound in `random` plays one track at once, from its start**, and the next after
  that track's length plus `mintime` to `maxtime` -- in Wallpaper Engine,
  the same as a script's `play()`: the pick is
  `rand() / 32767` times the count. So a scene whose tracks open quietly (Space
  Marine II's five fade in over 2 to 10 s) is quiet for those seconds on Windows
  too. **Almamu's renderer plays every file of a sound object at once**, each its own
  stream mixed in (`CSound::load`, `SDL_MixAudioFormat`), and reads no mode but
  `loop`: five soundtracks over one another, which is why it is loud from the first
  second. Not copied. The same function starts `loop` and `single` on a random
  file as well, and so does `SceneSound` since 2 November. It also sets the volume
  as `volume^2` times the scene's -- against the square root measured below; that
  is not changed yet.

- **A scene sound's `volume` goes up as its square root.** Augmentation at 0.5 and a
  Space Marine at 1.9 play at 0.75 and 1.33 of their files in recordings. Two Alien
  Isolation recordings at 0.5 and 0.65 say 0.99 and 0.96, so this is the closer of two
  readings rather than settled; Qt tops out at 1.

- **`colorchange` multiplies what the particle was born with.** Replacing it
  threw away a scene's tint.

- **`TEXB0001` exists**: no image format, and mipmaps of width, height, size
  and bytes, never packed. `util/white` is one, and refusing the container lost
  every particle system that named it.

- **Some workshop shaders lean on HLSL shortening a value** -- `float x =
  vec2`, `vec3 c = vec4`. A shader that fails to link is passed through
  `spell_out_conversions` and tried once more, so nothing that compiles is
  touched. Since 16 November that pass also follows a value's vector widths
  (`Fit`) and swizzles the longer side down where two meet in arithmetic --
  `v_TexCoord.xy - (offset * timer + pointer)`, a `vec2` less a `vec4` -- and
  a fragment input the shader writes (`v_TexCoord += ...`, read-only in GLSL,
  a copy in HLSL) gets a global copy first (`copy_written_inputs`). That is
  what SCP's `chromatic_aberration` needed; nothing else here takes the path.

- **A shader is handed the mouse wherever it is, off its screen too.** SCP's
  aberration adds a hundredth of `g_PointerPosition` to every channel's
  offset, and the `-10` that stood for "no mouse" moved its whole picture a
  tenth of the way: a red copy of TERMINAL LOCKOUT hung over the logo. With
  the recording's cursor it is closer to Windows than without the effect
  (6.6 levels off against 7.7). Particles and the parallax still feel only a
  mouse on their own screen (`point_at`). With following the mouse off in
  Settings the mouse is unknown, `-10` goes in, and the copy comes back --
  what Wallpaper Engine hands then was not found. **`shot` passes no mouse
  unless `WALLFLUX_POINTER` says so**: judge SCP with it.

- **A scene's scripts ask `thisScene` for the camera.** Magical Orb's
  ripple, fisheye, blur, god rays and pulse all start `if
  (thisScene.camerashake)`; with no such field every one of them kept its
  file value and the orb sat filled and still. Told it (`scene_json`, the
  names of `lib.sceneScript.d.ts`'s `IScene`), they follow the music as
  written: a calm ring in silence, the preview's diamond of light with
  music. The preview was taken with music.

- **Editor Extensions (DLC 1790230) holds no format knowledge.** It is all
  `dlc/pymidas`: Python, PyTorch and MiDaS, the depth map generator for the
  Depth Parallax effect. What the editor knows is in `ui/dist/scripts/scripts.js`,
  `locale/ui_en-us.json` (2 163 `ui_editor_*` keys) and `assets/presets/`,
  whose stock particle systems are the vocabulary to cover.

- **Compare with the videos by time average, not by one frame.** Recordings
  of Wallpaper Engine on Windows are in `~/WallFlux-ukazky/wpe-windows/`, BT.709
  and 2560x1440; a single frame of moving fog says nothing about its strength.
  Averaged over seven frames Warhammer's picture differs from the recording by
  single digits in every region. **Before calling a layer wrong, sample it over
  time**: DOOM's blood drips looked like a mask failing and are an animated
  sheet caught at another moment.

- **`sizerandom` is twice the size a particle is drawn.** The shader
  draws a particle as wide as the size it is handed, and the file's number
  is halved before that -- Wallpaper Engine stores min, range and exponent
  unchanged, so the halving is somewhere after, and was not found there;
  what settled it is measurement. Against Wallpaper Engine on Windows, at
  the same size of window: Arkham Knight 21.7 whole, 12.7 halved, 13.0 in
  Wallpaper Engine; Warhammer 89 / 73 / 74. Almamu's renderer halves it
  too, and the moon in the snowy forest, a particle, is a tenth of the
  picture high in its preview only that way. **Compare at the window's
  own size**: the Windows screenshots are 2016x1408 and the like, so the
  scene is cropped at the sides and everything in it is larger than in a
  16:9 picture.

- **A default can depend on whether the scene is flat.** Wallpaper Engine
  has two sets, pixels or metres: `sizerandom` 5 to 50 against 0.001 to 1,
  `turbulentvelocityrandom` 100 to 250 against 0.5 to 1. The flat ones are
  ours.

- **`turbulentvelocityrandom` is a direction, not a field.** Its `scale`
  is how far from `forward` (up) a start may stray, in half turns, and its
  `offset` is radians it is turned about `right` (out of the screen) --
  the engine's defaults name a forward and a right, which mean nothing
  otherwise, and Almamu reads it so. Which way the turn goes was taken from a recording: Star wars' embers fly
  at 49 degrees in the system's own axes, tracked frame by frame, and up turned by
  -0.5 radians is 61, give or take 18. It was the other way round until 30 October,
  read off one screenshot, and they flew off the bottom of the picture: the
  "orange bokeh" missing at the left of Star wars was those embers. Read as a
  field, with no speed of its own, they were born still and drawn nought long.

- **`distancemax` is a vector in forty boxes** -- "512 256 0" -- and read
  as a number it was nought: every one of those boxes let all its
  particles out at a single point. Pyramid Spaceships' exhaust and
  StarCraft's rain came back with it.

- **`turbulence` accelerates.** Read as a speed that carries a particle along,
  Gears of War's 45 000 sparks at 500 to 1000 ran together into red bands where
  the recording has them scattered evenly, held back by their drag. It works
  through `velocity`, so a system without `movement` is not moved by it, and a
  trail is as long as that velocity.

- **An instance's colour moves a system's colours in HSV, and in an old scene
  multiplies them too.** Wallpaper Engine works out the colour a system already
  has (the middle of its last colour initializer, else the start of its first
  `colorchange`, else white) and moves each colour range by the
  difference between the scene's colour and that one in hue, saturation and
  value, the hue going round and the other two held to nought to one. A flag
  makes the scene's colour a multiplier on the particles as well: for a scene
  whose `version` is below 5 -- all of them here but Ender Spartan -- and for a
  system with no colour of its own. `colorchange`'s two ends are moved as well;
  the recordings say so: Tomb Raider's
  torch, orange to red and set to white, burns white on Windows and orange if they
  are not. Dark Angels' flash went from blue to red 0.77 and white over 9.5 % of
  it to 0.64 and 4.6 % (Windows 0.58 and 1.7 %); Ender Spartan from 13.7 apart
  from its recording to 7.2, Star Destroyer from 2.3 to 0.9. Moved and then
  multiplied, a colourless system comes out twice its colour: Dark Core's tubes
  went red. The reading it replaced -- the scene's hue with the file's strength,
  `colorchange` kept as a change of brightness -- was fitted to recordings and
  explained none of this.

- **A puppet bends the finished picture, not the first pass.** An image whose model
  names a `puppet` `.mdl` is drawn through that mesh on the way back into the scene,
  after its effects (`wpe_format::puppet`, layout in its header; attachments are
  optional). Bent earlier, the effects' masks stayed behind and the buffer's edge cut
  the jaw off in a straight line. Corners are bent on the CPU and the shader is the
  ordinary flat one. Each `animationlayers` entry adds what it moves a bone by against
  the bone at rest. Alien Isolation's second jaw and tail are this.

- **Particles step by exactly the time owed.** Whole steps of a sixtieth on a 75 Hz
  screen left every fifth frame with nothing new, which read as smoke that stutters.
  `shot` shows nothing of this. And `SceneView`'s clock counts nanoseconds: whole
  milliseconds lost most of one a frame, four per cent at 60 Hz.

- **A web page is handed a path with no leading slash**, as Wallpaper Engine's `C:/...`
  has none: pages put `file:///` in front, and four slashes is a file that will not play.

- **An emitter's rate defaults to ten**, for every kind of emitter, as
  Wallpaper Engine fills it in. Ten wallpapers here have an emitter that states none,
  `ember_small`, `dust_motes_0` and `wildfire` among them. Read as nought they let
  nothing out, which looks like a wallpaper that has no embers; read as five, a
  guess that stood until 3 November, Dark Core's tubes burned half full -- a
  thinner, patchier fire, about 0.6 of the recording's along the lower tubes.

- **A system whose control point 0 is on the mouse lets its particles out at the
  mouse**, and none while there is no mouse. Alien Romulus' black "explosion on
  cursor", held at where the scene placed it, was the black blot under the
  alien's mouth. On Windows it is a swarm of small dark specks round the pointer,
  and it took three more things to get there:

  **DXT1 has an alpha.** A block whose first colour is not above its second has
  a transparent fourth entry, which is what Direct3D reads and
  `COMPRESSED_RGBA_S3TC_DXT1` means. `COMPRESSED_RGB_S3TC_DXT1` draws it opaque
  black, so every speck was a black square.

  **`sizechange` steps take over from one another.** Multiplied, 0 to 1.8 and 1.8
  to 0 peak at 3.24. Every other system with two steps meets at 1, where the two
  readings agree, so only Romulus can tell them apart.

  **An instance's `count` scales `mapsequencearoundcontrolpoint`'s too.** At 4 the
  particles leave along four fixed arms, a clover; at 3.2 they go round and fill
  the disc the recording has. Measured on this one wallpaper. It is still denser
  than Windows, on the only reference there is: a dark corner, the mouse still.

  **It was left out until 6 November** (`LEFT_OUT` in `render.rs`, now empty),
  the user having preferred none to specks three or four times too large. What
  made them so was a placed system's `size` and `speed` not reaching its
  `sizechange` and its pull -- see the bindings under Romulus in the open list;
  bound, they are Windows' sparse small dots, and they are drawn again. Its
  visibility is bound to a `mausverfolgung` the published project.json does not
  declare, so there is no setting that could switch it off instead.

- **`g_Texture0Resolution` of an effect's buffer says the size Wallpaper Engine's
  would be**: as big as the object in the scene, where ours is as big as it is on
  screen. A blur reaches so many texels, so told the real size Kratos' 5120-wide
  scene came out four times as blurred at 1280 and its shoulders melted. This is
  why a blur used to look heavier the smaller the screen.

- **Camera shake is two slow waves**, measured by phase correlation off recordings
  of Samurai, Alien: Isolation 4K and Space Marine II: 0.64 % of the width per unit
  of amplitude, at 0.168 Hz across and 0.22 Hz up per unit of speed.
  `camerashakeroughness` is not read. **`zoom` is the camera's as well**; Space
  Marine II says 1.07.

- **Lit images are drawn.** Alien Isolation's six layers ask for `LIGHTING`; it is
  the only wallpaper here that does. A light's `intensity` and the general
  `ambientcolor` animate on keyframes (`scene::Keyed`, straight lines between keys,
  handles left out), which `Bound` throws away, so `Lighting::read` reads the raw
  JSON. The colour sent is colour x intensity x radius squared, since the shader
  divides by distance squared. `g_ModelMatrix` times `g_ViewProjectionMatrix` has to
  come to the pass's `mvp`, which for an object with effects is its own buffer.
  With the renderer's old black fallback gone the scene pulses as on Windows.
  Reflection is still off, and particles are not lit.

- **A web wallpaper is told its settings, and hears the speakers.** Nothing used
  to reach a page, so Perfect Wallpaper drew its first picture whatever was chosen.
  `applyUserProperties` gets project.json's properties once the page has loaded,
  and `wallpaperRegisterAudioListener` must exist before the page's own scripts
  run, hence `WebEngineScript.DocumentCreation`. Reads `wallflux_speakers` thirty
  times a second, only while a page asked and is on show, because listening shows
  the microphone icon.

- **A preset has no `type`: it is `dependency` (another wallpaper's folder) plus
  `preset` (settings).** It is listed and drawn as that wallpaper, with its own
  values laid over; its files stay in its own folder. The dependency comes off the
  internet, so it has to be all digits. Its id is the preset's, so removing it
  removes the preset only.

- **Wallpaper Engine's own wallpapers are not packed and have no number.** `Package::
  from_folder` reads them; `deep_space` and the like must not reach Steam (`qml/
  steamprofile.js` filters them, one in the list would cost the whole answer).

  **All nineteen are listed, as Wallpaper Engine lists them, less what its
  `projects/defaultprojects/visibility.json` hides** -- `false` against a name,
  three of them so on this machine. Removing one is what Wallpaper Engine's own
  Remove does (`removeOfficialWallpaper` in its `scripts.js`): `false`
  written there, files left alone, since they belong to
  the install. There is no way back in Wallpaper Engine's window either. Four say no
  `type` and are taken by their file (`.json` a scene, `.exe` an application), and
  their `authorsteamid` names who made them, which is the panel's author. Only four
  scenes draw (`scenesThatDraw`); the rest are 3D models or come out black, and are
  listed dimmed so that they can still be read about and removed. Until 2 November
  only those four were listed, and Remove had no button for them -- reported as
  broken.

- **A scene's file is the one project.json names**, `gifscene.json` in
  `gifscene.pkg` for SCP Terminal Lockdown, not always `scene.json` in
  `scene.pkg` -- `WallpaperCore` and the renderer both take it from `file`.

- **A sweep at six seconds misses what is brief or late.** A lightning
  flash lives a tenth of a second and the Star Wars sparks start at ten,
  so a change to bursts moved one picture out of forty. Check those at
  the moment they happen -- the particle count `shot` prints is the
  quickest way to find it: the Star Destroyer jumps by a thousand at
  15.1 s.

- **`_rt_imageLayerComposite_<id>_a` means two things, depending on who
  says it.** Written by layer `<id>` itself, in a slot its effect already
  binds `previous` to, it is only a name for what that bind hands over --
  turning the effect down over it lost Arkham Knight's blur and godrays
  and the motion blur in eight more wallpapers. Written by another layer,
  it is that layer's *finished* picture, which the renderer keeps for any
  layer somebody reads. Almamu's renderer reads whichever of the pair a
  chain ended in, and its own TODO records a wallpaper getting the wrong
  picture that way.

  **And `g_TextureReductionScale` has to be set.** `blend` divides an
  offset in the scene's pixels by it; left at nought, every blend placed
  with `TRANSFORMUV` was thrown off to infinity and drew nothing. It is how
  many scene pixels one pixel of the picture stands for -- `skew` says so
  outright -- which for our buffers, made as big as the object is *on
  screen*, is the object's size over the buffer's.

  **The Space Marine's gun is steel, not brass.** It looked right gold
  for days, and that was the texture drawn raw because its whole chain
  had been dropped; the tint that greys it is the wallpaper's own, and the
  warm light comes from the muzzle, through layer 151. The wallpaper's
  `preview.gif` is what settled it. **When a picture moves, look at the
  preview before deciding which of the two was right.**

- **pipewire-pulse records in fragments of two seconds unless told otherwise.** A
  record stream opened with no buffer attributes got `node.latency` 88200/44100 and
  a `fragsize` of 705 600 bytes -- read with `pactl list source-outputs` -- so the
  samples arrived two seconds at a time and DOOM's letters moved one to two seconds
  behind its own music, which the speakers carry and the monitor hears, as Wallpaper
  Engine's loopback does. `connect()` asks for a fragment of one reading, 8 192 bytes:
  `node.latency` 1024/44100, and the meter moves 0.12 s after a sound starts,
  measured three times with `listen` and `pacat` -- the tenth of that is `listen`'s
  own interval.

- **Plasma calls listening to the speakers using the microphone.** A
  record stream on a sink's monitor puts `plasmashell_microphone` -- the
  microphone icon -- in the tray for as long as it is open. Measured by
  reading `RegisteredStatusNotifierItems` with and without one. The only
  streams it hides are four volume mixers' meters, matched by
  `application.id` in `libKF6PulseAudioQt` -- passing for one of them would
  be lying. So the renderer listens only while it is asked: `Scene::listens()`
  says at load whether a shader reads `g_AudioSpectrum16*` or a particle
  system's rate, field or swirl follows the sound, only such a scene asks,
  and `audio.rs` closes the stream once the bands have gone a second
  unread. A wallpaper under a window is not drawn and so does not ask, and
  the icon goes with it.

- **A scene's scripts are ES modules inside scene.json, on the value they
  drive**, and every object is a layer to them whether it is drawn or not:
  Ender Spartan turns an invisible `SharedNoise` with one script and reads
  its angles from ten others. So `script/host.js` keeps a layer for every
  object in the file, and the layers are where place, turn, size, alpha and
  visibility live while a scene with scripts runs -- `render.rs` copies
  them onto the objects scene.json placed itself (`from_scene`), never onto
  the children a particle system brought.

  **Angles are degrees to a script and radians in the file.** Dark Core's
  clock turns its hands with `* WEMath.rad2deg`; handed radians, its hour
  hand read -122 and spun.

  **A picture drawn at six seconds has to have run six seconds of
  scripts.** Most of them ease towards the music by `engine.frametime`, and
  one frame at t=6 is one call with a frametime of nought -- Dark Core's
  ring stayed dark however loud `shot` was told the room was. The first
  frame catches up in thirtieths of a second; every later frame is one step,
  told at most a tenth of a second went by, because 300 steps of Dark Core
  is a quarter of a second of render thread after a window uncovers it.

  **And a scene that reads the clock is never the same picture twice.**
  Eleven of Dark Core's scripts call `new Date()`. In a sweep, a scripted
  clock that moved is not a regression.

  **Every call has a deadline and a script that misses it is stopped.**
  A script runs on Qt's render thread; one that loops for ever would stop
  the desktop being drawn. QuickJS's interrupt handler ends any call past
  20 ms, a script that throws or is interrupted is not run again, and the
  value it drove stays where it was -- tested for both in `script.rs`.

  **Almamu's renderer has no `engine.registerAudioBuffers`**, which 39 of
  the 93 call: his scenes stay at the quiet end of what they say. Ours
  hands the 64 entries it measures, each group of them as loud as its
  loudest when a script asks for 16 or 32. Asking for them is what makes a scene
  listen, so the microphone rule above holds for scripts too.

- **The mouse comes from KWin, and the surfaces never take input.** A
  click on the desktop belongs to Plasma's desktop underneath ours; a
  wallpaper with an input region would swallow it. `workspace.cursorPos`
  and `cursorPosChanged` were measured on KWin 6 first, and so was what
  `callDBus` makes of two JS numbers -- **`int32`**, read off
  `dbus-monitor`. A slot taking doubles would never have been called, and
  D-Bus says nothing about a call whose signature matches no slot. The
  script sends at most every 16 ms and is loaded only while a scene is
  showing and uncovered; asked to stop, it is unloaded.

  **And a mouse that changes only the item is a mouse the scene never
  hears of** -- the framebuffer trap above, again: `setPointer` calls
  `update()`. `tst_sceneview` lays three Blue Dawns side by side with the
  mouse left, right and left, and fails with the `update()` taken out
  (measured: 0.006 apart either way) while the outer two agree and the
  middle one leans. Blue Dawn because Warhammer, the first choice, does
  not draw the same picture twice -- 900 apart with the mouse in one place
  -- and its "follows the mouse" had been that noise all along.

- **A layer-shell surface is told its screen, or it lands on the first.**
  LayerShellQt reads `QWindow::screen()` only when the surface is made, and
  by then Qt has put every surface back on the screen its position (0,0)
  is on: with a monitor beside the laptop both surfaces lay on the laptop,
  1366x768, the monitor showing Plasma's own wallpaper -- while the log
  said "asked for on HDMI-A-1". `layer->setScreen(window->screen())` in
  `Desktop::asWallpaper`. Check with a KWin script printing each window's
  `output.name`, not with our own log.

- **A wallpaper's window is never coloured, only what is inside it.** Qt
  gives a window born opaque an opaque region and does not take it back
  when it turns transparent -- measured with `WAYLAND_DEBUG=client` on a
  plain `Window`: born black, it sends `set_opaque_region`; born
  transparent and turned black and back, never. A surface made while
  something played (a layout switched, a profile loaded, a monitor
  plugged in) then kept KWin from drawing the desktop under it after Off:
  stale pixels, and windows dragged across them left trails. A screenshot
  composes afresh and shows the desktop, so it cannot show this (25
  November, the desktop PC with two monitors). The window is
  `"transparent"` and the black is `backdrop`, a rectangle in it.

- **`LayerBottom`, never `LayerBackground`.** Plasma draws its own desktop
  into a background-layer surface, and clicking the desktop raises it above
  anything else in that layer — the wallpaper vanished and the system one
  came back. Layers are a fixed order; bottom is still below ordinary windows.
- **Qt's own messages do not reach a redirected stderr.** Pipe the output
  anywhere and `qCInfo`/`console.log` go to the journal instead, leaving only
  raw ffmpeg noise in the file. Use `QT_FORCE_STDERR_LOGGING=1` (the older
  `QT_LOGGING_TO_CONSOLE` still works but warns) or `journalctl --user -f`.
  **Nothing can be debugged without knowing this.**
- **`Video` is a convenience item that hides `mediaStatus`**, which is the
  one property a playlist turning on the end of a clip needs. Its `stopped`
  signal is not a stand-in: it fires when the source is swapped as well as
  when a clip ends, so a playlist turning from one video to another would
  have turned twice. `Main.qml` uses `MediaPlayer` + `VideoOutput` +
  `AudioOutput` instead, which is what `Video` wraps anyway -- same traps,
  same `onSourceChanged: play()`.

  **And a video told to loop for ever never reports an end.** `loops` has
  to drop to 1 while a playlist is waiting on it, or `EndOfMedia` never
  arrives and nothing turns.

- **`Video.autoPlay` starts the first file and no other.** Hand it a second
  source and it loads, reports nothing and sits there stopped. Measured: 24
  switches gave 1 start. `onSourceChanged: play()` is why switching works.
- **`select()` must stay silent when the choice has not changed.** Emitting
  `currentChanged` anyway makes QML hand the player the same URL, which stops
  it for good — a black screen with no error. Regression test:
  `staysQuietWhenTheChoiceIsTheSame()`.
- **project.json's tags are the author's genres and nothing else.**
  Measured against the workshop's own list for the same wallpaper:
  `["Sci-Fi"]` on the disk against `Wallpaper, Scene, Sci-Fi, Audio
  responsive, 1920 x 1080, Everyone` in the answer. Steam adds the asset
  type, the kind, the resolution, the age rating and whether it answers to
  sound, and none of those is written into the folder. So a wallpaper
  looked at in the library used to show a third of the tags it showed in
  the Steam tab. The keyless details call that finds the author brings the
  full list back with it, so it costs nothing extra -- and **anything
  asking "does it carry this tag" has to read that list**, or the Audio
  responsive row says "not stated" about a wallpaper whose own chips
  underneath say otherwise.

- **The library's filter is built from the workshop's tags, asked for in
  one question.** `GetPublishedFileDetails` takes every id in a single
  POST -- all 56 on this machine, answered in one go, no key -- so the
  window asks about the whole library the first time it is opened, and
  about anything that turns up later. project.json carries the genres its
  author ticked and nothing else: ten distinct tags on the disk against
  twenty-nine in the answer. The TAGS section went from one chip, which
  was put there by hand so the section would exist at all, to five that
  are really carried; Audio responsive ticked finds ten wallpapers where
  it used to find none, and Mature finds one where project.json calls
  every wallpaper Everyone.

  **Both lists together, never one instead of the other.** Measured the
  other way round: one genre here is in project.json and *not* in the
  workshop's answer, because an author can change what is published
  without the downloaded copy hearing of it -- replacing the disk's list
  cost that genre its chip, and the count of genres fell from ten to nine.

  **And whatever the chips are built from, the grid has to match on.**
  The filter offering a tag that the predicate cannot see is a chip that
  finds nothing, which looks exactly like a library with nothing in it.

- **"Audio responsive" is not "has a sound".** The tag means the
  wallpaper follows the music somebody else is playing -- right for all
  seven scenes here whose renderer says they listen -- and it was read as
  whether the wallpaper makes a sound at all: "not stated" over a
  wallpaper playing its own soundtrack. Valve's Czech for the tag,
  "Reaguje na zvuk", invites exactly that. So the panel has two rows,
  *Follows the music* off the tag and *Sound of its own* off the
  wallpaper: a scene's scene.json, read during the scan by
  `wallflux_scene_has_sound` -- the package's table and that one entry,
  45 ms for all 41 packages where reading them whole is hundreds of
  megabytes -- and a video's streams from the ffprobe that already asks
  its size -- **and then its sound track heard out** (`audible`, ffmpeg's
  `astats` peak, a quarter of a second): fallout_wallpaper by sodaz and
  Halo Echoes 5 carry a track of noughts from end to end, -inf dB, and
  were "Sound of its own: yes" over a wallpaper that never makes one (25
  November); below -70 dB is no sound. Six of twelve videos and 19 of
  41 scenes carry one. A web page
  is left without the row: it plays what it likes.

- **Who made an installed wallpaper takes two questions, not one.**
  project.json has no author in it at all -- the keys are `title`, `type`,
  `file`, `preview`, `tags`, `description`, `contentrating`, `version`,
  `visibility`, `workshopid`, `workshopurl` and `general`, counted across
  all of them. The folder's name is the workshop id, and
  `ISteamRemoteStorage/GetPublishedFileDetails/v1/` answers `creator` for
  it without a key; the profile page below turns that into a name and a
  face. Both live in `qml/steamprofile.js` as a `.pragma library`, so the
  library and the workshop ask once between them. **`XMLHttpRequest` works
  inside a `.pragma library`** -- checked with a throwaway `qml` run before
  the file was written, since a shared script cannot see the engine.

- **Steam's profile page served as XML needs no key**, which is the only
  keyless way to put a name and a face to the steamID64 a UGC answer
  carries. `GET steamcommunity.com/profiles/<id64>?xml=1` gives
  `<steamID>` (the persona name) and `<avatarIcon>` in CDATA; the Web API's
  `ISteamUser/GetPlayerSummaries` wants a key and is not used. Asked from
  QML with `XMLHttpRequest`, once per account, and the answers kept in a
  map -- a grid of fifty cards is often a dozen authors.

  **Putting the answer back has to be a new object.** Assigning the same
  map back to the property changes nothing anybody is bound to, so the
  name arrives and the panel goes on showing nothing. `Object.assign({},
  ...)` is why it appears.

- **One author's work is `CreateQueryUserUGCRequest`, not a search.**
  Steam's text search matches the title and the description and has no
  idea who wrote anything. The user query takes an `AccountID_t`, which is
  the bottom half of a steamID64, and `k_EUserUGCList_Published` with
  `k_EUGCMatchingUGCType_All`. Those numbers were read out of
  `/opt/steamworks-sdk/public/steam/isteamugc.h` rather than guessed, the
  same as every other number in `steamworks.cpp`, and
  `SteamAPI_ISteamUGC_CreateQueryUserUGCRequest` is exported by
  `libsteam_api.so` -- checked with `nm -D`. Measured against a running
  Steam: four pages of one author's wallpapers came back.

- **A theme icon on its own, with no button round it, is `IconImage` from
  `QtQuick.Controls.impl`.** There is no public type for it and no
  `image://theme` provider, and it wants `color` for the same reason the
  tab icons do -- untinted it keeps whatever the icon theme drew it and
  goes invisible in one of the two themes. The panel's rows use it; a
  `ToolButton` would have brought a hover and a click nobody wants.

- **The picker needs `transientParent: null`.** The wallpaper window is a
  background layer-shell surface that takes no input; a window belonging to
  it gets dragged down there and cannot be clicked.
- **Every program linked on this machine is marked as needing x86-64-v3.**
  CachyOS's v3 repositories build glibc's `Scrt1.o` for v3 and it carries
  the mark into the binary whatever `-march` said, and glibc's loader
  elsewhere refuses to start it: "CPU ISA level is lower than required" on
  the user's A8-6410. Reproduced with Arch's own `ld-linux-x86-64.so.2`
  under `qemu-x86_64 -cpu Nehalem` (the host's glibc cannot run there at
  all). It mattered while the package was built here for every machine; the
  AUR builds it on the machine it is for, so the PKGBUILD no longer builds
  for v2 or takes `.note.gnu.property` off. A binary built here and copied
  elsewhere still needs both.
- **The tray's menu is changed in place, and reshaped only a moment later.**
  plasmashell builds menus of its own out of ours over D-Bus, and a submenu
  of ours thrown away while it had the menu open -- or was closing it, after
  a profile chosen there switched the layout -- crashed it in
  `QMenu::timerEvent` (24 November); the log said `DBusMenuExporterDBus::
  GetLayout: Condition failed: menu` around every rebuild. `Tray::setItems`
  keeps every action where the shape is the same (words, icons, ids,
  `enabled`, `visible`) and rebuilds 1.5 s later where it is not. Whether
  the panel has the menu open cannot be known: Qt says `aboutToShow` once
  and `aboutToHide` never. So keep the shape: Recent is always five places,
  the unused hidden, and a monitor's Remove is hidden, not left out.
  `tst_tray`; `~/WallFlux-ukazky/2026-11-24/vk/layoutrev.py` reads the ids.

- **Qt.labs.platform hands the panel a submenu with nothing in it.** Read
  over D-Bus (`com.canonical.dbusmenu GetLayout`): the submenu is a bare
  label, no `children-display`, no children -- a static one too, and
  `addMenu` no better -- so Recent wallpapers never opened. A `QMenu`
  submenu through `QSystemTrayIcon` arrives whole; `Tray` is that, fed a
  list from QML. Check a tray menu that way, not by trusting the QML.
- **Whether KWin has switched the screens off is said over Wayland and
  nowhere else.** Not in KWin's scripting (an Output has no dpms, read off
  `for (k in output)`), not on the session bus (every KWin and kded path
  looked through), not by PowerDevil. `org_kde_kwin_dpms` through KWayland
  is it, and **its `isSupported` is not to be trusted**: KWin's virtual
  outputs say false and report Off and On all the same, which is how it was
  tested -- `kscreen-doctor --dpms off` inside `kwin_wayland --virtual`.
- **A Popup takes its font from the window, and only an ApplicationWindow
  has one to give.** A `font.family` on a Control that wraps everything
  reaches the controls and not one popup or menu; `QGuiApplication::setFont`
  at run time reaches nothing already built. Hence the picker is an
  `ApplicationWindow`. There are no plain `Text` items here to miss it.
- **`GNUInstallDirs` keeps sysconfdir under the prefix**, so with `/usr` the
  autostart file would land in `/usr/etc/xdg/autostart`, silently. Hence the
  `if(CMAKE_INSTALL_PREFIX STREQUAL "/usr")` in CMakeLists.txt.
- **The PKGBUILD must not sit in the repository root.** `makepkg` works in
  `src/` and `pkg/` beside itself, and this project has a `src/` with the
  program in it: `makepkg -C` from the root would empty it.
- **The package is built with `options=('!lto')`, and a changed option
  wants `makepkg -fC`.** makepkg's `-flto=auto` reaches the C of QuickJS
  through `CFLAGS`, Cargo compiles it into the renderer's static library as
  LTO objects, and the final link stops on `undefined reference to
  JS_MarkValue` -- a build outside makepkg never sees the flag. And
  `makepkg -f` reuses `packaging/src/build`, whose CMakeCache keeps the
  flags of the first configure: switching LTO off changed nothing until
  `-C` cleaned it. (`-C` from `packaging/` cleans `packaging/src`, which is
  the point of the PKGBUILD not being in the root.)
- **`makepkg` builds from the tag on GitHub, not from the working tree.**
  Commit, tag and push first; a version with no tag on GitHub does not build
  at all.
- **An uncorked stream is not a stream that plays, and pactl cannot tell them
  apart.** speech-dispatcher's `sd_dummy` holds a stream open for as long as it
  runs; PipeWire pauses it after five seconds of underrun (`pulse.idle.timeout`,
  a rule for `~speech-dispatcher.*` in the stock `pipewire-pulse.conf`), and
  `pactl list sink-inputs` goes on reporting `corked: false`. Read that way,
  every wallpaper of every kind was silent from the moment it started on
  6 November -- reported as "no sound in 0.25.2", which no version had caused.
  So the stream is judged by its node's state in `pw-dump`, `running` or not;
  `pactl subscribe` still says when to look, since a node pausing or waking
  is a `change` on its sink-input -- measured with a silent `pacat
  --client-name=speech-dispatcher-test`.
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
- **KWin does not put WallFlux's window away when the desktop is shown.**
  The wallpaper's layer-shell scope is `desktop`, so KWin types that surface
  a desktop window, and a window of the same program belongs to the desktop
  as far as Peek at Desktop is concerned: the picker stayed while Firefox
  and the rest went. Changing the scope was weighed and not done -- with
  any name KWin does not know the surface is a normal window, which the
  cover script's `covers()` would count as covering itself, and which might
  turn up in the Overview. So `Main.qml` hears `ScreenCover.showingDesktop`
  and has KWin minimise the picker and bring it back
  (`ScreenCover::tuckAway`, a one-off script on `w.pid`): a Wayland client
  can minimise itself but cannot bring itself back. Only a picker that was
  open is brought back. **To test it, use the shortcut**, `qdbus6
  org.kde.kglobalaccel /component/kwin invokeShortcut "Show Desktop"`:
  `org.kde.KWin.showDesktop true` over D-Bus is undone by KWin within 0.3 s
  here, WallFlux running or not, and reads as the feature not working.

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
- **A window-wide `Shortcut` takes Escape before any popup does.** The
  picker used to hide itself on Escape, which meant that with a popup open
  Escape put the whole window away instead of closing the popup -- every
  one of them declares `Popup.CloseOnEscape` and never got the chance to
  use it. There is no Escape handler in this program now, on purpose: the
  key belongs to whatever is open, and the window is put away with the
  buttons in its corner.

  **And `CloseOnEscape` only works where the popup has the keyboard.**
  Setting the policy is not enough: a Popup does not take focus by being
  opened, so the key goes to whatever had it -- here the grid behind it.
  Measured with `activeFocus` on all three of the playlist popups: Save
  closed on Escape and the other two did not, because Save puts the cursor
  in its name field on the way up and they have nothing to put it in.
  `focus: true` on the Popup is what the other two were missing.

- **Do not translate a drop down's model.** It is a binding, so
  `retranslate()` rebuilds the list and the box drops back to its first
  row -- taking the choice with it. Measured: Order set to Random and the
  transition set to Fade to black both read blank after a language switch,
  while the three number boxes beside them survived, and a playlist saved
  then was saved in order with no transition and nobody told. The model is
  the words as they go into the file, which never change; `displayText`
  and the delegate are what translate, out of a function that takes the
  language it never reads. **Holding the chosen word in a property of your
  own and binding `currentIndex` to it does not work** -- a ComboBox
  assigns `currentIndex` itself when somebody picks a row, which breaks
  that binding for good.

- **Replacing a `Menu`'s background takes its width with it.** A Menu has
  no width of its own: it comes from the background it is given, and a
  plain `Rectangle` has none. Rounding the corners of the playlists menu
  that way left it twelve pixels wide -- its padding and nothing else --
  with its items sized to nothing and no text in sight, over two saved
  playlists that were plainly there. `contentWidth` reads -1 when this has
  happened. Say `implicitWidth` out loud, as the sort menu beside it
  already did, which is why that one never broke.

- **A `GridLayout` says nothing when a row loses a cell.** Taking the
  "change wallpaper" drop down out left its label behind, so every row
  under it sat one column over -- words in the right-hand column, drop
  downs and tick boxes in the left, and the whole panel looking shuffled
  rather than broken. It reads as four separate complaints about
  alignment, and it is one odd cell. Count the cells.

- **Wallpaper Engine's "post-processing" is the scene's bloom and nothing
  else.** `postprocessing: "disabled"` switches off the bloom's three passes,
  or the HDR bloom's ladder, and nothing else. An effect on a layer -- motion
  blur included -- is drawn whatever the setting. "ultra" turns on the HDR
  bloom only for a scene whose `general` asks for both `bloom` and HDR; none here does, so Ultra is High at
  thirty. Two packages here have bloom at all.

  **And a wallpaper's own preset is the picker's `qualityOf`**, by id, read
  by `drawnFor`, which each display asks for its own wallpaper; the panel's drop down writes it.

- **A `Switch` with its padding taken away is nought wide.** Fusion counts
  its width from the padding and the indicator's place in it, so the pill
  was drawn and no click reached it: every switch in Settings did nothing
  in 0.28.0. `implicitWidth` and `implicitHeight` said out loud, the
  indicator at `x: 0`.

- **A `Label` does not dim itself when it is switched off.** `enabled`
  reaches the mouse and not the colour, so a line beside a greyed-out tick
  box sat there in full black. It needs `opacity` said out loud.

- **A transition needs the old wallpaper after it has stopped being the
  wallpaper, and there is no second player to keep it in.** One video and
  one web view is all `Main.qml` has. What is held instead is a *picture*
  of the old one, grabbed the instant before the switch and drawn over the
  new one while it fades -- `grabToImage` catches a playing video frame,
  checked before any of this was built. So: `leaving` is that picture and
  `veil` is a black rectangle, both over everything and both nothing at
  all until a transition runs.

  - *Fade to black* needs no picture: the veil goes up, the wallpaper
    changes behind it, the veil goes down. Measured at the halfway point:
    the screen grabs as rgb 0, pure black.
  - *Fade* is the picture at full opacity fading to nothing over the new
    wallpaper. Measured at the halfway point: `leaving.opacity` 0.51 and
    the grab is plainly two wallpapers at once.
  - *None (reduce flicker)* is the same picture held for 300 ms and then
    dropped, which covers the black blink while the next video finds its
    first frame. No fade -- it is the transition that is meant to look
    like none.
  - *Random* was offered and is not any more, but a playlist written down
    while it was still says `random` and the window still plays it --
    rolled once per change and held for the length of it, so the end of a
    transition matches its start. Nothing had to be migrated, which is the
    whole reason a transition is a word and not a row number.

  **The grab never comes back when the window is not being drawn**, which
  is exactly what a covered screen is -- so there is a 400 ms rescue timer
  that changes the wallpaper without a transition. Without it a playlist
  behind a maximised window would simply stop changing. Measured: covered
  the whole time, the wallpaper still turned.

  **A transition is written down as a word**, not as the number of a row
  in a drop down: `""`, `soft`, `none`, `random`, `fade`, `black`. The
  rows have been reordered twice already and a number in a file would have
  meant something else each time.

- **A `Popup` that does not fit does not scroll, it overflows.** The
  workshop's filter holds fifty-nine chips, which is taller than the window
  on anything but a very tall screen: the bottom section and the footer ran
  off the edge and Clear could not be reached at all. It takes a height it
  is told there is room for and a Flickable inside. Told, because a Popup
  is not an Item and has no `Window` attached -- asking gives undefined and
  the height comes out as nothing, which collapses the popup to its footer
  floating over the page with no background.
- **A sentence built in C++ with `tr()` is built once**, in whatever
  language was installed at that moment, and stays that way.
  `QQmlEngine::retranslate()` refreshes bindings that translate, and a
  string handed over from C++ is not one. The heading changed with the
  language and the reason under it did not. So C++ reports *which* reason
  and the window chooses the words. **And a binding that gets its words out
  of a JavaScript function is not refreshed either** -- the function has to
  take the language as an argument it never reads, so that reading it is
  what makes the binding rebuild. It looks like a pointless parameter and
  it is not.
- **A `MouseArea` takes hover away from the one under it**, so anything
  shown *because* the pointer is there and holding a mouse area of its own
  flickers several times a second: the tick box on a card appeared, took
  the hover, which hid it, which gave the hover back. A `HoverHandler` on
  the card answers "is the pointer anywhere on this" without taking it
  from anybody.
- **`Row` is a QtQuick type.** An inline `component Row:` shadows it
  silently and whatever used the real one breaks somewhere else entirely.
  Named `DiscoverRow` for that reason.
- **`QStringLiteral` is UTF-16 and a compiled `.qm` is compressed**, so
  plain `strings` finds neither. Checking what actually went into a built
  binary needs `strings -a -e l` for the first and nothing at all for the
  second -- look for `:/i18n/wallflux_cs.qm` instead. Both of these were
  read as "the package is missing my change" before the method was
  suspected.
- **`qmllint` is not in `PATH`** on this machine; it is
  `/usr/lib/qt6/bin/qmllint`. Linting the files in `qml/` warns once, that
  `Look` is a singleton not declared in a qmldir: there is none in the
  source folder, and the module CMake builds declares it. Everything else
  should be clean -- `Application.screens` and `Application.font`, not
  `Qt.application`'s, which the linter does not know.
- **A `GridView` puts `currentIndex` back to nought whenever its model
  changes**, and this grid's model is a fresh array every time anything
  about any wallpaper changes -- a video answering with its size, Steam
  finishing a download, a card being looked at. So a row number is no
  answer to "which card is the ring on": it was handed back to the first
  card a moment after every click, which is why only the first wallpaper
  could be picked and why the arrow keys went nowhere. The id is the ring
  now (`picker.ringOn`), the grid's own `keyNavigationEnabled` is off and
  the four arrows are answered by `grid.walk()`, which scrolls to what it
  lands on. Putting the row back after each rebuild was tried first and is
  a race: the grid resets itself when it gets round to it, which is
  sometimes after the repair has run -- measured, with the ring landing on
  a card nobody had pressed an arrow towards.

  **And looking at a card rebuilt the whole grid.** The details answer was
  assigned into `workshopTags` even for an id already in it, and `shown`
  is built out of that map -- so one click destroyed every delegate and
  put the view back to its first row. Only what is new goes in now.
  Measured: the ring ten cards down, and the grid stayed where it was.

- **The grid read "what was played when" whether or not it sorted by it.**
  `shown` is a binding, so it depends on everything it reads -- and
  putting a wallpaper on the desktop changes `Wallpapers.recent`, which
  rebuilt the list and with it every card. A rebuild puts the view back to
  its first row, so a double click somewhere down the grid started the
  wallpaper and threw the window up to the top of the library. `recent` is
  read inside its own `case` now, where sorting by it happens. Measured:
  the view stayed where it was, 3 757 pixels down.

- **A view that was scrolled when the window changed size is left past the
  end of the grid, and in that state it draws nothing and will not be
  moved.** Measured on maximising with the grid scrolled down: 2 588 down
  a content height of 1 491, no cards drawn at all, and only a scroll of
  somebody's own brought them back. `positionViewAtIndex`,
  `returnToBounds`, `forceLayout` and writing `contentY` by hand were each
  measured doing nothing there -- and this is not something the ring
  introduced: it reproduces with the repair switched off, so it has been
  waiting in there for a grid that was scrolled when something resized it.
  `grid.positionViewAtBeginning()` **first**, and then
  `positionViewAtIndex` -- rewinding gives the view somewhere to count
  from, after which it goes where it is asked. Both in the one function,
  so nothing is drawn in between.

  **So one place does all of it.** `grid.keepTheRingInView()` is what the
  arrows, the panel opening, every model rebuild and every resize go
  through, and the grid asks for it on `onModelChanged`, `onWidthChanged`
  and `onHeightChanged` -- through a 150 ms timer, which is what puts the
  three of them together when a window is maximised and they all arrive at
  once.

  **And the rewind only when the view is really stranded.** Rewinding
  destroys every delegate on screen, and what comes back are new `Image`s
  loading asynchronously -- so a repair one case in many needs was the
  whole grid visibly blinking on every resize, which is what maximising a
  window and putting it back looks like. In bounds, `Contain` scrolls the
  least it can and does nothing at all when the card is on screen already.
  Measured on open, on maximising and on coming back: contentY 0 against a
  content height of 4 176 and then 1 491, and not one rewind.

- **A double click cannot be counted by the card it lands on.** A rebuild
  between the two presses destroys the delegate, so the second press
  reaches a different object which sees a first click -- and with the
  model rebuilding as often as it did, a double click on anything but the
  first card was two single clicks. `Picker.qml` counts it instead, over
  a 400 ms timer, which also waits out the reflow: opening the panel takes
  the grid from five columns to four, and a card that moves out from under
  the pointer cannot be clicked a second time.

  **And a double click never reaches that counting.** Qt delivers the
  second press as `doubleClicked`, not as a second `clicked`, so it goes
  straight to `apply()` and `chooseOrApply` is none the wiser -- the click
  it was counting is still running, and four tenths of a second after the
  wallpaper started, the panel did what that first click had decided. With
  the panel open on the card being double clicked, that decision was
  "shut", so playing a wallpaper closed the panel by itself. `apply()`
  stops the timer, which is what makes a double click mean one thing only.
  Both paths have to exist: where a model rebuild destroys the delegate
  between the presses, the second press lands on a new object which sees a
  first click and `doubleClicked` never comes at all.

  **Which is why a double click can only be ignored by waiting.** The
  workshop has nothing to play, so a double click there is to do nothing
  at all -- and since the first `clicked` arrives either way, the only way
  to take it back is not to have acted on it yet. A click out there waits
  the four tenths out and `doubleClicked` cancels it. Measured with
  `QtTest`, which can deliver what a mouse cannot: one click gives one
  toggle of the panel, a double click gives none, and two clicks 380 ms
  apart give one -- Qt no longer calls that a double click, and the second
  click rewinds the wait rather than being counted. The wait cannot lose
  the race, either: this machine's double click interval is 400 ms
  measured from the first *press*, and the timer starts at the first
  *release*, so the cancel is always that much ahead of it.

- **A ring round a card is a second frame, and it takes the name in with
  it.** The preview already has a band, so an arrow-key highlight drawn as
  a rectangle over the whole card put two frames round one picture and a
  box round the words under it. The band changes colour instead. Nothing
  is lost by borrowing it: a wallpaper that cannot be played is drawn at
  four tenths and says so that way too.

- **A card in the grid is a square, and the whole preview is sixteen by
  nine.** Both, in different places. The grid crops to a square and puts
  the name inside the picture over a gradient, which is what Wallpaper
  Engine's own picker draws and what fits five cards across a window
  rather than three; the details panel and the playlist strip keep the
  wallpaper's own shape, because the panel is the one place a preview is
  looked at whole and a square there would crop a third of it off for
  nothing. The price was said out loud before it was chosen: a square
  throws away the sides of every preview in the grid.

  **A `PreviewTile` takes the shape it is given.** It used to ask for
  nine sixteenths of its *own* width whatever height it was handed, so
  `Layout.preferredHeight` did not shorten the picture -- it let the name
  underneath be drawn over the bottom of it, with no warning anywhere.
  The frame fills the tile now and the name is inside the picture, so
  there is nothing left to overflow and both of the panel's two numbers
  are read.

- **A child's scale goes into the texture an effect reads; the source
  item's own does not.** The preview leans in a twentieth under the
  pointer, and the picture is drawn by a `MultiEffect` that rounds its
  corners -- so scaling the thing handed to `source` would have been
  thrown away with the rest of that item's transform. The Image sits
  inside a box with `clip: true` and the box is the source: the zoom is a
  child's, the crop is the box's, and the card itself does not move.
  Checked by drawing one at 1.0 and one at 1.3 side by side and looking at
  the grab, which is the only way this one shows itself.

- **A gradient stop of `"transparent"` is a transparent *black*.** The
  name's gradient fades the card's own base colour up out of the picture,
  and ending it at `"transparent"` interpolates through black on the way
  -- a dark bruise across the bottom of every card in the light theme.
  Each stop names the base colour and changes only the alpha.

- **A decision about where an item stands must not read the item's size
  in that place.** The panel's `short` read how tall the picture and the
  name are, and their height followed the width of whichever place they
  stood in -- the one not yet shown is nought wide -- so near the boundary
  they moved there and back for ever: a polish loop, the main thread at
  full speed, the window frozen, and **not one line in the log** (24
  November, on the laptop). Found by running the build under `gdb` (a
  process gdb starts may be traced; `ptrace_scope` 1 forbids attaching
  to one already running) and `kill -INT` when it hung: the stack was
  `QQuickLayout::updatePolish` into a binding into `setParentItem`. The
  picture and the name are the list's width in both places now.

- **The details panel scrolls from the author's row down.** The picture
  and the name stay put; everything under them is in the Flickable. Two
  wrong answers were tried first: a floor under the list
  (`Layout.minimumHeight`) overflows the column and cuts the button off the
  bottom, and scrolling the picture too means a hundred and sixty pixels of
  photograph in the content, so there is always more than fits and the bar
  never goes away. With the picture outside it, a maximised window has
  nothing to scroll and says so.

- **Setting `policy: ScrollBar.AsNeeded` on `Scroller` does nothing on its
  own.** Fusion does "as needed" by fading the `contentItem` *it* draws,
  and Scroller replaces that contentItem -- so the policy is set, the
  fading is gone with the thing that did it, and a full-length bar is drawn
  down the edge of content that fits. Scroller does the hiding itself now.
  Found in a maximised window, where there was plainly nothing to scroll
  and a bar saying there was.

- **A `ToolButton` with nothing round it is a button nobody sees.** Fusion
  draws no background until it is pointed at, so a magnifier beside the
  author's name read as decoration and was reported missing. The whole row
  is a `Button` with an edge on it now.

- **A picture four tenths there over a dark window is a pale grey room.**
  The window is tinted by whatever the panel is showing -- the preview,
  blurred past recognition, half the colour taken out, faded in over six
  tenths of a second. At the opacity the effect is drawn with and nothing
  else, a bright preview lifted the whole window and the small print under
  the heading went with it. The window's own colour goes back over it, three
  tenths of it, which is what the program this was copied from does too.
  Found by drawing the three strengths side by side and reading the words
  on each.

- **A property here can shadow a caller's id.** `PreviewTile` grew a
  `card` property for the colour its name fades into, and a
  `WallpaperCard`'s own id is `card` -- so every binding written inside
  that tile in `WallpaperCard.qml` read the property instead of the card.
  `qmllint` says `id-shadows-member` and names both; nothing at runtime
  does. It is `backing` now. Run `qmllint` over **every** qml file, not
  the two or three a change was in: this one went in with the squares and
  was found a day later, in a file that had not been touched.

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
- **Steam's library sets `LC_ALL=C` in our environment.** `steamclient.so`
  calls `setenv("LC_ALL", "C")` as `SteamAPI_InitSafe` loads it -- found
  with an `LD_PRELOAD` logging every `setenv` with its caller, 17 November.
  Our own locale is untouched (Qt has called `setlocale` by then), but every
  child inherits it: the Konsole of Update, the makepkg in it, and the
  WallFlux it starts at the end, which then sorted by code point. A
  restart from the menu did not help, since the new copy connects to
  Steam again. `Steamworks::open` puts `LC_ALL` back, set or unset, with a
  scope guard round the init. `pactl` prints English whatever the locale
  (checked under `cs_CZ`), so nothing here leaned on the C.

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
- **Steam unsubscribes and then leaves the folder exactly where it is.**
  `UnsubscribeItem` works and says so -- the call answers callback 1315
  with `k_EResultOK`, `GetNumSubscribedItems` drops by one and
  `GetSubscribedItems` no longer carries the id -- and the wallpaper is
  still on the disk, still in the grid, and a Refresh finds it again.
  Measured with the program running and again a minute after it had quit;
  Steam cleans up in its own time, if at all. So giving a wallpaper back is
  two halves: Steam lets go of it, and `WallpaperCore::discard()` takes the
  folder. Deleting without unsubscribing would be worse than useless -- the
  next thing Steam does is fetch it again.

  **`GetItemState` is not the way to ask whether something is subscribed
  here.** It answered 4 (installed) and nothing else for an item the acf
  and `GetSubscribedItems` both called subscribed, before and after the
  call. The subscribed list is what to read.

- **Asking Steam's CDN to resize a preview is a trap**, and it looks like
  the obvious saving. A preview is a few hundred kilobytes and a card draws
  it 240 across, and Steam's own pages put
  `?imw=...&imh=...&ima=fit&impolicy=Letterbox` on the very same URLs. For
  a photograph it is real -- 296 kB becomes 11 kB. But most of the
  workshop's previews are animated, and for those the picture is
  **re-encoded when it is asked for**: a size nobody has asked for before
  is made on the spot. Measured over six previews, 0.7 to 7.9 seconds each,
  where the file as stored came back in 0.03 to 0.09 -- and the resized
  copy was not even smaller, one of them 1 099 kB against 924 kB stored.
  Sixteen stored previews, eight at a time: 8 MB in 0.19 s. So the bytes
  were never the problem on a connection like this one; the re-encoding
  was, and it is what made a page take tens of seconds. Take the URL as
  Steam gives it. On a slow line the sums would come out differently for
  the photographs, and that is a comment and a URL away.
- **Steam will not page for ever.** Measured against Trending: page 1001
  comes back with its fifty, page 1024 is refused outright with result 25,
  and so are 2000 and 64 929. The total it reports -- three and a quarter
  million -- divided by the fifty it answers with is 64 929 pages that do
  not exist. The grid stops at a thousand, which is where Wallpaper Engine
  stops too.
- **A Popup closes on the press, not the click, so a button cannot toggle
  one** without being told to. The press lands outside the popup, which
  closes it, and the click that follows opens it again -- so the menu
  flickers and stays. `Popup.CloseOnPressOutsideParent` keeps a press on
  the button that owns it from counting as outside, and then
  `opened ? close() : open()` does what everybody expects.
- **A scene let go is memory glibc keeps.** `wallflux_scene_free` frees
  everything and the resident size does not move: glibc holds on to it for
  the next allocation, so the status bar's RAM only rose from wallpaper to
  wallpaper and stayed at the largest. `release()` in `sceneview.cpp` calls
  `malloc_trim(0)` after every free -- 1 ms. Measured over the library
  loaded and freed in turn, 255 MB and 227 after; the first eight alone,
  238 and 166. What stays is the GL driver's own.

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
- **A `static` member declared after `signals:` is a signal as far as moc
  is concerned.** It generates a call through `this` for it and the build
  stops with "invalid use of 'this' outside of a non-static member
  function", pointing at generated code rather than at the header. The
  access specifier is what decides, not the keyword: put it under `public:`.

- **Say it with the control, not with a line of small print under it.**
  The three number boxes had a sentence beneath them explaining that five
  seconds was the least however short they were set -- because nought was
  typeable and the floor was invisible. Putting the floor into
  `asClock()`, which the boxes are written back from, made the sentence
  pointless: the boxes read five by themselves. One line of C++ for one
  fewer line of apology.

- **Clamp the time, not one of its units.** The settings popup's three
  boxes take two digits each and carry what is over -- sixty seconds is a
  minute -- and the first way of doing it carried unit by unit, capping
  the hours at 23 on the way. Typing ninety seconds at 23:59 then gave
  23:00:30: *less* than before, because capping the hours threw away the
  minutes under them. Added up as one number of seconds, clamped once and
  handed back as a clock, it comes out 23:59:59. The arithmetic lives in
  `Playlists::asClock()` rather than in QML, because a rule in QML is a
  rule no test can reach.

- **Fusion paints the strip a SpinBox's arrows sit in whether or not there
  are any.** `up.indicator: null` and `down.indicator: null` take the
  arrows away and leave a grey block hanging off the end of the number,
  lit up whenever the box has the keyboard, with nothing to click. The box
  needs a background of its own.

- **A two-character field that is full refuses the next keystroke**, and
  the number boxes are two characters wide: a box reading 15 could not be
  typed over at all, only selected by hand first. They pick their whole
  number out when they are reached, so typing replaces it. **Through
  `Qt.callLater`**, because a click moves the focus and *then* sets the
  cursor, which would undo a selection made on the spot. A second click
  in a box that already has the keyboard puts the cursor where it was
  asked for, which is how a field is edited rather than replaced.

- **A `CheckBox` with no words still leaves room for them.** Its padding
  and the gap before the label are both still there, so one aligned to the
  right of a layout hangs over the edge it was aligned to. `padding: 0`
  and `spacing: 0`.

- **The playlist timer is tested by letting it run.** `tst_playlists`
  waits real seconds -- `holdingKeepsWhatWasLeft` about eight,
  `nextMovesOnAndRewindsTheClock` six, fourteen of the suite's time, the
  floor of five seconds being what each has to outwait -- on purpose: the one way to get this wrong is the
  multiplier, seconds read as minutes is a playlist sixty times too slow,
  and no unit test of the arithmetic catches that.

  The playlists menu is the same lesson from the other side: which
  playlist is playing is `Playlists`' own business, so its items are not
  checkable at all -- the tick is a Label reading the singleton, and there
  is no control state to break.

- **A Controls item cannot both be told what to be and asked what it is.**
  Caught twice now. The sort menu was the first; the second was the
  playlist's `SpinBox`, written as `value: picker.playlistMinutes` with
  `onValueModified: picker.playlistMinutes = value`. The box read 1 and
  the property was still 15, so a playlist set to a minute was saved as a
  quarter of an hour and sat there changing nothing. The control holds the
  number and the window reads it off when it saves. Bind `checked` to a property and set that property from
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
/usr/lib/qt6/bin/qmllint -I build/ qml/*.qml   # one warning is expected, below
cargo test --offline     # the scene renderer, when crates/ was touched
```

A change to the renderer is checked against the whole library, not one
wallpaper: `tools/sweep.sh <dir>` before and after, then `cmp` the two
directories picture by picture. The sweep draws with a copy of `shot` made
when it starts, so the source can be edited and built while it runs.
Every picture that moved has to be one
the change meant to move -- and looked at, because "different" is not
"better". Fixing a shader once turned a Space Marine's armour white.
The sweeps so far are in `~/WallFlux-ukazky/<date>/`.

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
5. `gh release create v<version> --notes-file <the changelog entry>`: the
   notes are what Settings' "What is new" opens, and `releases/latest` is
   what the updater asks. No package is attached any more.
6. `cd packaging && makepkg -f`, which builds from the tag on GitHub -- the
   check that the AUR will build it -- then `makepkg --printsrcinfo >
   .SRCINFO`, and commit the two with the release if they changed.
7. The user updates with the line Settings shows, as everybody else does:
   `install.sh` off main, which builds the tag `releases/latest` names --
   so a release is installable the moment step 5 is done, and a change to
   `install.sh` reaches everybody the moment it is on main.

   **Not in the AUR** (17 November, the user's word: registering there is
   given up for now). Should it ever be: copy `packaging/PKGBUILD` and
   `.SRCINFO` into a clone of `ssh://aur@aur.archlinux.org/wallflux.git`
   kept outside this repository, push, and decide with the user whether the
   README and Settings move to `paru`; the key is `~/.ssh/id_ed25519.pub`.

0.2.0 was tagged before CHANGELOG.md existed, so its tag does not contain
the file. Everything after it should.

## Open, in the order they are likely to be taken

**The user's word on 9 November, evening: at the start of the next session,
even when all they write is "pokračujeme", list everything below that is
open and let them choose. Do not pick for them.** 0.33.0 was released that
day; everything asked for on 9 November is in it.

**The AUR is given up for now** (17 November, the user's word): its new
accounts stayed closed, and WallFlux is installed and updated by
`install.sh` instead (see Updates of WallFlux itself). The rule of
16 November -- no releases until the AUR -- no longer stands, and
registering there is no longer to be reminded of.

**Done on 24 November: a wallpaper on each monitor** (see What works today),
released in 0.43.0 after the user tried it on the laptop with a monitor.

**Then the list as the user left it on 16 November, to be offered:**

- **A wallpaper to look into** (the user's word of 24 November):
  *fallout_wallpaper by sodaz*, said to play no sound, has none -- its
  track is silent, and the panel says so since 0.45.4; *Stargate Dialing
  Computer* (a page, 1795132779) has
  interactive parts -- the right sequence opens the gate -- and the
  wallpaper's surfaces take no input, so whether and how it can be played
  with is the question.
- **Wallpapers against Windows**: Augmentation's bolts (white-violet there,
  blue here), Space Visualizer's lava spark fans (fewer a minute), DOOM's D
  and M (follow the music at 0.75), Alien Isolation's reflections and lit particles, Blue Dawn's flare moved
  by hand (`MOVED` in `render.rs`, until its cause is found).
- **The grid**, to be optimised.
- **WallFlux itself, checked**: speed, size, installing, uninstalling,
  what it takes of the machine.
- **On the laptop** (AMD A8-6410, Radeon R5, 2014, 1366x768): the battery
  and a second monitor, with the user -- both done on 23 November (the
  battery drops to Low as it should; the monitor, an ENC CS2420 at
  1920x1200 on HDMI, found the two fixes of 0.42.3). Once, before those
  fixes, the monitor showed plain white and it could not be made to again
  in half an hour of trying; if it comes back, note what was on each
  screen. **Its scenes are solved** (19
  November): Wallpaper Engine itself was not installed there, so no scene
  had its stock shaders and all 42 graded Broken ("nothing could be
  drawn"); installed, all 42 grade Works and DOOM plays on Medium and Low.
  Radeonsi with OpenGL 4.6 was never the trouble. 0.42.2 was released from
  the laptop with two things found there: the details panel on a short
  screen, and wallpapers listed twice from two Steam libraries.
- **SCP Terminal Lockdown with following the mouse off** gets the unknown
  mouse's `-10` and shows a red copy of its bar; what Wallpaper Engine hands
  then was not found.
- **Further improvements and changes** as the user brings them.

Done on 16 November: playlists gone through with the user (0.39.1), the
filters' sections in one order (0.39.2), the three wallpapers' minor issues
(0.39.0).

0. **Waiting on decisions of the user's:**
   - **Making the repository public: decided on 16 November.** The history
     stays as it is, e-mail and all (the user's word). Where Wallpaper
     Engine's behaviour was found in its binaries came out of this file, the
     code's comments and CHANGELOG.md into `CLAUDE.local.md`, untracked; new
     findings of that kind go there and only the behaviour goes here.
     WallFlux goes to the AUR as `wallflux`, built from the tag, and the
     updater only says a new version is out.
   - **Trying it on the laptop** (the user's, with help asked for): a
     battery and several monitors there -- still to do; the repository is
     public and updates have been installed end to end. Properties on a web wallpaper changing
     live and pausing for another program's focus were confirmed by eye on
     10 November.

1. **The wallpaper backlog of 9 November** (memory `backlog-tapet-9-11`):
   Augmentation's bolts blue where Windows has them white-violet, Space
   Visualizer's spark fans fewer a minute, DOOM's D and M at 0.75, red specks
   in Star Citizen (unchecked), Alien Isolation's reflections and lit
   particles; formats nothing here uses yet (`controlpointangle*`,
   `brightness`, `movement` flags bit 0, `alphafade` and `oscillate*`
   against Wallpaper Engine, what scene scripts cannot reach); Blue Dawn's hand
   offset to take out once its cause is found. Put off by the user on
   10 November, to come back to.

2. **The tray icon itself.** Today it is `preferences-desktop-wallpaper`
   from the icon theme. A drawn one has been asked for, without any hurry.

3. **Scene wallpapers that look like their preview -- first, before
   anything else, decided on 28 October.** The music, the JavaScript and
   the mouse are done, and still not one scene looks like the preview in
   the grid once it is put together: the pieces are there and the
   composition is not. That comes before the user properties of the
   Settings tab, which wait behind it. Check each against its preview,
   whole library, `tools/sweep.sh` before and after.

   **29 October: most of it was the axes** -- y upside down, angles a
   fifty-seventh -- with blend modes, bloom, alignment and an opaque
   alpha besides; see the traps. The smoke that was too strong was every
   particle drawn twice its size. Screenshots of Wallpaper Engine on
   Windows are in `~/WallFlux-ukazky/wpe-windows/` (Arkham Knight, Kratos,
   Star wars, Warhammer) and still disagree in two places: Warhammer has
   star-shaped flares all over that Wallpaper Engine draws only at the
   muzzles, and Star wars lacks orange bokeh at the left. What is still
   visibly off besides: Augmentation's iris; red specks in Star Citizen; SCP Terminal
   Lockdown's chromatic aberration, a workshop shader that leans on HLSL
   truncating types (a `vec4` into a `vec3`, a `vec2` into a `float`, a
   `vec4` as texture coordinates) and needs a translation that knows types; a black square under
   Alien Romulus's mouth; Alien Isolation, which wants its 2D lights.

   **30 October, against the videos:** Warhammer and Star wars now match the
   recordings within a few levels. Star wars still has a little more haze at the
   left (39 against 32 in its darkest part) and Dark Core lacks its glowing pipes.
   `turbulence` carries a particle where Almamu accelerates it; not yet compared.

   **30 October, looked at by the user on the desktop:** no complaints about
   Warhammer, Star wars, Arkham Knight, Blue Dawn, Time Reactor, Future Clock,
   Dark Core, Star Destroyer and Pyramid Spaceships. Star Citizen is better than
   it was. **DOOM is clearly better and still wants work**; Ender Spartan looks
   right but has yet to be compared with Wallpaper Engine. A spot check, not the
   whole library.

   **30 October, the rest of the vocabulary:** `boids`, `collision*`, `remap*`,
   `maintaindistance*`, `inherit*` and `once` are read and run. Their defaults
   are Wallpaper Engine's; **how they work is the editor's description of
   them**, and no wallpaper here uses one to compare with.
   Stock rain (`rain_screen`), lightning (`thunderbolt`) and `dripping_water`
   are the ones to look at first when one turns up. Not done: `collisionmodel`
   (there are no models) and the `layerimage` emitter, both reported.

   **Alien Romulus and Space Marine 2 -- Honor and Glory were said to move
   with the mouse where they should not.** Romulus' whole answer to the
   pointer was the specks now left out. Space Marine 2 was looked at by the
   user on 2 November and is right: its light goes where the mouse goes.

4. **6 November evening, the user's word: particles to move as Wallpaper
   Engine moves them, before anything else.** Space Visualizer's swarm is the
   case that shows it (entry below and its 6 November paragraph). Check each
   finding by drawing the swarm alone (`~/WallFlux-ukazky/2026-11-06/nastroje/
   README.txt`: copies of the scene with one preset changed, motion maps
   beside Windows) and then the whole library in a sweep, since a step's
   meaning changes every scene that has it. Where each step was read is in
   `CLAUDE.local.md`.

   **9 November: `movement` and `colorchange` read; see the trap "Pushes
   are felt over less time at fifteen".** `colorchange` multiplies the born
   colour, as ours did. The lava sparks of Space Visualizer were the
   size a child did not take (see the instance-override trap); their shape
   is Windows' now, and there are fewer fans in a minute than in the
   recording, which comes down to how often their parents hear bands 5 to 15
   loud enough. Left: `movement`'s `flags` bit 0, which turns gravity by
   the object in a system not in world space (not read here; no scene known
   to set it); `alphafade` and `oscillate*` steps not yet compared.

5. **The list as the user set it on 6 November, in this order, and only this.**
   Done that day: fifteen frames a second and no thinning, both as Wallpaper
   Engine (see the frame-rate trap).
   - Done: several screens as Wallpaper Engine's "Clone single wallpaper" (see
     the Displays entry under What works today).
   - **Space Visualizer:** its answer to the music and its sky (entry below).
     **6 November**, unthinned and against the user's recording with its sound,
     averaged over a minute: the planet follows Windows at 0.92, the lava at
     0.98, the sky's mean colours within a level or two; the vortex fix (see
     its trap) laid the red star streaks flatter. **Left: the swarm.** On
     Windows a wide veil of fine red dust streams from the whole small planet
     towards the sun; ours is a narrow bright plume curling round its right
     edge. All five `particle_0x_*_response` systems keep control point 1 on the
     mouse (`locktopointer`) and push away from it at -10000 within a
     `threshold` of 128; the recording's cursor sits still on the small planet
     at (0.4504, 0.4479) of the screen, and drawn with it there ours did not
     change -- nor with the mouse handed to these world-space systems unscaled
     (they are at 1.4 and 1.5, and `point_at` passes it through the object's
     whole matrix, the ponytail in `stand_at`). That change was taken out again:
     it showed nothing and would move DOOM's, Rainbow Six's, Snow Lights' and
     3330497043's systems under the mouse.

     **6 November, later, drawn at fifteen** (as the desktop): the swarm is
     `REFRACT` with an overbright of 5, so it shows only where something
     bright is behind it -- over the black sky it is nothing, which is why it
     reads as a "veil" over the small planet's face on Windows. Its birth ring
     is right: `distancemax` defaults to 256 in Wallpaper Engine too, for a
     flat scene, and drawn alone
     ours is born at 179 screen pixels of 1280, where Windows' arc of specks
     is. Ruled out, each drawn and mapped against the recording: the vortex,
     the turbulence and the pull switched off; the pull with the recording's
     mouse, as read and with its sign turned over (identical to the pixel: no
     particle comes within its 128); the instance `rate` taken out. A third
     of the drag gives straighter strokes at Windows' 45 degrees, but our
     drag was fitted to this wallpaper's sparks and Wallpaper Engine's
     `movement` step was not known yet. Without the instance colour the strokes over
     the face come out white as on Windows, but by numbers over 12 s the
     face is within 1 to 4 levels of Windows either way and moves 25 to 35 %
     less (above its darkest: Windows 13.7/11.4/10.4 on the right of the
     face, ours 10.5/7.8/6.8, without the colour 11.1/9.5/8.3). Almamu
     multiplies a `colorchange` by the colour a particle was born with, as
     we do. Nothing changed.
   - **DOOM's D and M** following the music at 0.56: take the wallpaper apart
     again, and look at what Almamu's renderer does with this one. **6 November:**
     the letters' audio is `effects/pulse` with AUDIOPROCESSING 3 and no
     `frequencymin`/`frequencymax`, so bands 0 to 1 by the shader's defaults,
     `smoothstep(0.5, 1.5)`, amount 1 (the O's 2 and 5); their values are keyed
     by the editor's labels (`ui_editor_properties_audio_bounds` and so on),
     which we read through `named_by`. D's `shake` hears the left ear alone (1),
     M's the right (2). Almamu has nothing for this wallpaper and no Wallpaper
     Engine meter: `PulseAudioPlaybackRecorder.cpp` takes bins `band x 2` of a
     kissfft, `0.35 log10`, its own weighting curve and eases at 0.3 -- a
     visualiser, so no second opinion on the bass. Still open: why our band 0
     holds near 0.79 through DOOM's rumble where Windows' D and M follow it.
     **The wallpaper's own bass tracking is its particles**: no scripts at all,
     and round the D and the M every system that listens -- the Torches, both
     Smoke clouds D, `new_particle_system_3` -- states no band range, so hears
     bands 0 and 1, both ears, bounds "0.9 1"; the Os' torches hear 10 to 14
     and 10 to 15. Wallpaper Engine reads 16 entries an ear, mode 3 the
     largest of left plus right halved -- as ours.
     So the letters, effects and flames alike, all come back to the meter's two
     lowest bands of sixteen.
   - Done: **Alien Romulus' specks round the mouse**, drawn again in 0.25.2.
     **6 November:** `controlpointattract` as Wallpaper Engine has it: the
     whole strength at the point,
     none at the full `threshold`, a straight line between, never past the
     particle with `flags` bit 1 (on by default), 512 and 512 when unstated, and
     the file's `origin` not read (the engine's key is `offset`, left out of the
     pull). On the branch `pritahovani-jako-wpe`, not main: it changes the mouse
     in seven approved wallpapers. Drawn with it and the recording's cursor
     track, the specks land where Windows' do and are still clusters several
     pixels across where Windows has dots of two or three. The system is
     `animationmode` `randomframe` but its texture carries no sheet (no `TEXS`),
     and it draws as a `spritetrail` (0.05, at most 1) whose length the stock
     shader takes as `max(minlength, min(speed x length, maxlength))` sizes --
     we hand it the same three -- so a still speck is a sliver on Windows; what
     makes ours larger is the size or the speed it is handed, not found yet.
     **Found the same day: the override reaches more than we bound.**
     Wallpaper Engine binds a placed system's `instanceoverride` so: `count`
     the emitters' rates and the sequences' counts; `lifetime`
     `lifetimerandom`; `size` **`sizerandom` and both ends of every
     `sizechange`**; `speed` the emitters' and `velocityrandom`'s and
     `turbulentvelocityrandom`'s speeds and **`controlpointattract`'s scale,
     both vortices' speeds, `turbulence`'s speeds and `oscillateposition`'s
     frequencies**; `rate` also both turbulences' `timescale`. Binding the bold
     ones, Romulus' specks (size 0.51, speed 0.5) came out as the recording's
     sparse small dots with nothing fitted; Space Visualizer's swarm (speed 2)
     streams wider towards the sun, its bright rim weaker. On the branch
     `instance-jako-wpe`, not main: a sweep moved 16 of 41 pictures, most by
     under half a level; Gears against its recording 5.85 to 5.07 levels off on
     average and 2.05 to 0.98 in its swing, Dark Angels 8.31 to 7.85. The
     `rate` to `timescale` binding is not taken: `rate` already runs a system's
     whole clock here, and whether it does both in the engine is not known.
   - **Blue Dawn:** the weaker sun and the larger flare. **6 November, found so
     far:** the sun's core is 174 against Windows' 181 over 40 s, so hardly
     weaker; the difference is a white blob, 25 px across at 2560x1440, that
     never moves from the lens flare system's origin (2012, 1597) -- 249 there
     against 115 on Windows, where nothing stays put and the flare's dots turn
     round the sun. The blob is the parent system's own halos (`Lens_Flare.json`,
     children taken out and it stays). They have no `movement`, so they stay
     where they are born, in Almamu's renderer too, and the editor tags
     attract and turbulence "Movement operator required": nothing to fix there.
     Open: what puts Windows' halos elsewhere. Candidates are the parallax --
     the object has `parallaxDepth` 1.5 and the recording's mouse sits up and to
     the right of the sun (1877, 150), where ours leans the background so that
     the sun moves and the blob moves away from it -- and the `angles` script,
     which pivots the layer about `size x scale x (0, -0.5)`, nothing for a
     particle layer here.

     **Later that day:** the parallax is not it -- the background is at depth
     1.5 like the flare, so both move alike, in Almamu's formula as in ours
     (`(depth + amount) x displacement x width`); and the "cursor" at (1877, 150)
     is part of the background, Almamu draws it too. Almamu draws no blob at
     all, with the script or without it, and no flare (he has no children).
     **Moved onto the sun, at (2035, 1905) instead of (2012.6, 1597.9), ours
     looks like Windows** -- dots on a line through the sun on both sides,
     turning -- and over 40 s the old blob's place reads 128 against Windows'
     126, the ring round the sun 169 against 166, the sun's core 246 against
     238 (white over more of the picture than Windows). So Windows has the
     system about 308 scene pixels higher, seemingly fixed; why is not found.
     Wallpaper Engine gives a script `size` only on image layers, so
     `thisLayer.size` is
     undefined for this one, and `lib.sceneScript.d.ts` declares only
     `Vec3(x: Number|Vec2|String, ...)`.

     **The script is ruled out too.** Wallpaper Engine's script classes are
     JavaScript on the disk, `assets/scripts/jsclasses/baseclasses.js`, and
     its `Vec3` gives (0, 0, 0) for `undefined` and
     z = 0 for two numbers -- so the pivot is nought there as here, and the
     script only turns the system about its origin. **That file is the reference
     for every script class**; `host.js` should be checked against it. Also
     tried: the background's parallax shift in the recording, by phase
     correlation (up to 27 screen px, so the mouse did move); counting the
     flare's bright spots against the sun, which the clouds and stars drown.
     Still open: what puts the system's middle on the sun in Wallpaper Engine.

     **9 November, by the user's decision: moved by hand** (`MOVED` in
     `render.rs`, applied to the parsed scene so its script writes the moved
     origin back). Up 307.1 and right 22.4: where the blob was the recording
     reads 139 over 35 s, ours 202 before and 141 now; the sun's surround 167
     against 170. The dots lie on a line through the sun, as on Windows; which
     way the line points depends on how long the wallpaper has run, which the
     recording cannot say. The cause is still not found -- not the control
     points either, which the scene moves (see that trap) and which reach the
     children but not the parent's halos, having no `movement`. Take the entry
     out when it is.
   - Done: **Battlefield** has nothing unplayed. Its `phase` has an
     `animation` with options and a `previewvalue` and no keyframes at all, so
     the value is the 0 written beside it; the pulse it offsets (`effects/pulse`,
     `sin(time x speed + phase)`) runs, and the logo swings with the same 6.3 s
     period as the recording, two or three levels brighter.
   - Then the Settings tab and **Wallpaper Engine's quality presets**, globally and
     for one wallpaper -- Low/Medium/High/Ultra: frame rate, MSAA,
     post-processing, reflections, texture resolution. The user's word: very
     important, since it reaches into what the GPU is asked for.

   **Not to be worked on, by the user's word on 6 November -- fine to their eye:**
   Dark Angels, Gears of War (its "bubble" included), SCP Terminal Lockdown (its
   `chromatic_aberration` too), Tomb Raider's occasional pink haze, Terminator
   Factory's slightly brighter sky, Ender Spartan.

6. **Next, as the user left them on 4 November** (0.23.2 released: a scene's
   particle `count`, the perspective field of view and the instance colour as
   Wallpaper Engine has them, a step hearing its loudest band and a trail's cap.
   Confirmed by the user that day: Snow Lights, Augmentation, Gears, Dark Angels,
   and DOOM unchanged to the eye after the loudest band. **Space Visualizer
   "looks very nice" and is to be brought into line with Windows next** -- see
   its entry below):
   - **Every other scene that follows the music, afresh.** The evening out of the
     music changed all of them, and only Dark Core and DOOM have been checked
     against a recording with the system's sound in it since. Time Reactor was
     confirmed before the change; Augmentation, Space Visualizer and Snow Lights
     also have particles whose audio defaults changed in 0.23.1. Their recordings
     here are YouTube videos, whose sound may not be what Wallpaper Engine heard, so
     the user was asked for recordings of their own with the system's sound, a
     minute of music each. Compare as Dark Core and DOOM were: `clip` with the
     recording's front channels (`pan=stereo|c0=FL|c1=FR` -- `-ac 2` loses 7 dB),
     drawn at the screen's 2560x1440 and scaled down as the recorder did, regions
     measured over time against the recording.
   - **4 November: the user's recordings arrived** (`wpe-windows/*_2026.11.04-*`,
     1280x720, sound unaltered; Dark Angels and Gears are silent, Romulus,
     Augmentation and Space Visualizer quiet at peaks of 0.07). Side by side with
     `clip` in `~/WallFlux-ukazky/2026-11-04/`. Fixed: Gears' ripple (`count`),
     Snow Lights' near flakes (`perspectiveoverridefov`). Close by eye and waiting
     for the user's: Augmentation (Windows' rainbow rings a little more vivid),
     Space Visualizer, Snow Lights (the deer and the night come at the same
     moments), Dark Angels (our flame whiter at its core, Windows' more orange).
     `clip` now pipes frames into ffmpeg, so a minute at 2560x1440 needs no space.
     Later that day: Dark Angels' flame fixed by the instance colour (see the trap);
     the user confirmed Snow Lights, Augmentation, Gears and Dark Angels.
   - **Space Visualizer answers the music too weakly.** Against the user's
     recording, regions of the sky over the planet swing 2.7 and 3.6 levels (std)
     on Windows and 1.0 and 1.3 here; what Windows adds when it is loud is fog over
     the small planet, stars all over the sky and rays round the sun (the shine on
     composition layer 5767 picking up a sun the background's audio `pulse` makes
     brighter). The planet's surface follows Windows at 0.8 to 0.97. **Half of it
     was how a range is heard**: the loudest band, as Wallpaper Engine reads it
     (see the trap), moved the sky halfway (std 1.9, 2.5), and with it the lava
     sparks fired often enough to show their trails uncapped -- fixed too. Still
     short of Windows: fewer red streaks across the sky; the lava's fans of
     trails come out near white where Windows has them dark red (`REFRACT`, an
     overbright of 6, over a bright planet); the background's `pulse` is the
     stock shader averaging for itself and is weak as well, so the bands' values
     after the evening out are still suspect. DOOM's letters pulsing about 15 %
     weaker may be the same thing.

     **5 November, on the real machine, each group of systems drawn alone**
     (`~/WallFlux-ukazky/2026-11-05-real/sv/izolace/`, a hard-linked copy of the
     unpacked package with `keep.py`, max over 40 s): the long thick red streaks
     rising across our sky are the four `Stars` (`starfield`, `starfield2`); on
     Windows the sky has many short thin ones at about ten degrees instead. The
     lava's columns and rays were the spark trails' gravity, fixed (see the
     world-space trap). **The swarm of red dots round the sun on Windows is
     missing here**, and it is not the hearing: the five `particle_0x_audio_response`
     systems hear 0.3 to 0.99 through that recording and let out 120 000 at
     once unthinned, but every one is born on a ring of 256 (`distancemin` 256,
     the default `distancemax` 256) times the object's 1.4 and stopped by a drag
     of 6 before it gets anywhere -- a thin red ring round the small planet,
     whichever operator is switched off. Where Windows takes them from there is
     not found; candidates are `distancemax` when only `distancemin` is stated,
     and the turn of `starfield2`, whose band of dots through the sun may be the
     swarm.

     **Later that night, by motion maps** (the brightest of five seconds less
     their median, `~/WallFlux-ukazky/2026-11-05-real/sv/obloha/`): the swarm
     *is* those five systems. On Windows they leave their ring over the small
     planet and stream at 45 degrees -- their gravity's "100 100" -- towards
     the sun, at about 30 to 80 scene pixels a second, measured by phase
     correlation; here they do the same, faintly. Unthinned the plume and the
     swirl over the planet come close to Windows', so what is left of it is
     the particle budget, which is the whole library's to change. **The
     `Stars` streaks** rise at a median of 33 degrees here and 16 on Windows:
     their vortex (50 inside, 100 outside, bands 5 to 9) is measured from its
     middle while they fly thousands along its axis, so it pushes them at its
     outer speed all the way. (Superseded on 6 November: Wallpaper Engine measures
     from the middle and weakens the push with the angle, see the vortex trap.)
     Measured from the axis instead: 26, Augmentation
     the same to the byte, DOOM and Snow Lights moved by less than a hundredth
     of a level against their recordings. Cutting a vortex off past its outer
     distance gave 17 but would stop Augmentation's swirl, which states
     `distanceinner` 0, `distanceouter` 32 and `speedouter` 8000 and was
     checked against its video.
   - **Alien Romulus' specks round the mouse** (object 162, left out): on Windows a
     sparse ring of dots 2-3 px across at 1280, falling from the pointer; here, with
     it switched on and the mouse still, a dense black ball about 35 px. The system:
     emitter on control point 0 (the mouse), `mapsequencearoundcontrolpoint` 4 x
     0.8, `controlpointattract` -600 within 50 and +500 within 5000, an audio
     `vortex`, `spritetrail`, and a texture that is an 8 x 8 cluster of square
     blocks in 1095 of its 2048. Where it differs is not found yet.

     **5 November, with the mouse moved as in the recording** (`clip` takes
     `WALLFLUX_POINTER_PATH`, the cursor tracked out of the video,
     `~/WallFlux-ukazky/2026-11-05-real/romulus/`): the trail follows the
     cursor in the same places at the same moments, and there are as many
     specks a frame (28 here, 24 on Windows), but ours cover 5.6 times the
     dark area -- each speck two to two and a half times as large, and with
     the mouse still, 352 of them make a ball 65 pixels across. The texture is
     one pixel-art speck of 8 by 8 filling the 1095 image, and a particle's
     corners read the whole 2048, as the stock shader does; the sizes come
     out as the file says (sizerandom halved, the instance's 0.51, sizechange
     up to 1.8). Where Windows' factor of two comes from is not found, so 162
     stays left out.
   - Done and confirmed on 3 November: Dark Core as on Windows -- the triangle, the
     lines and bars, STANDBY and the tubes, colours and fill -- and DOOM, drawn at
     2560x1440 against its recording with sound: the second O's white glow comes
     in the same seconds as on Windows and about as often (2.5 % of frames against
     3.0), the Os pulse with Windows at 0.87. Left as it is, by the user's eye: the
     letters a level or two darker and their pulsing about 15 % weaker, the D and
     the M following Windows at only 0.5.

     **5 November, on the real machine, the first minute of `Doom_2026.11.03`**
     (`~/WallFlux-ukazky/2026-11-05-real/doom/`, letters as boxes at 640x360):
     D -3.4 levels, O -2.7 and -2.3, M -3.5; swing 0.83 to 0.86 of Windows';
     followed at 0.56, 0.77, 0.79, 0.58. **It is not how loud the bands are**:
     the letters' pulse is DOOM's own `effects/pulse` (bands averaged over both
     ears, `smoothstep(0.5, 1.5)`, so a band has to pass one to do half), and
     the sixteen handed to shaders times 1.5 put every letter 8 to 28 levels
     over Windows with a swing of 2.4 -- while the D and the M followed worse.
     **It is the bass.** Over that minute our band 0 sits at 0.79 on average and
     hardly moves (the evening out holds it at its own envelope through DOOM's
     rumble), and Windows' D and M follow our band 1 best (0.47, 0.53) and
     band 0 hardly at all (0.19); the Os follow 13 and 14 best where they read
     8 to 14. So the lowest entries are where ours and Wallpaper Engine's part:
     bins 1 to 4 of the 1920 (23 to 92 Hz) read off our 2048, or how the meter
     treats them in Wallpaper Engine, which was not re-read. Nothing changed.

     **Re-read the same night, and it is not the meter's reading either.**
     Wallpaper Engine's meter: bins 1 to N-1, the place `pow((bin-1)/(N-1),
     PLACING) x 64`, never more than one ahead, the weight `TILT -
     cos(pi (bin-1)/(N-1)) (1 - TILT)`, the root of weight times power, the
     loudest in each entry, then times the volume, 0.001 and N over half the
     window -- all as `audio.rs` has it. The evening out likewise: sixteen groups of eight, the loudest of all times 0.333 as a
     floor, envelopes put back to one after silence. Tried and worse: the
     meter's input three and ten times louder before the evening out (D
     followed at 0.23 and 0.09), and the bins read as at 48 kHz, 25 Hz apart
     (0.54). So the D's and the M's 0.56 is still unexplained.

     **6 November, plotted against time** (Windows' letters over our bands,
     `Doom_2026.11.03`): from 13 s the music is a sub-bass wub at 34 to 37 Hz,
     amplitude 0.06 to 0.13, in plateaus of 1.5 s every 2.1 s. Our raw band 0
     reads 0.6 through each plateau and the evening out lifts it to about one;
     **Windows' D and M ignore the plateaus altogether** and light only on the
     kicks at 21.5, 24.4 and 29.3 s, which show in our band 1. So in Wallpaper
     Engine the wub reads far below whatever the evening out divides by. And
     **Windows' output device was 5.1**: the recording carries six channels,
     FL and FR with the music, BL and BR a tenth of it, FC and LFE silent.
     Wallpaper Engine's meter takes channel 0 and channel 1 of the loopback's
     own format and nothing else, so the recording's FL and FR are what it heard. The user's Linux
     is 5.1 as well and stays so: measured with a tone at -50 dB from `pacat`
     and from `pw-play`, PipeWire upmixes nothing into the 5.1 sink here, and
     the meter's two channels off its monitor are FL and FR to the sample.
     Only a source that is itself 5.1 would differ -- downmixed by us, read
     front-only by Wallpaper Engine. Not changed.

     **Then the letters were drawn at the wrong rate.** `clip` drew thirty
     frames a second, and the evening out is stepped by the frame's time: at
     fifteen it does not ease at all (twenty a second times a fifteenth is over
     one). The same DOOM minute at fifteen, as the desktop has drawn since
     6 November: D follows Windows at 0.62 (was 0.56), M 0.64 (0.58), the Os
     0.78 and 0.75, levels 1 to 2 under Windows' where they were 3.5, swing
     0.89 to 0.98. `clip` draws fifteen now. Simulated on the meter's real 64
     entries (`examples/meter.rs` prints every reading of a sound file), no
     other band range, entry offset, floor or envelope speed beats the file's
     own bands 0 and 1 by more than a hundredth; our own letters follow our own
     bands at 0.70, which is about the ceiling the other effects on the letters
     leave. So what is left of the D and the M is not in the meter.
     **Superseded on 9 November: it was the meter's rate**, see the trap about
     the sound read as Wallpaper Engine reads it. The "ceiling" was wrong too:
     ours against ours with the time-driven noise in another phase follows at
     0.84 for the D and 0.86 for the M.
   - **The volume of a scene's sounds**: Wallpaper Engine sets `volume^2` times the
     scene's, ours plays the square root measured off recordings. The user finds the
     volume right as it is; left for later, by the user's word.
   - Done and confirmed on 2 November: Romulus' specks left out, Time Reactor's and
     Dark Core's smoke from nought, Wallpaper Engine's own wallpapers listed and
     removed as it does, Remove after a late Steam, music on a random first track.

   Older, still open:
   - Dark Angels and Gears of War: closed by the user on 6 November.
   - **Star Destroyer's** clouds and **Tomb Raider's** torch measured over time against
     their recordings: 80/21/12 against 75/21/12, and a white core in red with no yellow
     in either. Left alone, though the user remembered orange and yellow.
   - **Blue Dawn:** the video has a strong sun and a small flare under it, here a
     weaker sun and a large one, with the mouse in the same place.

### Small things seen and left

- Against the Windows recordings on 31 October, by eye, 39 scenes and 5 web
  pages (Mourning for Icarus, Blue Dawn, Dark Core, Diablo 4 and StarCraft: Shakuras
  have no recording). Sheets in `~/WallFlux-ukazky/2026-10-31/<id>/`. Left: Tomb Raider has some pink haze at
  her feet at some moments, from the torch's red smoke; Terminator Factory's sky is
  a little brighter.

- **Gears of War's spin and jaw are right; looked at by the user and measured.** Gear
  turn: 17 degrees a second in the recording, in `shot` and in the program at
  2560x1440 (11.3 and 11.5 for the small one). Dark Angels carries no sound at all,
  neither an object nor a file in 120 entries; its YouTube video does.

- What a scene's scripts do not reach yet: a particle override's `size`,
  `lifetime` and `count`; the camera's `camerashakeroughness`, which is not
  drawn at all; the last effect of a chain switched off, which draws into the
  scene and has nothing to hand on to (reported); an object hidden in
  scene.json that a script would show -- it is never loaded -- and the
  children of a particle system whose parent a script moves, which were
  placed once from where the parent started. An effect switched off by a
  script (16 November) is copied over (`Step::off`); a system's `speed` is
  set again from what it was before the override (`Unsped`); the shake's
  waves run on the speed added up, so a speed moved by the music quickens
  them rather than jumping.

- A saved playlist is renamed in Load (10 November): Rename turns the picked
  row into a field, Enter takes the name and Escape gives the row back. A
  name another playlist has is refused, since that would throw it away
  unasked; a running playlist goes on under its new name.
- Wallpaper Engine's Load and Save dialogs each carry a keyboard shortcut
  field. There is nothing here that could listen for one, so neither is
  drawn, and keyboard shortcuts are not going to be taken on.
- `Clear selection` empties what is ticked and nothing else. Checked by
  counting the chips of every section before picking, after picking and
  after clearing: ten genres, one tag, four picture sizes, unchanged
  throughout. The lists come from the whole library and from `Tags`, never
  from what is left after filtering, so there is nothing there to lose.
- The volume for a video went with the panel that used to be drawn over a
  preview: the only sound control now is the one in the footer, which is on
  or off. Nobody has asked for it back.
- `SteamAppId=431960` in the environment makes the window's resource class
  `steam_app_431960`, which is confusing when reading `workspace.windowList()`.
  Harmless; worth knowing before chasing it.
