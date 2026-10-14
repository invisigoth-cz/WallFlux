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
    Animation, Emitter, Initializer, Operator, ParticleSystem, Range, Shape,
};

/// Floats per vertex: position, then corner and rotation and size, then
/// colour, then the two remaining rotation axes, then velocity and lifetime.
pub const FLOATS_PER_VERTEX: usize = 17;
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
    velocity: [f32; 3],
    rotation: [f32; 3],
    size: f32,
    color: [f32; 3],
    alpha: f32,
    age: f32,
    lifetime: f32,
    /// Where in a sprite sheet this particle starts, for `randomframe`.
    frame: f32,
}

/// The ranges the initializers draw a new particle's values from.
struct Start {
    lifetime: Range<f32>,
    size: Range<f32>,
    alpha: Range<f32>,
    color: Range<[f32; 3]>,
    velocity: Range<[f32; 3]>,
    rotation: Range<[f32; 3]>,
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
        }
    }
}

pub struct System {
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
}

impl System {
    pub fn new(definition: &ParticleSystem, seed: u32) -> Self {
        let mut start = Start::default();
        for initializer in &definition.initializers {
            match *initializer {
                Initializer::Lifetime(r) => start.lifetime = r,
                Initializer::Size(r) => start.size = r,
                Initializer::Alpha(r) => start.alpha = r,
                Initializer::Color(r) => start.color = r,
                Initializer::Velocity(r) => start.velocity = r,
                Initializer::Rotation(r) => start.rotation = r,
                // Carried as a starting value but nothing turns it yet: the
                // operator that spins a particle is `angularmovement`.
                Initializer::AngularVelocity(_) => {}
            }
        }
        Self {
            emitters: definition
                .emitters
                .iter()
                .cloned()
                .map(|e| (e, 0.0))
                .collect(),
            start,
            operators: definition.operators.clone(),
            animation: definition.animation,
            maxcount: definition.maxcount,
            starttime: definition.starttime,
            live: Vec::new(),
            elapsed: 0.0,
            rng: Rng::seeded(seed),
            vertices: Vec::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.live.len()
    }

    /// The most particles this system can have at once, which is how big a
    /// buffer it needs.
    pub fn capacity(&self) -> usize {
        self.maxcount
    }

    /// The buffer to draw, four vertices per live particle.
    pub fn vertices(&self) -> &[f32] {
        &self.vertices
    }

    /// Moves the system on by `dt` seconds and rewrites the buffer.
    pub fn advance(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, MAX_STEP);
        self.elapsed += dt;

        if self.elapsed >= self.starttime {
            self.emit(dt);
        }

        for particle in &mut self.live {
            particle.age += dt;
            for operator in &self.operators {
                if let Operator::Movement { gravity, drag } = operator {
                    for (axis, gravity) in gravity.iter().enumerate() {
                        particle.velocity[axis] += gravity * dt;
                        // Drag as a proportion shed per second, which stays
                        // stable whatever the frame rate and never reverses
                        // the velocity the way `1 - drag * dt` would.
                        particle.velocity[axis] /= 1.0 + drag * dt;
                        particle.position[axis] += particle.velocity[axis] * dt;
                    }
                }
            }
        }
        self.live.retain(|p| p.age < p.lifetime);
        self.write_vertices();
    }

    fn emit(&mut self, dt: f32) {
        // Taken apart so the borrow of the emitters does not hold the whole
        // system while a particle is being built.
        let mut emitters = std::mem::take(&mut self.emitters);
        for (emitter, owed) in &mut emitters {
            *owed += emitter.rate * dt;
            while *owed >= 1.0 {
                *owed -= 1.0;
                if self.live.len() >= self.maxcount {
                    break;
                }
                let particle = self.spawn(emitter);
                self.live.push(particle);
            }
        }
        self.emitters = emitters;
    }

    fn spawn(&mut self, emitter: &Emitter) -> Particle {
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

        let speed = self.rng.range(emitter.speed);
        let drawn = self.rng.range3(self.start.velocity);
        let lifetime = self.rng.range(self.start.lifetime).max(f32::EPSILON);
        Particle {
            position,
            // Only six systems in the library set both, so which of the two
            // wins is nearly moot; adding them keeps either on its own right.
            velocity: std::array::from_fn(|i| drawn[i] + direction[i] * speed),
            rotation: self.rng.range3(self.start.rotation),
            size: self.rng.range(self.start.size),
            color: self.rng.range3(self.start.color),
            alpha: self.rng.range(self.start.alpha),
            age: 0.0,
            lifetime,
            frame: self.rng.next_f32(),
        }
    }

    fn write_vertices(&mut self) {
        self.vertices.clear();
        self.vertices
            .reserve(self.live.len() * VERTICES_PER_PARTICLE * FLOATS_PER_VERTEX);
        for particle in &self.live {
            let progress = particle.age / particle.lifetime;
            let alpha = particle.alpha * fade(&self.operators, particle.age, particle.lifetime);
            let size = particle.size * size_factor(&self.operators, progress);
            // The shader takes the fraction of this, so a multiplier above one
            // walks the sheet more than once over a particle's life.
            let frame = match self.animation {
                Some(Animation::Sequence { multiplier }) => progress * multiplier,
                Some(Animation::RandomFrame) => particle.frame,
                None => progress,
            };
            for corner in CORNERS {
                self.vertices.extend_from_slice(&particle.position);
                self.vertices.extend_from_slice(&[
                    corner[0],
                    corner[1],
                    particle.rotation[2],
                    size,
                ]);
                self.vertices.extend_from_slice(&[
                    particle.color[0],
                    particle.color[1],
                    particle.color[2],
                    alpha,
                ]);
                self.vertices
                    .extend_from_slice(&[particle.rotation[0], particle.rotation[1]]);
                self.vertices.extend_from_slice(&particle.velocity);
                self.vertices.push(frame);
            }
        }
    }
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
            let span = ramp.time.max - ramp.time.min;
            let t = if span > 0.0 {
                ((progress - ramp.time.min) / span).clamp(0.0, 1.0)
            } else {
                // A ramp with no span is a step at its start time.
                (progress >= ramp.time.min) as u8 as f32
            };
            out *= ramp.from + (ramp.to - ramp.from) * t;
        }
    }
    out
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

    fn system(json: &str) -> System {
        let definition = ParticleSystem::parse(json.as_bytes()).unwrap();
        System::new(&definition, 12345)
    }

    /// Steps a system the way a renderer would rather than in one jump, since
    /// one long step is deliberately clamped.
    fn run(system: &mut System, seconds: f32, step: f32) {
        for _ in 0..(seconds / step).round() as usize {
            system.advance(step);
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
        s.advance(1.0 / 60.0);
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
        s.advance(1.0 / 60.0);
        let size = |s: &System| s.vertices()[3 + 3];
        assert!((size(&s) - 100.0).abs() < 5.0, "started at {}", size(&s));
        run(&mut s, 0.5, 1.0 / 60.0);
        assert!((size(&s) - 200.0).abs() < 10.0, "halfway at {}", size(&s));
    }

    #[test]
    fn a_frame_so_long_it_would_empty_the_pool_is_capped() {
        let mut s = system(
            r#"{"maxcount":10000,"emitter":[{"name":"sphererandom","rate":1000,"distancemax":1}],
                "initializer":[{"name":"lifetimerandom","min":100,"max":100}]}"#,
        );
        // A stalled frame, as if the wallpaper had been paused for a minute.
        s.advance(60.0);
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
