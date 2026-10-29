// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! `particles/*.json`: a particle system.
//!
//! A system is an emitter that spawns particles, initializers that give each
//! new one its random starting values, and operators that change those values
//! every frame until the particle's lifetime runs out. The system names a
//! material, which says how its particles are drawn — see [`crate::material`].
//!
//! Three shapes here would catch out a reader built on the rest of the format:
//!
//! - Any field may be written as `null`, and often is, so null has to mean the
//!   same thing as absent rather than being an error.
//! - **Colours are 0-255 here**, not the 0-1 that scenes use. Of the 493
//!   systems in the library that set one, all but one carry a component above
//!   1, and that one is a near-black.
//! - Scalars arrive as bare JSON numbers while vectors arrive as
//!   space-separated strings, sometimes in the same object.
//!
//! Steps this reader does not know are collected in
//! [`ParticleSystem::unsupported`] rather than dropped, because a particle
//! system missing one of its operators still draws — just wrongly — and that
//! is worth reporting instead of hiding.

use crate::scene::Vector;
use serde_json::Value;

/// An inclusive range a starting value is drawn from.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

/// The volume an emitter spawns particles in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Sphere,
    Box,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Emitter {
    pub shape: Shape,
    pub origin: [f32; 3],
    /// Which axes the emitter spreads along. A zero component pins that axis,
    /// which is how a flat wallpaper keeps its particles in the plane.
    pub directions: [f32; 3],
    /// How far from the origin a particle starts, along each axis: a box
    /// may be wider than it is tall, and a sphere reads only the first.
    pub distance: Range<[f32; 3]>,
    /// Particles per second.
    pub rate: f32,
    /// Particles all at once: when the system starts, or for a child that
    /// answers to its parent, each time it is told to.
    pub instantaneous: u32,
    /// Speed given along the spawn direction, on top of any velocity an
    /// initializer sets.
    pub speed: Range<f32>,
    /// Set where the rate follows the sound.
    pub audio: Option<AudioResponse>,
}

/// A starting value handed to each new particle.
#[derive(Debug, Clone, PartialEq)]
pub enum Initializer {
    /// Seconds the particle lives for.
    Lifetime(Range<f32>),
    /// Width in the scene's own pixels.
    Size(Range<f32>),
    Alpha(Range<f32>),
    /// Already scaled to 0-1.
    Color(Range<[f32; 3]>),
    /// Scene pixels per second.
    Velocity(Range<[f32; 3]>),
    /// Radians about each axis.
    Rotation(Range<[f32; 3]>),
    /// Radians per second about each axis.
    AngularVelocity(Range<[f32; 3]>),
    /// A direction taken from a noise field rather than drawn at random, so
    /// particles that start near each other set off together.
    TurbulentVelocity(Turbulent),
    /// Strings each new particle onto the line between the system's first two
    /// control points instead of leaving it where the emitter put it.
    ///
    /// Particles are placed in the order they are born, `count` of them to
    /// cross the line, and then back the way they came. Threaded onto a ribbon
    /// in age order, that is a bolt of lightning: every system in the library
    /// using this is the same discharge preset.
    AlongControlPoints {
        count: u32,
    },
}

/// How many bands the engine splits the sound it hears into. Wallpapers name
/// the ones they listen to by index, and the highest any of them names is 15.
pub const AUDIO_BANDS: usize = 16;

/// The sound as the bands a wallpaper asks for, one set of them per ear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spectrum {
    pub left: [f32; AUDIO_BANDS],
    pub right: [f32; AUDIO_BANDS],
}

impl Spectrum {
    /// A silent room, which is what a renderer with nothing to listen to
    /// hands out.
    pub const SILENT: Self = Self {
        left: [0.0; AUDIO_BANDS],
        right: [0.0; AUDIO_BANDS],
    };

    /// The loudness across a range of bands, which is their average.
    ///
    /// The stock effect shaders spell this out for themselves: they walk the
    /// range adding bands up and divide by how many they added, one ear or
    /// both. A step of a particle system is read the same way.
    pub fn across(&self, bands: (usize, usize), ears: Channels) -> f32 {
        let (first, last) = bands;
        let reach = |of: &[f32; AUDIO_BANDS]| match of.get(first..=last) {
            Some(heard) if !heard.is_empty() => heard.iter().sum::<f32>() / heard.len() as f32,
            _ => 0.0,
        };
        match ears {
            Channels::Left => reach(&self.left),
            Channels::Right => reach(&self.right),
            Channels::Both => (reach(&self.left) + reach(&self.right)) / 2.0,
        }
    }
}

/// Which ear a step listens to.
///
/// The stock shaders spell the numbers out: one is the left channel, two the
/// right, three both together. A particle system's `audioprocessingmode`
/// carries the same numbers, and zero means it does not listen at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channels {
    Left,
    Right,
    Both,
}

/// How a step follows the sound.
///
/// What the step states is its loud end; the quiet end is `bounds.min` of it,
/// never nothing, which is why a wallpaper still moves in silence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioResponse {
    /// The bands to listen to, as the first and last index into the sixteen.
    pub bands: (usize, usize),
    /// Which ear they are taken from.
    pub ears: Channels,
    /// What the loudness is raised to before it is mapped, which is how a
    /// wallpaper says whether it answers to quiet sounds or only to loud ones.
    pub exponent: f32,
    /// What the step's own strength is multiplied by, from silence to loud.
    pub bounds: Range<f32>,
}

/// A noise field a step reads a direction out of.
///
/// `offset` is -0.5 in every system in the library that states one, which is
/// what says the field itself runs 0 to 1 and wants centring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turbulent {
    /// Speed given along the direction the field points.
    pub speed: Range<f32>,
    /// How tightly the field varies with position. The library's values are
    /// around 0.1 for an initializer and 0.002 for the operator, so a feature
    /// spans roughly ten and five hundred scene pixels.
    pub scale: f32,
    /// How fast the field itself moves. Left out by most systems, which means
    /// a field that stays put while particles drift through it.
    pub time_scale: f32,
    /// A particle's own place in the field, drawn from zero to this.
    pub phase_max: f32,
    /// Added to the field before it is used. For `turbulentvelocityrandom`,
    /// radians the start is turned about `right` instead -- and its `scale`
    /// is how far from `forward` a start may stray, in half turns.
    pub offset: f32,
    /// Which way `turbulentvelocityrandom` starts a particle, give or take
    /// its `scale`: up, unless the file says otherwise.
    pub forward: [f32; 3],
    /// What its `offset` turns the start about: the axis out of the screen.
    pub right: [f32; 3],
    /// Set where the field's speed follows the sound.
    pub audio: Option<AudioResponse>,
}

/// A value driven from one number to another over a slice of the lifetime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    /// Start and end as a fraction of the particle's lifetime.
    pub time: Range<f32>,
    pub from: f32,
    pub to: f32,
}

/// A swirl about an axis through the system's origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vortex {
    /// Set where the speed of the swirl follows the sound.
    pub audio: Option<AudioResponse>,
    pub axis: [f32; 3],
    /// Where the inner and outer speeds apply.
    pub distance: Range<f32>,
    /// Degrees a second at the inner and outer distances, straight-lined
    /// between them. Negative turns the other way.
    pub speed: Range<f32>,
}

/// A wave a particle is carried by.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Oscillation {
    /// Cycles a second.
    pub frequency: Range<f32>,
    /// How far the wave reaches.
    pub scale: Range<f32>,
    /// Where in the wave a particle starts, in cycles.
    pub phase: Range<f32>,
}

/// A change applied to every live particle each frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Movement {
        gravity: [f32; 3],
        drag: f32,
    },
    /// Fades in until `fade_in` of the way through its life and out from
    /// `fade_out` of the way. Both are fractions of the lifetime, not seconds:
    /// across 510 uses no value is above one, lifetimes of twenty seconds
    /// included, and the usual pairs are 0.1 and 0.9. Missing, each is 0.5 --
    /// Wallpaper Engine's own default, read out of `wallpaper64.exe`.
    AlphaFade {
        fade_in: f32,
        fade_out: f32,
    },
    SizeChange(Ramp),
    /// Drives a particle's alpha over its life, the same way.
    AlphaChange(Ramp),
    /// Drives a particle's colour from one to another over its life.
    ///
    /// **These colours are 0 to 1**, unlike the 0 to 255 that `colorrandom`
    /// states. Both appear in the same file.
    ColorChange {
        /// Start and end as a fraction of the particle's lifetime.
        time: Range<f32>,
        from: [f32; 3],
        to: [f32; 3],
    },
    /// Swirls a particle about the system's origin.
    Vortex(Vortex),
    /// Pulls a particle towards a point, or pushes it away where the
    /// strength is negative. Nothing past `reach` feels it.
    Attract {
        /// Where the point is -- or, when it follows the pointer, how far
        /// from the pointer.
        toward: [f32; 3],
        strength: f32,
        reach: f32,
        /// The point rides on the mouse.
        pointer: bool,
    },
    /// Beats a particle's alpha down and back up again.
    OscillateAlpha(Oscillation),
    /// Sways a particle to and fro without moving where it came from.
    OscillatePosition {
        wave: Oscillation,
        /// Which axes the sway applies to.
        mask: [f32; 3],
    },
    /// Spins a particle, and turns the spin itself over time.
    AngularMovement {
        /// Radians a second, per second.
        force: [f32; 3],
    },
    /// Drifts a particle through a noise field.
    Turbulence {
        field: Turbulent,
        /// Which axes the drift applies to. Most of the library pushes along
        /// x alone, which is what makes smoke sway rather than scatter.
        mask: [f32; 3],
    },
}

/// How the particles are turned into geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum Renderer {
    /// A quad facing the viewer.
    Sprite,
    /// One ribbon threaded through all the system's particles, oldest first.
    ///
    /// The shader reads a segment's place as `position / length` along the
    /// particle array and says in its own comment that new particles are at
    /// the end of it, so the ribbon is the live particles in the order they
    /// were born. Where they all start from one emitter and drift apart, that
    /// order traces the path they drifted along, which is what makes a wisp.
    Rope {
        /// How finely the engine smooths each segment. Nothing is smoothed
        /// yet, so this only says how smooth it was meant to be.
        subdivision: u32,
    },
    /// A ribbon per particle, threaded through where that particle has been
    /// over the last `length` seconds.
    ///
    /// Not the same as [`Self::Rope`], which threads one ribbon through the
    /// whole system, and not the same as [`Self::Trail`], which stretches a
    /// quad along the velocity and so cannot bend: a falling star curves, and
    /// only a path kept as it was walked curves with it.
    RopeTrail {
        /// Seconds of path the ribbon covers.
        length: f32,
        /// How many places along that path are kept.
        points: u32,
    },
    /// A quad stretched along the particle's velocity. The trail runs
    /// `length` times the particle's speed, kept between the two bounds.
    Trail {
        length: f32,
        min_length: f32,
        max_length: f32,
    },
    Other(String),
}

/// How a particle picks its frame from a sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Animation {
    /// Walks the sheet over the particle's life, `multiplier` times.
    Sequence {
        multiplier: f32,
    },
    RandomFrame,
}

#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub material: String,
    /// Cap on live particles, which is also the buffer size to allocate for.
    pub maxcount: usize,
    /// Seconds before the system starts emitting.
    pub starttime: f32,
    pub emitters: Vec<Emitter>,
    pub initializers: Vec<Initializer>,
    pub operators: Vec<Operator>,
    pub renderer: Renderer,
    pub animation: Option<Animation>,
    /// Points a step may aim at, in the system's own space, in the order the
    /// file lists them. Every system carries eight whether it uses them or
    /// not, so most of these are the origin.
    pub control_points: Vec<[f32; 3]>,
    /// Named steps this reader does not handle yet.
    pub unsupported: Vec<String>,
    /// Other systems this one brings with it.
    pub children: Vec<Child>,
}

/// A system another one brings with it, named by its file.
#[derive(Debug, Clone, PartialEq)]
pub struct Child {
    pub kind: ChildKind,
    pub file: String,
    /// Where the child sits, how big and how turned, in its parent's space.
    pub origin: [f32; 3],
    pub scale: [f32; 3],
    pub angles: [f32; 3],
    /// For a child riding along with its parent's particles, how many of
    /// them it rides with at once. Twenty when the file says nothing, which
    /// is Almamu's reading.
    pub maxcount: usize,
}

/// When a child system runs. Only `Static` says nothing in the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChildKind {
    /// Alongside its parent, all the time, as a system of its own.
    Static,
    /// A burst wherever a particle of the parent is born.
    EventSpawn,
    /// Riding along with each particle of the parent.
    EventFollow,
    /// A burst wherever a particle of the parent dies.
    EventDeath,
}

impl ParticleSystem {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        let root: Value = serde_json::from_slice(bytes)?;

        let mut unsupported = Vec::new();
        let mut emitters = Vec::new();
        let mut initializers = Vec::new();
        let mut operators = Vec::new();

        for step in array(&root, "emitter") {
            match name_of(step) {
                "sphererandom" => emitters.push(emitter(step, Shape::Sphere)),
                "boxrandom" => emitters.push(emitter(step, Shape::Box)),
                other => unsupported.push(format!("emitter {other}")),
            }
        }

        for step in array(&root, "initializer") {
            // Every initializer in the library is a range the starting value is
            // drawn from, so they differ only in which value they fill. A
            // system may state one end and not the other -- a quarter of the
            // colours in the library do -- and the stated end then stands for
            // both. Reading the missing one as zero fades those colours to
            // black, and where neither end is stated it makes the particles
            // invisible.
            let scalar = |default| bounds(step, "min", "max", default);
            let triple = |default| bounds3(step, "min", "max", default);
            initializers.push(match name_of(step) {
                "lifetimerandom" => Initializer::Lifetime(scalar(1.0)),
                // Five to fifty when the file says neither, as the engine
                // fills it in for a flat scene; nought drew nothing.
                "sizerandom" if step.get("min").is_none() && step.get("max").is_none() => {
                    Initializer::Size(Range { min: 5.0, max: 50.0 })
                }
                "sizerandom" => Initializer::Size(scalar(5.0)),
                "alpharandom" => Initializer::Alpha(scalar(1.0)),
                "colorrandom" => {
                    // White on the scale the file states, so a colour nobody
                    // gave stays out of the way.
                    let raw = triple(255.0);
                    Initializer::Color(Range {
                        min: scale_color(raw.min),
                        max: scale_color(raw.max),
                    })
                }
                "velocityrandom" => Initializer::Velocity(triple(0.0)),
                // The engine's defaults for a flat scene, out of
                // `wallpaper64.exe`, and Almamu's renderer has the same: a
                // hundred to two hundred and fifty, a scale and a time scale
                // of one, no offset. Read as nought, Star wars' embers were
                // born still and never rose.
                "turbulentvelocityrandom" => Initializer::TurbulentVelocity(Turbulent {
                    speed: Range {
                        min: num(step, "speedmin", 100.0),
                        max: num(step, "speedmax", 250.0),
                    },
                    scale: num(step, "scale", 1.0),
                    time_scale: num(step, "timescale", 1.0),
                    phase_max: num(step, "phasemax", 0.1),
                    offset: num(step, "offset", 0.0),
                    ..turbulent(step, 1.0)
                }),
                "rotationrandom" => Initializer::Rotation(triple(0.0)),
                "angularvelocityrandom" => Initializer::AngularVelocity(triple(0.0)),
                // Mirroring is the only way back from the far end that the
                // library asks for, and the others would run the wrong way
                // rather than not at all, so they are reported instead.
                "mapsequencebetweencontrolpoints"
                    if step.get("limitbehavior").and_then(Value::as_str) == Some("mirror") =>
                {
                    Initializer::AlongControlPoints {
                        count: num(step, "count", 2.0).clamp(2.0, 4096.0) as u32,
                    }
                }
                other => {
                    unsupported.push(format!("initializer {other}"));
                    continue;
                }
            });
        }

        // Read before the operators, because one of them aims at a control
        // point and whether that point follows the mouse decides whether it
        // can be applied at all.
        //
        // Two ways of saying the same thing: older files set `locktopointer`,
        // newer ones set the bottom bit of `flags`, and most of the files that
        // set the bit leave the older field out. Reading only the field misses
        // forty of the forty-two in the library -- including the system called
        // "Mouse interactive wildfire" -- and a mouse's push applied where the
        // mouse is not flattens whatever it lands on.
        let points: Vec<([f32; 3], bool)> = array(&root, "controlpoint")
            .iter()
            .map(|point| {
                let flagged = num(point, "flags", 0.0) as u32 & FOLLOWS_POINTER != 0;
                let said = point
                    .get("locktopointer")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                (vector(point, "offset", [0.0; 3]), flagged || said)
            })
            .collect();

        for step in array(&root, "operator") {
            operators.push(match name_of(step) {
                "movement" => Operator::Movement {
                    gravity: vector(step, "gravity", [0.0; 3]),
                    drag: num(step, "drag", 0.0),
                },
                "alphafade" => Operator::AlphaFade {
                    fade_in: num(step, "fadeintime", 0.5),
                    fade_out: num(step, "fadeouttime", 0.5),
                },
                // A ramp that leaves its end out runs down to nothing, as
                // Wallpaper Engine's defaults have it.
                "sizechange" => Operator::SizeChange(ramp(step)),
                "alphachange" => Operator::AlphaChange(ramp(step)),
                "colorchange" => Operator::ColorChange {
                    time: Range {
                        min: num(step, "starttime", 0.0),
                        max: num(step, "endtime", 1.0),
                    },
                    from: vector(step, "startvalue", [1.0; 3]),
                    to: vector(step, "endvalue", [0.0; 3]),
                },
                "controlpointattract" => {
                    let which = num(step, "controlpoint", 0.0).max(0.0) as usize;
                    let Some(&(at, pointer)) = points.get(which) else {
                        unsupported
                            .push("operator controlpointattract has no such control point".into());
                        continue;
                    };
                    let reach = num(step, "threshold", 0.0);
                    if reach <= 0.0 {
                        unsupported.push("operator controlpointattract states no reach".into());
                        continue;
                    }
                    let origin = vector(step, "origin", [0.0; 3]);
                    // A point on the mouse is where the mouse is; its resting
                    // offset is where the editor happened to leave it.
                    let base = if pointer { [0.0; 3] } else { at };
                    Operator::Attract {
                        toward: std::array::from_fn(|i| base[i] + origin[i]),
                        strength: num(step, "scale", 0.0),
                        reach,
                        pointer,
                    }
                }
                "vortex" => Operator::Vortex(Vortex {
                    audio: audio_response(step),
                    axis: vector(step, "axis", [0.0, 0.0, 1.0]),
                    distance: Range {
                        min: num(step, "distanceinner", 0.0),
                        max: num(step, "distanceouter", 0.0),
                    },
                    speed: Range {
                        min: num(step, "speedinner", 0.0),
                        max: num(step, "speedouter", 0.0),
                    },
                }),
                "oscillatealpha" => Operator::OscillateAlpha(oscillation(step)),
                "oscillateposition" => Operator::OscillatePosition {
                    wave: oscillation(step),
                    mask: vector(step, "mask", [1.0; 3]),
                },
                "angularmovement" => Operator::AngularMovement {
                    force: vector(step, "force", [0.0; 3]),
                },
                // The engine's defaults for the operator, which differ from
                // the initializer's of the same name: flat, and fast. Its
                // scale and time scale are left as they were -- the engine
                // and Almamu's renderer disagree about both.
                // ponytail: settle scale/timescale once a scene that omits them is compared with WPE
                "turbulence" => Operator::Turbulence {
                    field: Turbulent {
                        speed: Range {
                            min: num(step, "speedmin", 500.0),
                            max: num(step, "speedmax", 1000.0),
                        },
                        ..turbulent(step, 0.002)
                    },
                    mask: vector(step, "mask", [1.0, 1.0, 0.0]),
                },
                other => {
                    unsupported.push(format!("operator {other}"));
                    continue;
                }
            });
        }

        // The renderer is a list in the file but never holds more than one
        // entry in the library, and a system draws one way.
        let renderer = match array(&root, "renderer").first().copied() {
            Some(r) => match name_of(r) {
                "sprite" => Renderer::Sprite,
                "rope" => Renderer::Rope {
                    subdivision: num(r, "subdivision", 1.0).clamp(1.0, 64.0) as u32,
                },
                "ropetrail" => Renderer::RopeTrail {
                    length: num(r, "length", 0.2).clamp(0.0, 10.0),
                    // Four of the five systems in the library leave this out
                    // and the fifth says sixteen. Eight points across a fifth
                    // of a second is a smooth enough streak for a falling star
                    // and cheap enough for a system of five hundred.
                    points: num(r, "segments", 8.0).clamp(2.0, 48.0) as u32,
                },
                "spritetrail" => Renderer::Trail {
                    length: num(r, "length", 0.0),
                    min_length: num(r, "minlength", 0.0),
                    // Left out by half the systems that trail, and the shader
                    // caps the length against it, so absent has to mean
                    // uncapped rather than nothing.
                    max_length: num(r, "maxlength", f32::MAX),
                },
                other => Renderer::Other(other.to_owned()),
            },
            None => Renderer::Sprite,
        };

        let animation = match root.get("animationmode").and_then(Value::as_str) {
            Some("sequence") => Some(Animation::Sequence {
                multiplier: num(&root, "sequencemultiplier", 1.0).max(1.0),
            }),
            Some("randomframe") => Some(Animation::RandomFrame),
            Some(other) => {
                unsupported.push(format!("animationmode {other}"));
                None
            }
            None => None,
        };

        let mut children = Vec::new();
        for child in array(&root, "children") {
            let Some(file) = child.get("name").and_then(Value::as_str) else {
                continue;
            };
            let kind = match child.get("type").and_then(Value::as_str) {
                None | Some("static") => ChildKind::Static,
                Some("eventspawn") => ChildKind::EventSpawn,
                Some("eventfollow") => ChildKind::EventFollow,
                Some("eventdeath") => ChildKind::EventDeath,
                Some(other) => {
                    unsupported.push(format!("child {other}"));
                    continue;
                }
            };
            children.push(Child {
                kind,
                file: file.to_owned(),
                origin: vector(child, "origin", [0.0; 3]),
                scale: vector(child, "scale", [1.0; 3]),
                angles: vector(child, "angles", [0.0; 3]),
                maxcount: num(child, "maxcount", 20.0).max(0.0) as usize,
            });
        }

        Ok(Self {
            material: root
                .get("material")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            control_points: points.iter().map(|(at, _)| *at).collect(),
            maxcount: num(&root, "maxcount", 0.0).clamp(0.0, MAX_PARTICLES as f32) as usize,
            starttime: num(&root, "starttime", 0.0),
            emitters,
            initializers,
            operators,
            renderer,
            animation,
            children,
            unsupported,
        })
    }
}

/// The bit of a control point's `flags` that hangs it on the mouse.
const FOLLOWS_POINTER: u32 = 1;

/// Cap on what a file may ask to be allocated for. The largest system in the
/// library asks for 30 000; this leaves room above that without letting a
/// malformed file size a buffer.
const MAX_PARTICLES: usize = 100_000;

fn emitter(step: &Value, shape: Shape) -> Emitter {
    Emitter {
        shape,
        origin: vector(step, "origin", [0.0; 3]),
        // An emitter that named no axes would spawn every particle at one
        // point, so an absent mask means all three.
        directions: vector(step, "directions", [1.0; 3]),
        // A number or, for forty boxes in the library, "512 256 0" -- which
        // read as a number was nought, and every one of those boxes let all
        // its particles out at a single point.
        distance: Range {
            min: vector(step, "distancemin", [0.0; 3]),
            max: vector(step, "distancemax", [0.0; 3]),
        },
        rate: num(step, "rate", 0.0),
        instantaneous: num(step, "instantaneous", 0.0).max(0.0) as u32,
        speed: Range {
            min: num(step, "speedmin", 0.0),
            max: num(step, "speedmax", 0.0),
        },
        audio: audio_response(step),
    }
}

/// What a step says about following the sound, if it follows it at all.
///
/// The mode is the switch: the editor hides every other audio field while it
/// is zero, and a third of the steps in the library carry settings behind a
/// zero that were never meant to be used. Which ear a mode above zero means is
/// taken from the stock effect shaders, which carry the same numbers and spell
/// them out: one left, two right, three both.
fn audio_response(step: &Value) -> Option<AudioResponse> {
    let ears = match num(step, "audioprocessingmode", 0.0) as i32 {
        1 => Channels::Left,
        2 => Channels::Right,
        // Zero is the switch being off. Three is both, and so is anything
        // else above zero: no other mode appears in the library, and both
        // ears is the least surprising reading of one that did.
        ..=0 => return None,
        _ => Channels::Both,
    };
    let last = AUDIO_BANDS - 1;
    let first = (num(step, "audioprocessingfrequencystart", 0.0) as usize).min(last);
    Some(AudioResponse {
        bands: (
            first,
            (num(step, "audioprocessingfrequencyend", last as f32) as usize).clamp(first, last),
        ),
        ears,
        exponent: num(step, "audioprocessingexponent", 1.0).max(f32::EPSILON),
        bounds: pair(step, "audioprocessingbounds", Range { min: 0.0, max: 1.0 }),
    })
}

/// A pair of bounds where a system may state only one of them.
///
/// Half the oscillating systems in the library give a `scalemin` and no
/// `scalemax`, or the other way about. A missing end means the stated one
/// again, which is a steady value rather than a range down to nothing.
fn bounds(step: &Value, low: &str, high: &str, default: f32) -> Range<f32> {
    let min = step.get(low).filter(|v| !v.is_null());
    let max = step.get(high).filter(|v| !v.is_null());
    match (min.is_some(), max.is_some()) {
        (true, true) => Range {
            min: num(step, low, default),
            max: num(step, high, default),
        },
        (true, false) => {
            let only = num(step, low, default);
            Range {
                min: only,
                max: only,
            }
        }
        (false, true) => {
            let only = num(step, high, default);
            Range {
                min: only,
                max: only,
            }
        }
        (false, false) => Range {
            min: default,
            max: default,
        },
    }
}

/// The same as [`bounds`], for the fields written as vectors.
fn bounds3(step: &Value, low: &str, high: &str, default: f32) -> Range<[f32; 3]> {
    let stated = |key: &str| step.get(key).filter(|v| !v.is_null()).is_some();
    let read = |key: &str| vector(step, key, [default; 3]);
    match (stated(low), stated(high)) {
        (true, true) => Range {
            min: read(low),
            max: read(high),
        },
        (true, false) | (false, true) => {
            let only = read(if stated(low) { low } else { high });
            Range {
                min: only,
                max: only,
            }
        }
        (false, false) => Range {
            min: [default; 3],
            max: [default; 3],
        },
    }
}

fn oscillation(step: &Value) -> Oscillation {
    Oscillation {
        frequency: bounds(step, "frequencymin", "frequencymax", 0.0),
        scale: bounds(step, "scalemin", "scalemax", 0.0),
        phase: bounds(step, "phasemin", "phasemax", 0.0),
    }
}

/// `default_scale` is what the library's systems state, used for the rare one
/// that leaves it out: a field with no scale at all would push every particle
/// the same way and stop being turbulence.
/// A value driven from `startvalue` to `endvalue` between `starttime` and
/// `endtime` of a particle's life: one to nought across the whole of it where
/// the file says nothing, which is what `wallpaper64.exe` fills in.
fn ramp(step: &Value) -> Ramp {
    Ramp {
        time: Range {
            min: num(step, "starttime", 0.0),
            max: num(step, "endtime", 1.0),
        },
        from: num(step, "startvalue", 1.0),
        to: num(step, "endvalue", 0.0),
    }
}

fn turbulent(step: &Value, default_scale: f32) -> Turbulent {
    Turbulent {
        speed: Range {
            min: num(step, "speedmin", 0.0),
            max: num(step, "speedmax", 0.0),
        },
        scale: num(step, "scale", default_scale),
        time_scale: num(step, "timescale", 0.0),
        phase_max: num(step, "phasemax", 0.0),
        offset: num(step, "offset", -0.5),
        forward: vector(step, "forward", [0.0, 1.0, 0.0]),
        right: vector(step, "right", [0.0, 0.0, 1.0]),
        audio: audio_response(step),
    }
}

/// A pair written as one space-separated string, which is how the audio bounds
/// arrive: `"0.5 1"` is half the stated strength in silence and all of it when
/// the sound is loud.
fn pair(v: &Value, key: &str, default: Range<f32>) -> Range<f32> {
    v.get(key)
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value::<Vector<2>>(v.clone()).ok())
        .map_or(default, |v| Range {
            min: v.0[0],
            max: v.0[1],
        })
}

fn array<'a>(v: &'a Value, key: &str) -> Vec<&'a Value> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

fn name_of(step: &Value) -> &str {
    step.get("name").and_then(Value::as_str).unwrap_or("")
}

/// A number, however it was written. Missing, null and unreadable all give the
/// default, which is what keeps one `null` from losing a whole system.
fn num(v: &Value, key: &str, default: f32) -> f32 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default as f64) as f32,
        Some(Value::String(s)) => s.trim().parse().unwrap_or(default),
        _ => default,
    }
}

/// A vector, which the format writes as a space-separated string but sometimes
/// as a number or an array. [`Vector`] already knows all three forms.
fn vector(v: &Value, key: &str, default: [f32; 3]) -> [f32; 3] {
    v.get(key)
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value::<Vector<3>>(v.clone()).ok())
        .map_or(default, |v| v.0)
}

/// Particle colours are 0-255 where the rest of the format is 0-1.
fn scale_color(c: [f32; 3]) -> [f32; 3] {
    c.map(|x| x / 255.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &[u8] = br#"{
        "material": "materials/particle/halo.json",
        "maxcount": 500,
        "starttime": 0,
        "emitter": [{"name":"sphererandom","rate":20,"origin":"0 0 0",
                     "directions":"1 1 0","distancemin":32,"distancemax":512}],
        "initializer": [{"name":"lifetimerandom","min":3,"max":5},
                        {"name":"sizerandom","min":50,"max":200},
                        {"name":"velocityrandom","min":"-50 -50 0","max":"50 50 0"},
                        {"name":"colorrandom","min":"255 128 0","max":"255 255 255"}],
        "operator": [{"name":"movement","gravity":"0 0 0"},
                     {"name":"alphafade","fadeintime":0.5}]
    }"#;

    #[test]
    fn reads_the_shape_of_a_system() {
        let p = ParticleSystem::parse(EXAMPLE).unwrap();
        assert_eq!(p.material, "materials/particle/halo.json");
        assert_eq!(p.maxcount, 500);
        assert_eq!(p.renderer, Renderer::Sprite);
        assert_eq!(p.animation, None);
        assert!(p.unsupported.is_empty());
        assert_eq!(
            p.emitters,
            vec![Emitter {
                shape: Shape::Sphere,
                origin: [0.0; 3],
                directions: [1.0, 1.0, 0.0],
                distance: Range {
                    min: [32.0; 3],
                    max: [512.0; 3]
                },
                rate: 20.0,
                instantaneous: 0,
                speed: Range { min: 0.0, max: 0.0 },
                audio: None,
            }]
        );
        assert_eq!(
            p.initializers[0],
            Initializer::Lifetime(Range { min: 3.0, max: 5.0 })
        );
        assert_eq!(
            p.operators[1],
            Operator::AlphaFade {
                fade_in: 0.5,
                fade_out: 0.5
            }
        );
    }

    #[test]
    fn colours_come_out_on_the_scale_the_renderer_uses() {
        let p = ParticleSystem::parse(EXAMPLE).unwrap();
        let Some(Initializer::Color(range)) = p
            .initializers
            .iter()
            .find(|i| matches!(i, Initializer::Color(_)))
            .cloned()
        else {
            panic!("no colour initializer");
        };
        assert_eq!(range.min, [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(range.max, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn a_null_costs_only_its_own_field() {
        let json = br#"{
            "material": "m.json", "maxcount": null, "flags": null,
            "emitter": [{"name":"boxrandom","rate":null,"distancemax":100}],
            "initializer": [{"name":"sizerandom","min":1,"max":null}],
            "operator": [{"name":"movement","gravity":null,"drag":0.5}],
            "renderer": [{"name":"spritetrail","length":0.02,"minlength":null}],
            "animationmode": null
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(p.maxcount, 0);
        assert_eq!(p.emitters[0].rate, 0.0);
        // What the engine fills in for a flat scene when nothing is said.
        let bare = ParticleSystem::parse(
            br#"{"initializer":[{"name":"sizerandom"},
                                {"name":"turbulentvelocityrandom"}]}"#,
        )
        .unwrap();
        assert_eq!(bare.initializers[0], Initializer::Size(Range { min: 5.0, max: 50.0 }));
        let Initializer::TurbulentVelocity(field) = &bare.initializers[1] else {
            panic!("{:?}", bare.initializers[1]);
        };
        assert_eq!(field.speed, Range { min: 100.0, max: 250.0 });
        assert_eq!(p.emitters[0].distance.max, [100.0; 3]);
        // A null end is a missing end, and the stated one stands for both
        // rather than the range running down to nothing.
        assert_eq!(
            p.initializers[0],
            Initializer::Size(Range { min: 1.0, max: 1.0 })
        );
        assert_eq!(
            p.operators[0],
            Operator::Movement {
                gravity: [0.0; 3],
                drag: 0.5
            }
        );
        assert_eq!(
            p.renderer,
            Renderer::Trail {
                length: 0.02,
                min_length: 0.0,
                max_length: f32::MAX
            }
        );
        assert_eq!(p.animation, None);
    }

    #[test]
    fn an_unknown_step_is_reported_rather_than_dropped() {
        let json = br#"{
            "material": "m.json", "maxcount": 10,
            "emitter": [{"name":"newfangled"}],
            "initializer": [{"name":"mapsequencearoundcontrolpoint"}],
            "operator": [{"name":"controlpointattract"},{"name":"alphafade","fadeouttime":1}]
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert!(p.emitters.is_empty());
        assert!(p.initializers.is_empty());
        assert_eq!(p.operators.len(), 1);
        assert_eq!(
            p.unsupported,
            [
                "emitter newfangled",
                "initializer mapsequencearoundcontrolpoint",
                "operator controlpointattract has no such control point"
            ]
        );
    }

    /// The editor hides every other audio field while the mode is zero, and a
    /// third of the steps in the library carry settings behind a zero.
    #[test]
    fn a_step_follows_the_sound_only_once_its_mode_is_set() {
        let quiet = br#"{"material":"m.json","maxcount":10,
            "emitter":[{"name":"sphererandom","rate":100,"audioprocessingmode":0,
                        "audioprocessingbounds":"0.5 1"}]}"#;
        assert_eq!(
            ParticleSystem::parse(quiet).unwrap().emitters[0].audio,
            None
        );

        let loud = br#"{"material":"m.json","maxcount":10,
            "emitter":[{"name":"sphererandom","rate":100,"audioprocessingmode":3,
                        "audioprocessingbounds":"0.5 1","audioprocessingexponent":0.6,
                        "audioprocessingfrequencystart":4,"audioprocessingfrequencyend":10}]}"#;
        assert_eq!(
            ParticleSystem::parse(loud).unwrap().emitters[0].audio,
            Some(AudioResponse {
                bands: (4, 10),
                ears: Channels::Both,
                exponent: 0.6,
                bounds: Range { min: 0.5, max: 1.0 },
            })
        );
    }

    /// Bounds arrive as one string, and reading them as two missing numbers
    /// would leave the step at nothing however loud the sound got.
    /// The stock effect shaders carry the same numbers and spell them out,
    /// so a step asking for one ear gets that ear and not the pair.
    #[test]
    fn a_mode_says_which_ear_a_step_listens_to() {
        let with = |mode| {
            let json = format!(
                r#"{{"material":"m.json","maxcount":10,
                    "emitter":[{{"name":"sphererandom","rate":100,
                                "audioprocessingmode":{mode}}}]}}"#
            );
            ParticleSystem::parse(json.as_bytes()).unwrap().emitters[0]
                .audio
                .map(|a| a.ears)
        };
        assert_eq!(with(0), None);
        assert_eq!(with(1), Some(Channels::Left));
        assert_eq!(with(2), Some(Channels::Right));
        assert_eq!(with(3), Some(Channels::Both));
    }

    /// One ear, the other, or the pair averaged -- and only across the bands
    /// the step asked for.
    #[test]
    fn a_spectrum_averages_the_bands_it_is_asked_for() {
        let mut sound = Spectrum::SILENT;
        sound.left[4..=7].fill(1.0);
        sound.right[4..=7].fill(0.5);
        assert_eq!(sound.across((4, 7), Channels::Left), 1.0);
        assert_eq!(sound.across((4, 7), Channels::Right), 0.5);
        assert_eq!(sound.across((4, 7), Channels::Both), 0.75);
        // Half the range is loud and half is not.
        assert_eq!(sound.across((4, 11), Channels::Left), 0.5);
        assert_eq!(sound.across((0, 3), Channels::Both), 0.0);
    }

    #[test]
    fn audio_bounds_are_one_string_of_two_numbers() {
        let json = br#"{"material":"m.json","maxcount":10,
            "emitter":[{"name":"sphererandom","rate":100,"audioprocessingmode":1,
                        "audioprocessingbounds":"0.9 1"}]}"#;
        let audio = ParticleSystem::parse(json).unwrap().emitters[0]
            .audio
            .unwrap();
        assert_eq!(audio.bounds, Range { min: 0.9, max: 1.0 });
        // Nothing stated means the whole spectrum rather than one band.
        assert_eq!(audio.bands, (0, 15));
        assert_eq!(audio.exponent, 1.0);
    }

    #[test]
    fn a_child_without_a_type_runs_alongside_its_parent() {
        let p = ParticleSystem::parse(
            br#"{"children": [
                {"name": "particles/presets/snowstormfog.json", "origin": "0 650 0"},
                {"name": "particles/presets/sparktrails.json", "type": "eventspawn", "scale": "2 2 2", "maxcount": 1}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            p.children,
            [
                Child {
                    kind: ChildKind::Static,
                    file: "particles/presets/snowstormfog.json".into(),
                    origin: [0.0, 650.0, 0.0],
                    scale: [1.0; 3],
                    angles: [0.0; 3],
                    maxcount: 20,
                },
                Child {
                    kind: ChildKind::EventSpawn,
                    file: "particles/presets/sparktrails.json".into(),
                    origin: [0.0; 3],
                    scale: [2.0; 3],
                    angles: [0.0; 3],
                    maxcount: 1,
                },
            ]
        );
    }

    #[test]
    fn a_sequence_between_control_points_reads_its_line() {
        let json = br#"{
            "material": "m.json", "maxcount": 10,
            "controlpoint": [{"id":0},{"id":1,"offset":"512 512 0"}],
            "initializer": [{"name":"mapsequencebetweencontrolpoints","count":10,
                             "limitbehavior":"mirror"}]
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(
            p.initializers,
            [Initializer::AlongControlPoints { count: 10 }]
        );
        assert_eq!(p.control_points, [[0.0; 3], [512.0, 512.0, 0.0]]);
        assert!(p.unsupported.is_empty());
    }

    /// Any other way back from the far end would run the sequence wrongly
    /// rather than not at all, so it is reported instead of guessed at.
    #[test]
    fn a_sequence_that_does_not_mirror_is_reported() {
        let json = br#"{
            "material": "m.json", "maxcount": 10,
            "controlpoint": [{"id":0},{"id":1,"offset":"512 512 0"}],
            "initializer": [{"name":"mapsequencebetweencontrolpoints","count":10,
                             "limitbehavior":"repeat"}]
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert!(p.initializers.is_empty());
        assert_eq!(
            p.unsupported,
            ["initializer mapsequencebetweencontrolpoints"]
        );
    }

    /// All but two of the forty-two uses in the library hang their point on
    /// the mouse, and most say so only in the flags. Pushing from where the
    /// mouse is not flattens whatever the push lands on.
    #[test]
    fn a_point_the_mouse_carries_is_read_as_one() {
        for point in [
            r#"{"id":1,"offset":"0 0 0","locktopointer":true}"#,
            r#"{"id":1,"offset":"0 0 0","flags":1}"#,
            r#"{"id":1,"offset":"0 0 0","flags":3}"#,
        ] {
            let json = format!(
                r#"{{
                    "material": "m.json", "maxcount": 10,
                    "controlpoint": [{{"id":0,"offset":"0 0 0"}}, {point}],
                    "operator": [{{"name":"controlpointattract","controlpoint":1,
                                  "scale":-10000,"threshold":128}}]
                }}"#
            );
            let p = ParticleSystem::parse(json.as_bytes()).unwrap();
            assert!(p.unsupported.is_empty(), "{point}");
            assert_eq!(
                p.operators,
                [Operator::Attract {
                    toward: [0.0; 3],
                    strength: -10000.0,
                    reach: 128.0,
                    pointer: true,
                }],
                "{point}"
            );
        }
    }

    /// Bit two says the author moved the point, which is not the mouse.
    #[test]
    fn a_point_the_author_placed_is_not_the_mouse() {
        let json = br#"{
            "material": "m.json", "maxcount": 10,
            "controlpoint": [{"id":0},{"id":1,"flags":2,"offset":"512 512 0"}],
            "operator": [{"name":"controlpointattract","controlpoint":1,
                          "scale":100,"threshold":128}]
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(p.operators.len(), 1);
        assert!(p.unsupported.is_empty());
    }

    #[test]
    fn attracting_to_a_point_that_stays_put_reads_it() {
        let json = br#"{
            "material": "m.json", "maxcount": 10,
            "controlpoint": [{"id":0,"offset":"0 0 0"},
                             {"id":1,"offset":"100 200 0"}],
            "operator": [{"name":"controlpointattract","controlpoint":1,
                          "origin":"0 50 0","scale":-10000,"threshold":128}]
        }"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(
            p.operators,
            [Operator::Attract {
                toward: [100.0, 250.0, 0.0],
                strength: -10000.0,
                reach: 128.0,
                pointer: false,
            }]
        );
        assert!(p.unsupported.is_empty());
    }

    #[test]
    fn a_file_cannot_ask_for_an_unbounded_buffer() {
        let json = br#"{"material":"m.json","maxcount":1e12}"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(p.maxcount, MAX_PARTICLES);
    }
}

#[cfg(test)]
mod turbulence_tests {
    use super::*;

    #[test]
    fn reads_a_turbulent_velocity_with_the_fields_own_range() {
        let json = br#"{"material":"m.json","maxcount":1,
            "initializer":[{"name":"turbulentvelocityrandom","scale":0.1,
                            "speedmin":250,"speedmax":260,"offset":-0.5,
                            "timescale":0.2,"phasemax":2}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(
            p.initializers,
            vec![Initializer::TurbulentVelocity(Turbulent {
                speed: Range {
                    min: 250.0,
                    max: 260.0
                },
                scale: 0.1,
                time_scale: 0.2,
                phase_max: 2.0,
                offset: -0.5,
                forward: [0.0, 1.0, 0.0],
                right: [0.0, 0.0, 1.0],
                audio: None,
            })]
        );
        assert!(p.unsupported.is_empty());
    }

    #[test]
    fn reads_the_axes_turbulence_pushes_along() {
        let json = br#"{"material":"m.json","maxcount":1,
            "operator":[{"name":"turbulence","scale":0.002,"speedmin":100,
                         "speedmax":150,"mask":"1 0 0","phasemax":5}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        let [Operator::Turbulence { field, mask }] = p.operators.as_slice() else {
            panic!("not a turbulence operator: {:?}", p.operators);
        };
        assert_eq!(*mask, [1.0, 0.0, 0.0]);
        assert_eq!(field.scale, 0.002);
        assert_eq!(field.phase_max, 5.0);
        // Left out by this system, and by most of the library.
        assert_eq!(field.time_scale, 0.0);
    }

    #[test]
    fn a_field_that_states_no_scale_still_varies() {
        let json = br#"{"material":"m.json","maxcount":1,
            "operator":[{"name":"turbulence","speedmax":100}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        let [Operator::Turbulence { field, .. }] = p.operators.as_slice() else {
            panic!("not a turbulence operator");
        };
        assert!(field.scale > 0.0);
    }
}

#[cfg(test)]
mod rope_tests {
    use super::*;

    #[test]
    fn reads_a_rope_and_how_finely_it_was_meant_to_bend() {
        let json = br#"{"material":"m.json","maxcount":32,
            "renderer":[{"id":1,"name":"rope","subdivision":3}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(p.renderer, Renderer::Rope { subdivision: 3 });
        assert!(p.unsupported.is_empty());
    }

    #[test]
    fn a_rope_that_states_no_subdivision_still_has_one() {
        let json = br#"{"material":"m.json","renderer":[{"id":1,"name":"rope"}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(p.renderer, Renderer::Rope { subdivision: 1 });
    }

    /// Four of the five in the library state no count, and the ribbon has to
    /// have places to go through whatever they state.
    #[test]
    fn a_rope_trail_keeps_places_along_its_path() {
        let json =
            br#"{"material":"m.json","renderer":[{"id":1,"name":"ropetrail","length":0.2}]}"#;
        let p = ParticleSystem::parse(json).unwrap();
        assert_eq!(
            p.renderer,
            Renderer::RopeTrail {
                length: 0.2,
                points: 8
            }
        );

        let stated = br#"{"material":"m.json","renderer":[{"id":1,"name":"ropetrail",
            "length":0.5,"segments":16,"subdivision":16}]}"#;
        assert_eq!(
            ParticleSystem::parse(stated).unwrap().renderer,
            Renderer::RopeTrail {
                length: 0.5,
                points: 16
            }
        );
    }
}

#[cfg(test)]
mod one_sided_tests {
    use super::*;

    fn only(json: &str) -> Vec<Initializer> {
        ParticleSystem::parse(json.as_bytes()).unwrap().initializers
    }

    #[test]
    fn a_colour_stated_at_one_end_only_is_that_colour() {
        // 27 of the 111 colour initializers in the library are written this
        // way. Reading the missing end as zero fades half the particles to
        // black.
        let p = only(r#"{"initializer":[{"name":"colorrandom","min":"0 53 255"}]}"#);
        let Initializer::Color(range) = p[0] else {
            panic!("not a colour: {p:?}");
        };
        assert_eq!(range.min, range.max);
        assert_eq!(range.min, [0.0, 53.0 / 255.0, 1.0]);
    }

    #[test]
    fn a_rotation_stated_at_the_far_end_only_is_still_a_range_of_one() {
        let p = only(r#"{"initializer":[{"name":"rotationrandom","max":"0 0 2"}]}"#);
        assert_eq!(
            p[0],
            Initializer::Rotation(Range {
                min: [0.0, 0.0, 2.0],
                max: [0.0, 0.0, 2.0]
            })
        );
    }

    #[test]
    fn values_nobody_stated_fall_back_to_something_that_shows() {
        let p = only(r#"{"initializer":[{"name":"alpharandom"},{"name":"colorrandom"}]}"#);
        // Not zero, which would make every particle of the system invisible.
        assert_eq!(p[0], Initializer::Alpha(Range { min: 1.0, max: 1.0 }));
        assert_eq!(
            p[1],
            Initializer::Color(Range {
                min: [1.0; 3],
                max: [1.0; 3]
            })
        );
    }
}
