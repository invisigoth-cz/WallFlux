// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! `scene.json`, the description of what a scene wallpaper draws.
//!
//! A scene is a camera, a block of global settings, and a flat list of objects.
//! Each object is an image, a particle system, a sound or a text, and carries a
//! transform plus an optional chain of post-processing effects. An image object
//! names a model, which names a material, which names the textures and shader a
//! pass needs — see [`crate::material`].
//!
//! Two shapes in this file would break a reader that assumed the obvious:
//!
//! - Vectors are **space-separated strings**, not arrays: `"1.00000 0.50000 0"`.
//! - Almost any property may appear either as a plain value *or* as an object
//!   carrying that value plus a binding. See [`Bound`].
//!
//! Fields absent from a file are not errors. Older wallpapers omit much of what
//! newer ones write, and the `version` field is missing from more than half of
//! them and inconsistent where present, so nothing branches on it.

use serde::Deserialize;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];

/// A property that may carry a binding alongside its value.
///
/// Wallpaper Engine lets a property be driven by a setting the viewer can change
/// or by a JavaScript function. Both forms still store the plain value, so a
/// reader that ignores the binding always has something correct to draw — across
/// the wallpapers this was built from, not one binding lacked its value.
#[derive(Debug, Clone, Default)]
pub struct Bound<T> {
    pub value: T,
    /// The name of the user-facing setting driving this, if any.
    pub user: Option<String>,
    /// JavaScript driving this, if any: an ES module whose `update` is
    /// handed the value and answers with the next one, every frame.
    pub script: Option<String>,
    /// What the viewer set the script's own settings to, by name -- the
    /// values a `scriptProperties` export declares are its defaults.
    pub scriptproperties: Option<serde_json::Value>,
}

impl<T> Bound<T> {
    pub fn plain(value: T) -> Self {
        Self {
            value,
            user: None,
            script: None,
            scriptproperties: None,
        }
    }

    pub fn is_bound(&self) -> bool {
        self.user.is_some() || self.script.is_some()
    }
}

impl<'de, T> Deserialize<'de> for Bound<T>
where
    T: Deserialize<'de> + Default,
{
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        /// `user` is usually a name, but may be a name plus a condition.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum User {
            Name(String),
            Conditional { name: String },
        }

        // Option fields are already optional to serde; spelling out `default`
        // here would make it demand `T: Default` from every caller.
        #[derive(Deserialize)]
        struct Wrapped<T> {
            value: Option<T>,
            user: Option<User>,
            script: Option<String>,
            scriptproperties: Option<serde_json::Value>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either<T> {
            // The wrapped form must be tried first: a bare map would otherwise
            // have to match T, and for string-encoded vectors it never would.
            Wrapped(Wrapped<T>),
            Plain(T),
        }

        Ok(match Either::<T>::deserialize(d)? {
            Either::Plain(value) => Self::plain(value),
            Either::Wrapped(w) => Self {
                value: w.value.unwrap_or_default(),
                user: w.user.map(|u| match u {
                    User::Name(n) | User::Conditional { name: n } => n,
                }),
                script: w.script,
                scriptproperties: w.scriptproperties,
            },
        })
    }
}

/// Deserializes the space-separated string form into a fixed-size array.
///
/// Numbers are accepted too, so a field written either way reads the same.
mod vector {
    use serde::de::{Deserialize, Deserializer, Error};

    pub fn parse<const N: usize, E: Error>(s: &str) -> Result<[f32; N], E> {
        let mut out = [0.0; N];
        let mut parts = s.split_whitespace();
        for (i, slot) in out.iter_mut().enumerate() {
            let part = parts
                .next()
                .ok_or_else(|| E::custom(format!("vector {s:?} has fewer than {N} components")))?;
            *slot = part
                .parse()
                .map_err(|_| E::custom(format!("component {i} of {s:?} is not a number")))?;
        }
        Ok(out)
    }

    pub fn de<'de, D: Deserializer<'de>, const N: usize>(d: D) -> Result<[f32; N], D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Form {
            Text(String),
            Numbers(Vec<f32>),
            Single(f32),
            // An animated property keeps the curve the editor drew beside the
            // value that plays without it. The curve is not read; dropping it
            // leaves the property standing still, which is a wallpaper that
            // draws rather than one that will not load at all.
            Animated { value: serde_json::Value },
        }
        match Form::deserialize(d)? {
            Form::Text(s) => parse(&s),
            Form::Numbers(v) => {
                let mut out = [0.0; N];
                for (slot, got) in out.iter_mut().zip(v) {
                    *slot = got;
                }
                Ok(out)
            }
            Form::Single(v) => Ok([v; N]),
            Form::Animated { value } => de::<_, N>(value).map_err(D::Error::custom),
        }
    }
}

/// A vector written as `"x y z"`, usable anywhere a `[f32; N]` is wanted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector<const N: usize>(pub [f32; N]);

impl<const N: usize> Default for Vector<N> {
    fn default() -> Self {
        Self([0.0; N])
    }
}

impl<'de, const N: usize> Deserialize<'de> for Vector<N> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        vector::de::<D, N>(d).map(Self)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Camera {
    pub eye: Vector<3>,
    pub center: Vector<3>,
    pub up: Vector<3>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Vector([0.0, 0.0, 1.0]),
            center: Vector([0.0; 3]),
            up: Vector([0.0, 1.0, 0.0]),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Projection {
    pub width: u32,
    pub height: u32,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "lowercase")]
pub struct General {
    pub clearcolor: Vector<3>,
    pub clearenabled: Option<bool>,
    pub ambientcolor: Vector<3>,
    pub skylightcolor: Vector<3>,
    pub orthogonalprojection: Projection,
    pub fov: f32,
    pub nearz: f32,
    pub farz: f32,
    pub zoom: f32,
    pub bloom: Bound<bool>,
    pub bloomstrength: Option<f32>,
    pub bloomthreshold: Option<f32>,
    pub hdr: Bound<bool>,
    pub cameraparallax: Bound<bool>,
    pub cameraparallaxamount: f32,
    pub cameraparallaxdelay: f32,
    pub cameraparallaxmouseinfluence: f32,
    pub camerashake: Bound<bool>,
    pub camerashakeamplitude: f32,
    pub camerashakeroughness: f32,
    pub camerashakespeed: f32,
    pub camerafade: bool,
    pub camerapreview: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            clearcolor: Vector([0.0; 3]),
            clearenabled: None,
            ambientcolor: Vector([0.0; 3]),
            skylightcolor: Vector([0.0; 3]),
            orthogonalprojection: Projection::default(),
            fov: 50.0,
            nearz: 0.01,
            farz: 10_000.0,
            zoom: 1.0,
            bloom: Bound::default(),
            bloomstrength: None,
            bloomthreshold: None,
            hdr: Bound::default(),
            cameraparallax: Bound::default(),
            cameraparallaxamount: 0.5,
            cameraparallaxdelay: 0.1,
            cameraparallaxmouseinfluence: 0.5,
            camerashake: Bound::default(),
            camerashakeamplitude: 0.5,
            camerashakeroughness: 1.0,
            camerashakespeed: 3.0,
            camerafade: true,
            camerapreview: true,
        }
    }
}

/// A post-processing effect applied to an object.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EffectRef {
    pub file: String,
    pub name: String,
    pub id: i64,
    pub visible: Bound<bool>,
    /// Per-instance overrides of the effect's own passes, positionally matched.
    ///
    /// This is where a wallpaper turns an effect's options on, including audio
    /// reactivity, so an effect read only from its own file looks inert.
    pub passes: Vec<PassOverride>,
}

/// What a scene changes about one pass of an effect it uses.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct PassOverride {
    pub id: i64,
    pub combos: std::collections::BTreeMap<String, serde_json::Value>,
    pub constantshadervalues: std::collections::BTreeMap<String, Bound<serde_json::Value>>,
    pub textures: Vec<Option<String>>,
}

impl PassOverride {
    /// Whether this override switches audio reactivity on. See
    /// [`crate::material::Pass::combo`] for why case is ignored.
    pub fn uses_audio(&self) -> bool {
        self.combos
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("AUDIOPROCESSING"))
            .is_some_and(|(_, v)| v != &serde_json::Value::from(0) && v != &serde_json::Value::Null)
    }
}

impl Default for EffectRef {
    fn default() -> Self {
        Self {
            file: String::new(),
            name: String::new(),
            id: 0,
            visible: Bound::plain(true),
            passes: Vec::new(),
        }
    }
}

/// What an object actually is. Exactly one of the naming fields is populated.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    /// Names a model under `models/`, which in turn names a material.
    Image(String),
    /// Names a particle definition under `particles/`.
    Particle(String),
    /// Names one or more audio files under `sounds/`.
    Sound(Vec<String>),
    Text,
    /// Carries none of the known naming fields.
    Empty,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Object {
    pub id: i64,
    pub name: String,
    pub origin: Bound<Vector<3>>,
    pub angles: Bound<Vector<3>>,
    pub scale: Bound<Vector<3>>,
    pub size: Option<Vector<2>>,
    pub color: Bound<Vector<3>>,
    pub alpha: Bound<f32>,
    pub brightness: f32,
    /// How the finished layer mixes with what is under it -- the editor's
    /// "blend mode", `ApplyBlending`'s numbering in `common_blending.h`:
    /// 6 lighten, 7 screen, 9 add, 31 and so on. Nought is ordinary alpha.
    #[serde(rename = "colorBlendMode")]
    pub color_blend_mode: Bound<i64>,
    pub visible: Bound<bool>,
    /// How much this object lags the cursor. The most used feature in the
    /// format, and what gives a scene its sense of depth.
    #[serde(rename = "parallaxDepth")]
    pub parallax_depth: Vector<2>,
    pub effects: Vec<EffectRef>,
    pub solid: bool,
    pub perspective: bool,
    pub copybackground: bool,
    pub alignment: String,
    pub volume: Bound<f32>,
    /// How a sound object plays its files: `loop` goes through them in
    /// order and round again, `random` picks one, waits between `mintime`
    /// and `maxtime` seconds after it and picks again, `single` plays once.
    pub playbackmode: String,
    pub mintime: f32,
    pub maxtime: f32,
    /// Set where the sound waits to be started by something else -- a
    /// script, in Wallpaper Engine -- rather than playing from the start.
    pub startsilent: bool,
    /// What the scene changes about the particle system it places here. 462
    /// objects in the library carry one -- the snow storm dimmed to half,
    /// a fire made to burn twice as fast.
    pub instanceoverride: Option<InstanceOverride>,

    // The naming fields. Read through `kind()` rather than directly.
    #[serde(rename = "image")]
    image_field: Option<String>,
    #[serde(rename = "particle")]
    particle_field: Option<String>,
    #[serde(rename = "sound")]
    sound_field: Option<SoundField>,
    #[serde(rename = "text")]
    text_field: Option<Bound<serde_json::Value>>,

    // What a text object says about its text; see `wallflux::text`.
    pub font: String,
    pub pointsize: Bound<f32>,
    pub padding: Bound<f32>,
    pub horizontalalign: Bound<String>,
    pub verticalalign: Bound<String>,
    pub limitwidth: Bound<bool>,
    pub maxwidth: Bound<f32>,
    pub limitrows: Bound<bool>,
    pub maxrows: Bound<f32>,
    pub limituseellipsis: Bound<bool>,
    pub opaquebackground: Bound<bool>,
    pub backgroundcolor: Bound<Vector<3>>,
    pub backgroundbrightness: Bound<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SoundField {
    One(String),
    Many(Vec<String>),
}

/// What a scene changes about one particle system it places, each a
/// multiplier on what the system's own file says. Read the way Almamu's
/// renderer reads them, which is the only account of them there is.
///
/// The colour is written two ways: `colorn` from nought to one, and `color`
/// out of 255 in older scenes. Seventy objects say the one, seventeen the
/// other, and not one says both -- so they are the same thing.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InstanceOverride {
    pub alpha: Bound<f32>,
    pub size: Bound<f32>,
    pub lifetime: Bound<f32>,
    /// Particles per second.
    pub rate: Bound<f32>,
    /// How fast particles start out.
    pub speed: Bound<f32>,
    /// How many may be alive at once.
    pub count: Bound<f32>,
    pub colorn: Bound<Vector<3>>,
    pub color: Option<Bound<Vector<3>>>,
}

impl InstanceOverride {
    /// What the particles' colour is multiplied by, whichever way it was
    /// written.
    pub fn tint(&self) -> [f32; 3] {
        match &self.color {
            Some(color) => color.value.0.map(|c| c / 255.0),
            None => self.colorn.value.0,
        }
    }
}

impl Default for InstanceOverride {
    fn default() -> Self {
        Self {
            alpha: Bound::plain(1.0),
            size: Bound::plain(1.0),
            lifetime: Bound::plain(1.0),
            rate: Bound::plain(1.0),
            speed: Bound::plain(1.0),
            count: Bound::plain(1.0),
            colorn: Bound::plain(Vector([1.0; 3])),
            color: None,
        }
    }
}

impl Default for Object {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            origin: Bound::default(),
            angles: Bound::default(),
            scale: Bound::plain(Vector([1.0; 3])),
            size: None,
            color: Bound::plain(Vector([1.0; 3])),
            alpha: Bound::plain(1.0),
            brightness: 1.0,
            color_blend_mode: Bound::plain(0),
            visible: Bound::plain(true),
            parallax_depth: Vector([1.0; 2]),
            effects: Vec::new(),
            solid: false,
            perspective: false,
            copybackground: false,
            alignment: String::from("center"),
            volume: Bound::plain(1.0),
            playbackmode: String::from("loop"),
            mintime: 1.0,
            maxtime: 5.0,
            startsilent: false,
            instanceoverride: None,
            image_field: None,
            particle_field: None,
            sound_field: None,
            text_field: None,
            font: String::new(),
            pointsize: Bound::plain(32.0),
            padding: Bound::plain(32.0),
            horizontalalign: Bound::plain(String::from("center")),
            verticalalign: Bound::plain(String::from("center")),
            limitwidth: Bound::plain(false),
            maxwidth: Bound::plain(0.0),
            limitrows: Bound::plain(false),
            maxrows: Bound::plain(1.0),
            limituseellipsis: Bound::plain(false),
            opaquebackground: Bound::plain(false),
            backgroundcolor: Bound::default(),
            backgroundbrightness: Bound::plain(1.0),
        }
    }
}

impl Object {
    /// What this object draws or plays.
    ///
    /// The naming fields are present-but-null far more often than they are
    /// absent, so emptiness is decided by the value rather than the key.
    /// The object's text, its value and whatever writes it, which is how a
    /// scene draws a clock. The field is there on objects that are not text,
    /// and a text object can have a name left over in `image` or `particle`,
    /// so what makes one is naming a font as well.
    pub fn text(&self) -> Option<&Bound<serde_json::Value>> {
        self.text_field.as_ref().filter(|_| !self.font.is_empty())
    }

    pub fn kind(&self) -> ObjectKind {
        let named = |f: &Option<String>| f.as_deref().filter(|s| !s.is_empty()).map(str::to_owned);

        if self.text().is_some() {
            ObjectKind::Text
        } else if let Some(model) = named(&self.image_field) {
            ObjectKind::Image(model)
        } else if let Some(file) = named(&self.particle_field) {
            ObjectKind::Particle(file)
        } else if let Some(sound) = &self.sound_field {
            let files = match sound {
                SoundField::One(s) => vec![s.clone()],
                SoundField::Many(v) => v.clone(),
            };
            let files: Vec<_> = files.into_iter().filter(|s| !s.is_empty()).collect();
            if files.is_empty() {
                ObjectKind::Empty
            } else {
                ObjectKind::Sound(files)
            }
        } else {
            ObjectKind::Empty
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Scene {
    pub camera: Camera,
    pub general: General,
    pub objects: Vec<Object>,
}

impl Scene {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }

    /// Reads a scene with the values its `project.json` gives the user
    /// properties standing over the copies written into the scene.
    ///
    /// A value bound to a property is written twice: once in the scene, beside
    /// the binding, and once in `project.json`, where the property is
    /// declared. Wallpaper Engine plays the second. The two disagree more
    /// often than not -- 205 of the 365 bound values in this library -- since
    /// the scene keeps whatever the author last had in the editor. One
    /// wallpaper's scene says to tint everything red at full strength; its
    /// project says the tint is off.
    pub fn parse_with_properties(bytes: &[u8], project: Option<&[u8]>) -> serde_json::Result<Self> {
        let mut scene: serde_json::Value = serde_json::from_slice(bytes)?;
        let properties = project
            .and_then(|p| serde_json::from_slice::<serde_json::Value>(p).ok())
            .and_then(|p| p.pointer("/general/properties").cloned());
        if let Some(serde_json::Value::Object(properties)) = properties {
            bind_user_values(&mut scene, &properties);
        }
        serde_json::from_value(scene)
    }
}

/// Puts each property's value from `project.json` wherever the scene binds
/// it, leaving alone anything the project does not declare or gives a value
/// of a different kind.
fn bind_user_values(node: &mut serde_json::Value, properties: &serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;
    match node {
        Value::Object(map) => {
            if let (Some(user), Some(old)) = (map.get("user"), map.get("value"))
                && let Some(new) = user_value(user, old, properties)
            {
                map.insert("value".into(), new);
            }
            for child in map.values_mut() {
                bind_user_values(child, properties);
            }
        }
        Value::Array(items) => {
            for child in items {
                bind_user_values(child, properties);
            }
        }
        _ => {}
    }
}

fn user_value(
    user: &serde_json::Value,
    old: &serde_json::Value,
    properties: &serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    use serde_json::Value;
    let (name, condition) = match user {
        Value::String(name) => (name.as_str(), None),
        Value::Object(o) => (o.get("name")?.as_str()?, o.get("condition")),
        _ => return None,
    };
    let given = properties.get(name)?.get("value")?;
    // A binding with a condition is true when a drop-down is on that row,
    // which is how one choice shows one set of objects and hides another.
    let text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if let Some(condition) = condition {
        return Some(Value::Bool(text(given) == text(condition)));
    }
    match (old, given) {
        (Value::Bool(_), Value::Number(n)) => Some(Value::Bool(n.as_f64() != Some(0.0))),
        (Value::Bool(_), Value::Bool(_))
        | (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_)) => Some(given.clone()),
        _ => None,
    }
}

/// Counts the scripted properties in a scene, wherever they are.
///
/// Scripts turn up in at least fourteen distinct places: object transforms,
/// particle instance overrides, text, the visibility of an effect and the
/// constant shader values inside an effect pass. Listing those places would
/// mean missing the fifteenth, so this walks the raw document for the key
/// instead and stays right as wallpapers find new spots to put one.
///
/// Nothing here runs the scripts. The count says how much of a wallpaper will
/// be drawn at a fixed value rather than animating.
pub fn count_scripts(bytes: &[u8]) -> serde_json::Result<usize> {
    fn walk(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                let here = usize::from(map.contains_key("script"));
                here + map
                    .iter()
                    .filter(|(k, _)| *k != "script")
                    .map(|(_, v)| walk(v))
                    .sum::<usize>()
            }
            serde_json::Value::Array(items) => items.iter().map(walk).sum(),
            _ => 0,
        }
    }
    Ok(walk(&serde_json::from_slice(bytes)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_string_encoded_vectors() {
        let s = Scene::parse(br#"{"objects":[{"origin":"1 2 3","size":"640 480"}]}"#).unwrap();
        assert_eq!(s.objects[0].origin.value.0, [1.0, 2.0, 3.0]);
        assert_eq!(s.objects[0].size.unwrap().0, [640.0, 480.0]);
    }

    /// The same value is written both ways in the wild. Comparing the text
    /// instead of the number once put the use of parallax out by a factor of
    /// twelve, so this pins that they read as equal.
    #[test]
    fn differently_written_vectors_compare_equal() {
        let a = Scene::parse(br#"{"objects":[{"parallaxDepth":"1.000 1.000"}]}"#).unwrap();
        let b = Scene::parse(br#"{"objects":[{"parallaxDepth":"1.00000 1.00000"}]}"#).unwrap();
        assert_eq!(a.objects[0].parallax_depth.0, b.objects[0].parallax_depth.0);
        assert_eq!(a.objects[0].parallax_depth.0, [1.0, 1.0]);
    }

    /// The scene keeps what the author last had in the editor; the project
    /// says what the wallpaper plays with. Reading the first tinted one
    /// wallpaper red all over where its project has the tint switched off.
    #[test]
    fn a_bound_value_comes_from_the_project() {
        let scene = br#"{"objects":[
            {"alpha": {"user": "level", "value": 1}},
            {"visible": {"user": {"name": "style", "condition": "2"}, "value": false}},
            {"visible": {"user": "missing", "value": false}}
        ]}"#;
        let project = br#"{"general":{"properties":{
            "level": {"type": "slider", "value": 0.25},
            "style": {"type": "combo", "value": "2"}
        }}}"#;
        let s = Scene::parse_with_properties(scene, Some(project)).unwrap();
        assert_eq!(s.objects[0].alpha.value, 0.25);
        assert!(s.objects[1].visible.value);
        assert!(!s.objects[2].visible.value);
    }

    /// One wallpaper writes an animated vector as the curve beside its value.
    /// Reading only the three forms a static vector takes threw the whole
    /// scene away over a colour that was never going to be looked at.
    #[test]
    fn an_animated_vector_reads_as_the_value_it_rests_at() {
        let s = Scene::parse(
            br#"{"objects":[{"origin":{"animation":{"c0":[{"frame":0,"value":0}]},"value":"1 2 3"}}]}"#,
        )
        .unwrap();
        assert_eq!(s.objects[0].origin.value.0, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_property_may_be_plain_or_bound() {
        let s = Scene::parse(
            br#"{"objects":[
                {"visible": false},
                {"visible": {"value": true, "user": "toggle"}},
                {"visible": {"value": false, "user": {"name": "cond", "condition": "0"}}}
            ]}"#,
        )
        .unwrap();
        assert!(!s.objects[0].visible.value && !s.objects[0].visible.is_bound());
        assert!(s.objects[1].visible.value);
        assert_eq!(s.objects[1].visible.user.as_deref(), Some("toggle"));
        assert_eq!(s.objects[2].visible.user.as_deref(), Some("cond"));
    }

    #[test]
    fn a_bound_property_still_yields_its_value() {
        let s = Scene::parse(
            br#"{"objects":[{"scale":{"script":"export function update(v){return v}","value":"2 2 2"}}]}"#,
        )
        .unwrap();
        assert_eq!(s.objects[0].scale.value.0, [2.0; 3]);
        assert!(s.objects[0].scale.script.is_some());
    }

    #[test]
    fn object_kind_ignores_present_but_empty_names() {
        let s = Scene::parse(
            br#"{"objects":[
                {"image":"models/a.json","particle":null},
                {"image":null,"particle":"particles/b.json"},
                {"image":null,"particle":null},
                {"sound":["sounds/a.mp3","sounds/b.mp3"]}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            s.objects[0].kind(),
            ObjectKind::Image("models/a.json".into())
        );
        assert_eq!(
            s.objects[1].kind(),
            ObjectKind::Particle("particles/b.json".into())
        );
        assert_eq!(s.objects[2].kind(), ObjectKind::Empty);
        assert!(matches!(s.objects[3].kind(), ObjectKind::Sound(f) if f.len() == 2));
    }

    #[test]
    fn counts_scripts_wherever_they_hide() {
        let json = br#"{
            "general": {"bloom": {"script": "a", "value": true}},
            "objects": [{
                "scale": {"script": "b", "value": "1 1 1"},
                "instanceoverride": {"rate": {"script": "c", "value": 1}},
                "effects": [{"passes": [{"constantshadervalues": {"speed": {"script": "d", "value": 1}}}]}]
            }]
        }"#;
        assert_eq!(count_scripts(json).unwrap(), 4);
    }

    #[test]
    fn an_instance_override_changes_only_what_it_names() {
        let s = Scene::parse(
            br#"{"objects":[
                {"instanceoverride": {"alpha": 0.5, "colorn": "0.75 0.75 0.75", "id": 46}},
                {"instanceoverride": null}
            ]}"#,
        )
        .unwrap();
        let o = s.objects[0].instanceoverride.as_ref().unwrap();
        assert_eq!(o.alpha.value, 0.5);
        assert_eq!(o.colorn.value.0, [0.75; 3]);
        assert_eq!((o.size.value, o.rate.value, o.count.value), (1.0, 1.0, 1.0));
        assert!(s.objects[1].instanceoverride.is_none());
        assert_eq!(o.tint(), [0.75; 3]);

        let old = Scene::parse(br#"{"objects":[{"instanceoverride": {"color": "255 51 0"}}]}"#)
            .unwrap();
        assert_eq!(old.objects[0].instanceoverride.as_ref().unwrap().tint(), [1.0, 0.2, 0.0]);
    }

    #[test]
    fn an_effect_reference_carries_its_overrides() {
        let s = Scene::parse(
            br#"{"objects":[{"effects":[{"file":"effects/pulse/effect.json",
                 "passes":[{"combos":{"AUDIOPROCESSING":3},"textures":[null,"util/noise"]}]}]}]}"#,
        )
        .unwrap();
        let p = &s.objects[0].effects[0].passes[0];
        assert!(p.uses_audio());
        assert_eq!(p.textures, vec![None, Some("util/noise".into())]);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s = Scene::parse(b"{}").unwrap();
        assert!(s.objects.is_empty());
        assert_eq!(s.general.orthogonalprojection.width, 1920);
        assert_eq!(s.camera.up.0, [0.0, 1.0, 0.0]);
    }
}
