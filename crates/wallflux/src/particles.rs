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
    Anchor, Animation, AudioResponse, Boids, Collision, CollisionBehavior, CollisionShape, Emitter,
    Exponents, Inherit, Initializer, Operator, Oscillation, ParticleSystem, Range, Remap,
    RemapInput, RemapOperation, RemapOutput, RemapTransform, Shape, Spectrum, Turbulent, Vortex,
};
use wpe_format::scene::InstanceOverride;

/// What a step's own strength is multiplied by, given what is playing.
///
/// A step that does not follow the sound is left alone. One that does stands
/// still in a silent room: the bounds are which part of the loudness it hears,
/// nothing below the first and everything above the second, and the exponent
/// comes after. The stock shaders spell it out in `CreateAudioResponse`, and
/// DOOM agrees -- read as a range of strengths, its torches and its 15 000
/// sparks a second ran flat out in silence, where Wallpaper Engine shows none.
fn loudness(audio: Option<AudioResponse>, sound: &Spectrum) -> f32 {
    let Some(audio) = audio else {
        return 1.0;
    };
    let span = (audio.bounds.max - audio.bounds.min).max(f32::EPSILON);
    let t = ((sound.across(audio.bands, audio.ears) - audio.bounds.min) / span).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t)).powf(audio.exponent).clamp(0.0, 1.0)
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

/// How far ahead a system may start, and in how fine a step it gets there.
const MOST_AHEAD: f32 = 30.0;
const AHEAD_STEPS: f32 = 30.0;

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
    /// What is drawn this step, where a remap or a parent has had its say;
    /// the fields above are what it was born with.
    shown_size: f32,
    shown_alpha: f32,
    shown_color: [f32; 3],
    /// What the step's movement is multiplied by, from a remap of its speed.
    pace: f32,
    /// Where it was a step before this one started, so a rectangle sees it
    /// cross whichever side of the movement the file puts the two.
    previous: [f32; 3],
}

/// One of a system's particles as a child sees it.
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color: [f32; 3],
    pub alpha: f32,
    pub size: f32,
    pub rotation: [f32; 3],
    pub spin: [f32; 3],
}

impl Snapshot {
    /// Somewhere, with nothing else to say about it.
    #[cfg(test)]
    fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            velocity: [0.0; 3],
            color: [1.0; 3],
            alpha: 1.0,
            size: 1.0,
            rotation: [0.0; 3],
            spin: [0.0; 3],
        }
    }

    fn of(p: &Particle) -> Self {
        Self {
            position: p.position,
            velocity: p.velocity,
            color: p.shown_color,
            alpha: p.shown_alpha,
            size: p.shown_size,
            rotation: p.rotation,
            spin: p.spin,
        }
    }
}

/// Applies what a child takes from its parent's particle. Size and opacity
/// reach what is drawn; called on a new particle, that is what it is born with
/// too, since `spawn` copies one to the other afterwards.
fn inherit(particle: &mut Particle, what: Inherit, from: &Snapshot) {
    let times = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| a[i] * b[i]);
    let plus = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| a[i] + b[i]);
    match what {
        Inherit::Color { multiply: false } => particle.shown_color = from.color,
        Inherit::Color { multiply: true } => {
            particle.shown_color = times(particle.shown_color, from.color)
        }
        Inherit::Opacity { multiply: false } => particle.shown_alpha = from.alpha,
        Inherit::Opacity { multiply: true } => particle.shown_alpha *= from.alpha,
        Inherit::ColorOpacity { multiply } => {
            inherit(particle, Inherit::Color { multiply }, from);
            inherit(particle, Inherit::Opacity { multiply }, from);
        }
        Inherit::Velocity { add: false } => particle.velocity = from.velocity,
        Inherit::Velocity { add: true } => particle.velocity = plus(particle.velocity, from.velocity),
        Inherit::Size { multiply: false } => particle.shown_size = from.size,
        Inherit::Size { multiply: true } => particle.shown_size *= from.size,
        Inherit::Rotation { add: false } => particle.rotation = from.rotation,
        Inherit::Rotation { add: true } => particle.rotation = plus(particle.rotation, from.rotation),
        Inherit::AngularVelocity { add: false } => particle.spin = from.spin,
        Inherit::AngularVelocity { add: true } => particle.spin = plus(particle.spin, from.spin),
    }
}

/// What a remap reads that is not the particle's own.
struct Moment {
    elapsed: f32,
    time_of_day: f32,
    pointer: Option<[f32; 3]>,
}

impl Moment {
    /// Where a control point is, if it can be said: one on the mouse has no
    /// place while there is no mouse.
    fn place(&self, anchor: Anchor) -> Option<[f32; 3]> {
        match (anchor.pointer, self.pointer) {
            (false, _) => Some(anchor.at),
            (true, Some(pointer)) => Some(std::array::from_fn(|i| pointer[i] + anchor.at[i])),
            (true, None) => None,
        }
    }
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn minus(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

/// What a remap reads off a particle, one number standing for every axis.
fn remap_input(remap: &Remap, particle: &Particle, moment: &Moment) -> Option<[f32; 3]> {
    let point = moment.place(remap.points[0]);
    Some(match remap.input {
        RemapInput::LifetimeFraction => [particle.age / particle.lifetime; 3],
        RemapInput::MaxLifetime => [particle.lifetime; 3],
        // Twice what is drawn is the number the file's own sizes are in.
        RemapInput::Size => [particle.shown_size * 2.0; 3],
        RemapInput::Opacity => [particle.shown_alpha; 3],
        RemapInput::Speed => [length(particle.velocity); 3],
        RemapInput::Rotation => [particle.rotation[2]; 3],
        RemapInput::AngularSpeed => [particle.spin[2]; 3],
        RemapInput::DistanceToControlPoint => [length(minus(particle.position, point?)); 3],
        RemapInput::PositionBetweenControlPoints => {
            let (from, to) = (point?, moment.place(remap.points[1])?);
            let line = minus(to, from);
            let along = dot(line, line);
            match along > f32::EPSILON {
                true => [dot(minus(particle.position, from), line) / along; 3],
                false => [0.0; 3],
            }
        }
        RemapInput::Time => [moment.elapsed; 3],
        RemapInput::TimeOfDay => [moment.time_of_day; 3],
        RemapInput::Color => particle.shown_color,
        RemapInput::Position => particle.position,
        RemapInput::Velocity => particle.velocity,
        RemapInput::ControlPoint => point?,
        RemapInput::DeltaToControlPoint => minus(point?, particle.position),
        RemapInput::DirectionToControlPoint => {
            let delta = minus(point?, particle.position);
            let long = length(delta);
            if long > f32::EPSILON { delta.map(|c| c / long) } else { [0.0; 3] }
        }
        RemapInput::LayerOrigin => [0.0; 3],
    })
}

/// A fraction put through the remap's shape, nought to one. Only a remap with
/// no shape stops at the ends of its range: a wave over a clock has to go on
/// after the first second, and Valve's thunderbolt pulses on one that way.
fn remap_shape(remap: &Remap, fraction: f32, axis: usize, phase: f32) -> f32 {
    let at = fraction * remap.transform_scale;
    // Each axis its own stretch of noise, or a vector would only move on the
    // diagonal; each particle its own where the flags say so.
    let noise_at = |x: f32| {
        noise([x, axis as f32 * 17.0, if remap.per_particle { phase * 251.0 } else { 0.0 }])
    };
    match remap.transform {
        RemapTransform::None => fraction.clamp(0.0, 1.0),
        RemapTransform::Sine => 0.5 + 0.5 * (at * std::f32::consts::TAU).sin(),
        RemapTransform::Square => (at.rem_euclid(1.0) < 0.5) as u8 as f32,
        RemapTransform::Saw => at.rem_euclid(1.0),
        RemapTransform::Triangle => 1.0 - (at.rem_euclid(1.0) * 2.0 - 1.0).abs(),
        RemapTransform::Simplex => noise_at(at),
        RemapTransform::Fbm => {
            let (mut sum, mut most, mut weight, mut reach) = (0.0, 0.0, 1.0, at);
            for _ in 0..remap.transform_octaves {
                sum += weight * noise_at(reach);
                most += weight;
                weight *= 0.5;
                reach *= 2.0;
            }
            sum / most
        }
    }
}

/// How much of a remap applies this far through a particle's life, by the
/// blends it states, nought to one.
fn remap_weight(remap: &Remap, progress: f32) -> f32 {
    let coming = remap.blend_in.map_or(1.0, |range| along(range, progress));
    let going = remap.blend_out.map_or(1.0, |range| 1.0 - along(range, progress));
    coming * going
}

/// One remap, reading the particle and writing it back. `born` is whether the
/// particle is being made, when what it is born with is what is written.
///
/// ponytail: the engine's update for this is inside a 49 kB function the
/// decompiler gives up on, so its defaults are read and its working is the
/// editor's description of it. Compare against Wallpaper Engine once a wallpaper
/// in the library uses one -- none does today; Valve's rain and thunderbolt do.
fn apply_remap(remap: &Remap, particle: &mut Particle, moment: &Moment, born: bool) {
    let Some(input) = remap_input(remap, particle, moment) else {
        return;
    };
    let span = remap.input_range.max - remap.input_range.min;
    let value: [f32; 3] = std::array::from_fn(|axis| {
        let read = remap.input_component.map_or(input[axis], |c| input[c]);
        let fraction = if span != 0.0 { (read - remap.input_range.min) / span } else { 0.0 };
        let shaped = remap_shape(remap, fraction, axis, particle.phase);
        let (low, high) = (remap.output_range.min[axis], remap.output_range.max[axis]);
        low + (high - low) * shaped
    });
    let weight = remap_weight(remap, particle.age / particle.lifetime);
    let meet = |now: f32, value: f32| match remap.operation {
        RemapOperation::Assign => now + (value - now) * weight,
        RemapOperation::Multiply => now * (1.0 + (value - 1.0) * weight),
        RemapOperation::Add => now + value * weight,
        RemapOperation::Subtract => now - value * weight,
    };
    let each = |target: &mut [f32; 3]| {
        for axis in 0..3 {
            if remap.output_component.is_none_or(|c| c == axis) {
                target[axis] = meet(target[axis], value[axis]);
            }
        }
    };
    match remap.output {
        RemapOutput::Lifetime if born => particle.lifetime = meet(particle.lifetime, value[0]).max(f32::EPSILON),
        RemapOutput::Lifetime => {}
        // A file's sizes are twice what is drawn, and only an assignment or a
        // sum is in the file's units; a multiplier has none.
        RemapOutput::Size => {
            let value = match remap.operation {
                RemapOperation::Multiply => value[0],
                _ => value[0] / 2.0,
            };
            particle.shown_size = meet(particle.shown_size, value).max(0.0);
        }
        RemapOutput::Opacity => particle.shown_alpha = meet(particle.shown_alpha, value[0]),
        RemapOutput::Color => each(&mut particle.shown_color),
        RemapOutput::Position => each(&mut particle.position),
        RemapOutput::Velocity => each(&mut particle.velocity),
        RemapOutput::Rotation => each(&mut particle.rotation),
        RemapOutput::AngularSpeed => each(&mut particle.spin),
        // A multiplier is on the step's movement rather than on the velocity,
        // where one every step would compound; nought is as slow as it goes,
        // which is what Valve's drops on a window do between their runs.
        RemapOutput::Speed => match remap.operation {
            RemapOperation::Multiply => particle.pace = meet(particle.pace, value[0]).max(0.0),
            _ => {
                let now = length(particle.velocity);
                if now > f32::EPSILON {
                    let then = meet(now, value[0]).max(0.0);
                    particle.velocity = particle.velocity.map(|v| v / now * then);
                }
            }
        },
    }
}

/// Stops a particle at what it ran into, if it did.
fn collide(particle: &mut Particle, collision: &Collision, previous: [f32; 3], scene: Option<Range<[f32; 3]>>) {
    let struck = |particle: &mut Particle, normal: [f32; 3], depth: f32| {
        match collision.behavior {
            CollisionBehavior::Delete => particle.age = particle.lifetime,
            behavior => {
                for axis in 0..3 {
                    particle.position[axis] += normal[axis] * depth;
                }
                let into = dot(particle.velocity, normal);
                if into < 0.0 {
                    let back = match behavior {
                        CollisionBehavior::Bounce => 1.0 + collision.bounce,
                        _ => 1.0,
                    };
                    for axis in 0..3 {
                        particle.velocity[axis] -= normal[axis] * into * back;
                    }
                }
            }
        }
    };
    match collision.shape {
        CollisionShape::Plane { normal, distance } => {
            let depth = distance - dot(particle.position, normal);
            if depth > 0.0 {
                struck(particle, normal, depth);
            }
        }
        CollisionShape::Sphere { origin, radius } => {
            let away = minus(particle.position, origin);
            let long = length(away);
            if long < radius && long > f32::EPSILON {
                struck(particle, away.map(|c| c / long), radius - long);
            }
        }
        // Only on the way through from in front, and only within its edges.
        CollisionShape::Quad { origin, size, normal } => {
            let was = dot(minus(previous, origin), normal);
            let is = dot(minus(particle.position, origin), normal);
            let off = minus(particle.position, origin);
            let side = if normal[0].abs() > 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
            let width = cross(normal, side);
            let height = cross(normal, width);
            if was >= 0.0
                && is < 0.0
                && dot(off, width).abs() <= size[0] / 2.0
                && dot(off, height).abs() <= size[1] / 2.0
            {
                struck(particle, normal, -is);
            }
        }
        CollisionShape::Bounds => {
            let Some(scene) = scene else { return };
            for axis in 0..2 {
                let mut normal = [0.0; 3];
                if particle.position[axis] < scene.min[axis] {
                    normal[axis] = 1.0;
                    let depth = scene.min[axis] - particle.position[axis];
                    struck(particle, normal, depth);
                } else if particle.position[axis] > scene.max[axis] {
                    normal[axis] = -1.0;
                    let depth = particle.position[axis] - scene.max[axis];
                    struck(particle, normal, depth);
                }
            }
        }
    }
}

/// Where a step of `boids` sends each particle, by the velocity to add.
///
/// ponytail: every particle against every other, thinned to a slice of them
/// each step past 512 -- the engine's changelog says it skips too. A grid is
/// the next step if a flock of thousands ever turns up.
fn flock(live: &[Particle], boids: &Boids, step: u32, dt: f32) -> Vec<[f32; 3]> {
    let every = live.len() / 512 + 1;
    live.iter()
        .enumerate()
        .map(|(index, one)| {
            if (index + step as usize) % every != 0 {
                return [0.0; 3];
            }
            let (mut count, mut center, mut heading, mut apart) = (0.0, [0.0; 3], [0.0; 3], [0.0; 3]);
            for (other_index, other) in live.iter().enumerate() {
                if other_index == index {
                    continue;
                }
                let away = minus(one.position, other.position);
                let long = length(away);
                if long >= boids.neighbor_threshold {
                    continue;
                }
                count += 1.0;
                for axis in 0..3 {
                    center[axis] += other.position[axis];
                    heading[axis] += other.velocity[axis];
                    if long < boids.separation_threshold && long > f32::EPSILON {
                        apart[axis] += away[axis] / long;
                    }
                }
            }
            if count == 0.0 {
                return [0.0; 3];
            }
            let force: [f32; 3] = std::array::from_fn(|axis| {
                boids.cohesion * (center[axis] / count - one.position[axis])
                    + boids.alignment * (heading[axis] / count - one.velocity[axis])
                    + boids.separation * apart[axis] * boids.separation_threshold
            });
            let mut added = force.map(|f| f * dt * every as f32);
            // Held to the cap unless it was over it already, as the engine's
            // changelog says, so a flock can be thrown faster than it swims.
            let then: [f32; 3] = std::array::from_fn(|axis| one.velocity[axis] + added[axis]);
            let (now, faster) = (length(one.velocity), length(then));
            let most = boids.max_speed.max(now);
            if faster > most {
                added = std::array::from_fn(|axis| then[axis] / faster * most - one.velocity[axis]);
            }
            added
        })
        .collect()
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
    /// Each adds its push: Time Reactor's fire has one up its tube and one down.
    turbulent: Vec<Turbulent>,
    /// Stands over `color` where the system draws its colours this way.
    palette: Palette,
    /// How far a particle may land from where the emitter put it.
    offset: f32,
    /// Sent out around a control point rather than left where it was born.
    around: Option<([f32; 3], f32, Range<[f32; 3]>)>,
    /// Worked out once the rest of the particle is there to read.
    remaps: Vec<Remap>,
    /// What it takes from the parent's particle it answers to.
    inherits: Vec<Inherit>,
    /// A share of how fast the mouse is moving, for a point that is on it.
    point_velocity: Option<Range<f32>>,
}

#[derive(Clone, Default)]
enum Palette {
    #[default]
    Ranged,
    Hsv { hue: Range<f32>, saturation: Range<f32>, value: Range<f32> },
    List(Vec<[f32; 3]>),
}

/// Hue, saturation and value, each nought to one, as red, green and blue.
fn from_hsv(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let sector = hue.rem_euclid(1.0) * 6.0;
    let across = sector.fract();
    let (low, down, up) = (
        value * (1.0 - saturation),
        value * (1.0 - saturation * across),
        value * (1.0 - saturation * (1.0 - across)),
    );
    match sector as u32 {
        0 => [value, up, low],
        1 => [down, value, low],
        2 => [low, value, up],
        3 => [low, down, value],
        4 => [up, low, value],
        _ => [value, low, down],
    }
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
            turbulent: Vec::new(),
            palette: Palette::Ranged,
            offset: 0.0,
            around: None,
            remaps: Vec::new(),
            inherits: Vec::new(),
            point_velocity: None,
        }
    }
}

/// Whether an emitter is running at the moment.
#[derive(Default, Clone, Copy)]
struct Gate {
    ran: f32,
    /// Coming and going: how long is left of the present run or rest.
    left: f32,
    running: bool,
}

impl Gate {
    fn open(&mut self, emitter: &Emitter, dt: f32, rng: &mut Rng) -> bool {
        self.ran += dt;
        if self.ran < emitter.delay
            || (emitter.duration > 0.0 && self.ran - emitter.delay >= emitter.duration)
        {
            return false;
        }
        let Some(periodic) = emitter.periodic else {
            return true;
        };
        self.left -= dt;
        if self.left <= 0.0 {
            self.running = !self.running;
            self.left = rng.range(if self.running { periodic.run } else { periodic.rest });
        }
        self.running
    }
}

/// The line new particles are strung along, and how many of them cross it.
#[derive(Clone, Copy)]
struct Line {
    from: [f32; 3],
    to: [f32; 3],
    count: u32,
    /// Back the way it came from the far end, rather than from the start again.
    mirror: bool,
}

impl Line {
    /// How far along the line the particle born `nth` belongs.
    ///
    /// The walk turns round at each end rather than jumping back, so a ribbon
    /// threaded through these runs out to the far point and back along itself.
    fn at(&self, nth: u32) -> [f32; 3] {
        let last = self.count - 1;
        let step = match self.mirror {
            true => match nth % (2 * last) {
                step if step <= last => step,
                step => 2 * last - step,
            },
            false => nth % self.count,
        };
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
    /// Lets its particles out at the mouse, and none while there is no mouse.
    /// Alien Romulus' black "explosion on cursor", held at where the scene
    /// placed it, was a black blot under the alien's mouth.
    born_on_pointer: bool,
    /// When each emitter runs, in the same order.
    gates: Vec<Gate>,
    exponents: Exponents,
    /// Whether the particles live in 3D rather than on the screen.
    perspective: bool,
    /// How fast the system's own time runs: the scene's `rate` override, and
    /// after it whatever a script on that override says. **Not how many it
    /// lets out.** Warhammer's charges say 0.07, and on Windows they burn
    /// steady and hardly move -- 1.4 between frames where drawing a fourteenth
    /// of the particles at full speed moved 11.7, and this 2.0 -- because each
    /// one lives fourteen times as long and as many are alive as ever.
    clock: f32,
    /// What a script on the scene's `alpha` override makes of the alpha the
    /// file gave, as a share of it: Time Reactor's smoke is held at a fifth in
    /// silence. Over every particle alive, not only those born after.
    opacity: f32,
    /// The override's alpha as the file gives it, already in every particle.
    given_alpha: f32,
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
    /// How fast the mouse went over the last step.
    pointer_velocity: [f32; 3],
    /// Where it was at the last step.
    pointer_was: Option<[f32; 3]>,
    /// The edges of the scene, in the system's own space, for what collides
    /// with them.
    scene: Option<Range<[f32; 3]>>,
    /// Steps taken, which says which slice of a large flock moves.
    steps: u32,
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
    now: Snapshot,
    /// What each emitter owes this host, as the emitters' own pair does.
    owed: Vec<f32>,
}

/// Where a system's particles were born and where they died, in one step.
#[derive(Default)]
pub struct Events {
    pub born: Vec<Snapshot>,
    pub died: Vec<Snapshot>,
    /// Every live particle by its number, after the step.
    pub alive: Vec<(u32, Snapshot)>,
}

impl System {
    /// A system as its file describes it, changed by what the scene says
    /// about this one instance of it.
    pub fn new(definition: &ParticleSystem, over: &InstanceOverride, seed: u32) -> Self {
        let mut start = Start::default();
        for initializer in &definition.initializers {
            match initializer.clone() {
                Initializer::Lifetime(r) => start.lifetime = r,
                // What `sizerandom` says is twice what the shader is handed,
                // which draws a particle that size across. Taken whole,
                // Arkham Knight's fog covered the knight and reached the left
                // edge of the screen; halved, it measures as Wallpaper
                // Engine's on Windows does (13.0 against 12.7, where whole
                // was 21.7), and so does Warhammer's, and Almamu's renderer
                // halves it too.
                Initializer::Size(r) => {
                    start.size = Range { min: r.min / 2.0, max: r.max / 2.0 }
                }
                Initializer::Alpha(r) => start.alpha = r,
                Initializer::Color(r) => start.color = r,
                Initializer::Velocity(r) => start.velocity = r,
                Initializer::Rotation(r) => start.rotation = r,
                Initializer::TurbulentVelocity(field) => start.turbulent.push(field),
                // Carried as a starting value but nothing turns it yet: the
                // operator that spins a particle is `angularmovement`.
                Initializer::AngularVelocity(r) => start.angular = r,
                // Handled below, where the control points it needs are.
                Initializer::AlongControlPoints { .. } => {}
                Initializer::Remap(remap) => start.remaps.push(remap),
                Initializer::Inherit(what) => start.inherits.push(what),
                // A point that is not on the mouse does not move here.
                Initializer::InheritPointVelocity { point, share } => {
                    if point.pointer {
                        start.point_velocity = Some(share);
                    }
                }
                Initializer::HsvColor { hue, saturation, value } => {
                    start.palette = Palette::Hsv { hue, saturation, value }
                }
                Initializer::ColorList(colors) if !colors.is_empty() => {
                    start.palette = Palette::List(colors)
                }
                Initializer::ColorList(_) => {}
                Initializer::PositionOffset { distance } => start.offset = distance,
                Initializer::AroundControlPoint { point, count, speed } => {
                    start.around = Some((point, count, speed))
                }
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
        if let Some((_, count, around)) = &mut start.around {
            *count *= over.count.value;
            *around = times3(*around, [speed; 3]);
        }
        start.alpha = times(start.alpha, over.alpha.value);
        start.size = times(start.size, over.size.value);
        start.lifetime = times(start.lifetime, over.lifetime.value);
        // The scene's colour takes the hue and the file keeps how strong it is,
        // its largest channel: measured, not explained, off recordings.
        // Multiplied, Terminator's orange smoke set to 0.42 stays orange where it
        // burns grey, and Tomb Raider's torch, orange to red in its own
        // `colorchange` and set to white, stays orange where it burns white.
        // Replaced, Star Destroyer's smoke, 0.5 grey set to 0.24 0.31 0.35, comes
        // out twice as light as it is, and Star wars' dim brown light shafts set
        // to 0.75 cross the picture in white. Taken from the luminance instead,
        // Dark Angels' flash, red to yellow in its file, lost its yellow.
        let mut operators = definition.operators.clone();
        if let Some(tint) = over.tint() {
            let hued = |color: [f32; 3]| times_each(tint, brightness(color));
            start.color = Range { min: hued(start.color.min), max: hued(start.color.max) };
            for operator in &mut operators {
                if let Operator::ColorChange { from, to, .. } = operator {
                    *from = [brightness(*from); 3];
                    *to = [brightness(*to); 3];
                }
            }
        }
        start.velocity = times3(start.velocity, [speed; 3]);
        for field in &mut start.turbulent {
            field.speed = times(field.speed, speed);
        }
        let emitters = definition
            .emitters
            .iter()
            .cloned()
            .map(|mut e| {
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
                Initializer::AlongControlPoints { count, mirror } => Some((*count, *mirror)),
                _ => None,
            })
            .zip(
                definition
                    .control_points
                    .first()
                    .zip(definition.control_points.get(1)),
            )
            .map(|((count, mirror), (from, to))| Line {
                from: *from,
                to: *to,
                count,
                mirror,
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
            gates: vec![Gate::default(); definition.emitters.len()],
            exponents: definition.exponents,
            perspective: definition.perspective,
            emitters,
            born_on_pointer: definition.born_on_pointer,
            clock: over.rate.value.max(0.0),
            opacity: 1.0,
            given_alpha: over.alpha.value,
            start,
            operators,
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
            pointer_velocity: [0.0; 3],
            pointer_was: None,
            scene: None,
            steps: 0,
            burst: false,
            waits: false,
            events: None,
            serials: 0,
            hosts: Vec::new(),
            ride_limit: 0,
        }
    }

    /// Says where the edges of the scene are, in the system's own space.
    pub fn bound_by(&mut self, scene: Range<[f32; 3]>) {
        self.scene = Some(scene);
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

    /// Sets how fast the system's time runs, as the scene's override would.
    pub fn set_rate(&mut self, rate: f32) {
        self.clock = rate.max(0.0);
    }

    /// Sets the alpha the scene's override gives, as a script on it says.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.opacity = alpha.max(0.0) / self.given_alpha.max(f32::EPSILON);
    }

    /// Whether anything about the system follows the sound.
    pub fn listens(&self) -> bool {
        self.emitters.iter().any(|(emitter, _)| emitter.audio.is_some())
            || self.start.turbulent.iter().any(|field| field.audio.is_some())
            || self.operators.iter().any(|operator| match operator {
                Operator::Turbulence { field, .. } => field.audio.is_some(),
                Operator::Vortex(vortex) => vortex.audio.is_some(),
                _ => false,
            })
    }

    /// Lets out every emitter's all-at-once particles at `at`, which is in
    /// the system's own space.
    pub fn burst_at(&mut self, from: Snapshot, sound: &Spectrum) {
        self.burst(from.position, None, Some(from), sound);
    }

    fn burst(&mut self, at: [f32; 3], host: Option<u32>, from: Option<Snapshot>, sound: &Spectrum) {
        let emitters = std::mem::take(&mut self.emitters);
        for (emitter, _) in &emitters {
            for _ in 0..emitter.instantaneous {
                if self.live.len() >= self.maxcount {
                    break;
                }
                self.spawn_at(emitter, at, host, from.as_ref(), sound);
            }
        }
        self.emitters = emitters;
    }

    /// A particle from `emitter` as if the emitter stood at `at`.
    fn spawn_at(
        &mut self,
        emitter: &Emitter,
        at: [f32; 3],
        host: Option<u32>,
        from: Option<&Snapshot>,
        sound: &Spectrum,
    ) {
        let mut placed = emitter.clone();
        placed.origin = std::array::from_fn(|i| emitter.origin[i] + at[i]);
        let mut particle = self.spawn(&placed, sound);
        particle.host = host;
        if let Some(from) = from {
            for what in &self.start.inherits {
                inherit(&mut particle, *what, from);
            }
            // What it took is what it is, not a passing effect.
            particle.size = particle.shown_size;
            particle.alpha = particle.shown_alpha;
            particle.color = particle.shown_color;
        }
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
    pub fn follow(&mut self, hosts: &[(u32, Snapshot)], dt: f32, sound: &Spectrum) {
        let now = |serial: u32| hosts.iter().find(|(s, _)| *s == serial).map(|(_, at)| *at);
        let mut riding = std::mem::take(&mut self.hosts);
        for particle in &mut self.live {
            let Some(serial) = particle.host else {
                continue;
            };
            let then = riding.iter().find(|h| h.serial == serial).map(|h| h.now.position);
            match (now(serial), then) {
                (Some(now), Some(then)) => {
                    for axis in 0..3 {
                        particle.position[axis] += now.position[axis] - then[axis];
                    }
                }
                (Some(_), None) => {}
                (None, _) => particle.age = particle.lifetime,
            }
        }
        riding.retain_mut(|host| match now(host.serial) {
            Some(snapshot) => {
                host.now = snapshot;
                true
            }
            None => false,
        });
        for &(serial, snapshot) in hosts {
            if riding.len() >= self.ride_limit {
                break;
            }
            if riding.iter().any(|h| h.serial == serial) {
                continue;
            }
            riding.push(Host {
                serial,
                now: snapshot,
                owed: vec![0.0; self.emitters.len()],
            });
            self.burst(snapshot.position, Some(serial), Some(snapshot), sound);
        }

        let emitters = std::mem::take(&mut self.emitters);
        for host in &mut riding {
            for ((emitter, _), owed) in emitters.iter().zip(&mut host.owed) {
                *owed += emitter.rate * loudness(emitter.audio, sound) * dt * self.clock;
                while *owed >= 1.0 {
                    *owed -= 1.0;
                    if self.live.len() >= self.maxcount {
                        break;
                    }
                    self.spawn_at(emitter, host.now.position, Some(host.serial), Some(&host.now), sound);
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
            events.born.push(Snapshot::of(&particle));
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

    /// How many particles it will really have at once: what its emitters let
    /// out over a lifetime, however high the file sets the ceiling. DOOM's
    /// ceilings add up to 178 000 and its systems to a tenth of that, and
    /// thinning by the ceiling took the sparks away from a scene that could
    /// have afforded every one.
    pub fn expected(&self) -> usize {
        if self.waits || !matches!(self.ribbon, Ribbon::None) {
            return self.capacity();
        }
        let wanted: f32 = self
            .emitters
            .iter()
            .map(|(e, _)| e.rate * self.start.lifetime.max + e.instantaneous as f32)
            .sum();
        (wanted.ceil() as usize).min(self.maxcount)
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

    /// Whether its particles are seen through a camera.
    pub fn is_perspective(&self) -> bool {
        self.perspective
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
        // A clock standing still has not been running: Ender Dark Angels' rifle
        // says a rate of nought, and its preview has no flash at the muzzle.
        if self.starttime > 0.0 && !self.waits && self.clock > 0.0 {
            // Already running when the wallpaper appears, however long the
            // file says. `magic_charge_rays` says fifteen seconds, and in
            // Wallpaper Engine its rays are there from the first frame.
            // Seconds of the system's own time, whatever its clock says.
            let ahead = std::mem::take(&mut self.starttime).min(MOST_AHEAD);
            let clock = std::mem::replace(&mut self.clock, 1.0);
            for _ in 0..(ahead * AHEAD_STEPS) as u32 {
                self.advance(1.0 / AHEAD_STEPS, sound);
            }
            self.clock = clock;
        }
        let dt = dt.clamp(0.0, MAX_STEP) * self.clock;
        self.elapsed += dt;
        if let Some(events) = &mut self.events {
            events.born.clear();
            events.died.clear();
        }

        if !self.waits {
            if !self.burst {
                self.burst = true;
                self.burst([0.0; 3], None, None, sound);
            }
            if !self.born_on_pointer || self.pointer.is_some() {
                self.emit(dt, sound);
            }
        }

        let elapsed = self.elapsed;
        let ribbon = self.ribbon;
        let pointer = self.pointer;
        self.steps = self.steps.wrapping_add(1);
        self.pointer_velocity = match (pointer, self.pointer_was, dt > 0.0) {
            (Some(now), Some(then), true) => minus(now, then).map(|d| d / dt),
            _ => [0.0; 3],
        };
        self.pointer_was = pointer;
        // Against where they all were before the step, so the order the
        // particles happen to be in says nothing about who follows whom.
        let flocks: Vec<(Boids, Vec<[f32; 3]>)> = self
            .operators
            .iter()
            .filter_map(|operator| match operator {
                Operator::Boids(boids) => Some((*boids, flock(&self.live, boids, self.steps, dt))),
                _ => None,
            })
            .collect();
        for (_, added) in &flocks {
            for (particle, added) in self.live.iter_mut().zip(added) {
                for axis in 0..3 {
                    particle.velocity[axis] += added[axis];
                }
            }
        }
        let moment = Moment {
            elapsed,
            time_of_day: time_of_day(),
            pointer,
        };
        let scene = self.scene;
        let hosts = &self.hosts;
        for particle in &mut self.live {
            particle.age += dt;
            particle.shown_size = particle.size;
            particle.shown_alpha = particle.alpha;
            particle.shown_color = particle.color;
            let pace = std::mem::replace(&mut particle.pace, 1.0);
            let before = particle.position;
            let previous = particle.previous;
            for operator in &self.operators {
                match operator {
                    Operator::Remap(remap) => apply_remap(remap, particle, &moment, false),
                    Operator::Inherit(what) => {
                        let host = particle.host.and_then(|serial| hosts.iter().find(|h| h.serial == serial));
                        if let Some(host) = host {
                            inherit(particle, *what, &host.now);
                        }
                    }
                    // Worked out above, against everybody at once.
                    Operator::Boids(_) => {}
                    Operator::Collide(collision) => collide(particle, collision, previous, scene),
                    Operator::MaintainDistance { point, distance, strength } => {
                        let Some(at) = moment.place(*point) else { continue };
                        let away = minus(particle.position, at);
                        let long = length(away);
                        if long > f32::EPSILON {
                            let goal: [f32; 3] =
                                std::array::from_fn(|i| at[i] + away[i] / long * distance);
                            let share = if *strength > 0.0 { (strength * dt).min(1.0) } else { 1.0 };
                            for axis in 0..3 {
                                particle.position[axis] += (goal[axis] - particle.position[axis]) * share;
                            }
                        }
                    }
                    Operator::MaintainBetween { from, to } => {
                        let (Some(from), Some(to)) = (moment.place(*from), moment.place(*to)) else {
                            continue;
                        };
                        let line = minus(to, from);
                        let along = dot(line, line);
                        if along > f32::EPSILON {
                            let t = (dot(minus(particle.position, from), line) / along).clamp(0.0, 1.0);
                            particle.position = std::array::from_fn(|i| from[i] + line[i] * t);
                        }
                    }
                    Operator::Movement { gravity, drag } => {
                        for (axis, gravity) in gravity.iter().enumerate() {
                            particle.velocity[axis] += gravity * dt;
                            // Drag as a proportion shed per second, which stays
                            // stable whatever the frame rate and never reverses
                            // the velocity the way `1 - drag * dt` would.
                            particle.velocity[axis] /= 1.0 + drag * dt;
                            particle.position[axis] += particle.velocity[axis] * pace * dt;
                        }
                    }
                    // It pushes: the field accelerates the particle. Read as a
                    // speed that carries it along, Gears' 45 000 sparks at 500
                    // to 1000 ran together into red bands where the recording
                    // has them scattered evenly about, held back by their drag.
                    Operator::Turbulence { field, mask } => {
                        let along = eddy_direction(
                            field,
                            particle.position,
                            particle.phase * field.phase_max,
                            elapsed,
                        );
                        let speed = (field.speed.min
                            + (field.speed.max - field.speed.min) * particle.drift)
                            * loudness(field.audio, sound);
                        for (axis, mask) in mask.iter().enumerate() {
                            particle.velocity[axis] += along[axis] * mask * speed * dt;
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
                    // The same pull all the way out to half the reach the file
                    // states, then none: Almamu's reading, and Romulus' pointer
                    // says -600 inside 50 and 500 inside 5000, a ring of specks.
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
                        if distance > f32::EPSILON && distance < reach / 2.0 {
                            let force = strength * dt;
                            for (axis, away) in away.iter().enumerate() {
                                particle.velocity[axis] -= away / distance * force;
                            }
                        }
                    }
                    // A push along the way round, as Almamu reads it, which its
                    // drag holds to a speed: Augmentation's eye asks for 8000 and
                    // swirls on the beat. As degrees a second that was a turn and
                    // a half a frame, and a mess of straight spikes.
                    Operator::Vortex(vortex) => {
                        let round = cross(vortex.axis, particle.position);
                        let length = round.iter().map(|v| v * v).sum::<f32>().sqrt();
                        if length > 1e-3 {
                            let push = vortex_speed(vortex, particle.position)
                                * loudness(vortex.audio, sound)
                                * dt
                                / length;
                            for axis in 0..3 {
                                particle.velocity[axis] += round[axis] * push;
                            }
                        }
                    }
                    Operator::CapVelocity { most } => {
                        let speed = particle.velocity.iter().map(|v| v * v).sum::<f32>().sqrt();
                        if speed > *most {
                            particle.velocity = particle.velocity.map(|v| v / speed * most);
                        }
                    }
                    // As much of its speed taken off each second as the
                    // reduction says, fading out between the two distances.
                    Operator::ReduceNear { point, distance, reduction } => {
                        let away: f32 = std::array::from_fn::<f32, 3, _>(|i| particle.position[i] - point[i])
                            .iter()
                            .map(|d| d * d)
                            .sum::<f32>()
                            .sqrt();
                        let t = along(*distance, away);
                        let by = reduction.min + (reduction.max - reduction.min) * t;
                        particle.velocity = particle.velocity.map(|v| v / (1.0 + by.max(0.0) * dt));
                    }
                    // These change nothing that carries over to the next step,
                    // so they are applied as the buffer is written.
                    Operator::AlphaFade { .. }
                    | Operator::AlphaChange(_)
                    | Operator::SizeChange(_)
                    | Operator::ColorChange { .. }
                    | Operator::OscillateAlpha(_)
                    | Operator::OscillateSize(_)
                    | Operator::OscillatePosition { .. } => {}
                }
            }
            particle.previous = before;
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
            events.died.extend(dead.map(Snapshot::of));
        }
        self.live.retain(|p| p.age < p.lifetime);
        if let Some(events) = &mut self.events {
            events.alive.clear();
            events.alive.extend(self.live.iter().map(|p| (p.serial, Snapshot::of(p))));
        }
        self.write_vertices();
    }

    fn emit(&mut self, dt: f32, sound: &Spectrum) {
        // Taken apart so the borrow of the emitters does not hold the whole
        // system while a particle is being built.
        let mut emitters = std::mem::take(&mut self.emitters);
        let mut gates = std::mem::take(&mut self.gates);
        for ((emitter, owed), gate) in emitters.iter_mut().zip(&mut gates) {
            if !gate.open(emitter, dt, &mut self.rng) {
                continue;
            }
            *owed += emitter.rate * loudness(emitter.audio, sound) * dt;
            if emitter.one_per_step {
                *owed = owed.min(1.0);
            }
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
        self.gates = gates;
    }

    /// A place in a sphere between two radii, evenly spread: flat, a disc in
    /// the plane with a depth to it -- which a flat scene cannot see -- and
    /// in perspective a whole ball. Almamu's reading; drawn as a direction
    /// and a plain distance they bunched up around the middle and, pressed
    /// into an ellipse, flew off along its short axis: Star Destroyer's fire
    /// hung in the sky well below the hull it was meant to burn along.
    fn in_sphere(&mut self, inner: f32, outer: f32) -> [f32; 3] {
        let rng = &mut self.rng;
        if self.perspective {
            let around = rng.next_f32() * std::f32::consts::TAU;
            let z = rng.signed();
            let ring = (1.0 - z * z).sqrt();
            let radius = rng
                .range(Range { min: inner.powi(3), max: outer.powi(3) })
                .max(0.0)
                .cbrt();
            return [ring * around.cos() * radius, ring * around.sin() * radius, z * radius];
        }
        let around = rng.next_f32() * std::f32::consts::TAU;
        let radius = rng
            .range(Range { min: inner * inner, max: outer * outer })
            .max(0.0)
            .sqrt();
        [around.cos() * radius, around.sin() * radius, rng.signed() * outer]
    }

    fn spawn(&mut self, emitter: &Emitter, sound: &Spectrum) -> Particle {
        let flung: [f32; 3] = {
            let drawn = match emitter.shape {
                Shape::Sphere => self.in_sphere(emitter.distance.min[0], emitter.distance.max[0]),
                Shape::Box => std::array::from_fn(|i| {
                    let along = self.rng.range(Range {
                        min: emitter.distance.min[i],
                        max: emitter.distance.max[i],
                    });
                    if self.rng.next_f32() < 0.5 { -along } else { along }
                }),
            };
            std::array::from_fn(|i| {
                let at = drawn[i] * emitter.directions[i];
                match emitter.sign[i] {
                    s if s > 0.0 => at.abs(),
                    s if s < 0.0 => -at.abs(),
                    _ => at,
                }
            })
        };
        let mouse = self.pointer.filter(|_| self.born_on_pointer).unwrap_or([0.0; 3]);
        let position: [f32; 3] = std::array::from_fn(|i| mouse[i] + emitter.origin[i] + flung[i]);
        // Outward from the emitter's middle, for a speed the emitter states.
        let direction = {
            let length = (flung[0] * flung[0] + flung[1] * flung[1] + flung[2] * flung[2]).sqrt();
            match length > 1e-6 {
                true => flung.map(|x| x / length),
                false => [0.0, 1.0, 0.0],
            }
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
        let nudge: [f32; 3] = match self.start.offset {
            0.0 => [0.0; 3],
            distance => {
                let [x, y, _] = self.in_sphere(0.0, distance);
                [x, y, 0.0]
            }
        };
        let position: [f32; 3] = std::array::from_fn(|i| position[i] + nudge[i]);
        // Each one a part of a turn on from the last, about the point.
        let (position, thrown) = match self.start.around {
            Some((point, count, range)) => {
                let (sin, cos) =
                    (self.born as f32 / count * std::f32::consts::TAU).sin_cos();
                self.born = self.born.wrapping_add(1);
                let out = self.rng.range3(range);
                (
                    std::array::from_fn(|i| position[i] + point[i]),
                    [out[0] * cos - out[1] * sin, out[0] * sin + out[1] * cos, out[2]],
                )
            }
            None => (position, [0.0; 3]),
        };
        let exponents = self.exponents;
        let lifetime = self.rng.skewed(self.start.lifetime, exponents.lifetime).max(f32::EPSILON);
        let phase = self.rng.next_f32();

        // A turbulent start reads the field once, where the particle appears.
        let mut turbulent = [0.0; 3];
        for at in 0..self.start.turbulent.len() {
            let field = self.start.turbulent[at];
            let along = launch_direction(&field, position, phase * field.phase_max, self.elapsed);
            let speed = self.rng.range(field.speed) * loudness(field.audio, sound);
            for axis in 0..3 {
                turbulent[axis] += along[axis] * speed;
            }
        }

        // In this order, which is the order they have always been drawn in:
        // another would be every particle in the library somewhere else, and a
        // sweep with every picture moved says nothing about what a change did.
        let rotation = self.rng.range3(self.start.rotation);
        let size = self.rng.skewed(self.start.size, exponents.size);
        let color = match &self.start.palette {
            Palette::Ranged => std::array::from_fn(|i| {
                self.rng.skewed(
                    Range { min: self.start.color.min[i], max: self.start.color.max[i] },
                    exponents.color,
                )
            }),
            Palette::Hsv { hue, saturation, value } => {
                let (h, s, v) = (*hue, *saturation, *value);
                from_hsv(self.rng.range(h), self.rng.range(s), self.rng.range(v))
            }
            Palette::List(colors) => {
                colors[((self.rng.next_f32() * colors.len() as f32) as usize).min(colors.len() - 1)]
            }
        };
        let alpha = self.rng.skewed(self.start.alpha, exponents.alpha);
        let handed = match self.start.point_velocity {
            Some(share) => {
                let share = self.rng.range(share);
                self.pointer_velocity.map(|v| v * share)
            }
            None => [0.0; 3],
        };
        let mut particle = Particle {
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
            velocity: std::array::from_fn(|i| {
                drawn[i] + direction[i] * speed + turbulent[i] + thrown[i] + handed[i]
            }),
            rotation,
            size,
            color,
            alpha,
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
            shown_size: size,
            shown_alpha: alpha,
            shown_color: color,
            pace: 1.0,
            previous: position,
        };
        let moment = Moment {
            elapsed: self.elapsed,
            time_of_day: time_of_day(),
            pointer: self.pointer,
        };
        for remap in &self.start.remaps {
            apply_remap(remap, &mut particle, &moment, true);
        }
        particle.size = particle.shown_size;
        particle.alpha = particle.shown_alpha;
        particle.color = particle.shown_color;
        particle
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
        for particle in &self.live {
            let progress = particle.age / particle.lifetime;
            let mut alpha =
                self.opacity * particle.shown_alpha * fade(&self.operators, particle.age, particle.lifetime);
            let mut size = particle.shown_size * size_factor(&self.operators, progress);
            let mut color = particle.shown_color;
            let mut position = particle.position;
            for operator in &self.operators {
                match operator {
                    Operator::ColorChange { time, from, to } => {
                        let t = along(*time, progress);
                        // Over what the particle was born with: a tint the
                        // scene put on it stays on it.
                        color = std::array::from_fn(|i| {
                            particle.shown_color[i] * (from[i] + (to[i] - from[i]) * t)
                        });
                    }
                    Operator::OscillateAlpha(wave) => {
                        alpha *= wave_between(wave, particle.wobble, particle.age);
                    }
                    // Between the two ends, each particle at its own rate.
                    Operator::OscillateSize(wave) => {
                        size *= wave_between(wave, particle.wobble, particle.age);
                    }
                    Operator::OscillatePosition { wave, mask } => {
                        let (amount, swing) = wave_at(wave, particle.wobble, particle.age);
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
                // Just short of one, or the shader's fraction is the first.
                Some(Animation::Once) => progress.min(0.999),
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
            let alpha =
                self.opacity * particle.shown_alpha * fade(&self.operators, particle.age, particle.lifetime);
            let size =
                particle.shown_size * size_factor(&self.operators, particle.age / particle.lifetime);
            let tint = changed_color(&self.operators, particle);
            let color = [tint[0], tint[1], tint[2], alpha];
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
                let alpha = self.opacity * p.shown_alpha * fade(&self.operators, p.age, p.lifetime);
                let size = p.shown_size * size_factor(&self.operators, p.age / p.lifetime);
                let tint = changed_color(&self.operators, p);
                ([tint[0], tint[1], tint[2], alpha], size)
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

/// How far through the day it is, nought to one, by the clock's own zone.
/// ponytail: UTC rather than local, which only a remap on the time of day could
/// tell; `std` has no time zones, so a local one wants another dependency.
fn time_of_day() -> f32 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (seconds % 86_400) as f32 / 86_400.0
}

/// A particle's colour after `colorchange`, for the ribbons.
fn changed_color(operators: &[Operator], particle: &Particle) -> [f32; 3] {
    let mut color = particle.shown_color;
    for operator in operators {
        if let Operator::ColorChange { time, from, to } = operator {
            let t = along(*time, particle.age / particle.lifetime);
            color = std::array::from_fn(|i| particle.shown_color[i] * (from[i] + (to[i] - from[i]) * t));
        }
    }
    color
}

/// How strong a colour is, from nought to one.
fn brightness(color: [f32; 3]) -> f32 {
    color[0].max(color[1]).max(color[2])
}

fn times_each(color: [f32; 3], by: f32) -> [f32; 3] {
    color.map(|c| c * by)
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
    (amount, (time * frequency + phase).cos())
}

/// A value swinging between the two ends of its wave's scale.
fn wave_between(wave: &Oscillation, wobble: [f32; 3], time: f32) -> f32 {
    let (_, swing) = wave_at(wave, wobble, time);
    wave.scale.min + (wave.scale.max - wave.scale.min) * (swing + 1.0) / 2.0
}

/// How hard a vortex pushes a particle at its distance from the centre.
fn vortex_speed(vortex: &Vortex, position: [f32; 3]) -> f32 {
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

/// What `alphafade` and `alphachange` leave of a particle's alpha.
///
/// The fade's two times are points on the particle's life, as fractions of
/// it: in until the first, out from the second to the end. They were read as
/// seconds, with no fade out unless one was stated -- so smoke stayed at its
/// full alpha for all of its life and vanished at once, where Wallpaper
/// Engine has it thin out from halfway: the dense, hard-edged clouds.
fn fade(operators: &[Operator], age: f32, lifetime: f32) -> f32 {
    let life = match lifetime > 0.0 {
        true => (age / lifetime).clamp(0.0, 1.0),
        false => 1.0,
    };
    let mut out = 1.0;
    for operator in operators {
        match *operator {
            Operator::AlphaFade { fade_in, fade_out } => {
                if life <= fade_in && fade_in > 0.0 {
                    out *= life / fade_in;
                } else if life > fade_out && fade_out < 1.0 {
                    out *= 1.0 - (life - fade_out) / (1.0 - fade_out);
                }
            }
            Operator::AlphaChange(ramp) => {
                out *= (ramp.from + (ramp.to - ramp.from) * along(ramp.time, life)).max(0.0);
            }
            _ => {}
        }
    }
    out
}

/// What `sizechange` scales a particle's size by, `progress` being how far
/// through its life it is. Several steps take over from one another: the last
/// to have begun says, and they do not multiply -- 0 to 1.8 and then 1.8 to 0
/// peaks at 1.8, not at its square.
fn size_factor(operators: &[Operator], progress: f32) -> f32 {
    let mut out = None;
    for operator in operators {
        if let Operator::SizeChange(ramp) = operator
            && (out.is_none() || progress >= ramp.time.min)
        {
            let t = along(ramp.time, progress);
            out = Some(ramp.from + (ramp.to - ramp.from) * t);
        }
    }
    out.unwrap_or(1.0)
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

/// Which way the `turbulence` operator pushes: round the hills of a noise rather
/// than along its values, so what it stirs turns in eddies instead of draining
/// to one side. Almamu's reading, flattened, since no scene here moves in depth.
///
/// The clock moves the place asked about in the scene's own pixels, before the
/// scale: Rainbow Six says -50 at a scale of 0.0053, half a cell a second. Added
/// after it, that was fifty, a new direction every frame, and its fog only
/// shivered where the recording's rolls round the man with the shield.
fn eddy_direction(field: &Turbulent, position: [f32; 3], phase: f32, time: f32) -> [f32; 3] {
    const STEP: f32 = 0.01;
    let scale = field.scale * 2.0;
    let x = (position[0] + phase + time * field.time_scale) * scale;
    let y = position[1] * scale;
    let z = position[2] * scale;
    let across = noise([x, y + STEP, z]) - noise([x, y - STEP, z]);
    let up = noise([x - STEP, y, z]) - noise([x + STEP, y, z]);
    let length = (across * across + up * up).sqrt();
    if length < 1e-6 {
        return [0.0; 3];
    }
    [across / length, up / length, 0.0]
}

/// Which way `turbulentvelocityrandom` sends a particle off: a direction out
/// of the noise, held within `scale` half turns of `forward`, then turned
/// `offset` radians about `right`, and kept in the plane.
///
/// Its scale and offset are not the field's: the engine's defaults for it
/// are a forward and a right, which only mean something read this way, and
/// Almamu's renderer reads it so. Taken as a field, Star wars' embers --
/// scale 0.1, offset -0.5 -- went every way at once, down and into the
/// screen among them; held this way they rise to the left of where they
/// are born, which is where Wallpaper Engine shows them. Which way the turn
/// goes was settled by that picture, not by Almamu, whose y runs down.
fn launch_direction(field: &Turbulent, position: [f32; 3], phase: f32, time: f32) -> [f32; 3] {
    let normalized = |v: [f32; 3]| {
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        (length > 1e-4).then(|| v.map(|x| x / length))
    };
    let forward = normalized(field.forward).unwrap_or([0.0, 1.0, 0.0]);
    // The spread is a turn about `right`, and a turn about the way it is already
    // going moves nothing. Dark Angels' Torch says `right` "0 1 0" and in its video
    // it runs straight down the barrel; let wander, it lit a column up the picture
    // as far as the edge of the layer it is copied through. One preset, one video.
    if normalized(cross(forward, field.right)).is_none() {
        return forward;
    }
    let sampled = Turbulent {
        scale: 0.1,
        offset: -0.5,
        ..*field
    };
    let mut along = normalized(field_direction(&sampled, position, phase, time)).unwrap_or(forward);

    let most = field.scale / 2.0;
    let dot = along.iter().zip(&forward).map(|(a, b)| a * b).sum::<f32>();
    let angle = dot.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
    if field.scale < 2.0 && most > 1e-4 && angle > most {
        if let Some(axis) = normalized(cross(along, forward)) {
            along = turned(along, axis, (angle - most) * 180.0);
        }
    }
    // Star wars' embers fly up and to the left of the picture at 49 degrees in
    // the system's own axes, tracked frame by frame through a recording: a
    // start straight up turned by -0.5 radians is 61, give or take its 18. The
    // other way round was 119, and they flew off the bottom of the screen.
    along = turned(along, field.right, field.offset.to_degrees());
    along[2] = 0.0;
    normalized(along).unwrap_or(forward)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
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

    /// In the range, leaning towards its minimum as the exponent says.
    fn skewed(&mut self, r: Range<f32>, exponent: f32) -> f32 {
        r.min + (r.max - r.min) * self.next_f32().powf(exponent)
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
    fn a_step_that_follows_the_sound_hears_only_inside_its_bounds() {
        let step = Some(AudioResponse {
            bands: (4, 7),
            ears: Channels::Both,
            exponent: 1.0,
            bounds: Range { min: 0.5, max: 1.0 },
        });
        let fill = |level: f32| {
            let mut sound = Spectrum::SILENT;
            // Bands 4 to 7 of sixteen are entries 16 to 31 of 64.
            sound.left[16..=31].fill(level);
            sound.right[16..=31].fill(level);
            sound
        };

        assert_eq!(loudness(None, &Spectrum::SILENT), 1.0);
        assert_eq!(loudness(step, &Spectrum::SILENT), 0.0);
        assert_eq!(loudness(step, &fill(0.5)), 0.0);
        assert_eq!(loudness(step, &fill(0.75)), 0.5);
        assert_eq!(loudness(step, &fill(1.0)), 1.0);

        // Sound outside the bands it listens to is not its business.
        let mut elsewhere = Spectrum::SILENT;
        elsewhere.left[0..=15].fill(1.0);
        elsewhere.right[0..=15].fill(1.0);
        assert_eq!(loudness(step, &elsewhere), 0.0);
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

    /// The exponent comes after the bounds' smooth step, so a step with a high
    /// one stays quiet until the sound is really loud.
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
        assert!((with(1.0) - 0.15625).abs() < 0.001);
        assert!((with(2.0) - 0.0244).abs() < 0.001);
        assert!((with(0.5) - 0.3953).abs() < 0.001);
    }

    /// Out to the far point and back the way it came. A walk that jumped back
    /// instead would draw a ribbon across the whole bolt on every lap.
    #[test]
    fn the_walk_along_a_line_turns_round_at_each_end() {
        let line = Line {
            from: [0.0; 3],
            to: [9.0, 0.0, 0.0],
            count: 10,
            mirror: true,
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
    /// Valve's thunderbolt pulses its opacity on a sine over the system's
    /// clock, past the first second, and only from the middle of a life on.
    #[test]
    fn a_remap_writes_what_is_drawn_and_only_as_its_blend_allows() {
        let mut s = system(
            r#"{"material":"m","maxcount":1,
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                           {"name":"sizerandom","min":40,"max":40}],
            "operator":[{"name":"remapvalue","output":"size","operation":"remap",
                         "outputrangemin":100,"outputrangemax":100,
                         "blendinstart":0.5,"blendinend":0.6}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 30.0);
        assert_eq!(s.live[0].size, 20.0, "what it was born with stays");
        assert_eq!(s.live[0].shown_size, 20.0, "before its blend, nothing");
        run(&mut s, 5.5, 1.0 / 30.0);
        assert!((s.live[0].shown_size - 50.0).abs() < 1e-3, "{}", s.live[0].shown_size);
    }

    #[test]
    fn a_multiplying_remap_does_not_compound() {
        let mut s = system(
            r#"{"material":"m","maxcount":1,
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                           {"name":"sizerandom","min":40,"max":40}],
            "operator":[{"name":"remapvalue","output":"size",
                         "outputrangemin":2,"outputrangemax":2}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 30.0);
        assert_eq!(s.live[0].shown_size, 40.0);
    }

    #[test]
    fn a_wave_goes_on_past_the_end_of_its_input_range() {
        let wave = |transform, fraction| {
            let remap = wpe_format::ParticleSystem::parse(
                format!(
                    r#"{{"initializer":[{{"name":"remapinitialvalue","transformfunction":"{transform}",
                    "transforminputscale":1}}]}}"#
                )
                .as_bytes(),
            )
            .unwrap();
            let Initializer::Remap(remap) = remap.initializers[0] else { panic!() };
            remap_shape(&remap, fraction, 0, 0.0)
        };
        assert!((wave("sine", 1.25) - 1.0).abs() < 1e-3);
        assert_eq!(wave("saw", 1.5), 0.5);
        assert_eq!(wave("square", 1.25), 1.0);
        assert_eq!(wave("triangle", 1.5), 1.0);
        assert_eq!(wave("none", 1.5), 1.0, "only no shape stops at the end");
    }

    #[test]
    fn a_new_particle_is_written_by_its_remap_once() {
        let mut s = system(
            r#"{"material":"m","maxcount":1,
            "controlpoint":[{"id":0,"offset":"0 0 0"},{"id":1,"offset":"100 0 0"}],
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"sizerandom","min":40,"max":40},
                           {"name":"remapinitialvalue","input":"distancetocontrolpoint",
                            "inputcontrolpoint0":1,"inputrangemax":200}]}"#,
        );
        run(&mut s, 0.1, 0.1);
        // Half of the reach away, so half of the 20 it was going to be drawn.
        assert_eq!(s.live[0].size, 10.0);
    }

    fn collides(step: &str, position: [f32; 3], velocity: [f32; 3]) -> System {
        let mut s = system(&format!(
            r#"{{"material":"m","maxcount":1,
            "emitter":[{{"name":"sphererandom","instantaneous":1,"distancemax":0}}],
            "initializer":[{{"name":"lifetimerandom","min":10,"max":10}}],
            "operator":[{step}]}}"#
        ));
        run(&mut s, 0.01, 0.01);
        s.live[0].position = position;
        s.live[0].previous = position;
        s.live[0].velocity = velocity;
        s
    }

    #[test]
    fn a_plane_bounces_a_particle_back_by_its_factor() {
        let mut s = collides(
            r#"{"name":"collisionplane","distance":-200,"bouncefactor":0.5}"#,
            [0.0, -210.0, 0.0],
            [0.0, -100.0, 0.0],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position[1], -200.0);
        assert_eq!(s.live[0].velocity[1], 50.0);
    }

    #[test]
    fn a_plane_can_slide_a_particle_or_take_it_away() {
        let mut s = collides(
            r#"{"name":"collisionplane","collisionbehavior":"slide"}"#,
            [0.0, -1.0, 0.0],
            [5.0, -100.0, 0.0],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].velocity, [5.0, 0.0, 0.0]);
        let mut s = collides(
            r#"{"name":"collisionplane","collisionbehavior":"delete"}"#,
            [0.0, -1.0, 0.0],
            [0.0; 3],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn a_sphere_pushes_a_particle_out_to_its_surface() {
        let mut s = collides(
            r#"{"name":"collisionsphere","origin":"0 0 0","radius":50}"#,
            [10.0, 0.0, 0.0],
            [-100.0, 0.0, 0.0],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position[0], 50.0);
    }

    /// From in front only, and only inside its edges.
    #[test]
    fn a_rectangle_is_struck_only_from_in_front_and_within_its_edges() {
        let quad = r#"{"name":"collisionquad","origin":"0 0 0","size":"100 100"},
                      {"name":"movement"}"#;
        let mut front = collides(quad, [0.0, 1.0, 0.0], [0.0, -200.0, 0.0]);
        front.advance(0.01, &Spectrum::SILENT);
        // The rectangle comes first and the movement after, so it takes a step
        // for the crossing to be seen.
        front.advance(0.01, &Spectrum::SILENT);
        assert!(front.live[0].velocity[1] > 0.0, "{:?}", front.live[0].velocity);
        let mut behind = collides(quad, [0.0, -1.0, 0.0], [0.0, -200.0, 0.0]);
        behind.advance(0.01, &Spectrum::SILENT);
        assert!(behind.live[0].velocity[1] < 0.0);
        let mut beside = collides(quad, [500.0, 1.0, 0.0], [0.0, -200.0, 0.0]);
        beside.advance(0.01, &Spectrum::SILENT);
        beside.advance(0.01, &Spectrum::SILENT);
        assert!(beside.live[0].velocity[1] < 0.0);
    }

    #[test]
    fn the_edges_of_the_scene_keep_a_particle_in() {
        let mut s = collides(r#"{"name":"collisionbounds"}"#, [1000.0, 50.0, 0.0], [100.0, 0.0, 0.0]);
        s.bound_by(Range { min: [0.0; 3], max: [800.0, 600.0, 0.0] });
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position[0], 800.0);
        assert_eq!(s.live[0].velocity[0], -50.0);
    }

    #[test]
    fn a_particle_keeps_its_distance_from_a_point() {
        let mut s = collides(
            r#"{"name":"maintaindistancetocontrolpoint","distance":100}"#,
            [300.0, 0.0, 0.0],
            [0.0; 3],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position, [100.0, 0.0, 0.0]);
        // With a strength it goes there over time.
        let mut s = collides(
            r#"{"name":"maintaindistancetocontrolpoint","distance":100,"variablestrength":10}"#,
            [300.0, 0.0, 0.0],
            [0.0; 3],
        );
        s.advance(0.01, &Spectrum::SILENT);
        assert!((s.live[0].position[0] - 280.0).abs() < 1e-3, "{:?}", s.live[0].position);
    }

    #[test]
    fn a_particle_can_be_held_on_the_line_between_two_points() {
        let mut s = system(
            r#"{"material":"m","maxcount":1,
            "controlpoint":[{"id":0,"offset":"0 0 0"},{"id":1,"offset":"100 0 0"}],
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
            "operator":[{"name":"maintaindistancebetweencontrolpoints"}]}"#,
        );
        run(&mut s, 0.01, 0.01);
        s.live[0].position = [40.0, 30.0, 0.0];
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position, [40.0, 0.0, 0.0]);
        s.live[0].position = [400.0, 30.0, 0.0];
        s.advance(0.01, &Spectrum::SILENT);
        assert_eq!(s.live[0].position, [100.0, 0.0, 0.0]);
    }

    #[test]
    fn a_flock_closes_up_and_matches_its_heading() {
        let boids = Boids {
            neighbor_threshold: 150.0,
            separation_threshold: 20.0,
            max_speed: 500.0,
            separation: 0.0,
            alignment: 5.0,
            cohesion: 5.0,
        };
        let mut s = system(
            r#"{"material":"m","maxcount":2,
            "emitter":[{"name":"sphererandom","instantaneous":2,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 0.01, 0.01);
        s.live[0].position = [0.0; 3];
        s.live[1].position = [100.0, 0.0, 0.0];
        s.live[0].velocity = [0.0; 3];
        s.live[1].velocity = [0.0, 10.0, 0.0];
        let added = flock(&s.live, &boids, 0, 0.1);
        assert!(added[0][0] > 0.0 && added[1][0] < 0.0, "they close up: {added:?}");
        assert!(added[0][1] > 0.0 && added[1][1] < 0.0, "and match: {added:?}");
        // Nobody near, nothing to answer to.
        s.live[1].position = [1000.0, 0.0, 0.0];
        assert_eq!(flock(&s.live, &boids, 0, 0.1)[0], [0.0; 3]);
    }

    #[test]
    fn a_flock_keeps_apart_inside_its_separation_and_under_its_cap() {
        let boids = Boids {
            neighbor_threshold: 150.0,
            separation_threshold: 20.0,
            max_speed: 10.0,
            separation: 15.0,
            alignment: 0.0,
            cohesion: 0.0,
        };
        let mut s = system(
            r#"{"material":"m","maxcount":2,
            "emitter":[{"name":"sphererandom","instantaneous":2,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 0.01, 0.01);
        s.live[0].position = [0.0; 3];
        s.live[1].position = [10.0, 0.0, 0.0];
        s.live[0].velocity = [0.0; 3];
        s.live[1].velocity = [0.0; 3];
        let added = flock(&s.live, &boids, 0, 1.0);
        assert_eq!(added[0], [-10.0, 0.0, 0.0]);
    }

    /// A child burst from a dying particle takes on the colour it had, and a
    /// parent's size scales it, which is what the editor's list says.
    #[test]
    fn a_child_takes_what_it_is_told_to_from_its_parent() {
        let mut child = system(
            r#"{"material":"m","maxcount":4,
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                           {"name":"inheritinitialvaluefromevent","input":"setcolor"},
                           {"name":"inheritinitialvaluefromevent","input":"multiplysize"}]}"#,
        );
        child.wait_for_parent();
        let from = Snapshot { color: [0.5, 0.25, 1.0], size: 2.0, ..Snapshot::at([3.0, 4.0, 0.0]) };
        let size = child.start.size.min;
        child.burst_at(from, &Spectrum::SILENT);
        assert_eq!(child.live[0].color, [0.5, 0.25, 1.0]);
        assert_eq!(child.live[0].size, size * 2.0);
        assert_eq!(&child.live[0].position[..2], &[3.0, 4.0]);
    }

    #[test]
    fn a_line_that_does_not_mirror_starts_again() {
        let line = Line { from: [0.0; 3], to: [3.0, 0.0, 0.0], count: 4, mirror: false };
        let walked: Vec<f32> = (0..6).map(|n| line.at(n)[0]).collect();
        assert_eq!(walked, [0.0, 1.0, 2.0, 3.0, 0.0, 1.0]);
    }

    #[test]
    fn a_sheet_played_once_stops_on_its_last_frame() {
        let mut s = system(
            r#"{"material":"m","maxcount":1,"animationmode":"once",
            "emitter":[{"name":"sphererandom","instantaneous":1,"distancemax":0}],
            "initializer":[{"name":"lifetimerandom","min":1,"max":1}]}"#,
        );
        run(&mut s, 0.5, 0.1);
        let frame = s.vertices[FLOATS_PER_VERTEX - 1];
        assert!((frame - 0.5).abs() < 0.15, "{frame}");
    }

    /// A rate is how fast the system's time runs: at a tenth, a particle lives
    /// ten times as long and moves a tenth as far in the same second, and just
    /// as many are alive once it has got going.
    #[test]
    fn a_rate_is_how_fast_the_systems_time_runs() {
        let at = |rate: f32| {
            let definition = ParticleSystem::parse(
                br#"{"maxcount":100,"emitter":[{"name":"sphererandom","rate":10,"distancemax":0}],
                    "initializer":[{"name":"lifetimerandom","min":1,"max":1},
                                   {"name":"velocityrandom","min":"100 0 0","max":"100 0 0"}],
                    "operator":[{"name":"movement","gravity":"0 0 0"}]}"#,
            )
            .unwrap();
            let mut over = InstanceOverride::default();
            over.rate.value = rate;
            System::new(&definition, &over, 1)
        };
        let mut slow = at(0.1);
        run(&mut slow, 30.0, 1.0 / 60.0);
        let mut full = at(1.0);
        run(&mut full, 30.0, 1.0 / 60.0);
        assert!((slow.count() as i32 - full.count() as i32).abs() <= 2, "{} {}", slow.count(), full.count());
        let farthest = |s: &System| s.live.iter().map(|p| p.position[0]).fold(0.0, f32::max);
        // A whole life is as far as either gets, however long it took.
        assert!((85.0..=100.0).contains(&farthest(&full)), "{}", farthest(&full));
        assert!((85.0..=100.0).contains(&farthest(&slow)), "{}", farthest(&slow));
        // The youngest, which will not die during the step.
        let youngest = slow.live.last().unwrap();
        let (serial, before) = (youngest.serial, youngest.position[0]);
        slow.advance(0.1, &Spectrum::SILENT);
        let now = slow.live.iter().find(|p| p.serial == serial).unwrap().position[0];
        assert!((now - before - 1.0).abs() < 0.01, "{}", now - before);
    }

    /// `offset` turns the start about `right` the way a negative angle turns
    /// things: up goes to the right, in the system's own axes.
    #[test]
    fn a_turbulent_start_turns_by_its_offset() {
        let field = Turbulent {
            speed: Range { min: 1.0, max: 1.0 },
            scale: 0.001,
            time_scale: 1.0,
            phase_max: 0.0,
            offset: -0.5,
            forward: [0.0, 1.0, 0.0],
            right: [0.0, 0.0, 1.0],
            audio: None,
        };
        let along = launch_direction(&field, [10.0, 20.0, 0.0], 0.0, 0.0);
        let angle = along[1].atan2(along[0]).to_degrees();
        assert!((angle - 61.4).abs() < 1.0, "{angle}");
    }

    /// Every particle system Wallpaper Engine ships, which is the vocabulary
    /// a wallpaper not in this library can be expected to use: all of it is
    /// read and none of it runs away. A collision with a model and an emitter
    /// shaped like a layer's picture have nothing here to work from yet.
    #[test]
    fn every_stock_system_is_read_and_stays_finite() {
        let Some(assets) = wpe_format::assets::Assets::locate() else { return };
        fn walk(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, found);
                } else if path.extension().is_some_and(|e| e == "json")
                    && path.to_string_lossy().contains("/particles/")
                {
                    found.push(path);
                }
            }
        }
        let mut files = Vec::new();
        walk(assets.root(), &mut files);
        assert!(files.len() > 200, "{}", files.len());
        for file in files {
            let Ok(definition) = ParticleSystem::parse(&std::fs::read(&file).unwrap()) else {
                continue;
            };
            for what in &definition.unsupported {
                assert!(
                    ["operator collisionmodel", "emitter layerimage"].contains(&what.as_str()),
                    "{}: {what}",
                    file.display()
                );
            }
            let mut s = System::new(&definition, &InstanceOverride::default(), 1);
            s.bound_by(Range { min: [0.0; 3], max: [1920.0, 1080.0, 0.0] });
            s.thin_to(2000);
            run(&mut s, 3.0, 1.0 / 30.0);
            for p in &s.live {
                assert!(
                    p.position.iter().chain(&p.velocity).all(|c| c.is_finite()) && p.shown_size.is_finite(),
                    "{}",
                    file.display()
                );
            }
        }
    }

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
        // Ten a second for a second of its own time is ten, and the pool is
        // eight.
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
        assert_eq!(died.len(), 1);
        assert_eq!(died[0].position, [100.0, 50.0, 0.0]);
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
        let ride = |parent: &mut System, child: &mut System| {
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
    fn a_systems_start_time_is_how_far_ahead_it_begins() {
        let mut s = system(
            r#"{"maxcount":100,"starttime":2,
                "emitter":[{"name":"sphererandom","rate":50,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 1.0 / 60.0, 1.0 / 60.0);
        assert!(s.count() >= 90, "{} after one step", s.count());
    }

    /// The embers' box, wider than it is tall: "512 256 0" read as one
    /// number was nought, and every ember was born at the same point.
    #[test]
    fn a_box_spreads_each_axis_as_far_as_it_says() {
        let mut s = system(
            r#"{"maxcount":200,"emitter":[{"name":"boxrandom","rate":1000,
                 "directions":"1 1 0","distancemax":"512 256 0"}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 0.2, 1.0 / 60.0);
        assert_eq!(s.count(), 200);
        let widest = |axis: usize| s.live.iter().map(|p| p.position[axis].abs()).fold(0.0, f32::max);
        assert!((400.0..=512.0).contains(&widest(0)), "{} across", widest(0));
        assert!((200.0..=256.0).contains(&widest(1)), "{} up", widest(1));
        assert_eq!(widest(2), 0.0);
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

    /// A particle of `sizerandom` 1000 is drawn 500 across.
    #[test]
    fn sizerandom_is_twice_the_size_drawn() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"sizerandom","min":1000,"max":1000}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        assert_eq!(s.count(), 1);
        assert_eq!(s.vertices()[3 + 3], 500.0);
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
                "operator":[{"name":"alphafade","fadeintime":0.1,"fadeouttime":1}]}"#,
        );
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        assert_eq!(s.count(), 1);
        let early = written_alpha(&s);
        assert!(early < 0.1, "born at {early} instead of nearly nothing");
        // A tenth of a ten-second life.
        run(&mut s, 1.0, 1.0 / 60.0);
        assert!((written_alpha(&s) - 1.0).abs() < 0.01);
    }

    /// Smoke states how it comes in and nothing about going: it thins out
    /// from halfway through its life, which is the engine's default, rather
    /// than holding its full alpha and vanishing at the end.
    #[test]
    fn alphafade_without_a_fade_out_thins_from_halfway() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":4,"max":4},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"alphafade","fadeintime":0.1}]}"#,
        );
        run(&mut s, 2.0, 1.0 / 60.0);
        assert!(written_alpha(&s) > 0.95, "{}", written_alpha(&s));
        run(&mut s, 1.0, 1.0 / 60.0);
        let three_quarters = written_alpha(&s);
        assert!((three_quarters - 0.5).abs() < 0.05, "{three_quarters}");
    }

    #[test]
    fn a_ramp_with_no_end_runs_down_to_nothing() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":2,"max":2},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"alphachange"}]}"#,
        );
        run(&mut s, 1.0, 1.0 / 60.0);
        let halfway = written_alpha(&s);
        assert!((halfway - 0.5).abs() < 0.05, "{halfway}");
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
        // Drawn half what `sizerandom` says, as every particle is.
        assert!((size(&s) - 50.0).abs() < 2.5, "started at {}", size(&s));
        run(&mut s, 0.5, 1.0 / 60.0);
        assert!((size(&s) - 100.0).abs() < 5.0, "halfway at {}", size(&s));
    }

    /// Four arms at a count of four. Romulus' instance says 0.8 of everything,
    /// and on Windows its specks fill a disc, which 3.2 arms going round do.
    #[test]
    fn an_instance_count_scales_the_sequence_around_a_point() {
        let definition = |count: f32| {
            let mut over = InstanceOverride::default();
            over.count.value = count;
            let file = ParticleSystem::parse(
                br#"{"material":"m","maxcount":8,
                    "emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                    "initializer":[{"name":"lifetimerandom","min":9,"max":9},
                                   {"name":"mapsequencearoundcontrolpoint","count":4,
                                    "speedmin":"0 100 0","speedmax":"0 100 0"}]}"#,
            )
            .unwrap();
            let mut s = System::new(&file, &over, 12345);
            s.advance(1.0 / 30.0, &Spectrum::SILENT);
            s
        };
        let directions = |s: &System| {
            let mut seen: Vec<i32> = s
                .live
                .iter()
                .map(|p| (p.velocity[1].atan2(p.velocity[0]).to_degrees().round() as i32 + 360) % 360)
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen.len()
        };
        assert_eq!(directions(&definition(1.0)), 4);
        assert!(directions(&definition(0.8)) > 4);
    }

    /// Alien Romulus' specks grow to 1.8 and shrink from it. Multiplied they
    /// peak at its square and run together into one black blot at the mouse.
    #[test]
    fn size_steps_take_over_from_one_another() {
        let ramp = |min, max, from, to| Operator::SizeChange(wpe_format::particle::Ramp {
            time: Range { min, max },
            from,
            to,
        });
        let steps = [ramp(0.0, 0.05, 0.0, 1.8), ramp(0.2, 1.0, 1.8, 0.0)];
        assert_eq!(size_factor(&steps, 0.0), 0.0);
        assert_eq!(size_factor(&steps, 0.1), 1.8);
        assert!((size_factor(&steps, 0.6) - 0.9).abs() < 1e-5);
        assert_eq!(size_factor(&steps, 1.0), 0.0);
        assert_eq!(size_factor(&[], 0.5), 1.0);
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
            forward: [0.0, 1.0, 0.0],
            right: [0.0, 0.0, 1.0],
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
    fn turbulence_pushes_a_particle_no_harder_than_it_says() {
        let mut s = system(
            r#"{"maxcount":20,"emitter":[{"name":"boxrandom","rate":100,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"movement"},
                            {"name":"turbulence","scale":0.002,"speedmin":100,
                             "speedmax":150,"phasemax":5}]}"#,
        );
        run(&mut s, 2.0, 1.0 / 60.0);
        let mut moved = 0;
        for p in &s.live {
            let travelled = (p.position[0].powi(2) + p.position[1].powi(2)).sqrt();
            // Two seconds pushed at 150 a second each second: half of 150 t squared.
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
                "operator":[{"name":"movement"},
                            {"name":"turbulence","scale":0.002,"speedmin":100,
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
    fn a_scenes_colour_takes_the_hue_and_leaves_the_brightness() {
        let over = InstanceOverride {
            colorn: Some(wpe_format::scene::Bound::plain(wpe_format::scene::Vector([1.0, 0.0, 0.0]))),
            ..Default::default()
        };
        let definition = ParticleSystem::parse(
            br#"{"material":"m.json","maxcount":1,"emitter":[{"name":"boxrandom","rate":100}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10},
                               {"name":"colorrandom","min":"128 128 128","max":"128 128 128"}],
                "operator":[{"name":"colorchange","startvalue":"1 0.5 0","endvalue":"0 0 1"}]}"#,
        )
        .unwrap();
        let mut s = System::new(&definition, &over, 12345);
        run(&mut s, 0.1, 1.0 / 60.0);
        let [r, g, b] = s.live[0].shown_color;
        assert!((r - 128.0 / 255.0).abs() < 1e-3 && g == 0.0 && b == 0.0, "{:?}", [r, g, b]);
        // Its colour change is a change of brightness now, not of hue.
        for operator in &s.operators {
            if let Operator::ColorChange { from, to, .. } = operator {
                assert!(from[0] == from[1] && from[1] == from[2] && to[0] == to[2], "{from:?} {to:?}");
            }
        }
    }

    #[test]
    fn a_system_on_the_mouse_lets_out_nothing_without_one_and_there_with_one() {
        let mut s = system(
            r#"{"maxcount":10,"controlpoint":[{"id":0,"flags":1}],
                "emitter":[{"name":"sphererandom","rate":100,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}]}"#,
        );
        run(&mut s, 0.5, 1.0 / 60.0);
        assert_eq!(s.live.len(), 0);
        s.point_at(Some([300.0, 200.0, 0.0]));
        run(&mut s, 0.5, 1.0 / 60.0);
        assert!(!s.live.is_empty());
        assert!(s.live.iter().all(|p| (p.position[0] - 300.0).abs() < 1.0), "not at the mouse");
    }

    #[test]
    fn two_particles_in_one_place_are_not_pushed_as_one() {
        let mut s = system(
            r#"{"maxcount":30,"emitter":[{"name":"boxrandom","rate":200,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":10,"max":10}],
                "operator":[{"name":"movement"},
                            {"name":"turbulence","scale":0.002,"speedmin":100,
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

    /// Which way round a vortex sends a particle at (200, 0): up is the way of
    /// a positive speed, with the y of a scene running up.
    fn pushed_by_vortex(speed: &str) -> [f32; 3] {
        let mut s = system(&format!(
            r#"{{"maxcount":1,"emitter":[{{"name":"boxrandom","rate":1000,"distancemax":0}}],
                "initializer":[{{"name":"lifetimerandom","min":10,"max":10}},
                               {{"name":"velocityrandom","min":"0 0 0","max":"0 0 0"}}],
                "operator":[{{"name":"movement"}},
                            {{"name":"vortex","axis":"0 0 1","distanceinner":0,
                             "distanceouter":500,"speedinner":{speed},"speedouter":{speed}}}]}}"#,
        ));
        s.advance(1.0 / 60.0, &Spectrum::SILENT);
        s.live[0].position = [200.0, 0.0, 0.0];
        s.live[0].velocity = [0.0; 3];
        s.advance(0.1, &Spectrum::SILENT);
        s.live[0].velocity
    }

    #[test]
    fn a_vortex_pushes_a_particle_along_the_way_round() {
        let v = pushed_by_vortex("90");
        assert!((v[1] - 9.0).abs() < 0.01, "pushed {v:?}");
        assert!(v[0].abs() < 0.01, "pulled in or out: {v:?}");
        assert!(pushed_by_vortex("-90")[1] < -8.0, "a negative speed goes the other way");
    }

    #[test]
    fn oscillatealpha_dims_a_particle_and_brings_it_back() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100},
                               {"name":"alpharandom","min":1,"max":1}],
                "operator":[{"name":"oscillatealpha","frequencymin":6.283,"frequencymax":6.283,
                             "scalemin":0.2}]}"#,
        );
        let mut seen: Vec<f32> = Vec::new();
        for _ in 0..90 {
            s.advance(1.0 / 60.0, &Spectrum::SILENT);
            seen.push(written_alpha(&s));
        }
        let low = seen.iter().cloned().fold(f32::MAX, f32::min);
        let high = seen.iter().cloned().fold(f32::MIN, f32::max);
        assert!(high > 0.95, "never came back up: {high}");
        // Down to the end it states, and up to the end it leaves out.
        assert!((0.15..0.35).contains(&low), "dimmed to {low}");
    }

    #[test]
    fn oscillateposition_sways_a_particle_about_where_it_sits() {
        let mut s = system(
            r#"{"maxcount":1,"emitter":[{"name":"boxrandom","rate":1000,"distancemax":0}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}],
                "operator":[{"name":"oscillateposition","frequencymin":6.283,"frequencymax":6.283,
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
