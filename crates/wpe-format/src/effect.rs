//! `effects/*/effect.json`: the post-processing chain applied to an object.
//!
//! An effect is a small render graph. Each pass draws a material into a render
//! target, binding earlier targets as its input textures, and a pass may instead
//! be a command such as copying one target to another. Targets are either ones
//! the effect declares in `fbos` or ones the engine always provides, and both
//! are named with a leading `_rt_`.

use serde::Deserialize;

/// Whether a texture name refers to a render target rather than a file.
///
/// Materials sample these by name the same way they sample textures, so a
/// reader that treats every name as a path goes looking for files that were
/// never meant to exist.
pub fn is_render_target(name: &str) -> bool {
    name.starts_with("_rt_")
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Effect {
    pub name: String,
    pub description: String,
    pub group: String,
    pub passes: Vec<EffectPass>,
    /// Render targets this effect allocates for itself.
    pub fbos: Vec<Fbo>,
    /// Every file the effect needs, as listed by its author.
    pub dependencies: Vec<String>,
}

impl Effect {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Fbo {
    pub name: String,
    /// Divisor against the output size: 2 is half resolution.
    pub scale: f32,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct EffectPass {
    /// The material to draw. Empty when this pass is a [`Self::command`].
    pub material: String,
    /// Render target to draw into. Empty means the one already bound.
    pub target: String,
    /// Render targets to bind as input textures.
    pub bind: Vec<Bind>,
    /// A non-drawing operation, currently only `copy`.
    pub command: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Bind {
    /// A render target's name, or `previous` for whatever the last pass wrote.
    pub name: String,
    /// Texture slot to bind it to.
    pub index: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_render_targets() {
        assert!(is_render_target("_rt_FullFrameBuffer"));
        assert!(!is_render_target("materials/wood"));
        assert!(!is_render_target(""));
    }

    #[test]
    fn reads_a_drawing_pass() {
        let e = Effect::parse(
            br#"{"passes":[{"material":"materials/effects/pulse.json",
                 "target":"_rt_HalfCompoBuffer1","bind":[{"name":"previous","index":0}]}],
                 "fbos":[{"name":"_rt_HalfCompoBuffer1","scale":2,"format":"rgba8888"}]}"#,
        )
        .unwrap();
        assert_eq!(e.passes[0].material, "materials/effects/pulse.json");
        assert_eq!(e.passes[0].bind[0].name, "previous");
        assert_eq!(e.fbos[0].scale, 2.0);
    }

    #[test]
    fn reads_a_copy_pass() {
        let e =
            Effect::parse(br#"{"passes":[{"command":"copy","source":"_rt_A","target":"_rt_B"}]}"#)
                .unwrap();
        assert_eq!(e.passes[0].command, "copy");
        assert!(e.passes[0].material.is_empty());
    }
}
