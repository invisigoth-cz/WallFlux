// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The JavaScript a scene carries, run.
//!
//! A scene does not ship scripts as files. A value in `scene.json` -- an
//! object's scale, a particle system's rate, an effect's constant -- may carry
//! an ES module as its `script`, whose `update` is handed the value every
//! frame and answers with the next one. 93 of them in 7 of the 41 packages
//! here. They are run with QuickJS, through `rquickjs`.
//!
//! What a script finds around it is `script/host.js`: vectors, `engine`,
//! `thisLayer` and `thisScene.getLayer`, `createScriptProperties` and the
//! sound. Every object in the scene is a layer there, drawn or not -- some
//! scenes keep an invisible object only to carry a number other scripts read
//! off it -- and the layers are where an object's place, turn, size, alpha
//! and visibility live while the scene runs, whoever changed them.
//!
//! **Nothing a script does can stop the desktop being drawn.** Each call has
//! a deadline and is interrupted past it; a script that throws or runs over
//! is not run again, and the value it drove stays where it was.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::loader::{BuiltinLoader, BuiltinResolver};
use rquickjs::{Context, Ctx, Function, Module, Runtime};
use wpe_format::Scene as SceneFile;
use wpe_format::particle::Spectrum;
use wpe_format::scene::Bound;

const HOST: &str = include_str!("script/host.js");

/// The one module the library imports by name.
const WEMATH: &str = "
export const deg2rad = Math.PI / 180;
export const rad2deg = 180 / Math.PI;
export function mix(a, b, t) { return a + (b - a) * t; }
export function clamp(v, lo, hi) { return Math.min(Math.max(v, lo), hi); }
export function smoothStep(lo, hi, v) {
    const t = clamp((v - lo) / (hi - lo), 0, 1);
    return t * t * (3 - 2 * t);
}
";

/// How long one call into a script may take. A frame is sixteen
/// milliseconds; a script that wants more than this is not going to fit.
const DEADLINE: Duration = Duration::from_millis(20);

/// What a script's value is, as far as the renderer is concerned.
#[derive(Debug, Clone, PartialEq)]
pub enum Drives {
    /// A property of the object's own layer, read back with the layer.
    Layer,
    /// The rate of the particle system object `id` places.
    Rate { id: i64 },
    /// A constant of pass `pass` of effect `effect` on object `id`, by the
    /// name the scene gives it.
    Constant {
        id: i64,
        effect: usize,
        pass: usize,
        name: String,
    },
}

/// Where an object stands after a frame of scripts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub origin: [f32; 3],
    pub angles: [f32; 3],
    pub scale: [f32; 3],
    pub alpha: f32,
    pub visible: bool,
}

/// What one frame of scripts left behind.
pub struct Frame<'a> {
    /// Every object in the scene, by its id.
    pub layers: Vec<(i64, Layer)>,
    /// Each value a script drives that is not a layer's own.
    pub values: Vec<(&'a Drives, serde_json::Value)>,
    /// Every text in the scene, by the id of its object.
    pub texts: Vec<(i64, String)>,
}

pub struct Scripts {
    // The context goes first: it holds on to the runtime.
    context: Context,
    _runtime: Runtime,
    deadline: Rc<Cell<Instant>>,
    targets: Vec<Drives>,
    /// What each script drives, for saying which one failed.
    names: Vec<String>,
    ids: Vec<i64>,
    listens: bool,
    /// Scripts there are and that are not run, with why.
    pub skipped: Vec<String>,
}

/// One scripted value found in the scene: what it drives, its script, its
/// value and the settings chosen for it.
struct Found<'a> {
    layer: usize,
    property: Option<&'static str>,
    target: Drives,
    script: &'a str,
    start: serde_json::Value,
    chosen: Option<&'a serde_json::Value>,
    name: String,
}

impl Scripts {
    /// The scripts of `file`, ready to run, or nothing where it has none or
    /// none of them could be started. `project` is the wallpaper's
    /// project.json, whose settings a script is handed when it starts.
    pub fn load(file: &SceneFile, project: Option<&[u8]>) -> Option<Self> {
        let mut skipped = Vec::new();
        let found = find(file, &mut skipped);
        if found.is_empty() {
            return None;
        }
        match Self::start(file, project, found, skipped) {
            Ok(scripts) => Some(scripts),
            Err(e) => {
                eprintln!("wallflux scene: the scripts could not be started -- {e}");
                None
            }
        }
    }

    fn start(
        file: &SceneFile,
        project: Option<&[u8]>,
        found: Vec<Found<'_>>,
        mut skipped: Vec<String>,
    ) -> rquickjs::Result<Self> {
        let runtime = Runtime::new()?;
        // A scene's scripts hold a few numbers each; this is room for a
        // thousand times that, and a stop for one that grows without end.
        runtime.set_memory_limit(64 << 20);
        runtime.set_max_stack_size(1 << 20);
        let deadline = Rc::new(Cell::new(Instant::now() + DEADLINE));
        let watched = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() > watched.get())));
        runtime.set_loader(
            BuiltinResolver::default().with_module("WEMath"),
            BuiltinLoader::default().with_module("WEMath", WEMATH),
        );
        let context = Context::full(&runtime)?;

        let layers = layers_json(file);
        let user = user_json(project);
        let mut targets = Vec::new();
        let mut names = Vec::new();
        context.with(|ctx| -> rquickjs::Result<()> {
            deadline.set(Instant::now() + DEADLINE);
            ctx.eval::<(), _>(HOST)?;
            call::<()>(&ctx, "__layers", (layers,))?;
            for (index, one) in found.into_iter().enumerate() {
                deadline.set(Instant::now() + DEADLINE);
                let module = format!("{}#{index}", one.name);
                let namespace = Module::declare(ctx.clone(), module, one.script)
                    .and_then(|m| m.eval())
                    .and_then(|(m, promise)| {
                        promise.finish::<()>()?;
                        m.namespace()
                    });
                let namespace = match namespace {
                    Ok(n) => n,
                    Err(e) => {
                        skipped.push(format!("{}: {}", one.name, thrown(&ctx, e)));
                        continue;
                    }
                };
                let said: String = call(
                    &ctx,
                    "__script",
                    (
                        namespace,
                        one.layer,
                        one.property,
                        one.start.to_string(),
                        one.chosen.map_or("null".to_owned(), |c| c.to_string()),
                        user.clone(),
                    ),
                )
                .unwrap_or_else(|e| thrown(&ctx, e));
                if !said.is_empty() {
                    skipped.push(format!("{}: {said}", one.name));
                }
                targets.push(one.target);
                names.push(one.name);
            }
            Ok(())
        })?;
        let listens = context.with(|ctx| call::<bool>(&ctx, "__listens", ()).unwrap_or(false));
        Ok(Self {
            context,
            _runtime: runtime,
            deadline,
            targets,
            names,
            ids: file.objects.iter().map(|o| o.id).collect(),
            listens,
            skipped,
        })
    }

    /// Whether a script asked for the sound.
    pub fn listens(&self) -> bool {
        self.listens
    }

    /// Runs every script once, `frametime` seconds after the last time and
    /// `runtime` after the scene started, and says where that left things.
    pub fn run(&mut self, frametime: f32, runtime: f32, sound: &Spectrum) -> Option<Frame<'_>> {
        let deadline = &self.deadline;
        let names = &self.names;
        let state = self.context.with(|ctx| -> rquickjs::Result<String> {
            deadline.set(Instant::now() + DEADLINE);
            call::<()>(
                &ctx,
                "__frame",
                (
                    frametime,
                    runtime,
                    sound.left.to_vec(),
                    sound.right.to_vec(),
                ),
            )?;
            for (index, name) in names.iter().enumerate() {
                deadline.set(Instant::now() + DEADLINE);
                let said = call::<String>(&ctx, "__update", (index,)).unwrap_or_else(|e| {
                    // Interrupted, or out of memory: not something the
                    // script could catch, so it is stopped from here.
                    let _ = call::<()>(&ctx, "__break", (index,));
                    thrown(&ctx, e)
                });
                if !said.is_empty() {
                    eprintln!("wallflux scene: the script on {name} stopped -- {said}");
                }
            }
            deadline.set(Instant::now() + DEADLINE);
            call(&ctx, "__state", ())
        });
        let state: serde_json::Value = match state.map(|s| serde_json::from_str(&s)) {
            Ok(Ok(state)) => state,
            _ => return None,
        };
        let layers = state["layers"]
            .as_array()?
            .iter()
            .zip(&self.ids)
            .filter_map(|(layer, id)| Some((*id, layer_from(layer)?)))
            .collect();
        let values = state["values"]
            .as_array()?
            .iter()
            .zip(&self.targets)
            .filter(|(_, target)| **target != Drives::Layer)
            .map(|(value, target)| (target, value.clone()))
            .collect();
        let texts = state["texts"]
            .as_array()?
            .iter()
            .zip(&self.ids)
            .filter_map(|(text, id)| Some((*id, text.as_str()?.to_owned())))
            .collect();
        Some(Frame { layers, values, texts })
    }
}

fn call<'js, T: rquickjs::FromJs<'js>>(
    ctx: &Ctx<'js>,
    name: &str,
    args: impl rquickjs::function::IntoArgs<'js>,
) -> rquickjs::Result<T> {
    let function: Function = ctx.globals().get(name)?;
    function.call(args)
}

/// What a failed call threw, as words.
fn thrown(ctx: &Ctx<'_>, error: rquickjs::Error) -> String {
    match error {
        rquickjs::Error::Exception => {
            let value = ctx.catch();
            value
                .as_exception()
                .and_then(|e| e.message())
                .or_else(|| value.as_string().and_then(|s| s.to_string().ok()))
                .unwrap_or_else(|| "it threw".to_owned())
        }
        other => other.to_string(),
    }
}

fn layer_from(value: &serde_json::Value) -> Option<Layer> {
    let vector = |v: &serde_json::Value| -> Option<[f32; 3]> {
        let a = v.as_array()?;
        Some([
            a.first()?.as_f64()? as f32,
            a.get(1)?.as_f64()? as f32,
            a.get(2)?.as_f64()? as f32,
        ])
    };
    Some(Layer {
        origin: vector(&value[0])?,
        angles: vector(&value[1])?,
        scale: vector(&value[2])?,
        alpha: value[3].as_f64()? as f32,
        visible: value[4].as_bool()?,
    })
}

fn layers_json(file: &SceneFile) -> String {
    let layers: Vec<_> = file
        .objects
        .iter()
        .map(|o| {
            let mut layer = serde_json::json!({
                "name": o.name,
                "origin": o.origin.value.0,
                "angles": o.angles.value.0,
                "scale": o.scale.value.0,
                "size": o.size.map_or([0.0; 2], |s| s.0),
                "alpha": o.alpha.value,
                "visible": o.visible.value,
            });
            if let Some(text) = o.text() {
                layer["text"] = text.value.clone();
            }
            layer
        })
        .collect();
    serde_json::Value::Array(layers).to_string()
}

/// The wallpaper's settings by name, as `applyUserProperties` is handed
/// them: every one of them, since at the start none has been sent yet.
fn user_json(project: Option<&[u8]>) -> String {
    let properties = project
        .and_then(|p| serde_json::from_slice::<serde_json::Value>(p).ok())
        .and_then(|p| p.pointer("/general/properties").cloned());
    let values: serde_json::Map<_, _> = match properties {
        Some(serde_json::Value::Object(all)) => all
            .into_iter()
            .filter_map(|(name, p)| Some((name, p.get("value")?.clone())))
            .collect(),
        _ => serde_json::Map::new(),
    };
    serde_json::Value::Object(values).to_string()
}

/// Every script in the scene this renderer can do something with, in the
/// order of the file. The rest are named in `skipped`.
fn find<'a>(file: &'a SceneFile, skipped: &mut Vec<String>) -> Vec<Found<'a>> {
    let mut found = Vec::new();
    for (layer, object) in file.objects.iter().enumerate() {
        let mut take = |property: Option<&'static str>,
                        target: Drives,
                        what: &str,
                        script: &'a Option<String>,
                        start: serde_json::Value,
                        chosen: &'a Option<serde_json::Value>| {
            if let Some(script) = script {
                found.push(Found {
                    layer,
                    property,
                    target,
                    script,
                    start,
                    chosen: chosen.as_ref(),
                    name: format!("{}/{what}", object.name),
                });
            }
        };
        let vector = |b: &Bound<wpe_format::scene::Vector<3>>| {
            serde_json::json!(b.value.0.map(|c| c.to_string()).join(" "))
        };
        for (property, bound) in [
            ("origin", &object.origin),
            ("angles", &object.angles),
            ("scale", &object.scale),
        ] {
            take(
                Some(property),
                Drives::Layer,
                property,
                &bound.script,
                vector(bound),
                &bound.scriptproperties,
            );
        }
        let alpha = &object.alpha;
        take(
            Some("alpha"),
            Drives::Layer,
            "alpha",
            &alpha.script,
            serde_json::json!(alpha.value),
            &alpha.scriptproperties,
        );
        let visible = &object.visible;
        take(
            Some("visible"),
            Drives::Layer,
            "visible",
            &visible.script,
            serde_json::json!(visible.value),
            &visible.scriptproperties,
        );

        if let Some(over) = &object.instanceoverride {
            let rate = &over.rate;
            take(
                None,
                Drives::Rate { id: object.id },
                "rate",
                &rate.script,
                serde_json::json!(rate.value),
                &rate.scriptproperties,
            );
            for (what, bound) in [
                ("alpha", &over.alpha),
                ("size", &over.size),
                ("lifetime", &over.lifetime),
                ("speed", &over.speed),
                ("count", &over.count),
            ] {
                if bound.script.is_some() {
                    skipped.push(format!(
                        "{}/instanceoverride/{what}: only the rate follows a script yet",
                        object.name
                    ));
                }
            }
        }
        for (effect, used) in object.effects.iter().enumerate() {
            if used.visible.script.is_some() {
                skipped.push(format!(
                    "{}/effects/{effect}/visible: not run yet",
                    object.name
                ));
            }
            for (pass, over) in used.passes.iter().enumerate() {
                for (name, bound) in &over.constantshadervalues {
                    let target = Drives::Constant {
                        id: object.id,
                        effect,
                        pass,
                        name: name.clone(),
                    };
                    take(
                        None,
                        target,
                        name,
                        &bound.script,
                        bound.value.clone(),
                        &bound.scriptproperties,
                    );
                }
            }
        }
        if let Some(text) = object.text() {
            take(
                Some("text"),
                Drives::Layer,
                "text",
                &text.script,
                text.value.clone(),
                &text.scriptproperties,
            );
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(objects: &str) -> SceneFile {
        SceneFile::parse(format!(r#"{{"objects":[{objects}]}}"#).as_bytes()).unwrap()
    }

    fn script(source: &str) -> String {
        serde_json::to_string(source).unwrap()
    }

    const SILENCE: Spectrum = Spectrum::SILENT;

    /// A script on a property drives it, and one on another object can read
    /// it off that object's layer -- which is how Ender Spartan's effects
    /// follow an invisible object turned by a script of its own. The turn is
    /// handed over in degrees, which is what the scene's clocks count in.
    #[test]
    fn a_script_drives_its_value_and_another_reads_it() {
        let turner = script("export function update(v) { v.z += 90; return v; }");
        let reader =
            script("export function update() { return thisScene.getLayer('a').angles.z; }");
        let file = scene(&format!(
            r#"{{"id":1,"name":"a","visible":false,"angles":{{"script":{turner},"value":"0 0 0"}}}},
               {{"id":2,"name":"b","instanceoverride":{{"rate":{{"script":{reader},"value":1}}}}}}"#
        ));
        let mut scripts = Scripts::load(&file, None).unwrap();
        assert!(scripts.skipped.is_empty(), "{:?}", scripts.skipped);
        let frame = scripts.run(0.0, 0.0, &SILENCE).unwrap();
        let (_, a) = frame.layers[0];
        assert!(
            (a.angles[2] - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "{:?}",
            a.angles
        );
        assert!(!a.visible);
        assert_eq!(
            frame.values,
            vec![(&Drives::Rate { id: 2 }, serde_json::json!(90))]
        );
    }

    /// A script answering with a number still leaves words: `date.getDate()`
    /// is how Dark Core writes the day.
    #[test]
    fn a_script_writes_a_text_and_a_number_becomes_words() {
        let date = script("export function update() { return 22; }");
        let word = script("export function update() { return 'x'; }");
        let file = scene(&format!(
            r#"{{"id":1,"name":"a","font":"f.ttf","text":{{"script":{date},"value":"1"}}}},
               {{"id":2,"name":"b","font":"f.ttf","text":{{"script":{word},"value":"y"}}}},
               {{"id":3,"name":"c"}}"#
        ));
        let mut scripts = Scripts::load(&file, None).unwrap();
        assert!(scripts.skipped.is_empty(), "{:?}", scripts.skipped);
        let frame = scripts.run(0.0, 0.0, &SILENCE).unwrap();
        assert_eq!(frame.texts, vec![(1, "22".to_owned()), (2, "x".to_owned())]);
    }

    /// A script that throws, or never stops, is set aside and the rest go
    /// on: a script that runs for ever would otherwise stop the desktop
    /// being drawn.
    #[test]
    fn a_script_that_throws_or_never_ends_is_stopped_alone() {
        let throws = script("export function update() { throw new Error('no'); }");
        let forever = script("export function update() { for (;;) {} }");
        let fine = script("export function update(v) { return v * 2; }");
        let file = scene(&format!(
            r#"{{"id":1,"name":"a","alpha":{{"script":{throws},"value":1}}}},
               {{"id":2,"name":"b","alpha":{{"script":{forever},"value":1}}}},
               {{"id":3,"name":"c","alpha":{{"script":{fine},"value":1}}}}"#
        ));
        let mut scripts = Scripts::load(&file, None).unwrap();
        let started = Instant::now();
        for _ in 0..3 {
            scripts.run(0.0, 0.0, &SILENCE).unwrap();
        }
        let frame = scripts.run(0.0, 0.0, &SILENCE).unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "{:?}",
            started.elapsed()
        );
        let alphas: Vec<f32> = frame.layers.iter().map(|(_, l)| l.alpha).collect();
        assert_eq!(alphas, vec![1.0, 1.0, 16.0]);
    }

    /// The settings a script declares take what the viewer chose, the
    /// wallpaper's own settings reach `applyUserProperties`, and asking for
    /// the sound is what makes the scene listen.
    #[test]
    fn a_script_hears_its_settings_and_the_sound() {
        let source = script(
            "export var scriptProperties = createScriptProperties()
                 .addSlider({ name: 'gain', value: 1 }).finish();
             let speed = 0;
             const sound = engine.registerAudioBuffers(64);
             export function applyUserProperties(p) { if ('speed' in p) speed = p.speed; }
             export function update() { return scriptProperties.gain * speed + sound.average[63]; }",
        );
        let file = scene(&format!(
            r#"{{"id":1,"name":"a","instanceoverride":{{"rate":{{"script":{source},"value":0,
                 "scriptproperties":{{"gain":{{"value":3}}}}}}}}}}"#
        ));
        let project = br#"{"general":{"properties":{"speed":{"type":"slider","value":2}}}}"#;
        let mut scripts = Scripts::load(&file, Some(project)).unwrap();
        assert!(scripts.listens());
        let mut loud = Spectrum::SILENT;
        loud.left[15] = 0.5;
        loud.right[15] = 0.5;
        let frame = scripts.run(0.0, 0.0, &loud).unwrap();
        assert_eq!(frame.values[0].1, serde_json::json!(6.5));
    }
}
