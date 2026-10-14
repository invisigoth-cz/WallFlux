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
    /// How far from the origin a particle starts.
    pub distance: Range<f32>,
    /// Particles per second.
    pub rate: f32,
    /// Speed given along the spawn direction, on top of any velocity an
    /// initializer sets.
    pub speed: Range<f32>,
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
    /// Added to the field before it is used.
    pub offset: f32,
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
    /// Fades in over the first seconds of life and out over the last.
    AlphaFade {
        fade_in: f32,
        fade_out: f32,
    },
    SizeChange(Ramp),
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
    /// Named steps this reader does not handle yet.
    pub unsupported: Vec<String>,
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
            // drawn from, so they differ only in which value they fill.
            let scalar = || Range {
                min: num(step, "min", 0.0),
                max: num(step, "max", 0.0),
            };
            let triple = || Range {
                min: vector(step, "min", [0.0; 3]),
                max: vector(step, "max", [0.0; 3]),
            };
            initializers.push(match name_of(step) {
                "lifetimerandom" => Initializer::Lifetime(scalar()),
                "sizerandom" => Initializer::Size(scalar()),
                "alpharandom" => Initializer::Alpha(scalar()),
                "colorrandom" => {
                    let raw = triple();
                    Initializer::Color(Range {
                        min: scale_color(raw.min),
                        max: scale_color(raw.max),
                    })
                }
                "velocityrandom" => Initializer::Velocity(triple()),
                "turbulentvelocityrandom" => Initializer::TurbulentVelocity(turbulent(step, 0.1)),
                "rotationrandom" => Initializer::Rotation(triple()),
                "angularvelocityrandom" => Initializer::AngularVelocity(triple()),
                other => {
                    unsupported.push(format!("initializer {other}"));
                    continue;
                }
            });
        }

        for step in array(&root, "operator") {
            operators.push(match name_of(step) {
                "movement" => Operator::Movement {
                    gravity: vector(step, "gravity", [0.0; 3]),
                    drag: num(step, "drag", 0.0),
                },
                "alphafade" => Operator::AlphaFade {
                    fade_in: num(step, "fadeintime", 0.0),
                    fade_out: num(step, "fadeouttime", 0.0),
                },
                "sizechange" => Operator::SizeChange(Ramp {
                    time: Range {
                        min: num(step, "starttime", 0.0),
                        max: num(step, "endtime", 1.0),
                    },
                    from: num(step, "startvalue", 1.0),
                    to: num(step, "endvalue", 1.0),
                }),
                "colorchange" => Operator::ColorChange {
                    time: Range {
                        min: num(step, "starttime", 0.0),
                        max: num(step, "endtime", 1.0),
                    },
                    from: vector(step, "startvalue", [1.0; 3]),
                    to: vector(step, "endvalue", [1.0; 3]),
                },
                "vortex" => Operator::Vortex(Vortex {
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
                "turbulence" => Operator::Turbulence {
                    field: turbulent(step, 0.002),
                    mask: vector(step, "mask", [1.0; 3]),
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

        Ok(Self {
            material: root
                .get("material")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            maxcount: num(&root, "maxcount", 0.0).clamp(0.0, MAX_PARTICLES as f32) as usize,
            starttime: num(&root, "starttime", 0.0),
            emitters,
            initializers,
            operators,
            renderer,
            animation,
            unsupported,
        })
    }
}

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
        distance: Range {
            min: num(step, "distancemin", 0.0),
            max: num(step, "distancemax", 0.0),
        },
        rate: num(step, "rate", 0.0),
        speed: Range {
            min: num(step, "speedmin", 0.0),
            max: num(step, "speedmax", 0.0),
        },
    }
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
    }
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
                    min: 32.0,
                    max: 512.0
                },
                rate: 20.0,
                speed: Range { min: 0.0, max: 0.0 },
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
                fade_out: 0.0
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
        assert_eq!(p.emitters[0].distance.max, 100.0);
        assert_eq!(
            p.initializers[0],
            Initializer::Size(Range { min: 1.0, max: 0.0 })
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
                "operator controlpointattract"
            ]
        );
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
