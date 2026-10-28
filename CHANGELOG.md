# Changelog

Everything worth knowing about each release of WallFlux. The shape of this
file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.0] — 2026-10-28

### Added

- **A scene's own music plays.** Nineteen of the forty-one scenes carry
  theirs inside `scene.pkg`; it is unpacked into the cache and played the
  way the scene asks -- round and round, shuffled with a pause between
  tracks, or once -- quiet when anything else is playing, paused under a
  covering window, and once for the program rather than once per screen.
- **A scene that follows the music hears it.** Seven of the forty scenes
  answer to what the speakers are playing -- visualisers, and particles
  whose rate or motion follows the beat. The renderer listens to the
  default output itself, once for every screen, and only while such a
  scene is on show: Plasma puts its microphone icon in the tray for as
  long as anything listens to the speakers.
- **Particle systems bring their children**, running alongside, bursting
  where a parent's particle is born or dies, or riding along with one.
- **A scene's settings for each particle system it places are read** --
  alpha, size, lifetime, rate, speed, count and colour. Unread, every
  smoke kept its preset's colours, which is where the pastel blobs in half
  the library came from.

### Fixed in the renderer

- One effect that cannot be built no longer takes the rest of the object's
  effects with it; Arkham Knight's blur and godrays and the motion blur in
  eight more wallpapers are back.
- A layer can read what another layer finished as, and blends placed in
  the scene's own pixels land where they were meant to instead of at
  infinity.

## [0.12.0] — 2026-10-27

### Added

- **Scene wallpapers play.** Forty-one of the fifty-three wallpapers on the
  development machine are Wallpaper Engine's own packed scene format, and
  until now the picker listed them dimmed. They are drawn by WallFlux's own
  renderer -- Rust and OpenGL, under `crates/`, linked into the program --
  on the same surface as web and video wallpapers, so transitions, the
  pause under a covering window and every ticked screen work the same.
  Forty of the forty-one draw. The one that does not, Diablo IV Lilith, is
  a single 3D model, and the program says so rather than showing black.
- **What a scene is made of:** images, sprite sheets, particles with their
  emitters, operators, ribbons and trails, effects chained pass after
  pass, the composition and post-processing layers that put an effect over
  everything drawn before them, and the values a wallpaper's own
  `project.json` gives its settings -- which differ from the copy in the
  scene for 205 of 365 of them.
- **The renderer knows where the mouse is**, for the particles that clear
  around the pointer and the scenes whose layers move with it. Nothing in
  the program hands it the mouse yet; see below.

### Fixed in the renderer, found while bringing it back

- Effects whose second texture is something the renderer cannot make were
  drawn against white instead of left out, which whitened eight
  wallpapers -- Kratos's pauldrons and the Rainbow Six shields among them.
- Every object with effects now has buffers of its own size, as Wallpaper
  Engine gives every layer. Shared, screen-sized buffers put any effect
  that names a place in the picture somewhere else, and cut the top off
  some objects.
- Shaders that declare a texture below the include that samples it, and
  vertex shaders half a component narrower than their fragment shader,
  both compile now -- Direct3D takes either and OpenGL takes neither.
- A wallpaper whose settings are animated loads, instead of failing over
  the curve.

### Not yet

- **Scenes are silent and blind to the mouse.** The renderer answers to the
  music and the pointer, and nothing in the program hands it either yet.
- Lighting and reflection are drawn flat. One wallpaper in the library
  asks for them.

### Changed

- **Building needs `cargo`**, which compiles the renderer into the binary.
  Running needs nothing new.
- **The renderer is under the project's licence**, GPL-3.0-or-later with
  the Steamworks linking exception. Its `Cargo.toml` said MIT, left over
  from when it lived on a branch of its own.

## [0.11.0] — 2026-10-23

### Added

- **A wallpaper on every screen that is ticked for it.** The Displays tab
  draws the monitors as they are arranged -- read off their positions on
  the shared desktop, so it is the arrangement somebody set up in their own
  display settings -- and gives each a switch. One wallpaper, drawn on the
  ticked ones. The screens are Qt's own: `Qt.application.screens` hands
  over the connector name natively on Wayland, which is the same string
  KDE's display settings and `/sys/class/drm` use.
- **A filter in the library built from the tags the workshop really
  carries.** project.json holds the genres its author ticked and nothing
  else -- ten distinct tags across this machine against twenty-nine the
  workshop knows of the same wallpapers. One keyless POST asks about the
  whole library at once. Audio responsive ticked now finds ten wallpapers
  where it found none, and Mature finds one, where project.json calls
  every wallpaper Everyone.
- **A filter panel that folds.** Sixty-four chips is longer than any
  window, so each section's heading is a switch: the arrow turns, the
  chips slide out from under it, and a number beside the heading says how
  many are ticked while they are away.
- **A grid the arrow keys walk.** Arrows move the band round a card, Enter
  puts that one on the desktop, Space ticks it into the playlist.
- **The version, in the corner of the footer.** An `i` beside Turn off,
  which says it when it is hovered.

### Changed

- **A card in the grid is a square with its name inside the picture**, as
  Wallpaper Engine's own picker draws it, and the window opens at
  1400 x 900 with five of them across. It opened at 1100 x 720 before,
  where the grid came out 419 pixels tall and showed six wallpapers out of
  fifty-three. The details panel keeps the whole sixteen-by-nine preview:
  it is the one place a preview is looked at whole.
- **The panel is opened and closed by the card it is about**, so the cross
  has gone off the picture. It slides rather than appearing, with its
  contents held at their full width and cut off by the edge so nothing
  inside is squashed on the way. Leaving a tab shuts it, in either
  direction.
- **In the Steam tab the click is the only thing there is.** It opens the
  panel and closes it again, and a double click does nothing whatever,
  because nothing in that tab is ours to play.
- **The window is tinted by whatever the panel is showing**: the preview
  blurred past recognition, half the colour out of it, faded in over six
  tenths of a second. A card's band lights up under the pointer, and the
  preview leans in a twentieth.
- **The portrait picture sizes are back.** They were dropped as telephone
  sizes, which was wrong twice over: a monitor can be turned on its side,
  and whoever has turned one had no chip to tick.
- **A workshop wallpaper under half a megabyte no longer reads "0 MB".**
  Tenths below ten megabytes, and never nought.

### Fixed

- **The grid stays where it was put.** Its model is a fresh list every
  time anything about any wallpaper changes, and a GridView taking a new
  model throws away both the row its ring is on and where the view is
  scrolled -- so only the first card could be picked, the arrows went
  nowhere, and a double click started the wallpaper and threw the window
  up to the top of the library.
- **The grid no longer refreshes in front of you when the window is
  resized.** The repair above rewound the view every time, which destroys
  every card on screen and loads the previews again; it is asked for only
  when the view is really stranded past the end of the grid.
- **Playing a wallpaper no longer shuts the panel.** Qt delivers the
  second press of a double click as `doubleClicked`, which never passes
  the counting, so four tenths of a second after the wallpaper started the
  panel did what the first click had decided.
- **The playlists menu came up empty.** A Menu has no width of its own --
  it comes from the background it is given, and the plain rectangle it was
  handed to round its corners has none. It opened twelve pixels wide over
  two saved playlists that were plainly there.
- **Switching language emptied both drop downs in Playlist settings.**
  Translating a model is a binding, so rebuilding the list dropped the box
  back to its first row, and a playlist saved afterwards was saved in
  order with no transition and nobody told.
- **Fade to black never reached black.** The 400 ms rescue that changes
  the wallpaper behind a covering window was pre-empting a transition that
  deliberately takes half its length to get the veil up. Measured at
  rgb 0.46 where it should have been 0.

## [0.10.0] — 2026-10-22

### Added

- **Three of the playlist switches do what they say.** *Change wallpaper
  when the video ends* turns on the clip rather than on the clock, and
  holds the clock while it waits. *Allow changing while paused* decides
  whether the clock keeps counting while a window covers the wallpaper,
  and it picks up where it left off rather than starting the turn again.
  *Always begin with the first wallpaper* holds the first one out of the
  shuffle. Each behaves as Wallpaper Engine's own does, read off its
  guides rather than guessed at.
- **Wallpapers that fade into one another.** A playlist can cross from one
  wallpaper to the next, go out through black, or hold the last frame over
  the gap while the next one finds its first, over nought to three
  seconds. There is no second player to keep the old wallpaper in, so what
  is held is a picture of it, grabbed the instant before the switch.
- **A playlist behind a covering window still changes.** The grab a
  transition needs never comes back while the window is not being drawn,
  so there is a rescue that changes the wallpaper without one.

### Changed

- **There is no "change wallpaper" list any more.** It offered a timer,
  login, time of day, day of week and never; four of those were never
  going to be written. A playlist turns on its clock, and on a video
  ending where it was told to.
- **Nothing in Playlist settings is a placeholder.** "The first wallpaper
  plays only at startup" was the last one and it has gone: WallFlux is not
  started with the session in a way that would give it a meaning.
- **The five-second floor is in the boxes rather than in small print under
  them.** Type three noughts and they read five seconds, which is what the
  playlist will actually wait. The line explaining it is gone.
- **Playlist settings reads in three pieces** — what changes and how
  often, then the switches, then the two buttons — with room between them,
  the words centred over the three number boxes, and rounder corners on
  every popup.

### Fixed

- **A number box reading two digits could not be typed over.** Two
  characters is the whole field, so it was full and the next keystroke was
  refused. The boxes pick their whole number out when they are reached.
- **Playlist settings had its labels and its controls in the wrong
  columns.** Taking the "change wallpaper" drop down out left its label
  behind, and one odd cell in a two-column grid shifted every row under
  it.

## [0.9.0] — 2026-10-22

### Added

- **A playlist turns by the second.** The interval is hours, minutes and
  seconds rather than hours and minutes, and five seconds is the shortest
  it will take. Old playlists are read as they stand -- no seconds means
  none.
- **Opening a saved playlist starts it.** Confirm in Load fills the strip,
  brings the settings back and puts the first wallpaper on the desktop,
  rather than leaving somebody to find the Playlists button afterwards.
- **Transition time**, a slider from nought to three seconds, beside the
  transition it belongs to. Like the transition itself, it keeps what it
  is set to and nothing reads it yet.
- **Throwing a playlist away asks first.** Delete turns into "Really
  delete?" and only the second press does it. Forgetting a playlist cannot
  be undone and there is nowhere to undo it from.

### Changed

- **Configure is laid out as Wallpaper Engine's own.** Every drop down
  opens with nothing chosen, the three numbers are typed rather than
  clicked at -- the arrows are gone and each box takes two characters --
  and the tick boxes are out at the right, big, and fill in blue when they
  are on. Cancel sits beside Confirm.
- **The Playlists button keeps its name.** It used to turn into the name
  of whatever was playing; the tick in its menu says that now, as the sort
  menu does, and the count in brackets is gone.
- **Configure is switched off while there is nothing to configure** --
  no playlist on the strip and none saved.
- **The playlist strip says how many wallpapers are in it** and nothing
  else; the line of advice beside it has gone.

### Fixed

- **Escape hid the whole window instead of closing a popup.** A
  window-wide shortcut took the key before any popup could use it. There
  is no Escape handler in WallFlux now: the key belongs to whatever is
  open, and the window is put away with the buttons in its corner.
- **Escape then closed only one popup of the three.** `CloseOnEscape` is
  not enough on its own, because a popup does not take the keyboard by
  being opened -- Save worked only because it puts the cursor in its name
  field. Measured on all three and fixed for all three.
- **Typing a time could not overflow sensibly.** Sixty seconds is a minute
  now, ninety-five minutes is an hour and thirty-five, and longer than a
  day is clamped as a whole. Clamping the hours on their own let an
  interval get *shorter* when somebody added to it: 23:59 with ninety
  seconds typed in came out as 23:00:30.
- **The number boxes took as many digits as anybody cared to type.** Two,
  now.
- **The line under a greyed-out tick box stayed black.** A Label does not
  dim itself when it is switched off.
- **The Load list and the details panel drew a scrollbar beside content
  that fitted.**

## [0.8.0] — 2026-10-22

### Added

- **The playlist strip is in the window from the start**, empty or not --
  a place to put something has to be visible before there is anything in
  it. Along its top: Playlists, then Load, Save, Configure and Add
  wallpaper, laid out as Wallpaper Engine lays them out. Add wallpaper
  holds every tick box open so several cards can be ticked without
  hunting for the box on each one.
- **A saved playlist can be opened back onto the strip, and thrown away.**
  Load is a list of what has been saved; opening one brings its wallpapers
  and its settings back so it can be changed and saved again, and the bin
  beside it is the only way a playlist has ever had of being deleted.
- **Configure says what WallFlux is going to offer**: when the wallpaper
  changes (timer, login, time of day, day of week, never), the order, the
  transition, changing when a video ends, changing while paused and always
  beginning with the first one. Four of those choose between things that
  are not written yet and the popup says so; the list was the part worth
  deciding first.
- **Who made a wallpaper**, in both tabs: their avatar, their name, and a
  magnifier that turns the workshop grid into everything else they have
  published, with a chip beside the filters to come back from. Nothing
  here needs a Steam Web API key. project.json records no author, so for
  an installed wallpaper it takes two keyless questions rather than one.
- **The grid takes the keyboard.** Arrows walk the cards, Enter puts one
  on the desktop, Space ticks it into a playlist. A click moves the arrows
  there too, so the mouse and the keyboard do not each keep a place of
  their own.

### Changed

- **The details panel reads as a list rather than a column of headings**:
  an icon and a name on the left, the value on the right, a hairline under
  each, and the tags as chips under their own heading. The cross moved
  into the corner of the picture. The panel is wider, the picture and the
  name stay put, and everything under them scrolls -- with a scrollbar
  that is there only when there is something to scroll.
- **Previews are three hundred across** rather than two hundred and forty,
  in the library and in the workshop both.
- **The previews in the playlist strip are half the size they were**,
  twenty to a row, and the room for them comes with the first ticked
  wallpaper and goes with the last.
- **The tick box on a card is white with a black edge, and blue when it is
  ticked.** Fixed colours rather than the window's: the box sits on a
  picture, and against a dark preview the palette's own colours left it
  all but invisible.

### Fixed

- **An installed wallpaper showed a third of its tags.** project.json
  carries only the genres its author ticked -- measured, one wallpaper
  reads `Sci-Fi` on the disk and six tags in the workshop, which adds the
  asset type, the kind, the resolution, the age rating and whether it
  answers to sound. The workshop's list is read instead, and the Audio
  responsive row reads it too rather than contradicting the chips under it.
- **Searching for an author's work could not be undone.** Pressing
  Workshop left the last author's wallpapers in the grid: clearing the
  author alone changes nothing, because the grid is already in the
  Workshop's own mode and nothing asks again. It asks again now.
- **`AsNeeded` on a scrollbar did nothing.** Fusion fades the bar it draws
  itself, and WallFlux's scrollbar replaces that -- so the policy was set,
  the fading had gone with the thing that did it, and a full-length bar was
  drawn down the edge of content that fitted.
- **The arrow-key highlight was a second frame round the card**, and it
  took the name in with it. The band around the preview turns blue instead.

## [0.7.0] — 2026-10-21

### Added

- **Wallpapers that take turns.** Hovering a card in the library shows a
  tick box; the first tick opens a strip along the bottom with the chosen
  wallpapers in it, ten to a row, and under them Configure and Save. A
  saved playlist is started from the Playlists button beside the filters
  and stopped by clicking it again. Configure holds the name, how often it
  turns -- hours and minutes -- and whether it shuffles. Measured end to
  end: three wallpapers, a minute each, turning on the minute.
- **The first thing WallFlux remembers between runs.** Playlists are
  written to `playlists.json` under the configuration directory, so they
  are there at the next start. Nothing is *started* at the next start:
  the program still comes up as a tray icon and leaves the desktop as it
  found it.

### Changed

- **The Playlists tab is gone.** A playlist is made out of the grid of
  installed wallpapers, so it is made in that grid rather than in a tab of
  its own -- which is how Wallpaper Engine has it.

### Fixed

- **A playlist set to a minute was saved as a quarter of an hour.** The
  number in the box never reached what was saved: a Controls item bound to
  a property and also setting that property keeps its value to itself.
  Measured -- the box read 1 and the property was still 15. The box holds
  the number now and the window reads it off when it saves, and both
  numbers can be typed as well as clicked.
- **The tick box on a card no longer flickers.** It appeared because the
  pointer was on the card and then took the hover for itself, which hid
  it, which gave the hover back -- several times a second.

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

[Unreleased]: https://github.com/invisigoth-cz/WallFlux/compare/v0.13.0...HEAD
[0.13.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.2...v0.5.0
[0.3.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/invisigoth-cz/WallFlux/releases/tag/v0.1.0
