// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Simulating a particle system and laying it out the way its shaders read it.
//!
//! The particle shaders take no per-particle uniforms. Every value a particle
//! has arrives as a vertex attribute, four vertices to a particle, so a system
//! is one buffer and one draw call however many particles are alive. This
//! module owns the simulation and produces that buffer; putting it on the GPU
//! is [`crate::render`]'s job, which keeps the moving parts testable without a
//! graphics context.
//!
//! Each frame a particle is spawned by the emitter, given its starting values
//! by the initializers, and then changed by the operators until its lifetime
//! runs out. Operators come in two kinds, and the difference matters:
//! `movement` integrates and so has to run step by step, while `alphafade` and
//! `sizechange` are plain functions of how far through its life a particle is,
//! and are applied when the buffer is written. Computing those two per step
//! instead would let rounding pile up over a particle's whole life.

use wpe_format::particle::{
    Animation, AudioResponse, Emitter, Initializer, Operator, Oscillation, ParticleSystem, Range,
    Shape, Spectrum, Turbulent, Vortex,
};
use wpe_format::scene::InstanceOverride;

/// What a step's own strength is multiplied by, given what is playing.
///
/// A step that does not follow the sound is left alone. One that does is
/// never turned off: what it states is its loud end, and the quiet end is a
/// fraction of that, which is why a wallpaper still moves in a silent room.
///
/// ponytail: the loudness of a range of bands is their average, and the
/// exponent shapes it before it is mapped rather than after. The file states
/// neither; these are the readings that leave both settings doing what their
/// names say.
fn loudness(audio: Option<AudioResponse>, sound: &Spectrum) -> f32 {
    let Some(audio) = audio else {
        return 1.0;
    };
    let heard = sound
        .across(audio.bands, audio.ears)
        .clamp(0.0, 1.0)
        .powf(audio.exponent);
    audio.bounds.min + (audio.bounds.max - audio.bounds.min) * heard
}

/// Floats per vertex of a particle drawn as a sprite: position, then corner
/// and rotation and size, then colour, then the two remaining rotation axes,
/// then velocity and lifetime.
pub const FLOATS_PER_VERTEX: usize = 17;

/// Floats per vertex of a rope segment: the start with its size, the end with
/// the rope's length, a control point with the segment's place along the rope,
/// a second control point with the end's size, the end's colour, the corner,
/// and the start's colour.
pub const FLOATS_PER_ROPE_VERTEX: usize = 26;
const VERTICES_PER_PARTICLE: usize = 4;

/// The four corners of a particle, as the shader's `ComputeParticlePosition`
/// expects them: a unit square it offsets by the particle's size.
const CORNERS: [[f32; 2]; VERTICES_PER_PARTICLE] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// A frame longer than this is treated as this long. A system that was paused,
/// or a first frame that includes loading, would otherwise be handed a second
/// or more of catching up and emit its entire pool at once.
const MAX_STEP: f32 = 0.1;

struct Particle {
    position: [f32; 3],
    /// Where this particle has been, oldest first, for a system whose
    /// ribbons follow a path. Empty for every other system, which costs
    /// nothing but the handle.
    trail: Vec<[f32; 3]>,
    /// Seconds since the last place was kept.
    since: f32,
    velocity: [f32; 3],
    rotation: [f32; 3],
    size: f32,
    color: [f32; 3],
    alpha: f32,
    age: f32,
    lifetime: f32,
    /// Where in a sprite sheet this particle starts, for `randomframe`.
    frame: f32,
    /// This particle's own place in the turbulence field, as a fraction of
    /// whatever each step allows, so two particles side by side are not
    /// pushed identically.
    phase: f32,
    /// Its share of a turbulence operator's speed range.
    drift: f32,
    /// Radians a second about each axis.
    spin: [f32; 3],
    /// Where this particle sits in an oscillation: its frequency, how far it
    /// swings, and where in the wave it starts.
    wobble: [f32; 3],
    /// Its number in the system, for a child riding along with it to find
    /// it again from one step to the next.
    serial: u32,
    /// The parent's particle this one rides with, for a child that does.
    host: Option<u32>,
}

/// The ranges the initializers draw a new particle's values from.
struct Start {
    lifetime: Range<f32>,
    size: Range<f32>,
    alpha: Range<f32>,
    color: Range<[f32; 3]>,
    velocity: Range<[f32; 3]>,
    rotation: Range<[f32; 3]>,
    /// Radians a second about each axis, which only `angularmovement` turns
    /// into an actual spin.
    angular: Range<[f32; 3]>,
    /// Set when the system takes its starting direction from a noise field
    /// rather than at random.
    turbulent: Option<Turbulent>,
}

impl Default for Start {
    fn default() -> Self {
        Self {
            // A system with no lifetime initializer would otherwise divide by
            // zero working out how far through its life a particle is.
            lifetime: Range { min: 1.0, max: 1.0 },
            size: Range { min: 0.0, max: 0.0 },
            alpha: Range { min: 1.0, max: 1.0 },
            color: Range {
                min: [1.0; 3],
                max: [1.0; 3],
            },
            velocity: Range::default(),
            rotation: Range::default(),
            angular: Range::default(),
            turbulent: None,
        }
    }
}

/// The line new particles are strung along, and how many of them cross it.
#[derive(Clone, Copy)]
struct Line {
    from: [f32; 3],
    to: [f32; 3],
    count: u32,
}

impl Line {
    /// How far along the line the particle born `nth` belongs.
    ///
    /// The walk turns round at each end rather than jumping back, so a ribbon
    /// threaded through these runs out to the far point and back along itself.
    fn at(&self, nth: u32) -> [f32; 3] {
        let last = self.count - 1;
        let step = nth % (2 * last);
        let step = if step <= last { step } else { 2 * last - step };
        let t = step as f32 / last as f32;
        std::array::from_fn(|i| self.from[i] + (self.to[i] - self.from[i]) * t)
    }
}

/// How a system's particles are strung onto ribbons, if they are.
#[derive(Clone, Copy)]
enum Ribbon {
    /// Not a ribbon: every particle is its own quad.
    None,
    /// One ribbon threaded through every living particle, oldest first.
    Chain,
    /// A ribbon for each particle, through where that one has been.
    Trail {
        /// Seconds between the places kept along the path.
        spacing: f32,
        /// How many places are kept.
        points: usize,
    },
}

pub struct System {
    /// Whether the particles are strung onto ribbons, and how.
    ribbon: Ribbon,
    /// Each emitter with the fraction of a particle it is owed.
    emitters: Vec<(Emitter, f32)>,
    start: Start,
    operators: Vec<Operator>,
    animation: Option<Animation>,
    maxcount: usize,
    starttime: f32,
    live: Vec<Particle>,
    elapsed: f32,
    rng: Rng,
    vertices: Vec<f32>,
    /// Set where an initializer strings particles along a line.
    line: Option<Line>,
    /// Particles born since the system started, which is the only thing that
    /// says where the next one goes on that line.
    born: u32,
    /// Where the mouse is, in the system's own space, or nothing when there
    /// is no mouse to follow -- a point hung on it then pulls nowhere.
    pointer: Option<[f32; 3]>,
    /// Whether the emitters' all-at-once particles have been let out yet.
    burst: bool,
    /// Set for a child that emits only when its parent's particles are born
    /// or die, and then where they were.
    waits: bool,
    /// Where particles were born and died in the last step, kept only for a
    /// system with a child listening.
    events: Option<Events>,
    /// Particles taken in since the system started, which numbers them.
    serials: u32,
    /// For a child riding along with its parent's particles: the ones it
    /// rides with, and at most how many.
    hosts: Vec<Host>,
    ride_limit: usize,
}

/// One of a parent's particles a child rides along with.
struct Host {
    serial: u32,
    at: [f32; 3],
    /// What each emitter owes this host, as the emitters' own pair does.
    owed: Vec<f32>,
}

/// Where a system's particles were born and where they died, in one step.
#[derive(Default)]
pub struct Events {
    pub born: Vec<[f32; 3]>,
    pub died: Vec<[f32; 3]>,
    /// Every live particle by its number, after the step.
    pub alive: Vec<(u32, [f32; 3])>,
}

impl System {
    /// A system as its file describes it, changed by what the scene says
    /// about this one instance of it.
    pub fn new(definition: &ParticleSystem, over: &InstanceOverride, seed: u32) -> Self {
        let mut start = Start::default();
        for initializer in &definition.initializers {
            match *initializer {
                Initializer::Lifetime(r) => start.lifetime = r,
                Initializer::Size(r) => start.size = r,
                Initializer::Alpha(r) => start.alpha = r,
                Initializer::Color(r) => start.color = r,
                Initializer::Velocity(r) => start.velocity = r,
                Initializer::Rotation(r) => start.rotation = r,
                Initializer::TurbulentVelocity(field) => start.turbulent = Some(field),
                // Carried as a starting value but nothing turns it yet: the
                // operator that spins a particle is `angularmovement`.
                Initializer::AngularVelocity(r) => start.angular = r,
                // Handled below, where the control points it needs are.
                Initializer::AlongControlPoints { .. } => {}
            }
        }
        // On the ranges rather than on each particle, so a system the file
        // gives no alpha or colour at all is changed too.
        let times = |r: Range<f32>, by: f32| Range { min: r.min * by, max: r.max * by };
        let times3 = |r: Range<[f32; 3]>, by: [f32; 3]| Range {
            min: std::array::from_fn(|i| r.min[i] * by[i]),
            max: std::array::from_fn(|i| r.max[i] * by[i]),
        };
        let speed = over.speed.value;
        start.alpha = times(start.alpha, over.alpha.value);
        start.size = times(start.size, over.size.value);
        start.lifetime = times(start.lifetime, over.lifetime.value);
        start.color = times3(start.color, over.tint());
        start.velocity = times3(start.velocity, [speed; 3]);
        if let Some(field) = &mut start.turbulent {
            field.speed = times(field.speed, speed);
        }
        let emitters = definition
            .emitters
            .iter()
            .cloned()
            .map(|mut e| {
                e.rate *= over.rate.value;
                e.speed = times(e.speed, speed);
                (e, 0.0)
            })
            .collect();
        let maxcount = (definition.maxcount as f32 * over.count.value.max(0.0)).round() as usize;
        // The line runs between the first two control points. A system naming
        // the step without having them keeps the emitter's own placing.
        let line = definition
            .initializers
            .iter()
            .find_map(|i| match i {
                Initializer::AlongControlPoints { count } => Some(*count),
                _ => None,
            })
            .zip(
                definition
                    .control_points
                    .first()
                    .zip(definition.control_points.get(1)),
            )
            .map(|(count, (from, to))| Line {
                from: *from,
                to: *to,
                count,
            });
        Self {
            ribbon: match definition.renderer {
                wpe_format::particle::Renderer::Rope { .. } => Ribbon::Chain,
                wpe_format::particle::Renderer::RopeTrail { length, points } => Ribbon::Trail {
                    // The places are spread evenly across the path, so the
                    // first and the last of them are `length` apart.
                    spacing: length / (points - 1).max(1) as f32,
                    points: points as usize,
                },
                _ => Ribbon::None,
            },
            emitters,
            start,
            operators: definition.operators.clone(),
            animation: definition.animation,
            maxcount,
            starttime: definition.starttime,
            live: Vec::new(),
            elapsed: 0.0,
            rng: Rng::seeded(seed),
            vertices: Vec::new(),
            line,
            born: 0,
            pointer: None,
            burst: false,
            waits: false,
            events: None,
            serials: 0,
            hosts: Vec::new(),
            ride_limit: 0,
        }
    }

    /// Keeps where particles are born and die, for a child to answer to.
    pub fn listen(&mut self) {
        self.events = Some(Events::default());
    }

    /// What happened in the last step, for a system that is listened to.
    pub fn events(&self) -> Option<&Events> {
        self.events.as_ref()
    }

    /// Stops the system emitting on its own: it waits to be told where.
    pub fn wait_for_parent(&mut self) {
        self.waits = true;
    }

    /// Lets out every emitter's all-at-once particles at `at`, which is in
    /// the system's own space.
    pub fn burst_at(&mut self, at: [f32; 3], sound: &Spectrum) {
        self.burst(at, None, sound);
    }

    fn burst(&mut self, at: [f32; 3], host: Option<u32>, sound: &Spectrum) {
        let emitters = std::mem::take(&mut self.emitters);
        for (emitter, _) in &emitters {
            for _ in 0..emitter.instantaneous {
                if self.live.len() >= self.maxcount {
                    break;
                }
                self.spawn_at(emitter, at, host, sound);
            }
        }
        self.emitters = emitters;
    }

    /// A particle from `emitter` as if the emitter stood at `at`.
    fn spawn_at(&mut self, emitter: &Emitter, at: [f32; 3], host: Option<u32>, sound: &Spectrum) {
        let mut placed = emitter.clone();
        placed.origin = std::array::from_fn(|i| emitter.origin[i] + at[i]);
        let mut particle = self.spawn(&placed, sound);
        particle.host = host;
        self.take_in(particle);
    }

    /// Makes the system ride along with at most `limit` of its parent's
    /// particles, emitting where each of them is and carried along by it.
    pub fn ride_along(&mut self, limit: usize) {
        self.waits = true;
        self.ride_limit = limit;
    }

    /// Where the parent's particles are now, by number, in this system's own
    /// space. Each particle riding with one is moved as far as its host
    /// moved; one whose host has died goes with it; hosts not yet ridden are
    /// taken up, to the limit, with the emitters' all-at-once particles; and
    /// every host is emitted at as the emitters' rates say.
    pub fn follow(&mut self, hosts: &[(u32, [f32; 3])], dt: f32, sound: &Spectrum) {
        let now = |serial: u32| hosts.iter().find(|(s, _)| *s == serial).map(|(_, at)| *at);
        let mut riding = std::mem::take(&mut self.hosts);
        for particle in &mut self.live {
            let Some(serial) = particle.host else {
                continue;
            };
            let then = riding.iter().find(|h| h.serial == serial).map(|h| h.at);
            match (now(serial), then) {
                (Some(now), Some(then)) => {
                    for axis in 0..3 {
                        particle.position[axis] += now[axis] - then[axis];
                    }
                }
                (Some(_), None) => {}
                (None, _) => particle.age = particle.lifetime,
            }
        }
        riding.retain_mut(|host| match now(host.serial) {
            Some(at) => {
                host.at = at;
                true
            }
            None => false,
        });
        for &(serial, at) in hosts {
            if riding.len() >= self.ride_limit {
                break;
            }
            if riding.iter().any(|h| h.serial == serial) {
                continue;
            }
            riding.push(Host {
                serial,
                at,
                owed: vec![0.0; self.emitters.len()],
            });
            self.burst(at, Some(serial), sound);
        }

        let emitters = std::mem::take(&mut self.emitters);
        for host in &mut riding {
            for ((emitter, _), owed) in emitters.iter().zip(&mut host.owed) {
                *owed += emitter.rate * loudness(emitter.audio, sound) * dt;
                while *owed >= 1.0 {
                    *owed -= 1.0;
                    if self.live.len() >= self.maxcount {
                        break;
                    }
                    self.spawn_at(emitter, host.at, Some(host.serial), sound);
                }
            }
        }
        self.emitters = emitters;
        self.hosts = riding;
    }

    /// A new particle joins the live ones, and is noted for a child that
    /// answers to births.
    fn take_in(&mut self, mut particle: Particle) {
        particle.serial = self.serials;
        self.serials = self.serials.wrapping_add(1);
        if let Some(events) = &mut self.events {
            events.born.push(particle.position);
        }
        self.live.push(particle);
    }

    /// Says where the mouse is, in the system's own space, for the points that
    /// follow it.
    pub fn point_at(&mut self, pointer: Option<[f32; 3]>) {
        self.pointer = pointer;
    }

    pub fn count(&self) -> usize {
        self.live.len()
    }

    /// The most particles this system can have at once, which is how big a
    /// buffer it needs.
    pub fn capacity(&self) -> usize {
        match self.ribbon {
            // Every particle brings a ribbon of its own, so the buffer has to
            // hold a piece for each gap in each of them.
            Ribbon::Trail { points, .. } => self.maxcount * points.saturating_sub(1),
            _ => self.maxcount,
        }
    }

    /// Lowers that cap, for a scene asking for more than it can afford.
    pub fn thin_to(&mut self, capacity: usize) {
        self.maxcount = capacity;
        self.live.truncate(capacity);
    }

    /// The buffer to draw: four vertices per live particle, or four per rope
    /// segment where the system is a ribbon.
    pub fn vertices(&self) -> &[f32] {
        &self.vertices
    }

    /// Whether this system draws as ribbons rather than loose quads.
    pub fn is_rope(&self) -> bool {
        !matches!(self.ribbon, Ribbon::None)
    }

    /// How many four-vertex pieces the buffer holds.
    pub fn pieces(&self) -> usize {
        match self.ribbon {
            Ribbon::None => self.live.len(),
            Ribbon::Chain => self.live.len().saturating_sub(1),
            // A piece between each pair of places kept, for every particle.
            Ribbon::Trail { .. } => self
                .live
                .iter()
                .map(|p| p.trail.len().saturating_sub(1))
                .sum(),
        }
    }

    /// Moves the system on by `dt` seconds and rewrites the buffer.
    /// Runs the system on, with `sound` as what the speakers are playing.
    /// All zeroes is silence, which is what a step that follows the sound
    /// reads when there is nothing to listen to.
    pub fn advance(&mut self, dt: f32, sound: &Spectrum) {
        let dt = dt.clamp(0.0, MAX_STEP);
        self.elapsed += dt;
        if let Some(events) = &mut self.events {
            events.born.clear();
            events.died.clear();
        }

        if self.elapsed >= self.starttime && !self.waits {
            if !self.burst {
                self.burst = true;
                self.burst_at([0.0; 3], sound);
            }
            self.emit(dt, sound);
        }

        let elapsed = self.elapsed;
        let ribbon = self.ribbon;
        let pointer = self.pointer;
        for particle in &mut self.live {
            particle.age += dt;
            for operator in &self.operators {
                match operator {
                    Operator::Movement { gravity, drag } => {
                        for (axis, gravity) in gravity.iter().enumerate() {
                            particle.velocity[axis] += gravity * dt;
                            // Drag as a proportion shed per second, which stays
                            // stable whatever the frame rate and never reverses
                            // the velocity the way `1 - drag * dt` would.
                            particle.velocity[axis] /= 1.0 + drag * dt;
                            particle.position[axis] += particle.velocity[axis] * dt;
                        }
                    }
                    // ponytail: turbulence carries the particle through the
                    // field rather than accelerating it into one, so its
                    // speed is the speed the file states and a long-lived
                    // particle cannot be flung off screen. If a wallpaper
                    // turns out to want building momentum, this is the line
                    // to change.
                    Operator::Turbulence { field, mask } => {
                        let along = field_direction(
                            field,
                            particle.position,
                            particle.phase * field.phase_max,
                            elapsed,
                        );
                        let speed = (field.speed.min
                            + (field.speed.max - field.speed.min) * particle.drift)
                            * loudness(field.audio, sound);
                        for (axis, mask) in mask.iter().enumerate() {
                            particle.position[axis] += along[axis] * mask * speed * dt;
                        }
                    }
                    // A spin only turns once something drives it, which is
                    // what this operator is: the initializer alone sets the
                    // rate and leaves the particle still.
                    Operator::AngularMovement { force } => {
                        for (axis, force) in force.iter().enumerate() {
                            particle.spin[axis] += force * dt;
                            particle.rotation[axis] += particle.spin[axis] * dt;
                        }
                    }
                    // ponytail: the pull falls off in a straight line to
                    // nothing at the reach the file states, and a particle
                    // outside that reach is left alone. The file says nothing
                    // about the shape of it; this is the shape that keeps the
                    // push local, which is what the wallpapers using it want.
                    Operator::Attract {
                        toward,
                        strength,
                        reach,
                        pointer: follows,
                    } => {
                        let at = match (follows, pointer) {
                            (false, _) => *toward,
                            (true, Some(pointer)) => std::array::from_fn(|i| pointer[i] + toward[i]),
                            (true, None) => continue,
                        };
                        let away: [f32; 3] = std::array::from_fn(|i| particle.position[i] - at[i]);
                        let distance =
                            (away[0] * away[0] + away[1] * away[1] + away[2] * away[2]).sqrt();
                        if distance > f32::EPSILON && distance < *reach {
                            let force = strength * (1.0 - distance / reach) * dt;
                            for (axis, away) in away.iter().enumerate() {
                                particle.velocity[axis] -= away / distance * force;
                            }
                        }
                    }
                    // ponytail: a vortex carries the particle round rather
                    // than pulling it in, for the same reason turbulence
                    // does -- a speed the file states stays the speed.
                    Operator::Vortex(vortex) => {
                        let turn = vortex_turn(vortex, particle.position)
                            * loudness(vortex.audio, sound)
                            * dt;
                        particle.position = turned(particle.position, vortex.axis, turn);
                    }
                    // These change nothing that carries over to the next step,
                    // so they are applied as the buffer is written.
                    Operator::AlphaFade { .. }
                    | Operator::SizeChange(_)
                    | Operator::ColorChange { .. }
                    | Operator::OscillateAlpha(_)
                    | Operator::OscillatePosition { .. } => {}
                }
            }
            // Where it has been, kept after the operators have moved it so
            // the path is the one that was actually walked.
            if let Ribbon::Trail { spacing, points } = ribbon {
                particle.since += dt;
                if particle.since >= spacing {
                    // A frame long enough to span several places can only
                    // leave one: the ones in between were never walked.
                    particle.since = match spacing > 0.0 {
                        true => particle.since % spacing,
                        false => 0.0,
                    };
                    if particle.trail.len() >= points {
                        particle.trail.remove(0);
                    }
                    particle.trail.push(particle.position);
                }
            }
        }
        if let Some(events) = &mut self.events {
            let dead = self.live.iter().filter(|p| p.age >= p.lifetime);
            events.died.extend(dead.map(|p| p.position));
        }
        self.live.retain(|p| p.age < p.lifetime);
        if let Some(events) = &mut self.events {
            events.alive.clear();
            events.alive.extend(self.live.iter().map(|p| (p.serial, p.position)));
        }
        self.write_vertices();
    }

    fn emit(&mut self, dt: f32, sound: &Spectrum) {
        // Taken apart so the borrow of the emitters does not hold the whole
        // system while a particle is being built.
        let mut emitters = std::mem::take(&mut self.emitters);
        for (emitter, owed) in &mut emitters {
            *owed += emitter.rate * loudness(emitter.audio, sound) * dt;
            while *owed >= 1.0 {
                *owed -= 1.0;
                if self.live.len() >= self.maxcount {
                    break;
                }
                let particle = self.spawn(emitter, sound);
                self.take_in(particle);
            }
        }
        self.emitters = emitters;
    }

    fn spawn(&mut self, emitter: &Emitter, sound: &Spectrum) -> Particle {
        let direction = self.rng.direction(emitter.directions);
        let position = match emitter.shape {
            Shape::Sphere => {
                let distance = self.rng.range(emitter.distance);
                std::array::from_fn(|i| emitter.origin[i] + direction[i] * distance)
            }
            // A box is filled rather than shelled, so each axis is drawn
            // independently and the masked-out ones collapse to the origin.
            Shape::Box => std::array::from_fn(|i| {
                emitter.origin[i]
                    + self.rng.signed()
                        * emitter.distance.max
                        * emitter.directions[i].signum().abs()
            }),
        };

        // The emitter's own spread stays, as the crackle along the bolt: the
        // discharge preset asks for a sphere of 64 across a line of 724, which
        // would be a setting doing nothing if the line replaced it outright.
        let position = match self.line {
            Some(line) => {
                let along = line.at(self.born);
                self.born = self.born.wrapping_add(1);
                std::array::from_fn(|i| position[i] + along[i])
            }
            None => position,
        };

        let speed = self.rng.range(emitter.speed);
        let drawn = self.rng.range3(self.start.velocity);
        let lifetime = self.rng.range(self.start.lifetime).max(f32::EPSILON);
        let phase = self.rng.next_f32();

        // A turbulent start reads the field once, where the particle appears.
        let turbulent = match self.start.turbulent {
            Some(field) => {
                let along =
                    field_direction(&field, position, phase * field.phase_max, self.elapsed);
                let speed = self.rng.range(field.speed) * loudness(field.audio, sound);
                along.map(|axis| axis * speed)
            }
            None => [0.0; 3],
        };

        Particle {
            trail: match self.ribbon {
                Ribbon::Trail { points, .. } => {
                    let mut path = Vec::with_capacity(points);
                    path.push(position);
                    path
                }
                _ => Vec::new(),
            },
            since: 0.0,
            position,
            // Only six systems in the library set both, so which of the two
            // wins is nearly moot; adding them keeps either on its own right.
            velocity: std::array::from_fn(|i| drawn[i] + direction[i] * speed + turbulent[i]),
            rotation: self.rng.range3(self.start.rotation),
            size: self.rng.range(self.start.size),
            color: self.rng.range3(self.start.color),
            alpha: self.rng.range(self.start.alpha),
            age: 0.0,
            lifetime,
            frame: self.rng.next_f32(),
            phase,
            drift: self.rng.next_f32(),
            spin: self.rng.range3(self.start.angular),
            wobble: std::array::from_fn(|_| self.rng.next_f32()),
            // Both filled in as it joins the live ones.
            serial: 0,
            host: None,
        }
    }

    fn write_vertices(&mut self) {
        match self.ribbon {
            Ribbon::Chain => return self.write_chain(),
            Ribbon::Trail { .. } => return self.write_trails(),
            Ribbon::None => {}
        }
        self.vertices.clear();
        self.vertices
            .reserve(self.live.len() * VERTICES_PER_PARTICLE * FLOATS_PER_VERTEX);
        let elapsed = self.elapsed;
        for particle in &self.live {
            let progress = particle.age / particle.lifetime;
            let mut alpha = particle.alpha * fade(&self.operators, particle.age, particle.lifetime);
            let size = particle.size * size_factor(&self.operators, progress);
            let mut color = particle.color;
            let mut position = particle.position;
            for operator in &self.operators {
                match operator {
                    Operator::ColorChange { time, from, to } => {
                        let t = along(*time, progress);
                        color = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t);
                    }
                    Operator::OscillateAlpha(wave) => {
                        let (amount, swing) = wave_at(wave, particle.wobble, elapsed);
                        // Down from full and back, never past invisible.
                        alpha *= 1.0 - amount * (0.5 - 0.5 * swing);
                    }
                    Operator::OscillatePosition { wave, mask } => {
                        let (amount, swing) = wave_at(wave, particle.wobble, elapsed);
                        for (axis, mask) in mask.iter().enumerate() {
                            position[axis] += mask * amount * swing;
                        }
                    }
                    _ => {}
                }
            }
            // The shader takes the fraction of this, so a multiplier above one
            // walks the sheet more than once over a particle's life.
            let frame = match self.animation {
                Some(Animation::Sequence { multiplier }) => progress * multiplier,
                Some(Animation::RandomFrame) => particle.frame,
                None => progress,
            };
            for corner in CORNERS {
                self.vertices.extend_from_slice(&position);
                self.vertices.extend_from_slice(&[
                    corner[0],
                    corner[1],
                    particle.rotation[2],
                    size,
                ]);
                self.vertices
                    .extend_from_slice(&[color[0], color[1], color[2], alpha]);
                self.vertices
                    .extend_from_slice(&[particle.rotation[0], particle.rotation[1]]);
                self.vertices.extend_from_slice(&particle.velocity);
                self.vertices.push(frame);
            }
        }
    }
}

impl System {
    /// A ribbon for each particle, threaded through where that one has been.
    ///
    /// Each piece carries the whole path's length and its own place in it, as
    /// the chain's pieces carry theirs, so the shader lays the texture along
    /// every trail the same way whatever else is alive.
    fn write_trails(&mut self) {
        self.vertices.clear();
        self.vertices
            .reserve(self.pieces() * VERTICES_PER_PARTICLE * FLOATS_PER_ROPE_VERTEX);
        for particle in &self.live {
            let alpha = particle.alpha * fade(&self.operators, particle.age, particle.lifetime);
            let size =
                particle.size * size_factor(&self.operators, particle.age / particle.lifetime);
            let color = [
                particle.color[0],
                particle.color[1],
                particle.color[2],
                alpha,
            ];
            let length = particle.trail.len() as f32;
            for (piece, pair) in particle.trail.windows(2).enumerate() {
                let (start, end) = (pair[0], pair[1]);
                for corner in CORNERS {
                    self.vertices.extend_from_slice(&start);
                    self.vertices.push(size);
                    self.vertices.extend_from_slice(&end);
                    self.vertices.push(length);
                    // The control points: the piece's own ends, so it stays
                    // straight, as the chain's pieces do.
                    self.vertices.extend_from_slice(&start);
                    self.vertices.push(piece as f32);
                    self.vertices.extend_from_slice(&end);
                    self.vertices.push(size);
                    self.vertices.extend_from_slice(&color);
                    self.vertices.extend_from_slice(&corner);
                    self.vertices.extend_from_slice(&color);
                }
            }
        }
    }

    /// Writes the ribbon: one piece per pair of particles, each carrying the
    /// whole segment so the shader can interpolate across it.
    ///
    /// The control points a spline would bend through are set to the segment's
    /// own ends, which leaves every segment straight. Smoothing them is what
    /// the file's `subdivision` is for and is not done yet.
    fn write_chain(&mut self) {
        self.vertices.clear();
        let pieces = self.pieces();
        self.vertices
            .reserve(pieces * VERTICES_PER_PARTICLE * FLOATS_PER_ROPE_VERTEX);
        let length = self.live.len() as f32;

        for piece in 0..pieces {
            let (start, end) = (&self.live[piece], &self.live[piece + 1]);
            let shown = |p: &Particle| {
                let alpha = p.alpha * fade(&self.operators, p.age, p.lifetime);
                let size = p.size * size_factor(&self.operators, p.age / p.lifetime);
                ([p.color[0], p.color[1], p.color[2], alpha], size)
            };
            let (start_color, start_size) = shown(start);
            let (end_color, end_size) = shown(end);

            for corner in CORNERS {
                self.vertices.extend_from_slice(&start.position);
                self.vertices.push(start_size);
                self.vertices.extend_from_slice(&end.position);
                self.vertices.push(length);
                // The control points: the segment's own ends, so it stays
                // straight.
                self.vertices.extend_from_slice(&start.position);
                self.vertices.push(piece as f32);
                self.vertices.extend_from_slice(&end.position);
                self.vertices.push(end_size);
                self.vertices.extend_from_slice(&end_color);
                self.vertices.extend_from_slice(&corner);
                self.vertices.extend_from_slice(&start_color);
            }
        }
    }
}

/// How far through a ramp a particle is, as a fraction.
fn along(time: Range<f32>, progress: f32) -> f32 {
    let span = time.max - time.min;
    if span > 0.0 {
        ((progress - time.min) / span).clamp(0.0, 1.0)
    } else {
        // A ramp with no span is a step at its start time.
        (progress >= time.min) as u8 as f32
    }
}

/// How far a particle's wave reaches and where it stands in it.
///
/// `wobble` is the particle's own three draws: how fast its wave runs, how far
/// it reaches, and where in the wave it began. Two particles side by side
/// therefore sway apart rather than together.
fn wave_at(wave: &Oscillation, wobble: [f32; 3], time: f32) -> (f32, f32) {
    let pick = |r: Range<f32>, at: f32| r.min + (r.max - r.min) * at;
    let frequency = pick(wave.frequency, wobble[0]);
    let amount = pick(wave.scale, wobble[1]);
    let phase = pick(wave.phase, wobble[2]);
    let turns = time * frequency + phase;
    (amount, (turns * std::f32::consts::TAU).sin())
}

/// Degrees a second a vortex turns a particle at its distance from the centre.
fn vortex_turn(vortex: &Vortex, position: [f32; 3]) -> f32 {
    let distance =
        (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt();
    let span = vortex.distance.max - vortex.distance.min;
    let at = if span > 0.0 {
        ((distance - vortex.distance.min) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    vortex.speed.min + (vortex.speed.max - vortex.speed.min) * at
}

/// A point turned about an axis through the origin, by degrees.
fn turned(point: [f32; 3], axis: [f32; 3], degrees: f32) -> [f32; 3] {
    let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if length < 1e-6 || degrees == 0.0 {
        return point;
    }
    let axis = axis.map(|a| a / length);
    let (sin, cos) = degrees.to_radians().sin_cos();
    let cross = [
        axis[1] * point[2] - axis[2] * point[1],
        axis[2] * point[0] - axis[0] * point[2],
        axis[0] * point[1] - axis[1] * point[0],
    ];
    let dot = axis[0] * point[0] + axis[1] * point[1] + axis[2] * point[2];
    // Rodrigues: the part along the axis stays, the rest turns.
    std::array::from_fn(|i| point[i] * cos + cross[i] * sin + axis[i] * dot * (1.0 - cos))
}

/// What `alphafade` leaves of a particle's alpha: in over its first seconds,
/// out over its last.
fn fade(operators: &[Operator], age: f32, lifetime: f32) -> f32 {
    let mut out = 1.0;
    for operator in operators {
        if let Operator::AlphaFade { fade_in, fade_out } = *operator {
            if fade_in > 0.0 {
                out *= (age / fade_in).min(1.0);
            }
            if fade_out > 0.0 {
                out *= ((lifetime - age) / fade_out).clamp(0.0, 1.0);
            }
        }
    }
    out
}

/// What `sizechange` scales a particle's size by, `progress` being how far
/// through its life it is.
fn size_factor(operators: &[Operator], progress: f32) -> f32 {
    let mut out = 1.0;
    for operator in operators {
        if let Operator::SizeChange(ramp) = operator {
            let t = along(ramp.time, progress);
            out *= ramp.from + (ramp.to - ramp.from) * t;
        }
    }
    out
}

/// Which way a noise field points at a place, ready to be scaled by a speed.
///
/// The field's own samples run from zero to one, which is what the `offset` of
/// -0.5 that every system in the library states is there to centre. The result
/// is normalised, so the speed a file states is the speed a particle gets.
fn field_direction(field: &Turbulent, position: [f32; 3], phase: f32, time: f32) -> [f32; 3] {
    let at = position.map(|x| x * field.scale);
    let sample = flow(at, phase + time * field.time_scale);
    let centred = sample.map(|value| value + field.offset);
    let length =
        (centred[0] * centred[0] + centred[1] * centred[1] + centred[2] * centred[2]).sqrt();
    if length < 1e-6 {
        return [0.0; 3];
    }
    centred.map(|axis| axis / length)
}

/// Three smooth fields, each from zero to one, sampled far enough apart to be
/// independent of one another.
///
/// Value noise rather than anything more elaborate. The format says nothing
/// about which noise Wallpaper Engine uses, and what shows on screen is only
/// that particles near each other drift together and that the drift turns over
/// a distance the file's `scale` sets.
fn flow(at: [f32; 3], phase: f32) -> [f32; 3] {
    /// Far enough apart that the three samples share no corners.
    const APART: f32 = 31.7;
    std::array::from_fn(|axis| {
        let shift = APART * (axis + 1) as f32;
        noise([at[0] + phase + shift, at[1] + shift, at[2] + shift])
    })
}

/// Smooth value noise from zero to one.
fn noise(at: [f32; 3]) -> f32 {
    let cell = at.map(f32::floor);
    // Smoothstep, so the field has no creases where the cells meet.
    let blend: [f32; 3] = std::array::from_fn(|i| {
        let t = at[i] - cell[i];
        t * t * (3.0 - 2.0 * t)
    });
    let corner = cell.map(|v| v as i32);

    let mut value = 0.0;
    for dz in 0..2 {
        for dy in 0..2 {
            for dx in 0..2 {
                let weight = share(blend[0], dx) * share(blend[1], dy) * share(blend[2], dz);
                value += weight * hash(corner[0] + dx, corner[1] + dy, corner[2] + dz);
            }
        }
    }
    value
}

/// How much a corner contributes, given which side of the cell it is on.
fn share(blend: f32, side: i32) -> f32 {
    if side == 0 { 1.0 - blend } else { blend }
}

/// A fixed value per cell corner, from zero to one.
fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Xorshift, which is all a particle needs and keeps the crate dependency-free.
/// Seeded per system so two systems in a scene do not emit in lockstep.
struct Rng(u32);

impl Rng {
    fn seeded(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    /// In `-1.0..1.0`.
    fn signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }

    fn range(&mut self, r: Range<f32>) -> f32 {
        r.min + (r.max - r.min) * self.next_f32()
    }

    fn range3(&mut self, r: Range<[f32; 3]>) -> [f32; 3] {
        std::array::from_fn(|i| r.min[i] + (r.max[i] - r.min[i]) * self.next_f32())
    }

    /// A unit vector, with the axes the emitter does not name left at zero.
    /// That mask is what keeps a flat wallpaper's particles in its plane.
    fn direction(&mut self, mask: [f32; 3]) -> [f32; 3] {
        for _ in 0..8 {
            let v: [f32; 3] = std::array::from_fn(|i| self.signed() * mask[i]);
            let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if length > 1e-3 {
                return v.map(|x| x / length);
            }
        }
        // Every draw landed on the origin, which a zero mask guarantees.
        [0.0; 3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wpe_format::particle::Channels;

    fn system(json: &str) -> System {
        let definition = ParticleSystem::parse(json.as_bytes()).unwrap();
        System::new(&definition, &InstanceOverride::default(), 12345)
    }

    /// Silence is the quiet end of what the step stated, never nothing: a
    /// wallpaper in a silent room still moves, just less.
    #[test]
    fn a_step_that_follows_the_sound_runs_between_its_bounds() {
        let step = Some(AudioResponse {
            bands: (4, 7),
            ears: Channels::Both,
            exponent: 1.0,
            bounds: Range { min: 0.5, max: 1.0 },
        });
        let fill = |level: f32| {
            let mut sound = Spectrum::SILENT;
            sound.left[4..=7].fill(level);
            sound.right[4..=7].fill(level);
            sound
        };

        assert_eq!(loudness(None, &Spectrum::SILENT), 1.0);
        assert_eq!(loudness(step, &Spectrum::SILENT), 0.5);
        assert_eq!(loudness(step, &fill(1.0)), 1.0);
        assert_eq!(loudness(step, &fill(0.5)), 0.75);

        // Sound outside the bands it listens to is not its business.
        let mut elsewhere = Spectrum::SILENT;
        elsewhere.left[0..=3].fill(1.0);
        elsewhere.right[0..=3].fill(1.0);
        assert_eq!(loudness(step, &elsewhere), 0.5);
    }

    /// A step that names one ear gets that ear. The stock effect shaders
    /// carry the same numbers and spell them out.
    #[test]
    fn a_step_hears_only_the_ear_it_asked_for() {
        let mut sound = Spectrum::SILENT;
        sound.left.fill(1.0);
        let step = |ears| {
            Some(AudioResponse {
                bands: (0, 15),
                ears,
                exponent: 1.0,
                bounds: Range { min: 0.0, max: 1.0 },
            })
        };
        assert_eq!(loudness(step(Channels::Left), &sound), 1.0);
        assert_eq!(loudness(step(Channels::Right), &sound), 0.0);
        assert_eq!(loudness(step(Channels::Both), &sound), 0.5);
    }

    /// The exponent shapes the loudness before it is mapped, so a step with a
    /// high one stays near its quiet end until the sound is really loud.
    #[test]
    fn the_exponent_decides_how_soon_a_step_answers() {
        let mut quarter = Spectrum::SILENT;
        quarter.left.fill(0.25);
        quarter.right.fill(0.25);
        let with = |exponent| {
            loudness(
                Some(AudioResponse {
                    bands: (0, 15),
                    ears: Channels::Both,
                    exponent,
                    bounds: Range { min: 0.0, max: 1.0 },
                }),
                &quarter,
            )
        };
        assert!((with(1.0) - 0.25).abs() < 0.001);
        assert!((with(2.0) - 0.0625).abs() < 0.001);
        assert!((with(0.5) - 0.5).abs() < 0.001);
    }

    /// Out to the far point and back the way it came. A walk that jumped back
    /// instead would draw a ribbon across the whole bolt on every lap.
    #[test]
    fn the_walk_along_a_line_turns_round_at_each_end() {
        let line = Line {
            from: [0.0; 3],
            to: [9.0, 0.0, 0.0],
            count: 10,
        };
        let walk: Vec<f32> = (0..20).map(|n| line.at(n)[0]).collect();
        assert_eq!(
            walk,
            [
                0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0,
                2.0, 1.0, 0.0, 1.0,
            ]
        );
    }

    /// The emitter's own spread is the crackle along the bolt, so the line
    /// moves a particle rather than replacing where it was put.
    #[test]
    fn a_line_moves_the_particle_the_emitter_placed() {
        let mut s = system(
            r#"{"material":"m.json","maxcount":64,
                "controlpoint":[{"id":0},{"id":1,"offset":"512 512 0"}],
                "emitter":[{"name":"sphererandom","rate":100,"distancemin":64,
                            "distancemax":64}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"mapsequencebetweencontrolpoints","count":10,
                                "limitbehavior":"mirror"}]}"#,
        );
        s.advance(0.1, &Spectrum::SILENT);
        assert_eq!(s.live.len(), 10);
        // Nothing moves them after they are born, so each sits exactly the
        // emitter's own 64 away from the point on the line it was given.
        let far = |p: &Particle, from: [f32; 3]| {
            let d: [f32; 3] = std::array::from_fn(|i| p.position[i] - from[i]);
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        };
        assert!((far(&s.live[0], [0.0; 3]) - 64.0).abs() < 0.01);
        assert!((far(&s.live[9], [512.0, 512.0, 0.0]) - 64.0).abs() < 0.01);
    }

    /// Steps a system the way a renderer would rather than in one jump, since
    /// one long step is deliberately clamped.
    fn run(system: &mut System, seconds: f32, step: f32) {
        for _ in 0..(seconds / step).round() as usize {
            system.advance(step, &Spectrum::SILENT);
        }
    }

    #[test]
    fn the_emitter_spawns_at_the_rate_it_asks_for() {
        let mut s = system(
            r#"{"maxcount":1000,"emitter":[{"name":"sphererandom","rate":10,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        // Ten a second, give or take the particle the last frame part-owes.
        assert!((9..=10).contains(&s.count()), "spawned {}", s.count());
    }

    #[test]
    fn a_particle_is_retired_when_its_lifetime_runs_out() {
        let mut s = system(
            r#"{"maxcount":100,"emitter":[{"name":"sphererandom","rate":100,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":0.5,"max":0.5}]}"#,
        );
        run(&mut s, 0.5, 1.0 / 60.0);
        let peak = s.count();
        assert!(peak > 0);
        run(&mut s, 1.0, 1.0 / 60.0);
        // Emission and retirement are in balance by now, not still climbing.
        assert!(s.count() <= peak, "{} grew past {peak}", s.count());
    }

    #[test]
    fn the_pool_is_never_bigger_than_the_file_asked_for() {
        let mut s = system(
            r#"{"maxcount":5,"emitter":[{"name":"sphererandom","rate":10000,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        assert_eq!(s.count(), 5);
        assert_eq!(s.vertices().len(), 5 * 4 * FLOATS_PER_VERTEX);
    }

    /// The snow storm a scene dims to half, and a fire it doubles: what the
    /// scene says multiplies what the file says, a system with no alpha of
    /// its own included.
    #[test]
    fn the_scenes_override_multiplies_what_the_file_says() {
        let definition = ParticleSystem::parse(
            br#"{"maxcount":4,"emitter":[{"name":"sphererandom","rate":10,"distancemax":1}],
                 "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        )
        .unwrap();
        let mut over = InstanceOverride::default();
        over.alpha.value = 0.5;
        over.rate.value = 2.0;
        over.count.value = 2.0;
        let mut s = System::new(&definition, &over, 12345);
        run(&mut s, 0.5, 1.0 / 60.0);
        // Twenty a second for half a second is ten, and the pool is eight.
        assert_eq!(s.count(), 8);
        assert!(s.live.iter().all(|p| p.alpha == 0.5));
    }

    #[test]
    fn the_all_at_once_particles_come_once_when_the_system_starts() {
        let mut s = system(
            r#"{"maxcount":100,"emitter":[{"name":"sphererandom","rate":0,"instantaneous":5,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        assert_eq!(s.count(), 5);
    }

    /// A firework's parent dies and the child bursts where it was, and not
    /// before: a child that waits lets nothing out on its own.
    #[test]
    fn a_waiting_child_bursts_where_it_is_told_and_nowhere_else() {
        let mut parent = system(
            r#"{"maxcount":1,"emitter":[{"name":"sphererandom","rate":0,"instantaneous":1,
                 "origin":"100 50 0","distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":0.5,"max":0.5}]}"#,
        );
        let mut child = system(
            r#"{"maxcount":100,"emitter":[{"name":"sphererandom","rate":0,"instantaneous":20,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        parent.listen();
        child.wait_for_parent();
        let mut died = Vec::new();
        for _ in 0..60 {
            parent.advance(1.0 / 60.0, &Spectrum::SILENT);
            died.extend(parent.events().unwrap().died.iter().copied());
            child.advance(1.0 / 60.0, &Spectrum::SILENT);
        }
        assert_eq!(child.count(), 0);
        assert_eq!(died, [[100.0, 50.0, 0.0]]);
        child.burst_at(died[0], &Spectrum::SILENT);
        assert_eq!(child.count(), 20);
        assert!(child.live.iter().all(|p| p.position == [100.0, 50.0, 0.0]));
    }

    /// A shooting star's glow: born with the star, carried with it, gone when
    /// it goes -- and only one star ridden when the link says one.
    #[test]
    fn a_rider_is_carried_by_its_host_and_leaves_with_it() {
        let mut parent = system(
            r#"{"maxcount":2,"emitter":[{"name":"sphererandom","rate":0,"instantaneous":2,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":1,"max":1},
                               {"name":"velocityrandom","min":"100 0 0","max":"100 0 0"}],
                "operator":[{"name":"movement","gravity":"0 0 0"}]}"#,
        );
        let mut child = system(
            r#"{"maxcount":10,"emitter":[{"name":"sphererandom","rate":0,"instantaneous":1,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        parent.listen();
        child.ride_along(1);
        let step = 1.0 / 60.0;
        let mut ride = |parent: &mut System, child: &mut System| {
            parent.advance(step, &Spectrum::SILENT);
            child.follow(&parent.events().unwrap().alive, step, &Spectrum::SILENT);
            child.advance(step, &Spectrum::SILENT);
        };
        for _ in 0..30 {
            ride(&mut parent, &mut child);
        }
        assert_eq!(child.count(), 1);
        let host = parent.live[0].position[0];
        assert!((child.live[0].position[0] - host).abs() < 1e-3, "{} against {host}", child.live[0].position[0]);
        for _ in 0..40 {
            ride(&mut parent, &mut child);
        }
        assert_eq!(parent.count(), 0);
        assert_eq!(child.count(), 0);
    }

    #[test]
    fn nothing_is_emitted_before_the_systems_start_time() {
        let mut s = system(
            r#"{"maxcount":100,"starttime":2,
                "emitter":[{"name":"sphererandom","rate":50,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        assert_eq!(s.count(), 0);
        run(&mut s, 2.0, 1.0 / 60.0);
        assert!(s.count() > 0);
    }

    #[test]
    fn a_masked_axis_keeps_its_particles_in_the_plane() {
        let mut s = system(
            r#"{"maxcount":50,"emitter":[{"name":"sphererandom","rate":500,
                 "directions":"1 1 0","distancemin":10,"distancemax":100}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"sizerandom","min":1,"max":1}]}"#,
        );
        run(&mut s, 0.5, 1.0 / 60.0);
        assert!(s.count() > 10);
        for p in &s.live {
            assert_eq!(p.position[2], 0.0);
            let flat = (p.position[0].powi(2) + p.position[1].powi(2)).sqrt();
            assert!((10.0..=100.0).contains(&flat), "spawned at {flat} out");
        }
    }

    #[test]
    fn gravity_pulls_a_particle_the_way_it_points() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"movement","gravity":"0 -100 0"}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        assert_eq!(s.count(), 1);
        let y = s.live[0].position[1];
        // Half of g t squared, with the first frame's worth of slack.
        assert!((-52.0..=-48.0).contains(&y), "fell to {y}");
    }

    #[test]
    fn drag_slows_a_particle_without_turning_it_round() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"velocityrandom","min":"100 0 0","max":"100 0 0"}],
                "operator":[{"name":"movement","drag":5}]}"#,
        );
        run(&mut s, 2.0, 1.0 / 60.0);
        let p = &s.live[0];
        assert!(
            p.velocity[0] > 0.0,
            "velocity reversed to {}",
            p.velocity[0]
        );
        assert!(p.velocity[0] < 20.0, "barely slowed: {}", p.velocity[0]);
        assert!(p.position[0] > 0.0);
    }

    /// The alpha a particle is written to the buffer with, which is where the
    /// fade is applied.
    fn written_alpha(s: &System) -> f32 {
        s.vertices()[7 + 3]
    }

    #[test]
    fn alphafade_starts_a_particle_invisible_and_brings_it_in() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"alphafade","fadeintime":1}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        assert_eq!(s.count(), 1);
        let early = written_alpha(&s);
        assert!(early < 0.1, "born at {early} instead of nearly nothing");
        run(&mut s, 1.0, 1.0 / 60.0);
        assert!((written_alpha(&s) - 1.0).abs() < 0.01);
    }

    #[test]
    fn alphafade_takes_a_particle_out_before_it_dies() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":1,"max":1},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"alphafade","fadeouttime":0.5}]}"#,
        );
        run(&mut s, 0.9, 1.0 / 60.0);
        let late = written_alpha(&s);
        assert!(late < 0.3, "still at {late} with a tenth of its life left");
    }

    #[test]
    fn sizechange_drives_the_size_the_buffer_carries() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":1,"max":1},
                               {"name":"sizerandom","min":100,"max":100}],
                "operator":[{"name":"sizechange","starttime":0,"endtime":1,
                             "startvalue":1,"endvalue":3}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        let size = |s: &System| s.vertices()[3 + 3];
        assert!((size(&s) - 100.0).abs() < 5.0, "started at {}", size(&s));
        run(&mut s, 0.5, 1.0 / 60.0);
        assert!((size(&s) - 200.0).abs() < 10.0, "halfway at {}", size(&s));
    }

    #[test]
    fn the_field_stays_between_zero_and_one_and_never_jumps() {
        let mut last = None;
        for step in 0..200 {
            let at = [step as f32 * 0.05, 1.0, -2.0];
            let here = flow(at, 0.0);
            for axis in 0..3 {
                assert!(
                    (0.0..=1.0).contains(&here[axis]),
                    "field left its range at {at:?}: {here:?}"
                );
                // Each step is a twentieth of a cell, so a smooth field cannot
                // have moved far. A crease would show up here as a jump.
                if let Some(last) = last {
                    let last: [f32; 3] = last;
                    assert!(
                        (here[axis] - last[axis]).abs() < 0.25,
                        "field jumped at {at:?}: {last:?} to {here:?}"
                    );
                }
            }
            last = Some(here);
        }
    }

    #[test]
    fn the_field_points_different_ways_in_different_places() {
        let field = Turbulent {
            speed: Range { min: 1.0, max: 1.0 },
            scale: 0.002,
            time_scale: 0.0,
            phase_max: 0.0,
            offset: -0.5,
            audio: None,
        };
        let here = field_direction(&field, [0.0; 3], 0.0, 0.0);
        let far = field_direction(&field, [4000.0, 0.0, 0.0], 0.0, 0.0);
        assert!(
            here != far,
            "the same direction eight cells apart is not a field"
        );
        let length = (here[0] * here[0] + here[1] * here[1] + here[2] * here[2]).sqrt();
        assert!(
            (length - 1.0).abs() < 1e-3,
            "direction was not a unit: {length}"
        );
    }

    #[test]
    fn turbulence_carries_a_particle_no_faster_than_it_says() {
        let mut s = system(
            r#"{"maxcount":20,"emitter":[{"name":"boxrandom","rate":100,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"turbulence","scale":0.002,"speedmin":100,
                             "speedmax":150,"phasemax":5}]}"#,
        );
        run(&mut s, 2.0, 1.0 / 60.0);
        let mut moved = 0;
        for p in &s.live {
            let travelled = (p.position[0].powi(2) + p.position[1].powi(2)).sqrt();
            // Two seconds at 150 a second, and it started at the origin.
            assert!(travelled <= 300.0, "drifted {travelled} in two seconds");
            if travelled > 1.0 {
                moved += 1;
            }
        }
        assert!(moved > 0, "turbulence moved nothing at all");
    }

    #[test]
    fn turbulence_leaves_the_axes_its_mask_omits_alone() {
        let mut s = system(
            r#"{"maxcount":20,"emitter":[{"name":"boxrandom","rate":100,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"turbulence","scale":0.002,"speedmin":100,
                             "speedmax":150,"mask":"1 0 0","phasemax":5}]}"#,
        );
        run(&mut s, 2.0, 1.0 / 60.0);
        assert!(s.live.iter().any(|p| p.position[0].abs() > 1.0));
        for p in &s.live {
            assert_eq!(p.position[1], 0.0, "masked axis moved");
            assert_eq!(p.position[2], 0.0, "masked axis moved");
        }
    }

    #[test]
    fn two_particles_in_one_place_are_not_pushed_as_one() {
        let mut s = system(
            r#"{"maxcount":30,"emitter":[{"name":"boxrandom","rate":200,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"turbulence","scale":0.002,"speedmin":100,
                             "speedmax":150,"phasemax":5}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        let first = s.live[0].position;
        assert!(
            s.live.iter().any(|p| p.position != first),
            "every particle went the same way; the phase is not reaching the field"
        );
    }

    #[test]
    fn a_turbulent_start_sets_off_at_the_speed_it_asks_for() {
        let mut s = system(
            r#"{"maxcount":20,"emitter":[{"name":"sphererandom","rate":100,"distancemax":500}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"turbulentvelocityrandom","scale":0.1,
                                "speedmin":250,"speedmax":260,"offset":-0.5}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        assert!(s.count() > 0);
        for p in &s.live {
            let speed =
                (p.velocity[0].powi(2) + p.velocity[1].powi(2) + p.velocity[2].powi(2)).sqrt();
            assert!(
                (250.0..=260.0).contains(&speed),
                "set off at {speed}, not the speed the file states"
            );
        }
    }

    #[test]
    fn a_vortex_carries_a_particle_round_without_pulling_it_in() {
        let mut s = system(
            r#"{"maxcount":20,"emitter":[{"name":"sphererandom","rate":200,
                 "directions":"1 1 0","distancemin":200,"distancemax":200}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"vortex","axis":"0 0 1","distanceinner":0,
                             "distanceouter":500,"speedinner":90,"speedouter":90}]}"#,
        );
        let start: Vec<[f32; 3]> = {
            s.advance(1.0 / 60.0, &Spectrum::SILENT);
            s.live.iter().map(|p| p.position).collect()
        };
        run(&mut s, 1.0, 1.0 / 60.0);
        let mut turned_by = Vec::new();
        for (before, particle) in start.iter().zip(&s.live) {
            let after = particle.position;
            let radius = |p: &[f32; 3]| (p[0] * p[0] + p[1] * p[1]).sqrt();
            // A swirl keeps the distance it found the particle at.
            assert!(
                (radius(before) - radius(&after)).abs() < 1.0,
                "the vortex moved a particle from {} to {}",
                radius(before),
                radius(&after)
            );
            let angle = |p: &[f32; 3]| p[1].atan2(p[0]).to_degrees();
            let mut moved = angle(&after) - angle(before);
            if moved < -180.0 {
                moved += 360.0;
            }
            turned_by.push(moved);
        }
        assert!(!turned_by.is_empty());
        for moved in turned_by {
            // Ninety degrees a second, for about a second.
            assert!((80.0..=95.0).contains(&moved), "turned {moved} degrees");
        }
    }

    #[test]
    fn a_vortex_turns_the_other_way_when_its_speed_is_negative() {
        let positive = r#"{"maxcount":1,"emitter":[{"name":"sphererandom","rate":100,
                 "directions":"1 1 0","distancemin":100,"distancemax":100}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"vortex","axis":"0 0 1","distanceouter":500,
                             "speedinner":SPEED,"speedouter":SPEED}]}"#;
        let turn = |speed: &str| {
            let mut s = system(&positive.replace("SPEED", speed));
            s.advance(1.0 / 60.0, &Spectrum::SILENT);
            let before = s.live[0].position;
            run(&mut s, 0.5, 1.0 / 60.0);
            let after = s.live[0].position;
            after[1].atan2(after[0]) - before[1].atan2(before[0])
        };
        assert!(turn("90") > 0.0);
        assert!(turn("-90") < 0.0);
    }

    #[test]
    fn oscillatealpha_dims_a_particle_and_brings_it_back() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"oscillatealpha","frequencymin":1,"frequencymax":1,
                             "scalemin":0.8,"scalemax":0.8}]}"#,
        );
        let mut seen: Vec<f32> = Vec::new();
        for _ in 0..90 {
            s.advance(1.0 / 60.0, &Spectrum::SILENT);
            seen.push(written_alpha(&s));
        }
        let low = seen.iter().cloned().fold(f32::MAX, f32::min);
        let high = seen.iter().cloned().fold(f32::MIN, f32::max);
        assert!(high > 0.95, "never came back up: {high}");
        // Dimmed by at most the scale it states, and never to nothing.
        assert!((0.15..0.35).contains(&low), "dimmed to {low}");
    }

    #[test]
    fn oscillateposition_sways_a_particle_about_where_it_sits() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}],
                "operator":[{"name":"oscillateposition","frequencymin":1,"frequencymax":1,
                             "scalemin":30,"scalemax":30,"mask":"1 0 0"}]}"#,
        );
        let mut xs: Vec<f32> = Vec::new();
        for _ in 0..90 {
            s.advance(1.0 / 60.0, &Spectrum::SILENT);
            xs.push(s.vertices()[0]);
        }
        let low = xs.iter().cloned().fold(f32::MAX, f32::min);
        let high = xs.iter().cloned().fold(f32::MIN, f32::max);
        assert!(high > 20.0 && low < -20.0, "swayed {low} to {high}");
        assert!(high <= 30.1 && low >= -30.1, "swayed past its scale");
        // The particle itself never left the origin; only its drawing sways.
        assert_eq!(s.live[0].position, [0.0; 3]);
    }

    #[test]
    fn colorchange_drives_the_colour_the_buffer_carries() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":1,"max":1},
                               {"name":"colorrandom","min":"255 255 255","max":"255 255 255"}],
                "operator":[{"name":"colorchange","starttime":0,"endtime":1,
                             "startvalue":"1 0 0","endvalue":"0 0 1"}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        let red = &s.vertices()[7..10];
        assert!(red[0] > 0.9 && red[2] < 0.1, "started at {red:?}");
        run(&mut s, 0.5, 1.0 / 60.0);
        let middle = &s.vertices()[7..10];
        assert!(
            (0.3..0.7).contains(&middle[0]) && (0.3..0.7).contains(&middle[2]),
            "halfway at {middle:?}"
        );
    }

    #[test]
    fn angularmovement_is_what_turns_a_particle() {
        let spinning = r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"angularvelocityrandom","min":"0 0 2","max":"0 0 2"}]
                OPERATOR}"#;
        // Without the operator the starting rate is carried but nothing turns.
        let mut still = system(&spinning.replace("OPERATOR", ""));
        run(&mut still, 1.0, 1.0 / 60.0);
        assert_eq!(still.live[0].rotation, [0.0; 3]);

        let mut turning =
            system(&spinning.replace("OPERATOR", r#","operator":[{"name":"angularmovement"}]"#));
        run(&mut turning, 1.0, 1.0 / 60.0);
        // Two radians a second, for about a second.
        assert!(
            (1.9..2.1).contains(&turning.live[0].rotation[2]),
            "turned to {}",
            turning.live[0].rotation[2]
        );
    }

    #[test]
    fn a_rope_joins_its_particles_into_one_ribbon_oldest_first() {
        let mut s = system(
            r#"{"maxcount":8,"renderer":[{"name":"rope","subdivision":3}],
                "emitter":[{"name":"boxrandom","rate":60,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"sizerandom","min":10,"max":10},
                               {"name":"velocityrandom","min":"100 0 0","max":"100 0 0"}],
                "operator":[{"name":"movement"}]}"#,
        );
        assert!(s.is_rope());
        run(&mut s, 0.2, 1.0 / 60.0);
        assert!(s.count() >= 3, "only {} particles", s.count());

        // One piece per pair, and nothing left over.
        assert_eq!(s.pieces(), s.count() - 1);
        assert_eq!(
            s.vertices().len(),
            s.pieces() * 4 * FLOATS_PER_ROPE_VERTEX,
            "the buffer does not match the pieces"
        );

        // The first piece runs from the oldest particle to the next, which is
        // what makes the ribbon trace the path they drifted along.
        let oldest = s.live[0].position;
        let next = s.live[1].position;
        assert_eq!(&s.vertices()[0..3], &oldest);
        assert_eq!(&s.vertices()[4..7], &next);
        // Every vertex of a piece carries the whole segment, so the shader can
        // read across it; only the corner differs.
        let stride = FLOATS_PER_ROPE_VERTEX;
        assert_eq!(&s.vertices()[0..3], &s.vertices()[stride..stride + 3]);
        assert_eq!(
            &s.vertices()[20..22],
            &[0.0, 0.0],
            "the first corner should be the near one"
        );
        assert_eq!(&s.vertices()[stride + 20..stride + 22], &[1.0, 0.0]);
        // The rope states its own length and where each piece sits along it.
        assert_eq!(s.vertices()[7], s.count() as f32);
        assert_eq!(s.vertices()[11], 0.0);
        assert_eq!(s.vertices()[stride * 4 + 11], 1.0, "second piece");
    }

    #[test]
    fn a_rope_with_one_particle_has_nothing_to_join() {
        let mut s = system(
            r#"{"maxcount":1,"renderer":[{"name":"rope"}],
                "emitter":[{"name":"boxrandom","rate":60,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 0.1, 1.0 / 60.0);
        assert_eq!(s.count(), 1);
        assert_eq!(s.pieces(), 0);
        assert!(s.vertices().is_empty());
    }

    /// A rope trail keeps a path per particle: it stops growing at the count
    /// the file states, and the buffer never outgrows what was reserved for it.
    #[test]
    fn a_rope_trail_keeps_a_bounded_path_per_particle() {
        let mut s = system(
            r#"{"maxcount":3,"renderer":[{"name":"ropetrail","length":0.2,"segments":5}],
                "emitter":[{"name":"boxrandom","rate":60,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"velocityrandom","min":"0 100 0","max":"0 100 0"}],
                "operator":[{"name":"movement"}]}"#,
        );
        // Four gaps across five places, for each of three particles.
        assert_eq!(s.capacity(), 3 * 4);

        run(&mut s, 2.0, 1.0 / 60.0);
        assert_eq!(s.count(), 3);
        for particle in &s.live {
            assert_eq!(
                particle.trail.len(),
                5,
                "the path should be full but capped"
            );
        }
        assert_eq!(s.pieces(), s.capacity());
        assert_eq!(
            s.vertices().len(),
            s.pieces() * VERTICES_PER_PARTICLE * FLOATS_PER_ROPE_VERTEX
        );

        // The path is where the particle has been, not where it is: rising at
        // a hundred a second, its places stand apart along the way it came.
        let oldest = s.live[0].trail[0];
        let newest = *s.live[0].trail.last().unwrap();
        assert!(
            newest[1] > oldest[1],
            "{newest:?} should be above {oldest:?}"
        );
    }

    #[test]
    fn a_frame_so_long_it_would_empty_the_pool_is_capped() {
        let mut s = system(
            r#"{"maxcount":10000,"emitter":[{"name":"sphererandom","rate":1000,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        // A stalled frame, as if the wallpaper had been paused for a minute.
        s.advance(60.0, &Spectrum::SILENT);
        assert!(s.count() <= 100, "one frame emitted {}", s.count());
    }

    #[test]
    fn every_live_particle_reaches_the_buffer() {
        let mut s = system(
            r#"{"maxcount":100,"emitter":[{"name":"sphererandom","rate":60,"distancemax":10}],
                "initializer":[{"name":"lifetimerandom","min":1,"max":2},
                               {"name":"sizerandom","min":10,"max":20},
                               {"name":"colorrandom","min":"255 0 0","max":"255 0 0"}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        assert_eq!(
            s.vertices().len(),
            s.count() * 4 * FLOATS_PER_VERTEX,
            "buffer does not match the pool"
        );
        // The colour the initializer asked for, on the 0-1 scale the shader
        // wants rather than the 0-255 the file used.
        assert_eq!(&s.vertices()[7..10], &[1.0, 0.0, 0.0]);
    }
}
