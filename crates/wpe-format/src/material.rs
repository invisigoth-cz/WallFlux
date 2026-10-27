//! `models/*.json` and `materials/*.json`: how an object gets from a name to
//! pixels.
//!
//! An image object names a model, a model names a material, and a material is a
//! list of passes. A pass is what the renderer hands to the GPU: a shader, the
//! textures it samples, the blend and depth state, the preprocessor defines
//! that select branches inside the shader, and the constant uniform values.
//!
//! A shader named here is not necessarily in the wallpaper. Most are built into
//! Wallpaper Engine and live in its `assets/shaders` directory, which must be
//! read from the viewer's own installation — those files are not ours to ship.

use crate::scene::Bound;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Model {
    /// Path to the material, relative to the package root.
    pub material: String,
    /// Take the size from the texture rather than from the object.
    pub autosize: bool,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub nopadding: bool,
    /// Covers the whole output rather than the object's own size.
    pub fullscreen: bool,
    /// Draws what is already in the framebuffer, for layers that only exist to
    /// run an effect over what is behind them.
    pub passthrough: bool,
    pub solidlayer: bool,
}

impl Model {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Material {
    pub passes: Vec<Pass>,
}

impl Material {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Pass {
    /// Shader name without extension. Resolved against the wallpaper first,
    /// then against Wallpaper Engine's own assets.
    pub shader: String,
    /// Texture slots, in binding order. A slot may be empty, in which case the
    /// shader supplies its own or the slot goes unused.
    pub textures: Vec<Option<String>>,
    pub blending: String,
    pub cullmode: String,
    pub depthtest: String,
    pub depthwrite: String,
    /// Preprocessor defines selecting branches in the shader. `audioprocessing`
    /// is the one that turns a wallpaper into an audio-reactive one.
    pub combos: BTreeMap<String, serde_json::Value>,
    /// Uniform values. Each may be bound to a setting or a script, so these
    /// carry the same wrapper as scene properties.
    pub constantshadervalues: BTreeMap<String, Bound<serde_json::Value>>,
}

impl Pass {
    /// Looks a combo up without regard to case.
    ///
    /// Shaders test these as uppercase identifiers and scene files write them
    /// that way, but a minority of hand-written materials use lowercase and
    /// Wallpaper Engine evidently folds the case before defining them. A
    /// case-sensitive lookup silently misses those.
    pub fn combo(&self, name: &str) -> Option<&serde_json::Value> {
        self.combos
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// Whether this pass asks for the audio spectrum to be fed to its shader.
    pub fn uses_audio(&self) -> bool {
        self.combo("AUDIOPROCESSING")
            .is_some_and(|v| v != &serde_json::Value::from(0) && v != &serde_json::Value::Null)
    }

    /// The preprocessor defines for this pass, named as the shaders expect.
    pub fn defines(&self) -> impl Iterator<Item = (String, &serde_json::Value)> {
        self.combos.iter().map(|(k, v)| (k.to_ascii_uppercase(), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_model_to_material_link() {
        let m = Model::parse(br#"{"autosize":true,"material":"materials/a.json"}"#).unwrap();
        assert_eq!(m.material, "materials/a.json");
        assert!(m.autosize);
    }

    #[test]
    fn reads_a_pass_with_an_empty_texture_slot() {
        let m = Material::parse(
            br#"{"passes":[{
                "shader":"genericimage2",
                "textures":[null,"util/noise"],
                "blending":"translucent",
                "combos":{"audioprocessing":3},
                "constantshadervalues":{"Alpha":1.0,"bounds":"0 1"}
            }]}"#,
        )
        .unwrap();
        let p = &m.passes[0];
        assert_eq!(p.shader, "genericimage2");
        assert_eq!(p.textures, vec![None, Some("util/noise".into())]);
        assert!(p.uses_audio());
        assert_eq!(p.constantshadervalues["Alpha"].value, 1.0);
    }

    #[test]
    fn a_pass_without_audio_processing_says_so() {
        let m = Material::parse(br#"{"passes":[{"combos":{"audioprocessing":0}},{}]}"#).unwrap();
        assert!(!m.passes[0].uses_audio());
        assert!(!m.passes[1].uses_audio());
    }

    /// Scene files write combos uppercase, some materials lowercase, and the
    /// shaders only ever test uppercase. Matching case-sensitively once hid
    /// every audio-reactive pass in the library.
    #[test]
    fn combos_match_whatever_case_they_were_written_in() {
        for json in [
            br#"{"passes":[{"combos":{"AUDIOPROCESSING":3}}]}"#.as_slice(),
            br#"{"passes":[{"combos":{"audioprocessing":3}}]}"#.as_slice(),
            br#"{"passes":[{"combos":{"AudioProcessing":3}}]}"#.as_slice(),
        ] {
            assert!(Material::parse(json).unwrap().passes[0].uses_audio());
        }
    }

    #[test]
    fn defines_are_emitted_uppercase() {
        let m =
            Material::parse(br#"{"passes":[{"combos":{"spritesheet":1,"KERNEL":9}}]}"#).unwrap();
        let mut names: Vec<_> = m.passes[0].defines().map(|(k, _)| k).collect();
        names.sort();
        assert_eq!(names, ["KERNEL", "SPRITESHEET"]);
    }

    #[test]
    fn a_scripted_uniform_still_yields_its_value() {
        let m = Material::parse(
            br#"{"passes":[{"constantshadervalues":{"speed":{"script":"s","value":3.0}}}]}"#,
        )
        .unwrap();
        let v = &m.passes[0].constantshadervalues["speed"];
        assert_eq!(v.value, 3.0);
        assert!(v.script.is_some());
    }
}
