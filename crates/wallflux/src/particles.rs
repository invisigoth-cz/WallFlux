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
    Animation, Emitter, Initializer, Operator, Oscillation, ParticleSystem, Range, Shape,
    Turbulent, Vortex,
};

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

pub struct System {
    /// Threads every particle onto one ribbon instead of drawing each as its
    /// own quad.
    rope: bool,
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
                Initializer::TurbulentVelocity(field) => start.turbulent = Some(field),
                // Carried as a starting value but nothing turns it yet: the
                // operator that spins a particle is `angularmovement`.
                Initializer::AngularVelocity(r) => start.angular = r,
            }
        }
        Self {
            rope: matches!(
                definition.renderer,
                wpe_format::particle::Renderer::Rope { .. }
            ),
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

    /// Whether this system draws as one ribbon rather than loose quads.
    pub fn is_rope(&self) -> bool {
        self.rope
    }

    /// How many four-vertex pieces the buffer holds.
    pub fn pieces(&self) -> usize {
        if self.rope {
            self.live.len().saturating_sub(1)
        } else {
            self.live.len()
        }
    }

    /// Moves the system on by `dt` seconds and rewrites the buffer.
    pub fn advance(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, MAX_STEP);
        self.elapsed += dt;

        if self.elapsed >= self.starttime {
            self.emit(dt);
        }

        let elapsed = self.elapsed;
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
                        let speed =
                            field.speed.min + (field.speed.max - field.speed.min) * particle.drift;
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
                    // ponytail: a vortex carries the particle round rather
                    // than pulling it in, for the same reason turbulence
                    // does -- a speed the file states stays the speed.
                    Operator::Vortex(vortex) => {
                        let turn = vortex_turn(vortex, particle.position) * dt;
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
        let phase = self.rng.next_f32();

        // A turbulent start reads the field once, where the particle appears.
        let turbulent = match self.start.turbulent {
            Some(field) => {
                let along =
                    field_direction(&field, position, phase * field.phase_max, self.elapsed);
                let speed = self.rng.range(field.speed);
                along.map(|axis| axis * speed)
            }
            None => [0.0; 3],
        };

        Particle {
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
        }
    }

    fn write_vertices(&mut self) {
        if self.rope {
            self.write_rope();
            return;
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
    /// Writes the ribbon: one piece per pair of particles, each carrying the
    /// whole segment so the shader can interpolate across it.
    ///
    /// The control points a spline would bend through are set to the segment's
    /// own ends, which leaves every segment straight. Smoothing them is what
    /// the file's `subdivision` is for and is not done yet.
    fn write_rope(&mut self) {
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
        s.advance(1.0 / 60.0);
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
            s.advance(1.0 / 60.0);
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
            s.advance(1.0 / 60.0);
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
            s.advance(1.0 / 60.0);
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
            s.advance(1.0 / 60.0);
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
        s.advance(1.0 / 60.0);
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
