# Changelog

Everything worth knowing about each release of WallFlux. The shape of this
file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.28.0] — 2026-11-09

### Changed

- **Settings, reworked** into General, Audio, Performance / Display and
  Appearance, laid out as a desktop's settings are now: a search across
  every section (accents and case ignored), the sections down the left with
  icons, the section's name large over one card to a setting, switches as
  pills and a round volume handle, and a section sliding in when opened.
- **General**: the language, starting at login, the tray icon, starting
  minimised and minimising on close (off, closing the window quits), and a
  reset. With the tray icon off, starting WallFlux again from the menu
  shows the window of the copy already running.
- **Audio**: the volume from 0 to 100 %, mute (the same as the status bar's
  button), and the two times the wallpaper goes quiet by itself -- while
  another program plays, and under a maximised window -- now switches, on
  by default.
- **Performance / Display**: default scaling -- fill, fit with bars, or
  stretch -- for scenes and videos, beside the frame rate, the music, the
  mouse, parallax and particles.
- **Appearance**: six themes -- Light, Dark, System, Steam, Hard Light and
  Hard Dark, the last two high contrast -- chosen from tiles that show
  each, with the colours easing from one to the next; an accent colour; the
  tint by the wallpaper; and the status bar, which can be hidden.

## [0.27.0] — 2026-11-09

### Added

- **A Settings tab**, in sections down its left. *Appearance*: dark theme,
  the window tinted by the wallpaper. *System*: frame rate (Wallpaper
  Engine's presets, 10, 15, 25 and 30 a second), following the music, the
  mouse, parallax and particles. *General*: language, the wallpaper's sound
  and volume, starting at login, and a reset to the defaults. With the
  music off nothing listens to the speakers, so Plasma's microphone icon
  stays away; with particles off a scene costs a good deal less.
- **The settings are remembered between runs**, the ticked screens with
  them, in `~/.config/WallFlux/wallflux.conf`. The wallpaper itself still
  does not play by itself at startup.

### Fixed

- **Blue Dawn's lens flare sits on its sun**, as on Windows, where it was a
  white blob under it. Its offset is measured off a recording; why
  Wallpaper Engine draws it there is not known yet.
- **Control points a scene moves are read**, and points given in the
  scene's own coordinates are placed there: Augmentation's lightning
  strikes where it does on Windows.

## [0.26.0] — 2026-11-09

### Changed

- **Particles move as Wallpaper Engine moves them at fifteen frames a
  second.** Read out of `wallpaper64.exe`: at 20 frames a second or fewer it
  runs each frame's particle step twice at half the time, and drag,
  turbulence, vortices and pulls are felt over less time than gravity and
  the move -- about half at the desktop's fifteen. Gears of War and Star wars
  come out nearer their recordings; Space Visualizer's swarm streams wide
  across the small planet instead of curling round its edge.
- **The sound is read as Wallpaper Engine reads it: at 48 kHz, with its own
  1920-sample transform.** DOOM's D and M no longer light up through the deep
  35 Hz drone, only on the kicks, as on Windows (they follow the recording at
  0.75 where they did at 0.64). Every wallpaper that answers to music hears
  it this way now.

### Fixed

- **Space Visualizer's lava sparks are hair-thin red lines**, as on Windows,
  where they were thick bright plumes. A particle system spawned by another
  now takes the whole of the scene's override for it, size and lifetime
  included, and every burst gets a pool of its own. Star Citizen and
  Terminator Resistance come out nearer their recordings with it.

## [0.25.3] — 2026-11-06

### Fixed

- **Wallpapers make their sound again.** A program that keeps a sound stream
  open without playing anything -- speech-dispatcher does, for as long as it
  runs -- was taken for another program using the speakers, so every
  wallpaper stayed quiet. What counts now is whether PipeWire is actually
  running the stream, read with `pw-dump`, which is why the package depends
  on `pipewire`.

## [0.25.2] — 2026-11-06

### Fixed

- **Alien Romulus' specks round the mouse are back, and look as on Windows**:
  small sparse dots, where they used to be splats several times too large and
  were left out for it. A placed particle system's size and speed now reach
  every step Wallpaper Engine binds them to (read off `wallpaper64.exe`): the
  size also its growth and shrinking, the speed also its pulls, swirls,
  turbulence and sway.
- **A pull towards a control point works as Wallpaper Engine's does**: full
  strength at the point, none at its stated reach and a straight line between,
  never overshooting, with the engine's defaults. The mouse pushes particles a
  little differently in wallpapers that react to it.
- Space Visualizer's swarm streams wider towards the sun; Gears of War and
  Dark Angels come out nearer their recordings.

## [0.25.1] — 2026-11-06

### Fixed

- **A particle vortex pushes as Wallpaper Engine's does**, read off
  `wallpaper64.exe`: a particle's distance is from the vortex's middle (from
  its axis only where the file's `flags` say so), the axis is normalised, and
  the push weakens with the angle between the particle and the axis. Space
  Visualizer's red star streaks cross the sky flatter, nearer Windows'. Snow
  Lights, Augmentation and DOOM come out the same against their recordings.

## [0.25.0] — 2026-11-06

### Changed

- **Wallpapers are drawn fifteen times a second**, scenes and web pages alike,
  where a scene used to be drawn as fast as the screen would take it -- 144 a
  second on a 144 Hz monitor -- for a wallpaper. Fifteen is Wallpaper Engine's
  own default: its first-run dialog picks the Medium preset, and its presets
  are 10, 15, 25 and 30. Video plays at its own rate, as before.
- **No scene loses particles any more.** Scenes over 48 000 particles were
  thinned; Wallpaper Engine never thins one, and at fifteen frames a second the
  heaviest here fit whole -- Space Visualizer 30 ms a frame and DOOM 26 at
  2560x1440 with loud music, of the 66 ms a frame has. Space Visualizer,
  DOOM, Gears of War and Rainbow Six Siege are the four that change.
- **Several screens show one wallpaper worked out once**, as Wallpaper
  Engine's "Clone single wallpaper": the first ticked screen plays it and the
  others show its picture, cropped to their shape. Each screen used to run the
  scene, the video or the page all over again -- twice the CPU on two screens,
  a video's sound twice over, and the screens never in step. Measured on two
  virtual screens with Space Visualizer: 7 % of a core against 13.

### Fixed

- **A second screen can show the first one's picture at all**: the windows'
  OpenGL contexts are told to share, which QtWebEngine no longer asks for by
  itself in Qt 6.12.

## [0.24.3] — 2026-11-05

### Changed

- **Nothing in the program**: this release carries the notes of 5 November --
  Wallpaper Engine on the user's Windows draws fifteen frames a second where
  this program has no cap at all, the meter re-read against `wallpaper64.exe`
  and found the same, and what is known about Space Visualizer's sky and Alien
  Romulus' specks -- so the version number says which notes go with which
  build.

## [0.24.2] — 2026-11-05

### Fixed

- **Space Visualizer's stars cross the sky more nearly level**, rising at 26
  degrees where they rose at 33 (Windows: 16). A particle vortex measures how
  far a particle is from the axis it turns about, not from its middle, so
  stars flying along that axis are no longer pushed at its outer speed all the
  way.

## [0.24.1] — 2026-11-05

### Fixed

- **Space Visualizer's lava throws low fans of sparks**, where they climbed in
  tall columns and shot rays across the planet. A particle system in world
  space feels its gravity at its object's size, not the scene's.

## [0.24.0] — 2026-11-05

### Changed

- **Particle systems are stepped on several cores at once**, which halves what
  the heaviest scenes cost: Space Visualizer from 62 to 30 ms a frame unthinned
  at 2560x1440, DOOM from 59 to 22 at 150 000 particles. Nothing drawn changes --
  each system draws from a generator of its own, and a sweep of the whole
  library came out identical.
- **A scene may keep 48 000 particles alive instead of 24 000**, the most that
  keeps Space Visualizer and DOOM at 60 frames a second at 2560x1440 with loud
  music on a Ryzen 9 5900X and an RX 6900 XT. Space Visualizer's sky, DOOM,
  Gears of War and Rainbow Six are thinned less; Snow Lights not at all.

## [0.23.3] — 2026-11-05

### Fixed

- **Space Visualizer's lava throws fans of red sparks**, as on Windows, where it
  scattered short white dashes all leaning one way. A particle system marked
  *Worldspace* is placed by its object once, as each particle is born, and lives
  in the scene's own axes after that -- its gravity and its trails no longer
  turned and squashed with the object. And a system with two renderers draws
  its particles both ways: `sparktrails` is a streak at the head and a line from
  where each spark was born.
- **Space Visualizer's stars fly across the sky from left to right**, as its
  setting says they do, red against the red sky. They fly along z and their
  object is turned a quarter about y, and only a turn about z was being read.
  They rise a little as they go, as on Windows: a particle vortex turns the
  other way from what was read, and takes Wallpaper Engine's own distances and
  speed where the file states none.
- **Space Visualizer's sun has its band of light from the top left**, the
  stars of a second system turned about two axes, lit where they cross the sun.
- **A thinned spark trail keeps to the count its file allows**, and the two
  renderers of one system keep the same count.

## [0.23.2] — 2026-11-04

### Fixed

- **Dark Angels' muzzle flash burns yellow, not white**, and every particle
  system a scene recolours takes its colour as Wallpaper Engine gives it: moved
  in hue, saturation and value from the system's own, and in a scene from before
  version 5 multiplied as well. Checked against recordings on Windows: Dark
  Angels' flame, Ender Spartan, Star Destroyer, Tomb Raider's torch and Dark
  Core's tubes.
- **Gears of War's ripple comes back**, once every half a minute as on Windows.
  A scene's particle `count` thins how many are let out, not how many may live,
  and with the pool thinned instead the ripple had room for none.
- **Snow Lights' nearest snowflakes pass the camera as large stars**: a scene's
  own field of view for perspective particles is used, where it gives one.
- **Particles that follow the music hear the loudest of their bands**, as
  Wallpaper Engine reads them, not the average. Space Visualizer, which listens
  across a wide range, answers the music about twice as strongly; DOOM, which
  listens to two bands, moved by tenths of a level, towards its recording.
- **Trails of sparks are as long as Wallpaper Engine draws them** when their
  preset does not say: capped at ten times the spark's size. Uncapped, Space
  Visualizer's lava threw lines across the whole planet.

## [0.23.1] — 2026-11-03

### Fixed

- **Fire, smoke, torches and sparks that follow the music listen to the bass**,
  as Wallpaper Engine has them when their preset says no more: the two lowest
  bands, from four fifths of their loudness up. They listened to the whole
  spectrum. DOOM was checked against a recording of it on Windows with its sound:
  the white glow on its second O comes at the same moments and about as often.
  Space Visualizer, Snow Lights and Augmentation have such particles too.

## [0.23.0] — 2026-11-03

Every scene that follows the music reads it the way Wallpaper Engine reads it, all
the way through -- checked against a recording of Dark Core on Windows with the
system's sound in it.

### Fixed

- **The music is evened out before a scene hears it**, as Wallpaper Engine does it:
  each part of the sound is measured against how loud the music has lately been, so
  the loudest of it reads about the same whether it is played loud or quiet. Without
  that, everything that follows the music was four to nine times too weak after the
  first second. Dark Core's lines and bars under STANDBY are as long as on Windows,
  its triangle pulses as brightly, and STANDBY goes away while music plays.
- **A scene answers the music when it plays it, not a second or two later.** The
  sound was recorded in two-second pieces; DOOM's letters moved behind their own
  music.
- **Dark Core's tubes burn orange**, as its settings say and as Wallpaper Engine
  burns them, rather than blue and purple.
- **Fire, dust and embers whose preset says nothing about how many to let out let
  out ten a second**, Wallpaper Engine's own number, not five. Dark Core's lower
  tubes are full along their length, and Terminator Resistance has more embers; nine
  other wallpapers change slightly.
- **A script that asks for the average of both ears gets it in Wallpaper Engine's
  order**: the two ears averaged first, and the loudest of each group taken after.

## [0.22.1] — 2026-11-02

### Fixed

- **A scene's music starts on a track picked at random**, whatever its mode, as
  Wallpaper Engine starts it. A sound object set to go round in order, or to play
  once, always began on its first file.
- **How a scene's music plays was checked against Wallpaper Engine itself**: one
  track at a time, from its start, with the pause the wallpaper asks for between
  them. Space Marine II's soundtracks open quietly, and do so on Windows too.

## [0.22.0] — 2026-11-02

Removing a wallpaper works for every wallpaper in the library, and Wallpaper Engine's
own wallpapers are all there, as Wallpaper Engine has them.

### Added

- **All of Wallpaper Engine's own wallpapers are listed**: sixteen here, where four
  were. Wallpaper Engine shows every one it ships less those its `visibility.json`
  hides, and so does WallFlux. Each has its author, and the ones the renderer cannot
  draw yet are listed dimmed, to be read about and removed.
- **One of Wallpaper Engine's own can be removed**, the way Wallpaper Engine removes
  it: it is hidden in `visibility.json` and its files stay, since they belong to the
  install. Hidden in WallFlux, it is hidden in Wallpaper Engine too.
- **Remove asks first.** The button says "Really remove?" and only the second click
  takes the wallpaper away, as Wallpaper Engine asks before it does.

### Fixed

- **Remove stays usable when Steam starts after WallFlux**, which is what a login
  does. Steam was asked once at startup and never again, so the button stayed grey
  with Steam running; looking at a wallpaper asks again, and finding out that Steam
  is not running no longer takes over a second.

## [0.21.0] — 2026-11-02

The sound a wallpaper hears is now worked out the way Wallpaper Engine works it out,
read out of its own program, and three wallpapers the user looked at on the desktop
behave as they do on Windows.

### Sound

- **The music is read as Wallpaper Engine reads it.** Its own reading, taken out of
  `wallpaper64.exe`: 64 numbers per ear, the bass weighed down so music comes out
  level, nothing held from one reading to the next and nothing capped at one. The
  old meter held every band near the top, so a wallpaper that smooths the sound for
  itself stood still: Dark Core's equaliser bars under STANDBY hardly moved and its
  red triangle never blinked. This changes every wallpaper that answers to music,
  DOOM among them. How Wallpaper Engine goes from 64 numbers to 16 or 32 was not
  found; the loudest of each group is used.

### Fixed in the renderer

- **Time Reactor's smoke follows its alpha script**: faint in silence, full with the
  music. Unrun, it burned five times too strong and filled a third of its tubes at once.
- **Smoke a script slows starts from nothing.** Time Reactor's and Dark Core's tubes
  fill over tens of seconds in silence, as on Windows. The head start of 3.2 seconds
  fitted to a Dark Core recording is gone; that recording had started late.

### Changed

- **Alien Romulus draws no specks round the mouse.** Wallpaper Engine draws them; ours
  came out three or four times too large, and none was preferred to those. The
  wallpaper offers no setting to switch them off.

## [0.20.0] — 2026-11-01

A whole library looked at side by side with recordings from Windows, and what that
found. Flying lights stop flickering, fog rolls, and two effects that had been read
wrongly from the start now do what the wallpapers ask of them.

### Fixed in the renderer

- **Flying lights no longer flicker.** The speed of `oscillatealpha` is radians a
  second and its two scales are two ends. Read as cycles, an ember blinked twenty times
  a second down to a third of its brightness: Ender Spartan, Star Wars 1811296056,
  Terminator Resistance 4k and Warhammer 3341930145.
- **An effect's `unique` buffer is its own.** A blur on the same object wrote over the
  frame motion blur was keeping, and Snow Lights' deer came out pale and see-through.
- **Turbulence stirs in eddies**, and its clock runs in the scene's pixels. It was a new
  direction every frame, so Rainbow Six's fog shivered instead of rolling. This moves
  every wallpaper with the operator, 21 here.
- **Every turbulent start adds its push.** Only the last counted, and half of Time
  Reactor's tubes stayed empty.
- **A jet whose `right` lies along its `forward` goes straight.** Dark Angels' smoke
  runs down the barrel, and the straight edge above the muzzle is gone.
- **A vortex pushes sideways**, rather than turning a particle by degrees. Augmentation's
  eye swirls on the beat instead of drawing straight spikes.
- **`controlpointattract` pulls evenly out to half its threshold**, which changes the
  specks round the mouse in Alien Romulus and the fog round it in Rainbow Six.
- **`g_TextureReductionScale` goes with the size a buffer claims.** Dark Angels' light
  lands on its cathedral.
- **A mirrored animation that starts off rest holds its first stretch.** Alien
  Isolation's jaw first comes out forty seconds in.

### Sound

- **DOOM's letters move with the music.** The top band was lifted so far that they sat at
  the end of their travel for most of a track. Fitted to four minutes of a recording.
- **A scene's music plays at the square root of its `volume`**, which is closer to the
  recordings and louder than it was: Augmentation at 0.5 now plays at 0.71. The
  recordings do not all agree on this one.
- **A system whose rate a script drives starts ahead**, so Dark Core's fires are lit from
  the first frames rather than after forty seconds.

## [0.19.0] — 2026-11-01

Bent puppets, particles that run smoothly at any refresh rate, and a second round of
comparison with recordings from Windows. Six wallpapers that ask for more particles
than the renderer keeps stop losing the small systems to the large ones.

### New

- **Puppets.** An image bent on the bones of a `.mdl` is drawn bent. Alien Isolation's
  second jaw and its tail move.

### Fixed in the renderer

- **The largest particle systems give way first.** Every system used to lose the same
  share, so a torch of 41 flames was left with 13. Gears of War's eyes are as white as
  they are on Windows; DOOM and four others change too.
- **A texture repeats when the file says it does.** Gears of War's ripple smeared one
  column of its map into fine stripes, and it was the same for the 20 other textures in
  the library that repeat.
- **A post-processing layer is as big as the screen.** It took the size stored in the
  file, and Gears of War's blur reached twice as far as it should.
- **The transparent part of a DXT1 texture is transparent.** It came out opaque black,
  which is what turned Alien Romulus' specks at the mouse into a black disc.
- **Size changes take over from one another** and do not multiply, and a scene's count
  for a system goes for the sequence that places its particles.
- **A colour a scene binds to a particle system is the scene's own.** Dark Core's tubes
  burn blue, purple and orange, as they do on Windows.
- **Particles step by exactly the time owed.** A 75 Hz screen left every fifth frame
  with nothing new, which read as smoke that stutters.
- **The clock of a scene no longer loses four per cent at 60 Hz.**
- **A colour's strength is its largest channel**, which keeps a yellow flash yellow.

### Fixed in the program

- **A web wallpaper's files are found.** A path with its leading slash made four
  slashes in the page's address.
- **A preset plays the music of the wallpaper it is built on.**

## [0.18.0] — 2026-10-31

Most of the library now matches recordings of Wallpaper Engine on Windows, compared
wallpaper by wallpaper: 39 scenes and 5 web wallpapers. Web wallpapers are told their
settings and hear the music, presets and four of Wallpaper Engine's own wallpapers
are in the library, and lit scenes are lit.

### New

- **A web wallpaper gets its settings and the sound.** Pages were told nothing, so
  they drew their defaults whatever had been chosen. They now receive their settings
  when they load and what the speakers are playing while they ask for it.
- **Presets.** A workshop item that is another wallpaper plus a file of settings is
  in the library and plays as that wallpaper with those settings.
- **Four of the wallpapers that come with Wallpaper Engine**: Beach, Deep Space, Razer
  Bedroom and Razer Vortex. The others are 3D models or do not draw yet.
- **Scene lights.** Lights and the ambient colour are drawn, and move on the keyframes
  the scene gives them. Alien Isolation goes dark and pulses green and red.
- **Camera shake and zoom.**

### Fixed in the renderer

- **A blur reaches as far on any screen.** It was heavier the smaller the screen: God
  of War's armour melted into the fog.
- **A scene's colour for a particle system takes the hue and leaves the brightness**,
  over the system's own colour changes and down to its children. Smoke, torches and
  muzzle flashes across the library change colour with this.
- **A particle system that states no rate lets particles out.** Terminator Resistance
  had no embers at all.
- **`turbulence` accelerates a particle.** Gears of War's sparks ran together into red
  bands.
- **What is born on the mouse is born on the mouse.** Alien Romulus had a black blot
  under the alien's mouth.
- **A shader that asks where the mouse is gets told.** Space Marine II's Honor and Glory
  was missing most of the helmet.
- **A scene with a scripted or bound bloom setting loads.**

## [0.17.1] — 2026-10-30

Warhammer and Star wars now match recordings of Wallpaper Engine on Windows within
a few levels, and the smoke, fire and rays of the rest of the library come out
closer to it for the same reasons.

### Fixed in the renderer

- **A particle system's rate is how fast its time runs.** It was read as how many
  particles it lets out, so a slowed system flickered with one or two particles
  where Wallpaper Engine shows it burning steadily at its full number. Warhammer's
  muzzle flares now match a recording from Windows within a few per cent.
- **Random turns are random.** A particle system that left the turn of its
  particles unsaid drew them all at one angle: Warhammer's fan of rays was a single
  beam across the picture, and every puff of a smoke lay the same way.
- **Falling snow and everything else seen through a camera is on the screen.** Those
  systems were drawn somewhere off it, because of a matrix written the wrong way.
- **Star wars' embers fly the right way**, up across the picture rather than off the
  bottom of it, which is where its orange lights had gone.
- **A system's children keep to its strength and its speed.** A faint flare threw
  rays as bright as the brightest one's.

## [0.17.0] — 2026-10-30

A scene's clocks and calendars are on the desktop, and the wallpapers that were
missing parts of themselves on a desktop set to a language that writes a decimal
comma have them back.

### New in the renderer

- **Text.** Clocks, calendars and any other words a scene carries are drawn in
  its own font, at the size and in the place Wallpaper Engine puts them, through
  the layer's effects, and rewritten as its scripts change them. 90 of the 93
  scripts in the library now run; the other three switch an effect or drive a
  particle override other than the rate.
- **The rest of the particle vocabulary.** Flocking (`boids`), collisions with a
  plane, a sphere, a rectangle and the edges of the scene, `remapvalue` and
  `remapinitialvalue`, keeping a distance from a control point or to a line
  between two, children taking values from their parent's particles, sprite
  sheets played once and control-point lines that start again. All 289 particle
  files Wallpaper Engine ships are now read. Their defaults come from the engine
  itself; how they behave comes from its editor's descriptions, and no wallpaper
  here uses them yet to compare with. A collision with a 3D model and an emitter
  shaped like a layer's picture are not done, and are reported.
- **Compared against Wallpaper Engine on Windows**, recording by recording: fire
  and smoke that take the scene behind them, a step following the sound standing
  still in silence, sphere and box emitters, systems already running when the
  wallpaper appears, `colorchange` over a particle's own colour, emitters that
  come and go, perspective particles, `TEXB0001` textures, and more of the stock
  steps (`oscillatesize`, `hsvcolorrandom`, `colorlist`, `positionoffsetrandom`,
  `capvelocity`, `reducemovementnearcontrolpoint`, `vortex_v2`,
  `mapsequencearoundcontrolpoint`).

### Fixed in the renderer

- **Scenes with scripts lost layers on a Czech desktop**, and on any other whose
  language writes a decimal comma. The script engine read "0.5" as nought, so
  every layer at less than full size or strength disappeared: Future Clock's
  colours and scrolling code, and Time Reactor's dials. The tool used for
  checking pictures never switches language, which is why it went unseen.
- **Things turn about their origin.** A layer aligned to one of its edges turned
  about its middle instead, which left Dark Core's clock hands standing at twelve
  and started its second hand from the wrong place.

### Dependencies

- `ab_glyph` (Apache-2.0) for reading fonts, built into the renderer; nothing
  new to install.

## [0.16.2] — 2026-10-29

### Fixed in the renderer

- **Particles are their right size.** Every particle was drawn twice as
  large as Wallpaper Engine draws it, which is why smoke covered the
  figures it should drift around, Kratos sat in a haze and Arkham
  Knight's fog reached the edge of the screen. Measured against
  Wallpaper Engine on Windows: the light the smoke adds now comes out
  within a few per cent of it, where it was two thirds more.

## [0.16.1] — 2026-10-29

### Fixed in the renderer

- **Particles are drawn on the desktop.** Smoke, sparks, fog and every
  other particle system showed for one frame and were then drawn without
  the indices that make up their shapes, which left nothing to see. Qt
  puts its own OpenGL state back after each frame, and on the way it took
  the indices off the particles, which were still bound. Pictures made
  with `shot` reset nothing and were right all along, which is why this
  went unseen. The renderer now leaves nothing bound when it is done.

## [0.16.0] — 2026-10-29

### Fixed in the renderer

- **Smoke thins out as it should.** The particle operators' missing values
  are now Wallpaper Engine's own, read out of its player: a fade is a share
  of the particle's life rather than seconds, and fades out from halfway
  when the file says nothing, so smoke no longer holds its full strength
  and vanishes at once. Changes of size and alpha run down to nothing, and
  a change of colour ends at black, where nothing else is said.
- **Fading alpha over a particle's life** (`alphachange`) is drawn.
- **A scene whose file is not called scene.json plays** -- SCP Terminal
  Lockdown, whose scene is gifscene.json in gifscene.pkg.

## [0.15.0] — 2026-10-29

### Added

- **A scene's videos play.** A scene can give an image a video instead of
  a picture, and those layers used to be left out. Diablo IV Lilith, which
  is nothing but such an image and had been taken for a 3D model, now plays,
  and so every scene on the development machine draws -- 41 of 41. The
  clip is decoded by `ffmpeg`, once however many layers show it.

### Changed

- `ffmpeg` is a dependency of the package in its own right.
- The package is built without LTO, which broke the link of the scene
  scripts.

## [0.14.0] — 2026-10-29

### Fixed in the renderer

- **Scenes are composed the right way up.** A scene measures its y from
  the bottom, and WallFlux measured it from the top: every layer that was
  not centred was mirrored to the other half of the screen, and every
  particle -- rain, snow, embers -- moved upside down. A scene that is
  mostly a centred background looked almost right while it was wrong,
  which is why not one of them looked like its preview.
- **Turned objects are turned by as much as they say.** The scene stores
  its angles in radians and they were read as degrees, so 308 objects in
  28 wallpapers were turned a fifty-seventh of the way.
- **A scene is opaque.** Where layers overlapped, the picture handed on
  was partly transparent; smoke and lens flares showed whatever was behind
  the wallpaper.

### Added

- **A layer's blend mode** -- lighten, screen, add and the rest of the
  editor's list -- mixes it into what is under it, as Wallpaper Engine does,
  instead of drawing it over the top.
- **Bloom**, for the scenes that turn it on, drawn with Wallpaper Engine's
  own shaders.
- **Alignment**: a layer can hang from one of its edges rather than its
  middle.
- **A scene's JavaScript runs**, under QuickJS: what turns a clock's hands,
  swells a ring with the bass and keeps a particle system's rate on the
  beat. 82 of the 93 scripts here run; the rest write text, which is not
  drawn yet.
- **A scene follows the mouse** -- layers that lean away from it,
  particles pushed aside -- with the position asked of KWin, since the
  wallpaper's surfaces take no input and must not.
- **The details panel says whether a wallpaper has a sound of its own**,
  apart from whether it follows the music, which is what the workshop's
  "Audio responsive" tag means.

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

[Unreleased]: https://github.com/invisigoth-cz/WallFlux/compare/v0.28.0...HEAD
[0.28.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.27.0...v0.28.0
[0.27.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.26.0...v0.27.0
[0.26.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.25.3...v0.26.0
[0.25.3]: https://github.com/invisigoth-cz/WallFlux/compare/v0.25.2...v0.25.3
[0.25.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.25.1...v0.25.2
[0.25.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.25.0...v0.25.1
[0.25.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.24.3...v0.25.0
[0.24.3]: https://github.com/invisigoth-cz/WallFlux/compare/v0.24.2...v0.24.3
[0.24.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.24.1...v0.24.2
[0.24.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.24.0...v0.24.1
[0.24.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.23.3...v0.24.0
[0.23.3]: https://github.com/invisigoth-cz/WallFlux/compare/v0.23.2...v0.23.3
[0.23.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.23.1...v0.23.2
[0.23.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.23.0...v0.23.1
[0.23.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.22.1...v0.23.0
[0.22.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.22.0...v0.22.1
[0.22.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.21.0...v0.22.0
[0.21.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.20.0...v0.21.0
[0.20.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.17.1...v0.18.0
[0.17.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.17.0...v0.17.1
[0.17.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.16.2...v0.17.0
[0.16.2]: https://github.com/invisigoth-cz/WallFlux/compare/v0.16.1...v0.16.2
[0.16.1]: https://github.com/invisigoth-cz/WallFlux/compare/v0.16.0...v0.16.1
[0.16.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/invisigoth-cz/WallFlux/compare/v0.13.0...v0.14.0
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
