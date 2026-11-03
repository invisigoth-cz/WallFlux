// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Drawing a scene with the shaders the wallpaper brought with it.
//!
//! Each visible image object becomes a quad placed in the scene's own pixel
//! coordinates, drawn with the program its material names and the textures its
//! pass binds. Objects are drawn in the order the scene lists them, which is
//! the order they are meant to stack.

use std::collections::{BTreeMap, BTreeSet};

use glow::HasContext;
use wpe_format::particle::ChildKind;
use wpe_format::particle::Renderer as ParticleRenderer;
use wpe_format::particle::{Response, Spectrum};
use wpe_format::puppet::{Playing, Puppet};
use wpe_format::scene::ObjectKind;
use wpe_format::shader::{Stage, combo_defaults, preprocess, uniforms};
use wpe_format::{Assets, Material, Model, Package, ParticleSystem, Resolver, Scene as SceneFile};

use crate::particles::{self, FLOATS_PER_ROPE_VERTEX, FLOATS_PER_VERTEX, Snapshot};
use crate::script::{Drives, Scripts};
use crate::text::{Align, Face, Style};
use crate::texture::Texture;
use crate::transform::{self, Matrix};
use crate::video::Video;

/// How a pass combines with what is already drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blending {
    Opaque,
    Translucent,
    Additive,
}

impl Blending {
    fn parse(name: &str) -> Self {
        match name {
            // What every effect material in the library asks for, and six
            // others besides: ordinary alpha blending.
            "translucent" | "normal" => Self::Translucent,
            "additive" => Self::Additive,
            // An unnamed or unknown mode draws over what is there, which is
            // what a background layer wants and the safest thing to guess.
            _ => Self::Opaque,
        }
    }

    fn apply(self, gl: &glow::Context) {
        unsafe {
            match self {
                Self::Opaque => gl.disable(glow::BLEND),
                Self::Translucent => {
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                }
                Self::Additive => {
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE);
                }
            }
        }
    }
}

impl Object {
    /// Whether drawing this object needs a picture of the scene under it: a
    /// layer draws that picture, a blend mode mixes into it.
    fn reads_frame(&self) -> bool {
        self.layer.is_some()
            || self.passes.iter().any(|pass| !pass.screen_slots.is_empty())
            || self.effects.iter().any(|step| step.reads.iter().any(|(_, from)| matches!(from, Buffer::Frame)))
    }

    /// How many pixels of the output this object covers, which is as much of
    /// the canvas as it needs. The projection carries the scale from the
    /// scene's own units to the output's.
    fn on_screen(&self, projection: &Matrix, output: (u32, u32), most: (u32, u32)) -> (u32, u32) {
        // A post-processing layer is the screen, whatever size the file
        // remembers it at: Gears of War's says 1248x702 in a 1920x1080 scene.
        if self.layer == Some(Layer::Screen) {
            return (output.0.min(most.0), output.1.min(most.1));
        }
        let across = |scale: f32, size: f32, pixels: u32, most: u32| {
            let width = (scale.abs() * pixels as f32 / 2.0) * size.abs();
            (width.ceil() as u32).clamp(1, most)
        };
        (
            across(
                projection[0],
                self.size[0] * self.scale[0],
                output.0,
                most.0,
            ),
            across(
                projection[5],
                self.size[1] * self.scale[1],
                output.1,
                most.1,
            ),
        )
    }
}

impl Pass {
    /// What this pass's samplers for the scene so far are to be handed.
    fn screen_binds<'a>(&self, frame: Option<&'a Target>) -> Vec<(usize, &'a Texture)> {
        frame
            .map(|frame| self.screen_slots.iter().map(|slot| (*slot, &frame.texture)).collect())
            .unwrap_or_default()
    }

    /// The size of the first bound image, which is what an auto-sized object
    /// takes its own size from.
    fn first_image_size(&self) -> Option<(u32, u32)> {
        self.textures.iter().flatten().next().map(|t| t.image_size)
    }
}

struct Pass {
    program: glow::Program,
    /// Textures in binding order; a slot may be empty.
    textures: Vec<Option<Texture>>,
    blending: Blending,
    constants: BTreeMap<String, serde_json::Value>,
    /// Set when the first texture is a sprite sheet the particles animate.
    sheet: Option<Sheet>,
    /// Set when the first texture is a sprite sheet an image object plays.
    film: Option<Film>,
    /// How much of the first texture the image fills.
    ///
    /// Of the 384 textures in the library, 98 are stored at the power-of-two
    /// size with the image in a corner and the rest empty. Reading those from
    /// corner to corner samples the empty part, which clamping then smears
    /// across the object.
    covered: [f32; 2],
    /// Whether the shader reads the spectrum, which it declares only behind
    /// the switch that turns its audio response on.
    hears: bool,
    /// Uniform by the label a material names it with, for a script that
    /// names a constant the same way.
    labels: BTreeMap<String, String>,
    /// Set when the first texture is a video: its name, for the scene to
    /// play it into the first slot.
    video: Option<String>,
    /// Units whose sampler stands for the scene drawn so far, and which the
    /// compiled shader really reads. Fire and smoke are made this way: a
    /// particle whose material sets `REFRACT` multiplies itself by what is
    /// behind it, so it burns on a lit hull and all but vanishes against a
    /// dark sky. Thirteen wallpapers here ask for it. Left unbound, the
    /// sampler read the particle's own texture instead, and Star Destroyer's
    /// and DOOM's flames came out as yellow blobs hanging in the air.
    screen_slots: Vec<usize>,
}

/// A sprite sheet played by an image object, frame by frame.
///
/// An image object does not step through a sheet the way a particle does. Its
/// shader is handed the rectangle of the current frame instead, so the frame
/// has to be chosen here, and the sheet may be spread over several images with
/// a frame naming the one it sits on.
struct Film {
    /// One texture per image the sheet is spread over.
    pages: Vec<Texture>,
    frames: Vec<Still>,
    /// Seconds a frame is held. Sheets in the library hold every frame for the
    /// same time, and the first frame of the older sprite version states
    /// something that is plainly not a duration -- 26 seconds where every
    /// other frame says a 25th of one -- so the frames are polled for the
    /// interval they agree on rather than trusting any single one.
    interval: f32,
}

/// Which frame of a looping sheet is showing, kept apart from the textures so
/// it can be checked without a graphics context.
fn showing(frames: usize, interval: f32, time: f32) -> usize {
    if frames == 0 || !interval.is_finite() || interval <= 0.0 {
        return 0;
    }
    let length = interval * frames as f32;
    let along = (time % length + length) % length;
    ((along / interval) as usize).min(frames - 1)
}

/// Where one frame sits, as the texture matrix the image shaders want.
#[derive(Debug, Clone, Copy)]
struct Still {
    page: usize,
    offset: [f32; 2],
    size: [f32; 2],
}

impl Film {
    /// Builds the sheet an image object plays, if its texture holds one.
    fn of(gl: &glow::Context, source: &wpe_format::Texture) -> Option<Self> {
        let sprite = source.sprite.as_ref()?;
        if sprite.frames.is_empty() || source.images.len() < 2 && sprite.frames.len() < 2 {
            return None;
        }

        let mut pages = Vec::new();
        for page in 0..source.images.len() {
            pages.push(Texture::upload_page(gl, source, page, false).ok()?);
        }

        let mut frames = Vec::new();
        for frame in &sprite.frames {
            let page = usize::try_from(frame.image).ok()?;
            let size = pages.get(page)?;
            let (width, height) = (size.width as f32, size.height as f32);
            frames.push(Still {
                page,
                offset: [frame.x / width, frame.y / height],
                size: [frame.width / width, frame.height / height],
            });
        }

        let mut held: Vec<f32> = sprite.frames.iter().map(|f| f.duration).collect();
        held.sort_by(f32::total_cmp);
        let interval = held[held.len() / 2];
        if !interval.is_finite() || interval <= 0.0 {
            return None;
        }
        Some(Self {
            pages,
            frames,
            interval,
        })
    }

    /// The frame showing at a moment, the sheet looping for as long as the
    /// wallpaper runs.
    fn at(&self, time: f32) -> Still {
        self.frames[showing(self.frames.len(), self.interval, time)]
    }
}

/// A sprite sheet's layout, in the form the shaders' `ComputeSpriteFrame`
/// reads it: sizes as a fraction of the whole texture, so the padding a
/// power-of-two texture carries is already accounted for.
#[derive(Debug, Clone, Copy)]
struct Sheet {
    frame_width: f32,
    frame_height: f32,
    frames: f32,
    /// A frame's height over its width, which stops a tall frame being drawn
    /// square.
    ratio: f32,
}

/// What a pass is being built to draw.
///
/// The particle shaders are the same files as any other, compiled with
/// different options: they take their geometry from vertex attributes instead
/// of a quad, so they have to be told that no geometry shader is coming and
/// that the attributes carry velocity and lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Image,
    Particles(Form),
    /// A pass of an effect hung on an object: it draws the object's own
    /// picture again, having been handed it as a texture.
    Effect,
}

/// How a particle system turns into geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// A quad per particle, square to the screen.
    Sprite,
    /// A quad per particle, stretched along its own velocity.
    Trail,
    /// One ribbon threaded through every particle, a quad per pair.
    Rope,
}

/// What an object draws with.
enum Drawing {
    /// One quad, placed by the object's own transform.
    Image,
    /// Boxed so an image object, which is most of a scene, does not carry a
    /// whole particle system's worth of empty space.
    Particles(Box<particles::System>),
}

struct Object {
    name: String,
    origin: [f32; 3],
    size: [f32; 2],
    scale: [f32; 3],
    angles: [f32; 3],
    alpha: f32,
    brightness: f32,
    color: [f32; 3],
    passes: Vec<Pass>,
    drawing: Drawing,
    /// For a trailing particle system, the `length`, `max` and `min` its
    /// shader stretches each particle between.
    trail: Option<[f32; 3]>,
    /// Every pass of every effect hung on this object that could be built,
    /// flattened into the order they are drawn.
    effects: Vec<Step>,
    /// The scene's own number for the object, which is how another layer
    /// names the picture it wants to read.
    id: i64,
    /// Whether another layer reads what this one finishes as, in which case
    /// the last pass is drawn a second time into a picture that stays.
    kept: bool,
    /// For a particle system that emits only where its parent's particles
    /// are born or die, or rides along with them.
    answers: Option<Answer>,
    /// Set when the object draws what is already on the screen rather than a
    /// picture of its own: a layer that exists to carry effects.
    layer: Option<Layer>,
    /// How far the object moves with the mouse when the scene has parallax.
    depth: [f32; 2],
    /// Where on the image its origin is, from the middle, in its own sizes:
    /// `alignment` saying `bottom` puts the origin on the bottom edge, which
    /// is half the image below the middle. Dark Core lines 23 parts up that
    /// way; centred, every one of them hung half its height too low.
    anchor: [f32; 2],
    /// Set on the objects scene.json places itself, as against the systems a
    /// particle system brings along -- which a script moving the object by
    /// its id does not mean.
    from_scene: bool,
    /// Whether to draw it, which a script may change while the scene runs.
    shown: bool,
    /// Set on a text object: its first pass draws the picture this keeps up.
    text: Option<Box<TextBox>>,
    /// What a frame tells this object's shaders about where it is looking.
    view: ObjectView,
    /// The mesh that bends the image, drawn in place of its quad.
    puppet: Option<Box<PuppetMesh>>,
}

/// A puppet on the GPU. Bent here rather than in the shader: its few thousand
/// corners cost nothing to work out, and it leaves the program the same one a
/// flat image uses, with the same vertex layout.
struct PuppetMesh {
    puppet: Puppet,
    playing: Vec<Playing>,
    array: glow::VertexArray,
    vertices: glow::Buffer,
    bent: Vec<f32>,
}

impl PuppetMesh {
    unsafe fn new(
        gl: &glow::Context,
        puppet: Puppet,
        playing: Vec<Playing>,
    ) -> Result<Self, String> {
        unsafe {
            let array = gl.create_vertex_array()?;
            let vertices = gl.create_buffer()?;
            let indices = gl.create_buffer()?;
            gl.bind_vertex_array(Some(array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            let stride = 5 * size_of::<f32>() as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 3 * size_of::<f32>() as i32);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices));
            let bytes: &[u8] = std::slice::from_raw_parts(
                puppet.triangles.as_ptr() as *const u8,
                std::mem::size_of_val(puppet.triangles.as_slice()),
            );
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            gl.bind_vertex_array(None);
            Ok(Self { puppet, playing, array, vertices, bent: Vec::new() })
        }
    }

    /// Bends it to `time`, in the unit the image's quad is in: a quad's corners
    /// are half a size from its middle and its rows run down, where a puppet's
    /// are in pixels with y up.
    fn bend(&mut self, time: f32, size: [f32; 2]) {
        self.puppet.bend_into(&self.playing, time, &mut self.bent);
        for corner in self.bent.chunks_exact_mut(5) {
            corner[0] /= size[0].max(1.0);
            corner[1] /= -size[1].max(1.0);
        }
    }

    /// Hands the GPU the mesh as bent, over a picture that fills `covered` of
    /// whichever texture it is about to be drawn from.
    unsafe fn upload(&self, gl: &glow::Context, covered: [f32; 2]) {
        let mut floats = self.bent.clone();
        for corner in floats.chunks_exact_mut(5) {
            corner[3] *= covered[0];
            corner[4] *= covered[1];
        }
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            let bytes: &[u8] = std::slice::from_raw_parts(
                floats.as_ptr() as *const u8,
                std::mem::size_of_val(floats.as_slice()),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
        }
    }
}

/// Worked out once a frame, before anything is drawn.
#[derive(Clone, Copy)]
struct ObjectView {
    /// How many of the scene's pixels one of its buffer's stands for. Wallpaper
    /// Engine's buffers are as big as the object in the scene and ours as big
    /// as it is on screen, and a blur reaches so many texels: told its own
    /// size, Kratos' 5120-wide scene came out twice as blurred on a 2560-wide
    /// screen, and four times at 1280, and its shoulders melted into the fog.
    scene_per_pixel: f32,
    /// Where the mouse is, as fractions of the output down from its top-left
    /// corner, or well off it when there is none.
    pointer: [f32; 2],
    /// Clip space back to the object's own pixels from its middle.
    from_clip: Matrix,
    /// Clip space back to the object itself, one to minus one across it.
    from_clip_to_unit: Matrix,
    /// The object's quad into the scene, for a shader lighting it in the
    /// scene's own pixels.
    model: Matrix,
    lights: LightsNow,
}

/// What Wallpaper Engine's lit shaders are handed, as they stand this frame.
#[derive(Clone, Copy, Default)]
struct LightsNow {
    ambient: [f32; 3],
    positions: [[f32; 3]; 4],
    /// Colour times strength, and the fourth light's three channels across
    /// the three alphas. A shader divides by the distance squared, so a light
    /// of radius r is multiplied by r squared to be its colour at r away.
    colors: [[f32; 4]; 3],
}

impl LightsNow {
    fn of(lighting: &wpe_format::scene::Lighting, time: f32) -> Self {
        let mut now = Self { ambient: std::array::from_fn(|i| lighting.ambient.at(i, time)), ..Self::default() };
        for (index, light) in lighting.lights.iter().take(4).enumerate() {
            now.positions[index] = light.position;
            let strength = light.intensity.at(0, time) * light.radius * light.radius;
            for channel in 0..3 {
                let value = light.color[channel] * strength;
                match index {
                    3 => now.colors[channel][3] = value,
                    _ => now.colors[index][channel] = value,
                }
            }
        }
        now
    }
}

impl Default for ObjectView {
    fn default() -> Self {
        Self {
            scene_per_pixel: 1.0,
            pointer: [-10.0, -10.0],
            from_clip: transform::IDENTITY,
            from_clip_to_unit: transform::IDENTITY,
            model: transform::IDENTITY,
            lights: LightsNow::default(),
        }
    }
}

/// A text object: how to draw it and what it last drew.
struct TextBox {
    face: Face,
    style: Style,
    shows: String,
}

impl TextBox {
    /// Puts `words` into the object's picture, if they are not there already.
    fn write(object: &mut Object, gl: &glow::Context, words: &str) {
        let Some(text) = &mut object.text else { return };
        if text.shows == words {
            return;
        }
        let picture = text.face.draw(words, &text.style);
        words.clone_into(&mut text.shows);
        object.size = [picture.width as f32, picture.height as f32];
        object.anchor = picture.anchor;
        if let Some(Some(texture)) = object.passes.first_mut().and_then(|p| p.textures.first_mut()) {
            texture.write_rgba(gl, picture.width, picture.height, &picture.rgba);
        }
    }
}

/// A text object, drawn as any image is once its picture exists. What it asks
/// for is in `crate::text`.
fn build_text(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    object: &wpe_format::scene::Object,
    skipped: &mut Vec<String>,
) -> Option<(Vec<Pass>, Box<TextBox>, [f32; 2], [f32; 2])> {
    let spare = resolve.get("fonts/NotoSans-Regular.ttf").map(|b| b.into_owned());
    // A `systemfont_` name means one of Windows' own, which are not here.
    let own = resolve.get(&object.font).map(|b| b.into_owned());
    let Some(face) = Face::new(own, spare) else {
        skipped.push(format!("{}: no font could be read for its text", object.name));
        return None;
    };
    let material = resolve.get("materials/fonts/basefontrgba.json")?;
    let material = Material::parse(&material).ok()?;
    let mut passes = build_passes(gl, resolve, &material, Kind::Image, object, skipped);
    let brighter = |color: [f32; 3], by: f32| color.map(|c| c * by);
    let style = Style {
        points: object.pointsize.value,
        size: object.size.map(|s| s.0),
        // Effect padding: nothing else has any room set aside for it.
        padding: if object.effects.is_empty() { 0.0 } else { object.padding.value.max(0.0) },
        across: Align::parse(&object.horizontalalign.value),
        down: Align::parse(&object.verticalalign.value),
        fill: brighter(object.color.value.0, object.brightness),
        background: object
            .opaquebackground
            .value
            .then(|| brighter(object.backgroundcolor.value.0, object.backgroundbrightness.value)),
        max_width: object.limitwidth.value.then_some(object.maxwidth.value),
        max_rows: object.limitrows.value.then_some(object.maxrows.value.max(1.0) as usize),
        ellipsis: object.limituseellipsis.value,
    };
    let words = match object.text().map(|t| &t.value) {
        Some(serde_json::Value::String(words)) => words.clone(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    let picture = face.draw(&words, &style);
    let texture = Texture::from_rgba(gl, picture.width, picture.height, &picture.rgba)
        .map_err(|e| skipped.push(format!("{}: {e}", object.name)))
        .ok()?;
    passes.first_mut()?.textures = vec![Some(texture)];
    let size = [picture.width as f32, picture.height as f32];
    let anchor = picture.anchor;
    Some((passes, Box::new(TextBox { face, style, shows: words }), size, anchor))
}

/// A child system that answers to its parent's particles -- bursting where
/// they are born or die, or riding along with them: which object the parent
/// is, which of those it does, and where the child sits inside it.
struct Answer {
    parent: usize,
    kind: ChildKind,
    origin: [f32; 3],
    scale: [f32; 3],
}

impl Answer {
    /// A place in the parent's space, in the child's.
    fn inside(&self, at: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|i| match self.scale[i] {
            0.0 => 0.0,
            scale => (at[i] - self.origin[i]) / scale,
        })
    }
}

/// The scene moving with the mouse, as `general` sets it up.
///
/// Wallpaper Engine's own formula is not on the disk. This is the one
/// Almamu's linux-wallpaperengine arrived at (`CScene::renderFrame`,
/// `CImage`, `CParticle::applyParallaxToModelMatrix`), which is the best
/// reading available: the mouse's distance from the middle of the screen,
/// times `amount` and `mouseinfluence`, eased towards at `delay` per second;
/// each object moved by that times its own depth plus `amount`, in units of
/// the scene's width.
/// The camera wandering by itself, in fractions of the picture's width. On
/// Windows it is two slow waves, measured off the recordings of Samurai, Alien:
/// Isolation 4K and Space Marine II by following the picture frame to frame: a
/// reach of 0.64 per cent of the width per unit of amplitude, at 0.168 Hz across
/// and 0.22 Hz up per unit of speed.
// ponytail: `camerashakeroughness` is left out. The recordings show it as a
// beat on top that nobody would miss; put it in if a scene turns on it.
struct Shake {
    amplitude: f32,
    speed: f32,
}

impl Shake {
    fn at(&self, time: f32) -> [f32; 2] {
        let wave = |hz: f32| (std::f32::consts::TAU * hz * self.speed * time).sin();
        [wave(0.168), wave(0.22)].map(|w| 0.0064 * self.amplitude * w)
    }
}

struct Parallax {
    amount: f32,
    delay: f32,
    influence: f32,
    /// Where the mouse is from the middle of the screen, as a fraction of it
    /// along each of the scene's axes.
    aim: [f32; 2],
    /// How far the scene has eased towards that.
    drift: [f32; 2],
    /// When it last eased, so a frame that took longer eases further.
    eased_at: f32,
}

/// The three layers the editor offers for putting an effect over what is
/// already drawn. Its own descriptions are the specification:
///
/// - *Post-processing Layer* (`fullscreenlayer`) "will always be aligned to
///   the screen and will not move with the wallpaper". Its shader writes
///   `a_Position` straight out as a clip-space position, so it is drawn with a
///   quad that is already the whole screen and no matrix.
/// - *Full Composition Layer* (`projectlayer`) covers the wallpaper, aligned
///   to it rather than to the screen.
/// - *Adjustable Composition Layer* (`composelayer`) is a rectangle of its own
///   size.
///
/// The last two share `composelayer.vert`, which takes its position from the
/// texture coordinate -- so it always fills whatever it is drawn into -- and
/// uses the matrix only to work out where on the screen to read from. Put
/// into an object's own buffers, that is the part of the screen under the
/// layer, ready for its effects, which then land where the layer is placed.
#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Placed,
    Screen,
}

/// The render target the layers read: everything drawn so far.
const FULL_FRAME: &str = "_rt_FullFrameBuffer";

pub struct Scene {
    objects: Vec<Object>,
    quad: glow::VertexArray,
    buffer: glow::Buffer,
    /// Allocated only if the scene has a particle system to draw.
    particles: Option<Buffers>,
    /// Stands in for a texture slot a pass leaves empty.
    white: Texture,
    /// One per object, in the same order, made for those with effects and
    /// remade when the object's size on screen changes.
    canvases: Vec<Option<Canvas>>,
    /// What has been drawn so far, copied out for a layer to read. Made only
    /// for a scene that has one, at the size of the output.
    frame: Option<Target>,
    /// Set when the scene moves with the mouse.
    parallax: Option<Parallax>,
    shake: Option<Shake>,
    zoom: f32,
    lighting: wpe_format::scene::Lighting,
    pointer: Option<(f32, f32)>,
    /// Set when the scene turns on its glow.
    bloom: Option<Bloom>,
    /// The videos the scene's images play, one player for each, however
    /// many layers show it.
    videos: BTreeMap<String, Video>,
    /// The buffers this scene's effects ask for by name, and what to divide
    /// the output by for each.
    buffers: BTreeMap<String, u32>,
    /// How far the particle systems have been simulated.
    simulated: f32,
    /// What the speakers are playing, as the meter last read it. Silence
    /// until a caller says otherwise, which is what a tool drawing to a file
    /// wants: the same wallpaper twice gives the same picture.
    heard: Spectrum,
    /// What the scene makes of that, a frame at a time, and when it last did:
    /// the bands everything in the scene that follows the sound reads.
    response: Response,
    responded_at: Option<f32>,
    sound: Spectrum,
    /// The scene's JavaScript, where it has any, and the time it last ran.
    scripts: Option<Scripts>,
    scripted_at: Option<f32>,
    /// The camera's field of view, in degrees, for perspective particles.
    fov: f32,
    pub scene_size: (u32, u32),
    /// Passes that would not compile, named so they can be looked at.
    pub skipped: Vec<String>,
}

/// The pair of offscreen pictures an object with effects is drawn through.
///
/// One holds what the object looks like, the next holds what an effect made of
/// it, and they take turns. Both are the size the object covers on screen, so
/// an effect's texture coordinates run from nought to one across the object
/// and nowhere else.
///
/// They used to be the size of the output and shared, with each object using
/// the corner it needed and its coordinates stopping where it did. Any shader
/// that works relative to where it is reading -- a blur, a tint -- could not
/// tell. One that names a place -- `t2` turning about a centre of (0.16, 0.26),
/// and the stock `spin`, `twirl`, `iris` and `godrays` -- found that place
/// somewhere else in the object, read past its edge, and smeared what it
/// found there into a rectangle. Wallpaper Engine gives every layer buffers
/// of its own, which its names say: `_rt_imageLayerComposite_151_a` is layer
/// 151's.
///
/// Beside the pair are the buffers the effects asked for by name, which is
/// where a chain of passes does its working out: a blur reads a quarter-sized
/// one so that one gaussian reaches four times as far across the picture.
struct Canvas {
    size: (u32, u32),
    targets: [Target; 2],
    named: BTreeMap<String, Target>,
    /// What the object finished as, for a layer that reads it. Only for an
    /// object somebody reads.
    kept: Option<Target>,
}

struct Target {
    frame: glow::Framebuffer,
    texture: Texture,
}

impl Target {
    /// The whole picture, which is more than the part of it an object uses.
    fn size(&self) -> (u32, u32) {
        (self.texture.width, self.texture.height)
    }

    unsafe fn free(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.frame);
            gl.delete_texture(self.texture.raw());
        }
    }
}

impl Canvas {
    /// There is one of these per object with effects now, rather than one per
    /// scene, so the old ones are worth giving back when they are remade.
    unsafe fn free(&self, gl: &glow::Context) {
        unsafe {
            for target in self.targets.iter().chain(self.named.values()).chain(&self.kept) {
                target.free(gl);
            }
        }
    }
}

/// Where one pass of an effect reads from, or writes to.
#[derive(Clone)]
enum Buffer {
    /// One of the two pictures an object's effects take turns in. The first of
    /// them is where the object itself is drawn.
    Canvas(usize),
    /// A buffer an effect asked for by name, at whatever size it asked for.
    Named(String),
    /// Another layer's finished picture, as it stood when that layer was
    /// drawn. `_rt_imageLayerComposite_151_a` means this to anybody but layer
    /// 151 itself: Almamu's renderer leaves a layer's result in whichever of
    /// the pair its chain happened to end in, and its own notes record a
    /// wallpaper reading `_a` and getting the wrong picture for it.
    Layer(i64),
    /// What the scene looked like just before this object, which a layer
    /// with a blend mode mixes itself into.
    Frame,
    /// Wherever the scene itself is being drawn, which only the last pass of
    /// the last effect writes to.
    Screen,
}

/// One pass of one effect: what it draws, what it reads, where it writes.
struct Step {
    /// None when the pass is a `copy` command rather than something drawn.
    pass: Option<Pass>,
    into: Buffer,
    /// Texture unit to buffer, as the effect asked. Always covers unit zero,
    /// because a pass that names nothing there still draws what came in.
    reads: Vec<(usize, Buffer)>,
    /// Which of the object's effects in scene.json this is a pass of, and
    /// which pass, which is how a script names the constant it drives.
    scene_pass: (usize, usize),
}

/// Maps an object's own quad onto the whole of whatever it is drawn into,
/// which is how it gets into the canvas without its place in the scene.
#[rustfmt::skip]
const FILL: Matrix = [
    2.0, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

unsafe fn make_target(gl: &glow::Context, size: (u32, u32)) -> Result<Target, String> {
    unsafe {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            size.0 as i32,
            size.1 as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        for (name, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
        }
        let frame = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(frame));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            return Err("an offscreen picture is not complete".into());
        }
        Ok(Target {
            frame,
            texture: Texture::borrowed(texture, size.0, size.1),
        })
    }
}

/// The pictures a scene's effects are drawn through: the pair every object with
/// an effect takes turns in, and one for each buffer the effects name.
unsafe fn make_canvas(
    gl: &glow::Context,
    size: (u32, u32),
    wanted: &BTreeMap<String, u32>,
    kept: bool,
) -> Result<Canvas, String> {
    unsafe {
        let targets = [make_target(gl, size)?, make_target(gl, size)?];
        let kept = match kept {
            true => Some(make_target(gl, size)?),
            false => None,
        };
        let mut named = BTreeMap::new();
        for (name, divisor) in wanted {
            // A buffer at a quarter of the output is a quarter of it each way,
            // and never smaller than a pixel.
            let smaller = ((size.0 / divisor).max(1), (size.1 / divisor).max(1));
            named.insert(name.clone(), make_target(gl, smaller)?);
        }
        Ok(Canvas {
            size,
            targets,
            named,
            kept,
        })
    }
}

/// The buffers every particle system in the scene is drawn through, sized for
/// the largest of them and refilled per system per frame.
struct Buffers {
    vao: glow::VertexArray,
    vertices: glow::Buffer,
    /// A ribbon's vertices carry a whole segment each, so they need a layout
    /// and a buffer of their own.
    rope: glow::VertexArray,
    rope_vertices: glow::Buffer,
    #[allow(dead_code)]
    indices: glow::Buffer,
}

impl Scene {
    /// Loads a wallpaper, keeping its particles within `budget`.
    ///
    /// A viewer who knows what their machine can take should be able to say
    /// so, which is why the budget is an argument and not a constant buried
    /// in here. [`DEFAULT_BUDGET`] is what to pass when nobody has said.
    pub fn load_within(
        gl: &glow::Context,
        dir: &std::path::Path,
        budget: usize,
    ) -> Result<Self, String> {
        let package = open_package(dir)?;
        let assets = Assets::locate();
        let resolve = Resolver::new(&package, assets.as_ref());

        let project = std::fs::read(dir.join("project.json")).ok();
        let entry = scene_entry(project.as_deref());
        let bytes = package
            .get(&entry)
            .ok_or_else(|| format!("no {entry} in this wallpaper"))?;
        let file = SceneFile::parse_with_properties(bytes, project.as_deref())
            .map_err(|e| format!("scene.json: {e}"))?;
        let scene_size = (
            file.general.orthogonalprojection.width,
            file.general.orthogonalprojection.height,
        );

        let mut objects = Vec::new();
        let mut skipped = Vec::new();
        let mut videos = BTreeMap::new();
        let mut buffers = BTreeMap::new();
        // The layers that finish as a picture another can read: those with
        // effects, and those somebody asks to read even without any.
        let asked_for: BTreeSet<i64> = file
            .objects
            .iter()
            .flat_map(|o| &o.effects)
            .flat_map(|e| &e.passes)
            .flat_map(|p| &p.textures)
            .flatten()
            .filter_map(|name| other_layer(name))
            .collect();
        let carriers: BTreeSet<i64> = file
            .objects
            .iter()
            .filter(|o| o.visible.value && (!o.effects.is_empty() || asked_for.contains(&o.id)))
            .map(|o| o.id)
            .collect();
        let folder = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        for object in &file.objects {
            if !object.visible.value || LEFT_OUT.contains(&(folder, object.id)) {
                continue;
            }
            match object.kind() {
                ObjectKind::Particle(file) => {
                    if let Some(system) = load_particles(gl, &resolve, object, &file, 0, &mut objects, &mut skipped) {
                        objects[system].from_scene = true;
                    }
                    continue;
                }
                ObjectKind::Text => {
                    let Some((passes, text, size, anchor)) = build_text(gl, &resolve, object, &mut skipped) else {
                        continue;
                    };
                    objects.push(Object {
                        name: object.name.clone(),
                        origin: object.origin.value.0,
                        size,
                        scale: object.scale.value.0,
                        angles: object.angles.value.0,
                        alpha: object.alpha.value,
                        brightness: 1.0,
                        color: [1.0; 3],
                        passes,
                        drawing: Drawing::Image,
                        trail: None,
                        effects: build_effects(gl, &resolve, object, &carriers, &mut buffers, &mut skipped),
                        id: object.id,
                        kept: false,
                        answers: None,
                        layer: None,
                        depth: object.parallax_depth.0,
                        anchor,
                        from_scene: true,
                        shown: true,
                        text: Some(text),
                        view: ObjectView::default(),
                        puppet: None,
                    });
                    continue;
                }
                ObjectKind::Image(_) => {}
                _ => continue,
            }
            let ObjectKind::Image(model_path) = object.kind() else {
                continue;
            };
            let Some(model_bytes) = resolve.get(&model_path) else {
                continue;
            };
            let Ok(model) = Model::parse(&model_bytes) else {
                continue;
            };
            let Some(material_bytes) = resolve.get(&model.material) else {
                continue;
            };
            let Ok(material) = Material::parse(&material_bytes) else {
                continue;
            };

            let mut passes = build_passes(gl, &resolve, &material, Kind::Image, object, &mut skipped);
            if passes.is_empty() {
                continue;
            }
            if let Err(e) = play_videos(gl, &resolve, &mut passes, &mut videos, &video_cache(dir)) {
                skipped.push(format!("{}: {e}", object.name));
                continue;
            }

            // Most objects carry no size. `autosize` on the model says to take
            // it from the image instead, which is what makes a small decoration
            // small; falling back to the scene would blow every one of them up
            // to fill the screen.
            let size = match object.size.map(|s| s.0) {
                Some(size) => size,
                None => passes
                    .first()
                    .and_then(|p| p.first_image_size())
                    .filter(|_| model.autosize)
                    .map(|(w, h)| [w as f32, h as f32])
                    .unwrap_or([scene_size.0 as f32, scene_size.1 as f32]),
            };
            let puppet = model
                .puppet
                .as_deref()
                .and_then(|path| resolve.get(path))
                .and_then(|bytes| Puppet::parse(&bytes))
                .and_then(|puppet| {
                    let playing = object
                        .animationlayers
                        .iter()
                        .filter(|layer| layer.visible.value)
                        .map(|layer| Playing {
                            animation: layer.animation,
                            rate: layer.rate.value,
                            blend: layer.blend.value,
                        })
                        .collect();
                    unsafe { PuppetMesh::new(gl, puppet, playing) }.ok().map(Box::new)
                });
            if model.puppet.is_some() && puppet.is_none() {
                skipped.push(format!("{}: its puppet could not be read, so it stays flat", object.name));
            }
            let effects = build_effects(gl, &resolve, object, &carriers, &mut buffers, &mut skipped);
            let layer = match (model.passthrough, model.fullscreen) {
                (false, _) => None,
                (true, false) => Some(Layer::Placed),
                (true, true) => Some(Layer::Screen),
            };
            // A layer with nothing on it would copy the screen back onto
            // itself, and its shader cannot be drawn straight to the screen
            // anyway: it fills whatever it is drawn into.
            if layer.is_some() && effects.is_empty() {
                continue;
            }
            objects.push(Object {
                name: object.name.clone(),
                origin: object.origin.value.0,
                size,
                scale: object.scale.value.0,
                angles: object.angles.value.0,
                alpha: object.alpha.value,
                brightness: object.brightness,
                color: object.color.value.0,
                passes,
                drawing: Drawing::Image,
                trail: None,
                effects,
                id: object.id,
                kept: false,
                answers: None,
                layer,
                depth: object.parallax_depth.0,
                anchor: anchor_of(&object.alignment),
                from_scene: true,
                shown: true,
                text: None,
                view: ObjectView::default(),
                puppet,
            });
        }

        if objects.is_empty() {
            return Err("nothing in this wallpaper could be drawn".into());
        }
        let read: BTreeSet<i64> = objects
            .iter()
            .flat_map(|o| &o.effects)
            .flat_map(|step| &step.reads)
            .filter_map(|(_, from)| match from {
                Buffer::Layer(id) => Some(*id),
                _ => None,
            })
            .collect();
        for object in &mut objects {
            object.kept = read.contains(&object.id);
        }
        thin_particles(&mut objects, budget, &mut skipped);
        // Where the scene's edges are from where each system stands.
        for object in &mut objects {
            let inside = |edge: f32, axis: usize| match object.scale[axis] {
                0.0 => 0.0,
                scale => (edge - object.origin[axis]) / scale,
            };
            let scene = wpe_format::particle::Range {
                min: [inside(0.0, 0), inside(0.0, 1), 0.0],
                max: [inside(scene_size.0 as f32, 0), inside(scene_size.1 as f32, 1), 0.0],
            };
            if let Drawing::Particles(system) = &mut object.drawing {
                system.bound_by(scene);
            }
        }
        let scripts = Scripts::load(&file, project.as_deref());
        if let Some(scripts) = &scripts {
            skipped.extend(scripts.skipped.iter().map(|why| format!("script {why}")));
        }

        let (quad, buffer) = unsafe { make_quad(gl)? };
        let widest = objects
            .iter()
            .filter_map(|o| match &o.drawing {
                Drawing::Particles(s) => Some(s.capacity()),
                Drawing::Image => None,
            })
            .max();
        let particles = match widest {
            Some(capacity) => Some(unsafe { make_particle_buffers(gl, capacity)? }),
            None => None,
        };
        let bloom = match file.general.bloom.value {
            false => None,
            true => build_bloom(gl, &resolve, &file.general, scene_size)
                .map_err(|e| skipped.push(format!("bloom: {e}")))
                .ok(),
        };
        Ok(Self {
            objects,
            quad,
            buffer,
            particles,
            white: Texture::white(gl)?,
            canvases: Vec::new(),
            frame: None,
            bloom,
            videos,
            parallax: (file.general.cameraparallax.value
                && file.general.cameraparallaxmouseinfluence != 0.0)
                .then_some(Parallax {
                    amount: file.general.cameraparallaxamount,
                    delay: file.general.cameraparallaxdelay,
                    influence: file.general.cameraparallaxmouseinfluence,
                    aim: [0.0; 2],
                    drift: [0.0; 2],
                    eased_at: 0.0,
                }),
            shake: file.general.camerashake.value.then_some(Shake {
                amplitude: file.general.camerashakeamplitude,
                speed: file.general.camerashakespeed,
            }),
            zoom: file.general.zoom,
            lighting: wpe_format::scene::Lighting::read(bytes),
            pointer: None,
            buffers,
            simulated: 0.0,
            heard: Spectrum::SILENT,
            response: Response::default(),
            responded_at: None,
            sound: Spectrum::SILENT,
            scripts,
            scripted_at: None,
            fov: file.general.fov,
            scene_size,
            skipped,
        })
    }

    /// Hands the scene what the speakers are playing, which the steps that
    /// follow the sound read on the next frame. A caller that never says
    /// anything leaves the scene running as it would in a silent room.
    pub fn hear(&mut self, sound: Spectrum) {
        self.heard = sound;
    }

    /// Whether anything in the scene follows the sound: a shader that reads
    /// the spectrum, or a particle system whose rate, field or swirl does. A
    /// scene that does not is never handed the sound, so nothing listens to
    /// the speakers on its behalf.
    pub fn listens(&self) -> bool {
        self.objects.iter().any(|object| {
            object.passes.iter().any(|pass| pass.hears)
                || object.effects.iter().filter_map(|step| step.pass.as_ref()).any(|pass| pass.hears)
                || matches!(&object.drawing, Drawing::Particles(system) if system.listens())
        }) || self.scripts.as_ref().is_some_and(Scripts::listens)
    }

    /// Says where the mouse is: a fraction of an output of size `output`
    /// across and down from its top-left corner, or nothing when there is no
    /// mouse over the wallpaper. Holds until it is said again.
    ///
    /// Eleven wallpapers here hang a particle force on the mouse -- most of
    /// them a push that clears a hole wherever the pointer goes. Without one
    /// that push is simply not applied: applied where the mouse rests in the
    /// editor, it would carve a hole the wallpaper never asked for.
    // Nothing on the screen passes a mouse yet; `shot` does, and the bridge
    // into the Qt program will.
    #[allow(dead_code)]
    pub fn point_at(&mut self, pointer: Option<(f32, f32)>, output: (u32, u32)) {
        let projection = transform::orthographic_cover(
            (self.scene_size.0 as f32, self.scene_size.1 as f32),
            (output.0 as f32, output.1 as f32),
        );
        self.pointer = pointer;
        // Clip space runs up the screen, the pointer down it.
        let clip = pointer.map(|(x, y)| [x * 2.0 - 1.0, 1.0 - y * 2.0]);
        if let Some(parallax) = &mut self.parallax {
            // Worked out through the projection rather than by hand, so the
            // sign of each axis is whatever the scene's own is.
            let at = |c: [f32; 2]| transform::unapply_flat(&projection, c).unwrap_or([0.0; 2]);
            let (middle, right, top) = (at([0.0, 0.0]), at([1.0, 0.0]), at([0.0, 1.0]));
            let half = [(right[0] - middle[0]).abs(), (top[1] - middle[1]).abs()];
            parallax.aim = match clip {
                Some(c) => {
                    let p = at(c);
                    [
                        (p[0] - middle[0]) / (2.0 * half[0]).max(f32::EPSILON),
                        (p[1] - middle[1]) / (2.0 * half[1]).max(f32::EPSILON),
                    ]
                }
                None => [0.0; 2],
            };
        }
        for object in &mut self.objects {
            if let Drawing::Particles(system) = &mut object.drawing {
                let model = transform::model(object.origin, object.size, object.scale, object.angles);
                let mvp = transform::multiply(&projection, &model);
                let local = clip.and_then(|c| transform::unapply_flat(&mvp, c));
                system.point_at(local.map(|[x, y]| [x, y, 0.0]));
            }
        }
    }

    pub fn object_names(&self) -> impl Iterator<Item = &str> {
        self.objects.iter().map(|o| o.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Runs the particle systems up to `time` however long it takes.
    ///
    /// For looking at a wallpaper at a chosen moment rather than watching it:
    /// drawing caps how much it will catch up in one frame, and a tool that
    /// wants a scene as it would be after a minute has to ask for it.
    #[allow(dead_code)]
    pub fn settle(&mut self, time: f32) {
        // A minute of a scene is already more than any system needs to reach
        // its steady state.
        self.advance_to(time, (60.0 / STEP) as usize);
    }

    /// Live particles across every system, which is what says whether the
    /// systems are running at the moment the scene was drawn.
    #[allow(dead_code)]
    pub fn particle_count(&self) -> usize {
        self.objects
            .iter()
            .filter_map(|o| match &o.drawing {
                Drawing::Particles(s) => Some(s.count()),
                Drawing::Image => None,
            })
            .sum()
    }

    /// Draws the scene onto an output of the given size.
    ///
    /// `time` is seconds since the wallpaper started, not a step, so a caller
    /// may jump straight to a moment and the particle systems still arrive
    /// there the same way they would have by running.
    pub fn draw(&mut self, gl: &glow::Context, output: (u32, u32), time: f32) {
        let dt = self.responded_at.map_or(0.0, |at| time - at);
        self.responded_at = Some(time);
        self.sound = self.response.step(&self.heard, dt);
        self.run_scripts(gl, time);
        self.advance_to(time, STEPS_PER_FRAME);
        for video in self.videos.values_mut() {
            video.show(gl, time);
        }
        let aspect = output.0 as f32 / output.1.max(1) as f32;
        let shift = self.shake.as_ref().map_or([0.0; 2], |s| {
            let [x, y] = s.at(time);
            [x * 2.0, y * 2.0 * aspect]
        });
        let projection = transform::viewed(
            &transform::orthographic_cover(
                (self.scene_size.0 as f32, self.scene_size.1 as f32),
                (output.0 as f32, output.1 as f32),
            ),
            self.zoom,
            shift,
        );
        let camera = transform::viewed(
            &transform::perspective_cover(
                (self.scene_size.0 as f32, self.scene_size.1 as f32),
                (output.0 as f32, output.1 as f32),
                self.fov,
            ),
            self.zoom,
            shift,
        );
        let pointer = self.pointer.map_or([-10.0, -10.0], |(x, y)| [x, y]);
        let lights = LightsNow::of(&self.lighting, time);
        for object in self.objects.iter_mut().filter(|o| matches!(o.drawing, Drawing::Image)) {
            if let Some(puppet) = &mut object.puppet {
                puppet.bend(time, object.size);
            }
            let used = object.on_screen(&projection, output, output);
            let middle =
                middle_of(object.origin, object.anchor, object.size, object.scale, object.angles[2]);
            let onto = transform::multiply(
                &projection,
                &transform::model(middle, [1.0, 1.0], object.scale, object.angles),
            );
            let halves = [object.size[0].abs() / 2.0, object.size[1].abs() / 2.0];
            let onto_unit =
                transform::multiply(&onto, &transform::model([0.0; 3], halves, [1.0; 3], [0.0; 3]));
            object.view = ObjectView {
                // Wallpaper Engine's buffer for a post-processing layer is the
                // back buffer, so a blur there reaches screen pixels. Told the
                // size in the file, Gears of War's reached twice as far.
                scene_per_pixel: if object.layer == Some(Layer::Screen) {
                    1.0
                } else {
                    object.size[0].abs() / used.0.max(1) as f32
                },
                pointer,
                from_clip: transform::inverse_flat(&onto).unwrap_or(transform::IDENTITY),
                from_clip_to_unit: transform::inverse_flat(&onto_unit).unwrap_or(transform::IDENTITY),
                model: transform::model(
                    middle,
                    [object.size[0], -object.size[1]],
                    object.scale,
                    object.angles,
                ),
                lights,
            };
        }
        // Whatever the caller was drawing into, which an effect leaves for a
        // moment and has to put back.
        let screen = unsafe {
            std::num::NonZeroU32::new(gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32)
                .map(glow::NativeFramebuffer)
        };
        if (self.bloom.is_some() || self.objects.iter().any(Object::reads_frame))
            && self.frame.as_ref().is_none_or(|f| f.size() != output)
        {
            if let Some(old) = self.frame.take() {
                unsafe { old.free(gl) };
            }
            self.frame = unsafe { make_target(gl, output) }.ok();
        }
        self.canvases.resize_with(self.objects.len(), || None);
        for (object, canvas) in self.objects.iter().zip(&mut self.canvases) {
            if object.effects.is_empty() {
                continue;
            }
            let size = object.on_screen(&projection, output, output);
            if canvas.as_ref().is_some_and(|c| c.size == size) {
                continue;
            }
            if let Some(old) = canvas.take() {
                unsafe { old.free(gl) };
            }
            *canvas = unsafe { make_canvas(gl, size, &self.buffers, object.kept) }.ok();
        }

        unsafe {
            // Making the canvas leaves its last picture bound, and an object
            // with no effects of its own is drawn into whatever is bound. On
            // the frame the canvas is built -- the first one, and every one
            // after a resize -- every such object before the first object with
            // effects lands in an offscreen buffer nobody reads. It looks like
            // those objects failing to load rather than like a buffer being
            // left bound, which is why it is worth saying out loud here.
            gl.bind_framebuffer(glow::FRAMEBUFFER, screen);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
        }
        let drift = match &mut self.parallax {
            Some(p) => {
                let ease = (p.delay * (time - p.eased_at)).clamp(0.0, 1.0);
                p.eased_at = time;
                let goal = p.aim.map(|a| a * p.amount * p.influence);
                p.drift = std::array::from_fn(|i| p.drift[i] + (goal[i] - p.drift[i]) * ease);
                Some((p.drift, p.amount))
            }
            None => None,
        };
        let width = self.scene_size.0 as f32;
        let kept: BTreeMap<i64, &Target> = self
            .objects
            .iter()
            .zip(&self.canvases)
            .filter_map(|(object, canvas)| Some((object.id, canvas.as_ref()?.kept.as_ref()?)))
            .collect();
        for (object, canvas) in self.objects.iter().zip(&self.canvases) {
            if !object.shown {
                continue;
            }
            let mut origin = object.origin;
            if let Some((drift, amount)) = drift {
                for axis in 0..2 {
                    origin[axis] += (object.depth[axis] + amount) * drift[axis] * width;
                }
            }
            // A scene's y runs up and a picture's rows run down, so an image
            // is turned over on its quad to stand the right way up. A particle
            // system is not: its particles live in the scene's own axes.
            let size = match object.drawing {
                Drawing::Image => [object.size[0], -object.size[1]],
                Drawing::Particles(_) => object.size,
            };
            origin = middle_of(origin, object.anchor, object.size, object.scale, object.angles[2]);
            let model = transform::model(origin, size, object.scale, object.angles);
            let mvp = match &object.drawing {
                Drawing::Particles(system) if system.is_perspective() => {
                    transform::multiply(&camera, &model)
                }
                _ => transform::multiply(&projection, &model),
            };
            match (&object.drawing, &self.particles) {
                (Drawing::Image, _) => unsafe {
                    gl.bind_vertex_array(Some(self.quad));
                    // A layer reads everything drawn before it, so the
                    // picture is taken now, not once a frame.
                    let frame = match (object.reads_frame(), &self.frame) {
                        (true, Some(frame)) => {
                            take_frame(gl, screen, frame, output);
                            Some(frame)
                        }
                        _ => None,
                    };
                    match (canvas, object.effects.is_empty()) {
                        (Some(canvas), false) => draw_through_effects(
                            gl,
                            object,
                            frame,
                            canvas,
                            &kept,
                            self.buffer,
                            &self.white,
                            &mvp,
                            time,
                            output,
                            screen,
                            &projection,
                            &self.sound,
                        ),
                        _ => {
                            for pass in &object.passes {
                                write_quad(gl, self.buffer, pass.covered, 0.5);
                                draw_own_pass(
                                    gl,
                                    pass,
                                    object,
                                    &mvp,
                                    time,
                                    &self.white,
                                    &pass.screen_binds(frame),
                                    &self.sound,
                                    pass.blending,
                                    pass.covered,
                                );
                            }
                        }
                    }
                },
                (Drawing::Particles(system), Some(buffers)) => unsafe {
                    // Taken for each system that wants it, since the one
                    // before it is part of what it refracts.
                    // ponytail: a whole-screen copy per such system; DOOM has a
                    // dozen. Copy only the system's bounds if that ever shows.
                    let frame = match (object.reads_frame(), &self.frame) {
                        (true, Some(frame)) => {
                            take_frame(gl, screen, frame, output);
                            Some(frame)
                        }
                        _ => None,
                    };
                    draw_particles(
                        gl,
                        object,
                        system,
                        buffers,
                        &mvp,
                        time,
                        &self.white,
                        &self.sound,
                        frame,
                    );
                },
                // Unreachable: the buffers are allocated whenever a particle
                // system survived loading.
                (Drawing::Particles(_), None) => {}
            }
        }
        if let (Some(bloom), Some(frame)) = (&mut self.bloom, &self.frame) {
            unsafe {
                gl.bind_vertex_array(Some(self.quad));
                bloom.draw(gl, frame, self.buffer, screen, output, time, &self.white, &self.sound);
            }
        }
        // A scene is opaque. Its colours are blended right, but what blending
        // leaves in the alpha channel is the square of each layer's alpha and
        // means nothing -- and whatever shows the picture next reads it:
        // Pyramid Spaceships came out a quarter transparent where its smoke
        // was, and Blue Dawn nine tenths around its lens flare. Wallpaper
        // Engine and Almamu's renderer both mask alpha on the last pass; this
        // is the same thing done once.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, screen);
            gl.viewport(0, 0, output.0 as i32, output.1 as i32);
            gl.color_mask(false, false, false, true);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.color_mask(true, true, true, true);
            // Which index buffer is bound belongs to the bound vertex array,
            // so a caller binding none afterwards -- Qt does, resetting its
            // state -- took the indices off the particles' array and every
            // particle after the first frame was drawn without them. None of
            // ours is left bound for anybody to write into.
            gl.bind_vertex_array(None);
        }
    }

    /// Steps every particle system up to `time`, taking at most `most` of
    /// them and dropping whatever time is left over.
    ///
    /// Fixed slices rather than whatever the last frame took, so a system
    /// looks the same whether the compositor asked for 60 frames or 144.
    ///
    /// The cap is what keeps a heavy scene from running away. A frame has to
    /// simulate the time the frame before it took; if the simulation cannot
    /// keep up, that debt compounds, and it compounds fast -- one wallpaper
    /// here went from 14 ms a frame to 1.6 seconds within twelve frames, which
    /// is a slideshow and then a standstill. Capping the steps turns that into
    /// something that merely runs slow.
    /// Runs the scene's scripts for a frame at `time` and puts what they
    /// leave where the drawing reads it.
    fn run_scripts(&mut self, gl: &glow::Context, time: f32) {
        let Some(scripts) = &mut self.scripts else {
            return;
        };
        // The first frame catches up from the start in steps no longer than
        // a thirtieth of a second, as the particles do, so a scene first
        // drawn some seconds in -- which is what a picture of one is -- has
        // had its scripts run up to that moment: most of them ease towards
        // the music by `engine.frametime`. After that it is one step a
        // frame, and a frame after a long pause -- a covered screen -- is
        // told no more than a few steps' worth of time went by, rather than
        // spending a quarter of a second replaying what nobody saw.
        let (from, steps) = match self.scripted_at {
            None => (0.0, ((time / SCRIPT_STEP).ceil()).clamp(1.0, MOST_SCRIPT_STEPS) as usize),
            Some(at) => ((time - 3.0 * SCRIPT_STEP).max(at).min(time), 1),
        };
        let frametime = (time - from) / steps as f32;
        self.scripted_at = Some(time);
        let mut last = None;
        for step in 1..=steps {
            last = scripts.run(frametime, from + frametime * step as f32, &self.sound);
        }
        let Some(frame) = last else {
            return;
        };
        let placed = |objects: &mut Vec<Object>, id: i64| -> Vec<usize> {
            (0..objects.len()).filter(|&i| objects[i].from_scene && objects[i].id == id).collect()
        };
        for (id, layer) in frame.layers {
            for index in placed(&mut self.objects, id) {
                let object = &mut self.objects[index];
                object.origin = layer.origin;
                object.angles = layer.angles;
                object.scale = layer.scale;
                object.alpha = layer.alpha;
                object.shown = layer.visible;
            }
        }
        for (id, words) in &frame.texts {
            for index in placed(&mut self.objects, *id) {
                TextBox::write(&mut self.objects[index], gl, words);
            }
        }
        for (target, value) in frame.values {
            match target {
                Drives::Layer => {}
                Drives::Rate { id } => {
                    let Some(rate) = value.as_f64() else { continue };
                    for index in placed(&mut self.objects, *id) {
                        if let Drawing::Particles(system) = &mut self.objects[index].drawing {
                            system.set_rate(rate as f32);
                        }
                    }
                }
                Drives::Alpha { id } => {
                    let Some(alpha) = value.as_f64() else { continue };
                    for index in placed(&mut self.objects, *id) {
                        if let Drawing::Particles(system) = &mut self.objects[index].drawing {
                            system.set_alpha(alpha as f32);
                        }
                    }
                }
                Drives::Constant { id, effect, pass, name } => {
                    for index in placed(&mut self.objects, *id) {
                        let steps = self.objects[index].effects.iter_mut();
                        for step in steps.filter(|s| s.scene_pass == (*effect, *pass)) {
                            if let Some(pass) = &mut step.pass {
                                let uniform = pass.labels.get(name).unwrap_or(name).clone();
                                pass.constants.insert(uniform, value.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    fn advance_to(&mut self, time: f32, most: usize) {
        let sound = self.sound;
        // In slices no longer than a step that come out to exactly the time
        // owed. Whole steps of a sixtieth on a 75 Hz screen left every fifth
        // frame with nothing new to show, which is how smoke came to stutter.
        let mut steps = ((time - self.simulated).max(0.0) / STEP).ceil() as usize;
        if time - self.simulated < 1e-4 {
            return;
        }
        if steps > most {
            self.simulated = time - most as f32 * STEP;
            steps = most;
        }
        let slice = (time - self.simulated) / steps as f32;
        for _ in 0..steps {
            for index in 0..self.objects.len() {
                let (before, rest) = self.objects.split_at_mut(index);
                let Object { drawing, answers, .. } = &mut rest[0];
                let Drawing::Particles(system) = drawing else {
                    continue;
                };
                // A parent is always loaded before its children, so it has
                // already taken this step and what it says is this step's.
                if let Some(answer) = answers
                    && let Drawing::Particles(parent) = &before[answer.parent].drawing
                    && let Some(events) = parent.events()
                {
                    match answer.kind {
                        ChildKind::EventFollow => {
                            let hosts: Vec<_> = events
                                .alive
                                .iter()
                                .map(|&(serial, from)| {
                                    (serial, Snapshot { position: answer.inside(from.position), ..from })
                                })
                                .collect();
                            system.follow(&hosts, slice, &sound);
                        }
                        kind => {
                            let places = match kind {
                                ChildKind::EventSpawn => &events.born,
                                _ => &events.died,
                            };
                            for &from in places {
                                let position = answer.inside(from.position);
                                system.burst_at(Snapshot { position, ..from }, &sound);
                            }
                        }
                    }
                }
                system.advance(slice, &sound);
            }
        }
        self.simulated = time;
    }
}

/// A particle system and the systems it brings with it, each an object of its
/// own: the parent first, then its children in the order the file lists them.
///
/// A child is loaded as its parent would be, with the parent's place in the
/// scene and its own place inside the parent added together. The parent may
/// draw nothing itself -- some exist only to carry their children -- and the
/// children still come. A child that bursts where its parent's particles are
/// born or die, or rides along with them, is told which object its parent is.
///
/// Says which object is the system itself, if it could be drawn.
fn load_particles(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    object: &wpe_format::scene::Object,
    path: &str,
    depth: u32,
    objects: &mut Vec<Object>,
    skipped: &mut Vec<String>,
) -> Option<usize> {
    let Some(bytes) = resolve.get(path) else {
        return None;
    };
    let definition = match ParticleSystem::parse(&bytes) {
        Ok(d) => d,
        Err(e) => {
            skipped.push(format!("{}: {path}: {e}", object.name));
            return None;
        }
    };
    let built = build_particles(gl, resolve, object, &definition, skipped);
    let drawn = built.is_some().then_some(objects.len());
    objects.extend(built);

    let parent = transform::model(
        object.origin.value.0,
        [1.0, 1.0],
        object.scale.value.0,
        object.angles.value.0,
    );
    for (index, child) in definition.children.iter().enumerate() {
        // A burst at a particle needs particles: with no parent drawn there
        // is nothing to burst at.
        let answers = match (child.kind, drawn) {
            (ChildKind::Static, _) => None,
            (_, None) => continue,
            (kind, Some(parent)) => Some(Answer {
                parent,
                kind,
                origin: child.origin,
                scale: child.scale,
            }),
        };
        // Nothing in the library goes deeper than a child of a child, and a
        // file that named itself would otherwise load for ever.
        if depth >= 4 {
            skipped.push(format!("{}: children nested too deep at {}", object.name, child.file));
            continue;
        }
        let mut placed = object.clone();
        // A number of its own, since a system is seeded from it: with its
        // parent's, a child and its siblings would all emit in step.
        placed.id = object.id.wrapping_mul(1_000_003).wrapping_add(index as i64 + 1);
        // Its alpha and its clock go on down. Warhammer's rays are children of
        // charges set anywhere from 0.13 to 0.76 at a rate of 0.07, and on
        // Windows they keep to their charge's strength and its slow time. The
        // rest stays with the parent: handed down, Star Citizen's embers, five
        // times the size, took their glows with them and burned the sky white.
        placed.instanceoverride = object.instanceoverride.as_ref().map(|over| {
            wpe_format::scene::InstanceOverride {
                alpha: over.alpha.clone(),
                rate: over.rate.clone(),
                colorn: over.colorn.clone(),
                color: over.color.clone(),
                ..Default::default()
            }
        });
        placed.origin.value.0 = transform::apply(&parent, child.origin);
        placed.scale.value.0 = std::array::from_fn(|i| object.scale.value.0[i] * child.scale[i]);
        placed.angles.value.0 = std::array::from_fn(|i| object.angles.value.0[i] + child.angles[i]);
        let made = load_particles(gl, resolve, &placed, &child.file, depth + 1, objects, skipped);
        if let Some((answer, made)) = answers.zip(made) {
            let parent = answer.parent;
            if let Drawing::Particles(system) = &mut objects[made].drawing {
                match answer.kind {
                    ChildKind::EventFollow => system.ride_along(child.maxcount),
                    _ => system.wait_for_parent(),
                }
            }
            objects[made].answers = Some(answer);
            if let Drawing::Particles(system) = &mut objects[parent].drawing {
                system.listen();
            }
        }
    }
    drawn
}

/// One particle system as an object, or None if it cannot be drawn.
fn build_particles(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    object: &wpe_format::scene::Object,
    definition: &ParticleSystem,
    skipped: &mut Vec<String>,
) -> Option<Object> {
    let mut trail = None;
    let form = match definition.renderer {
        ParticleRenderer::Sprite => Form::Sprite,
        // Both kinds of ribbon are drawn by the same shader, which takes its
        // geometry as it is given: what differs is only what the system
        // threads the ribbon through.
        ParticleRenderer::Rope { .. } | ParticleRenderer::RopeTrail { .. } => Form::Rope,
        ParticleRenderer::Trail {
            length,
            min_length,
            max_length,
        } => {
            trail = Some([length, max_length, min_length]);
            Form::Trail
        }
        ref other => {
            skipped.push(format!(
                "{}: {other:?} particles are not drawn yet",
                object.name
            ));
            return None;
        }
    };
    if definition.emitters.is_empty() || definition.maxcount == 0 {
        return None;
    }
    for step in &definition.unsupported {
        skipped.push(format!("{}: {step} is not applied yet", object.name));
    }

    let material_bytes = resolve.get(&definition.material)?;
    let material = Material::parse(&material_bytes).ok()?;
    let passes = build_passes(
        gl,
        resolve,
        &material,
        Kind::Particles(form),
        object,
        skipped,
    );
    if passes.is_empty() {
        return None;
    }

    for effect in &object.effects {
        if effect.visible.value {
            skipped.push(format!(
                "{}: effect {} is not applied yet",
                object.name,
                effect_name(effect)
            ));
        }
    }

    // Seeded from the object's own id so two systems in one scene do not
    // emit in step, and so a screenshot of a wallpaper is reproducible.
    let over = object.instanceoverride.clone().unwrap_or_default();
    let system = particles::System::new(definition, &over, object.id as u32);
    Some(Object {
        name: object.name.clone(),
        origin: object.origin.value.0,
        // Particles carry their own size in the buffer, so the object's
        // transform must place and turn them without scaling a quad.
        size: [1.0, 1.0],
        scale: object.scale.value.0,
        angles: object.angles.value.0,
        alpha: object.alpha.value,
        brightness: object.brightness,
        color: object.color.value.0,
        passes,
        drawing: Drawing::Particles(Box::new(system)),
        trail,
        // No particle system in the library carries one.
        effects: Vec::new(),
        id: object.id,
        kept: false,
        answers: None,
        layer: None,
        // A particle system never stands still under the mouse, however deep
        // it was put: the same floor Almamu's renderer puts under it.
        depth: object
            .parallax_depth
            .0
            .map(|d| if d.abs() < 0.65 { 0.65f32.copysign(d) } else { d }),
        anchor: [0.0; 2],
        from_scene: false,
        shown: true,
        text: None,
        view: ObjectView::default(),
        puppet: None,
    })
}

/// The longest step a scene's scripts are run in, and the most steps the
/// first frame catches up by: ten seconds, after which the scripts jump.
///
/// ponytail: a step costs Dark Core, the most scripted scene here, about
/// 0.8 ms -- 43 scripts, and every layer handed back as JSON. Hand back only
/// the layers a script touched if a scene with hundreds of objects shows up.
const SCRIPT_STEP: f32 = 1.0 / 30.0;
const MOST_SCRIPT_STEPS: f32 = 300.0;

/// How many particles a scene may keep alive at once before it is thinned.
///
/// Measured here: a frame costs about four tenths of a millisecond plus a
/// fifth of a microsecond for every particle, so this is a few milliseconds of
/// one. It is also where the library divides -- four wallpapers ask for
/// between 51 000 and 183 000 particles and every other one asks for under
/// 24 000, half of them for under 600 -- so nothing but those four is touched.
pub const DEFAULT_BUDGET: usize = 24_000;

/// Holds the largest systems in a scene down until the scene fits.
///
/// They are what costs and they can spare it: a third of 45 000 sparks is still
/// a shower, where a third of a torch's 41 flames is a torch a third as bright
/// -- which is what Gears of War's eyes and DOOM's torches were while every
/// system was scaled alike.
fn thin_particles(objects: &mut [Object], budget: usize, skipped: &mut Vec<String>) {
    let expected: Vec<usize> = objects
        .iter()
        .filter_map(|o| match &o.drawing {
            Drawing::Particles(system) => Some(system.expected()),
            Drawing::Image => None,
        })
        .collect();
    let wanted: usize = expected.iter().sum();
    if wanted <= budget {
        return;
    }

    let most = most_for_each(expected, budget);
    for object in objects {
        if let Drawing::Particles(system) = &mut object.drawing
            && system.expected() > most
        {
            // A ribbon needs a pair to have anything between them.
            let least = if system.is_rope() { 2 } else { 1 };
            system.thin_to(most.max(least));
        }
    }
    skipped.push(format!(
        "this scene asks for {wanted} particles at once and is thinned to {budget}"
    ));
}

/// The most any one system may keep for them all to come to `budget`, those
/// under it keeping everything they asked for.
fn most_for_each(mut wanted: Vec<usize>, budget: usize) -> usize {
    wanted.sort_unstable();
    let mut left = budget;
    for (i, &one) in wanted.iter().enumerate() {
        let sharing = wanted.len() - i;
        if one * sharing > left {
            return left / sharing;
        }
        left -= one;
    }
    usize::MAX
}

/// Which texture unit a sampler's name stands for: `g_Texture2` is unit two.
fn texture_slot(name: &str) -> Option<usize> {
    name.strip_prefix("g_Texture")?
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
}

/// Draws an object into the canvas and its effects back out of it.
///
/// The object goes in without its place in the scene, so the picture holds just
/// the object; each pass of each effect then draws what came before it through
/// its own material, into whichever buffer the chain named, and the last pass
/// lands wherever the scene is being drawn, placed as the object always would
/// have been.
#[allow(clippy::too_many_arguments)]
/// Copies what has been drawn so far into `frame`, for a layer to read.
unsafe fn take_frame(
    gl: &glow::Context,
    screen: Option<glow::Framebuffer>,
    frame: &Target,
    output: (u32, u32),
) {
    unsafe {
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, screen);
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(frame.frame));
        let (w, h) = (output.0 as i32, output.1 as i32);
        gl.blit_framebuffer(0, 0, w, h, 0, 0, w, h, glow::COLOR_BUFFER_BIT, glow::NEAREST);
        gl.bind_framebuffer(glow::FRAMEBUFFER, screen);
    }
}

unsafe fn draw_through_effects(
    gl: &glow::Context,
    object: &Object,
    frame: Option<&Target>,
    canvas: &Canvas,
    kept: &BTreeMap<i64, &Target>,
    buffer: glow::Buffer,
    white: &Texture,
    mvp: &Matrix,
    time: f32,
    output: (u32, u32),
    screen: Option<glow::Framebuffer>,
    projection: &Matrix,
    sound: &Spectrum,
) {
    let used = object.on_screen(projection, output, canvas.size);
    // The object fills the same fraction of every buffer, whatever that
    // buffer's size: a quarter-sized buffer holds a quarter-sized object.
    let part = |size: (u32, u32)| {
        (
            (used.0 * size.0).div_ceil(canvas.size.0).max(1),
            (used.1 * size.1).div_ceil(canvas.size.1).max(1),
        )
    };
    // Where the object's texture coordinates have to stop in a given buffer,
    // which is the one thing a pass reading it needs to be told.
    let covered = |target: &Target| {
        let size = target.size();
        let filled = part(size);
        [
            filled.0 as f32 / size.0 as f32,
            filled.1 as f32 / size.1 as f32,
        ]
    };
    let find = |which: &Buffer| match which {
        Buffer::Canvas(index) => Some(&canvas.targets[*index]),
        Buffer::Named(name) => canvas.named.get(name),
        Buffer::Layer(id) => kept.get(id).copied(),
        Buffer::Frame => frame,
        Buffer::Screen => None,
    };

    unsafe {
        let into = |target: &Target| {
            let filled = part(target.size());
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.frame));
            gl.viewport(0, 0, filled.0 as i32, filled.1 as i32);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        };

        into(&canvas.targets[0]);
        // A layer draws the screen under it rather than a picture of its own.
        // The placed kind's shader fills the buffer by itself and wants the
        // real matrix, to know where on the screen it is; the screen kind's
        // wants a quad that is already the whole buffer, and no matrix.
        let handed: Vec<_> = frame
            .filter(|_| object.layer.is_some())
            .map(|f| (0, &f.texture))
            .into_iter()
            .collect();
        let (matrix, reach) = match object.layer {
            None => (&FILL, 0.5),
            Some(Layer::Placed) => (mvp, 0.5),
            Some(Layer::Screen) => (&transform::IDENTITY, 1.0),
        };
        for pass in &object.passes {
            write_quad(gl, buffer, pass.covered, reach);
            // Straight onto an empty picture. Blending here would fold the
            // object's own alpha into its colour, and the effect that comes
            // after would fold it in a second time, which darkens everything
            // that is not fully opaque.
            let mut handed = handed.clone();
            handed.extend(pass.screen_binds(frame));
            draw_pass(
                gl,
                pass,
                object,
                matrix,
                time,
                white,
                &handed,
                sound,
                Blending::Opaque,
            );
        }

        for step in &object.effects {
            // A buffer a step names is one the canvas was built with, since
            // the canvas is built from the same list -- or another layer's
            // picture, missing only if that layer's effects all failed to
            // build after all.
            let Some(sources) = step
                .reads
                .iter()
                .map(|(unit, from)| find(from).map(|target| (*unit, target)))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let Some(read) = sources.iter().find(|(unit, _)| *unit == 0) else {
                continue;
            };
            let read = read.1;

            let Some(pass) = &step.pass else {
                // A copy command, which is how a chain keeps a picture of its
                // own from one frame to the next. The whole buffer goes, since
                // what reads it next frame may want a different corner.
                if let Some(target) = find(&step.into) {
                    let size = read.size();
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(read.frame));
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(target.frame));
                    gl.blit_framebuffer(
                        0,
                        0,
                        size.0 as i32,
                        size.1 as i32,
                        0,
                        0,
                        size.0 as i32,
                        size.1 as i32,
                        glow::COLOR_BUFFER_BIT,
                        glow::NEAREST,
                    );
                }
                continue;
            };
            let bound: Vec<_> = sources
                .iter()
                .map(|(unit, target)| (*unit, &target.texture))
                .collect();
            match find(&step.into) {
                // Back into the scene, placed where the object belongs.
                None => {
                    // Kept first, as a pass in the middle of a chain would be
                    // drawn: the screen bound last is what the next object
                    // without effects of its own draws into.
                    if let Some(target) = &canvas.kept {
                        into(target);
                        write_quad(gl, buffer, covered(read), 1.0);
                        draw_pass(
                            gl,
                            pass,
                            object,
                            &transform::IDENTITY,
                            time,
                            white,
                            &bound,
                            sound,
                            Blending::Opaque,
                        );
                    }
                    gl.bind_framebuffer(glow::FRAMEBUFFER, screen);
                    gl.viewport(0, 0, output.0 as i32, output.1 as i32);
                    // A post-processing layer goes back over the whole
                    // screen, wherever the scene put its origin.
                    let (placed, reach) = match object.layer {
                        Some(Layer::Screen) => (&transform::IDENTITY, 1.0),
                        _ => (mvp, 0.5),
                    };
                    write_quad(gl, buffer, covered(read), reach);
                    // A puppet bends the finished picture, not the first pass:
                    // bent earlier, the masks of the effects stayed behind and
                    // the buffer's edge cut the jaw off in a straight line.
                    draw_own_pass(
                        gl,
                        pass,
                        object,
                        placed,
                        time,
                        white,
                        &bound,
                        sound,
                        // A layer asked to be added to the scene still is
                        // once its effects have been through it: the effect's
                        // own material speaks only for its chain. Drawn with
                        // that, a glow on a black picture came out as a black
                        // rectangle with a glow on it.
                        match object.passes.first().map(|p| p.blending) {
                            Some(Blending::Additive) => Blending::Additive,
                            _ => pass.blending,
                        },
                        covered(read),
                    );
                }
                // A pass in the middle of a chain covers its whole buffer and
                // most of them ignore the matrix entirely, so the quad is
                // written where it already belongs and left there.
                Some(target) => {
                    into(target);
                    write_quad(gl, buffer, covered(read), 1.0);
                    draw_pass(
                        gl,
                        pass,
                        object,
                        &transform::IDENTITY,
                        time,
                        white,
                        &bound,
                        sound,
                        Blending::Opaque,
                    );
                }
            }
        }
    }
}

/// Builds every pass of every effect hung on an object.
///
/// An effect is a small chain: each pass draws the picture so far through a
/// material of its own, into a buffer the effect names, and the last pass lands
/// wherever the object was going to be drawn. The two pictures the object takes
/// turns in carry the result from one effect to the next, so `previous` -- what
/// every pass calls the picture it was handed -- is whichever of the pair holds
/// what came in.
///
/// An effect that will not build is left out and the rest still run. It used
/// to take the whole object's effects with it, on the theory that half a chain
/// looks worse than none -- and Arkham Knight stood stock still, because one
/// blur out of five wanted a buffer the renderer does not make, and took the
/// sway and the electricity down with it.
fn build_effects(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    object: &wpe_format::scene::Object,
    carriers: &BTreeSet<i64>,
    buffers: &mut BTreeMap<String, u32>,
    skipped: &mut Vec<String>,
) -> Vec<Step> {
    // Built against two stand-ins -- the first of the pair for what comes in,
    // the scene for where it goes -- because which buffers those really are
    // depends on how many of the effects survive.
    let (incoming, outgoing) = (Buffer::Canvas(0), Buffer::Screen);
    let mut built: Vec<Vec<Step>> = object
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.visible.value)
        .filter_map(|(index, effect)| {
            build_effect(gl, resolve, effect, object.id, carriers, &incoming, &outgoing, buffers)
                .map(|mut steps| {
                    for step in &mut steps {
                        step.scene_pass.0 = index;
                    }
                    steps
                })
                .map_err(|e| skipped.push(format!("{}: effect {}: {e}", object.name, effect_name(effect))))
                .ok()
        })
        .collect();
    match object.color_blend_mode.value {
        // Nothing to mix, but another layer reads this one: it goes through
        // the plain pass, which is what gives it a picture to keep.
        0 if built.is_empty() && carriers.contains(&object.id) => {
            match build_blend(gl, resolve, 0, object.effects.len()) {
                Ok(steps) => built.push(steps),
                Err(e) => skipped.push(format!("{}: to be read: {e}", object.name)),
            }
        }
        0 => {}
        mode => match build_blend(gl, resolve, mode, object.effects.len()) {
            Ok(steps) => built.push(steps),
            Err(e) => skipped.push(format!("{}: blend mode {mode}: {e}", object.name)),
        },
    }

    let count = built.len();
    let mut steps = Vec::new();
    for (index, effect) in built.into_iter().enumerate() {
        // Which of the pair this effect reads, and where it leaves its result:
        // the other one, or the scene itself if nothing follows it.
        let input = Buffer::Canvas(index % 2);
        let output = match index + 1 == count {
            true => Buffer::Screen,
            false => Buffer::Canvas((index + 1) % 2),
        };
        let place = |buffer: Buffer| match buffer {
            Buffer::Canvas(_) => input.clone(),
            Buffer::Screen => output.clone(),
            named => named,
        };
        for mut step in effect {
            step.into = place(step.into);
            for (_, from) in &mut step.reads {
                *from = place(from.clone());
            }
            steps.push(step);
        }
    }
    // A buffer an effect binds without ever declaring it still has to exist,
    // or the pass reading it would read nothing at all.
    for step in &steps {
        let named = std::iter::once(&step.into).chain(step.reads.iter().map(|(_, from)| from));
        for buffer in named {
            if let Buffer::Named(name) = buffer {
                buffers
                    .entry(name.clone())
                    .or_insert_with(|| divisor_of(name));
            }
        }
    }
    steps
}

/// Builds the passes of one effect, reading from `input` and ending in
/// `output`, or says why it cannot be built.
fn build_effect(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    effect: &wpe_format::scene::EffectRef,
    own: i64,
    carriers: &BTreeSet<i64>,
    input: &Buffer,
    output: &Buffer,
    buffers: &mut BTreeMap<String, u32>,
) -> Result<Vec<Step>, String> {
    let parsed = resolve
        .get(&effect.file)
        .and_then(|bytes| wpe_format::Effect::parse(&bytes).ok())
        .ok_or("could not be read")?;
    for fbo in &parsed.fbos {
        let divisor = match fbo.scale >= 1.0 {
            true => fbo.scale.round() as u32,
            // One effect in the library states no scale at all. The names
            // are the engine's own, and they say what they mean.
            false => divisor_of(&fbo.name),
        };
        let name = match fbo.unique {
            true => format!("{} of {}", fbo.name, effect.file),
            false => fbo.name.clone(),
        };
        buffers.insert(name, divisor);
    }
    // Motion blur keeps last frame in `_rt_FullCompoBuffer1`, says so with `unique`,
    // and a blur ahead of it on the same object scribbles on a buffer of that
    // name every frame. Shared, Snow Lights' deer mixed with nothing and came
    // out pale and see-through.
    let own_name = |name: &str| match parsed.fbos.iter().any(|fbo| fbo.unique && fbo.name == name) {
        true => format!("{name} of {}", effect.file),
        false => name.to_owned(),
    };
    let mut steps = Vec::new();
    for (at, pass) in parsed.passes.iter().enumerate() {
        let into = match pass.target.is_empty() {
            true => output.clone(),
            false => Buffer::Named(own_name(&pass.target)),
        };
        if pass.command == "copy" {
            steps.push(Step {
                pass: None,
                into,
                reads: vec![(0, which_buffer(&own_name(&pass.source), input))],
                scene_pass: (0, at),
            });
            continue;
        }
        if !pass.command.is_empty() {
            return Err(format!("asks for {}, which is not done", pass.command));
        }
        let mut reads: Vec<_> = pass
            .bind
            .iter()
            .map(|bind| (bind.index as usize, which_buffer(&own_name(&bind.name), input)))
            .collect();
        // A pass that binds nothing to its first unit still draws what came
        // in: that is what the engine had left bound there.
        if !reads.iter().any(|(unit, _)| *unit == 0) {
            reads.push((0, input.clone()));
        }
        // The scene's overrides line up with the effect's passes in order,
        // commands counted among them, and may stop short of the last.
        //
        // A slot the effect binds is the effect's, whatever the scene writes
        // there. Arkham Knight's blur names `_rt_imageLayerComposite_12_a` in
        // the slot its last pass binds `previous` to, and its godrays `_12_b`
        // in theirs -- layer 12's own pair, the very one the bind already
        // hands over -- and turning the effect down over a name for something
        // it was getting anyway lost the blur and the rays.
        //
        // Another layer's buffer is that layer's finished picture, read like
        // any other buffer the pass binds.
        let mut over = effect.passes.get(at).cloned();
        if let Some(over) = &mut over {
            for bind in &pass.bind {
                if let Some(slot) = over.textures.get_mut(bind.index as usize) {
                    *slot = None;
                }
            }
            for (slot, name) in over.textures.iter_mut().enumerate() {
                let Some(id) = name.as_deref().and_then(other_layer).filter(|id| *id != own) else {
                    continue;
                };
                if !carriers.contains(&id) {
                    return Err(format!("reads layer {id}, which has no picture of its own"));
                }
                reads.push((slot, Buffer::Layer(id)));
                *name = None;
            }
        }
        let built = resolve
            .get(&pass.material)
            .ok_or_else(|| format!("{} is missing", pass.material))
            .and_then(|bytes| Material::parse(&bytes).map_err(|e| e.to_string()))
            .and_then(|material| {
                let first = material.passes.first().ok_or("it has no passes")?;
                build_pass(gl, resolve, first, Kind::Effect, over.as_ref())
            })?;
        reads.extend(built.screen_slots.iter().map(|slot| (*slot, Buffer::Frame)));
        steps.push(Step {
            pass: Some(built),
            into,
            reads,
            scene_pass: (0, at),
        });
    }
    Ok(steps)
}

/// Gives every pass whose first texture is a video the texture its player
/// plays into, opening the player the first time a video is named.
fn play_videos(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    passes: &mut [Pass],
    videos: &mut BTreeMap<String, Video>,
    cache: &std::path::Path,
) -> Result<(), String> {
    for pass in passes.iter_mut() {
        let Some(name) = pass.video.clone() else { continue };
        let header = resolve
            .get(&format!("materials/{name}.tex"))
            .and_then(|bytes| wpe_format::Texture::parse(&bytes).ok())
            .ok_or_else(|| format!("video {name} could not be read"))?;
        if !videos.contains_key(&name) {
            let mp4 = header
                .base()
                .ok_or("the video has no surface")?
                .decompress()
                .map_err(|e| e.to_string())?;
            videos.insert(name.clone(), Video::open(gl, &mp4, cache, &name)?);
        }
        // As big as the scene's author saw it, which is what an autosized
        // layer takes its size from -- not the size it is decoded at.
        let texture = videos[&name].texture(header.image_size);
        match pass.textures.first_mut() {
            Some(slot) => *slot = Some(texture),
            None => pass.textures.push(Some(texture)),
        }
    }
    Ok(())
}

/// Where a scene's videos are written out: beside the music the program
/// unpacks into its own cache, `~/.cache/WallFlux/wallflux/scenes/<id>`,
/// which is what Qt calls that program's cache location.
fn video_cache(dir: &std::path::Path) -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    let id = dir.file_name().map(|n| n.to_owned()).unwrap_or_default();
    base.join("WallFlux/wallflux/scenes").join(id)
}

/// The glow a scene can turn on in its general settings, drawn the way
/// Wallpaper Engine draws it with four shaders of its own: what is brighter
/// than the threshold taken out at a quarter of the size, blurred down that
/// and then across at an eighth, and added back over the finished scene.
/// Almamu's renderer does the same with effect files of its own; these are
/// the engine's.
struct Bloom {
    /// `downsample_quarter_bloom`, `blur_h_bloom`, `downsample_eighth_blur_v`
    /// and `combine`, in that order.
    passes: Vec<Pass>,
    /// The quarter, the quarter blurred, and the eighth, made with the output.
    targets: Option<[Target; 3]>,
    /// The passes are drawn for an object, and this one is nobody's.
    carrier: Object,
}

fn build_bloom(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    general: &wpe_format::scene::General,
    scene: (u32, u32),
) -> Result<Bloom, String> {
    // A texel of the scene, which is how far apart the blurs sample: they
    // reach a fixed part of the picture whatever size it is drawn at.
    let texel = format!("{} {}", 1.0 / scene.0.max(1) as f32, 1.0 / scene.1.max(1) as f32);
    let mut passes = Vec::new();
    for shader in ["downsample_quarter_bloom", "blur_h_bloom", "downsample_eighth_blur_v", "combine"] {
        let material = wpe_format::material::Pass {
            shader: shader.into(),
            blending: "normal".into(),
            ..Default::default()
        };
        let mut pass = build_pass(gl, resolve, &material, Kind::Effect, None)?;
        pass.constants.insert("g_TexelSize".into(), texel.clone().into());
        pass.constants.insert("g_BloomStrength".into(), general.bloomstrength.as_ref().map_or(2.0, |b| b.value).into());
        pass.constants.insert("g_BloomThreshold".into(), general.bloomthreshold.as_ref().map_or(0.65, |b| b.value).into());
        passes.push(pass);
    }
    Ok(Bloom {
        passes,
        targets: None,
        carrier: Object {
            name: "bloom".into(),
            origin: [0.0; 3],
            size: [1.0, 1.0],
            scale: [1.0; 3],
            angles: [0.0; 3],
            alpha: 1.0,
            brightness: 1.0,
            color: [1.0; 3],
            passes: Vec::new(),
            drawing: Drawing::Image,
            trail: None,
            effects: Vec::new(),
            id: -1,
            kept: false,
            answers: None,
            layer: None,
            depth: [0.0; 2],
            anchor: [0.0; 2],
            from_scene: false,
            shown: false,
            text: None,
            view: ObjectView::default(),
            puppet: None,
        },
    })
}

impl Bloom {
    /// Draws the glow over everything on `screen`, with `frame` to copy it
    /// into first.
    #[allow(clippy::too_many_arguments)]
    unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        frame: &Target,
        buffer: glow::Buffer,
        screen: Option<glow::Framebuffer>,
        output: (u32, u32),
        time: f32,
        white: &Texture,
        sound: &Spectrum,
    ) {
        let part = |by: u32| ((output.0 / by).max(1), (output.1 / by).max(1));
        unsafe {
            if self.targets.as_ref().is_none_or(|t| t[0].size() != part(4) || t[2].size() != part(8)) {
                for old in self.targets.take().iter().flatten() {
                    old.free(gl);
                }
                self.targets = (|| Some([
                    make_target(gl, part(4)).ok()?,
                    make_target(gl, part(4)).ok()?,
                    make_target(gl, part(8)).ok()?,
                ]))();
            }
            let Some([quarter, blurred, eighth]) = &self.targets else {
                return;
            };
            take_frame(gl, screen, frame, output);
            write_quad(gl, buffer, [1.0, 1.0], 1.0);
            let steps: [(Option<&Target>, Vec<(usize, &Texture)>); 4] = [
                (Some(quarter), vec![(0, &frame.texture)]),
                (Some(blurred), vec![(0, &quarter.texture)]),
                (Some(eighth), vec![(0, &blurred.texture)]),
                (None, vec![(0, &frame.texture), (1, &eighth.texture)]),
            ];
            for (pass, (into, reads)) in self.passes.iter().zip(steps) {
                let size = into.map_or(output, Target::size);
                gl.bind_framebuffer(glow::FRAMEBUFFER, into.map_or(screen, |t| Some(t.frame)));
                gl.viewport(0, 0, size.0 as i32, size.1 as i32);
                draw_pass(gl, pass, &self.carrier, &transform::IDENTITY, time, white, &reads, sound, Blending::Opaque);
            }
        }
    }
}

/// Where the middle of an object goes, given where its origin is. The origin
/// is what it turns about, so the way from one to the other turns with it:
/// Dark Core's hands are `bottom`, standing on the point they turn on. Moved
/// straight down, they turned about their own middles and stood at twelve
/// whatever the time.
fn middle_of(origin: [f32; 3], anchor: [f32; 2], size: [f32; 2], scale: [f32; 3], turn: f32) -> [f32; 3] {
    let (x, y) = (anchor[0] * size[0] * scale[0], anchor[1] * size[1] * scale[1]);
    let (sin, cos) = turn.sin_cos();
    [origin[0] - (x * cos - y * sin), origin[1] - (x * sin + y * cos), origin[2]]
}

/// Where an `alignment` puts the origin on an image, from its middle and in
/// its own sizes, y up. Almamu's renderer reads the words the same way.
fn anchor_of(alignment: &str) -> [f32; 2] {
    let x = match () {
        _ if alignment.contains("left") => -0.5,
        _ if alignment.contains("right") => 0.5,
        _ => 0.0,
    };
    let y = match () {
        _ if alignment.contains("top") => 0.5,
        _ if alignment.contains("bottom") => -0.5,
        _ => 0.0,
    };
    [x, y]
}

/// The pass a layer's blend mode adds after its effects: the finished layer
/// mixed into the scene under it by `ApplyBlending`, and written over it.
///
/// It is what Wallpaper Engine's own `materials/util/effectpassthrough.json`
/// is for -- `genericimage3` with `BLENDMODE` set, reading the screen so far
/// at unit four -- and what Almamu's renderer adds in the same place. Drawn
/// as an ordinary layer instead, a "lighten" layer that is mostly black
/// blacked out everything under it: Space Marine II was outlines on a void.
fn build_blend(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    mode: i64,
    after: usize,
) -> Result<Vec<Step>, String> {
    let path = "materials/util/effectpassthrough.json";
    let material = resolve
        .get(path)
        .ok_or_else(|| format!("{path} is missing"))
        .and_then(|bytes| Material::parse(&bytes).map_err(|e| e.to_string()))?;
    let first = material.passes.first().ok_or("it has no passes")?;
    let over = wpe_format::scene::PassOverride {
        combos: match mode {
            0 => BTreeMap::new(),
            mode => BTreeMap::from([("BLENDMODE".into(), mode.into())]),
        },
        ..Default::default()
    };
    let mut pass = build_pass(gl, resolve, first, Kind::Effect, Some(&over))?;
    // The layer's colour and alpha went in with its first pass; the shader
    // would multiply them in a second time.
    pass.constants.insert("g_Color4".into(), "1 1 1 1".into());
    Ok(vec![Step {
        pass: Some(pass),
        into: Buffer::Screen,
        reads: match mode {
            0 => vec![(0, Buffer::Canvas(0))],
            _ => vec![(0, Buffer::Canvas(0)), (4, Buffer::Frame)],
        },
        scene_pass: (after, 0),
    }])
}

/// The layer a `_rt_imageLayerComposite_<id>_a` belongs to. Only `_a`: what
/// `_b` would mean to another layer nothing in the library asks.
fn other_layer(name: &str) -> Option<i64> {
    name.strip_prefix("_rt_imageLayerComposite_")?
        .strip_suffix("_a")?
        .parse()
        .ok()
}

/// What a pass means by the buffer it names. `previous` is the picture the
/// effect was handed; anything else is a buffer in its own right.
fn which_buffer(name: &str, input: &Buffer) -> Buffer {
    match name {
        "previous" => input.clone(),
        other => Buffer::Named(other.to_owned()),
    }
}

/// What to divide the output by for a buffer, read from its name because that
/// is all an effect binding one without declaring it gives to go on.
fn divisor_of(name: &str) -> u32 {
    if name.contains("Quarter") {
        4
    } else if name.contains("Half") {
        2
    } else {
        1
    }
}

/// What to call an effect. Only 33 of the 390 in the library fill in a name,
/// so the file is what identifies one.
fn effect_name(effect: &wpe_format::scene::EffectRef) -> &str {
    let from_file = effect
        .file
        .trim_start_matches("effects/")
        .trim_end_matches("effect.json")
        .trim_end_matches('/');
    match (from_file, effect.name.as_str()) {
        ("", "") => "an unnamed effect",
        ("", named) => named,
        (file, _) => file,
    }
}

fn build_passes(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    material: &Material,
    kind: Kind,
    object: &wpe_format::scene::Object,
    skipped: &mut Vec<String>,
) -> Vec<Pass> {
    let mut passes = Vec::new();
    for pass in &material.passes {
        match build_pass(gl, resolve, pass, kind, None) {
            Ok(built) => passes.push(built),
            Err(e) => skipped.push(format!("{}: {}: {e}", object.name, pass.shader)),
        }
    }
    passes
}

#[allow(clippy::too_many_arguments)]
/// Draws an object into the scene: as its puppet where it has one, and as the
/// quad already written where it does not.
#[allow(clippy::too_many_arguments)]
unsafe fn draw_own_pass(
    gl: &glow::Context,
    pass: &Pass,
    object: &Object,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
    bound: &[(usize, &Texture)],
    sound: &Spectrum,
    blending: Blending,
    covered: [f32; 2],
) {
    unsafe {
        let Some(puppet) = &object.puppet else {
            draw_pass(gl, pass, object, mvp, time, white, bound, sound, blending);
            return;
        };
        set_common(gl, pass, object, mvp, time, white, bound, sound);
        blending.apply(gl);
        let quad = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING) as u32)
            .map(glow::NativeVertexArray);
        gl.bind_vertex_array(Some(puppet.array));
        puppet.upload(gl, covered);
        gl.draw_elements(glow::TRIANGLES, puppet.puppet.triangles.len() as i32, glow::UNSIGNED_SHORT, 0);
        gl.bind_vertex_array(quad);
    }
}

unsafe fn draw_pass(
    gl: &glow::Context,
    pass: &Pass,
    object: &Object,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
    bound: &[(usize, &Texture)],
    sound: &Spectrum,
    blending: Blending,
) {
    unsafe {
        set_common(gl, pass, object, mvp, time, white, bound, sound);
        blending.apply(gl);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}

/// Uploads a system's particles and draws them in one call.
#[allow(clippy::too_many_arguments)]
unsafe fn draw_particles(
    gl: &glow::Context,
    object: &Object,
    system: &particles::System,
    buffers: &Buffers,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
    sound: &Spectrum,
    frame: Option<&Target>,
) {
    let vertices = system.vertices();
    if vertices.is_empty() {
        return;
    }
    let (vao, buffer) = match system.is_rope() {
        true => (buffers.rope, buffers.rope_vertices),
        false => (buffers.vao, buffers.vertices),
    };
    unsafe {
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
        let bytes: &[u8] = std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            std::mem::size_of_val(vertices),
        );
        gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);

        for pass in &object.passes {
            set_common(gl, pass, object, mvp, time, white, &pass.screen_binds(frame), sound);
            pass.blending.apply(gl);
            // A flat wallpaper's particles face the screen, so the frame the
            // shader turns a particle in is the scene's own axes.
            set_vec3(gl, pass.program, "g_OrientationRight", [1.0, 0.0, 0.0]);
            set_vec3(gl, pass.program, "g_OrientationUp", [0.0, 1.0, 0.0]);
            set_vec3(gl, pass.program, "g_OrientationForward", [0.0, 0.0, 1.0]);
            // Which way the screen's own axes lie, which a refraction's offset
            // is worked out along.
            set_vec3(gl, pass.program, "g_ViewRight", [1.0, 0.0, 0.0]);
            set_vec3(gl, pass.program, "g_ViewUp", [0.0, 1.0, 0.0]);
            // A trail and a ribbon both turn to face the viewer, and both
            // work out which way that is from the eye. On a flat wallpaper it
            // is straight down z, so the eye is given in the object's own
            // frame and the matrix between them left alone. Leaving the matrix
            // unset reads as all zeros, which collapses that direction to
            // nothing and takes the geometry with it.
            set_matrix(
                gl,
                pass.program,
                "g_ModelMatrixInverse",
                &transform::IDENTITY,
            );
            set_vec3(gl, pass.program, "g_EyePosition", [0.0, 0.0, -FAR_AWAY]);
            if let Some(trail) = object.trail {
                // How long a trail runs: its speed times the first, held
                // between the other two.
                set_vec4(
                    gl,
                    pass.program,
                    "g_RenderVar0",
                    [trail[0], trail[1], trail[2], 0.0],
                );
            }
            if let Some(sheet) = &pass.sheet {
                set_vec4(
                    gl,
                    pass.program,
                    "g_RenderVar1",
                    [
                        sheet.frame_width,
                        sheet.frame_height,
                        sheet.frames,
                        sheet.ratio,
                    ],
                );
            }
            gl.draw_elements(
                glow::TRIANGLES,
                (system.pieces() * INDICES_PER_PARTICLE) as i32,
                glow::UNSIGNED_INT,
                0,
            );
        }
    }
}

/// The uniforms every pass is given, whichever way it is drawn.
#[allow(clippy::too_many_arguments)]
unsafe fn set_common(
    gl: &glow::Context,
    pass: &Pass,
    object: &Object,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
    bound: &[(usize, &Texture)],
    sound: &Spectrum,
) {
    unsafe {
        gl.use_program(Some(pass.program));

        // A shader that answers to music declares the spectrum itself, behind
        // the same switch that turns its audio response on, so a pass that
        // wants none of this has no such uniform and is passed over.
        set_floats(gl, pass.program, "g_AudioSpectrum16Left", &Spectrum::fewer::<16>(&sound.left));
        set_floats(gl, pass.program, "g_AudioSpectrum16Right", &Spectrum::fewer::<16>(&sound.right));

        let still = pass.film.as_ref().map(|film| (film, film.at(time)));
        // A pass of an effect is handed buffers at units its own material never
        // named, so the units run past the end of what the material listed.
        let units = pass
            .textures
            .len()
            .max(bound.iter().map(|(unit, _)| unit + 1).max().unwrap_or(0));
        for unit in 0..units {
            let slot = pass.textures.get(unit).and_then(|slot| slot.as_ref());
            let handed = bound.iter().find(|(at, _)| *at == unit).map(|(_, t)| *t);
            // An empty slot still has to be given something, or the pass reads
            // whatever texture was last left on that unit -- another object's.
            let texture = match (unit, &still, handed) {
                // An effect is handed the pictures it works on.
                (_, _, Some(handed)) => handed,
                (0, Some((film, still)), _) => &film.pages[still.page],
                _ => slot.unwrap_or(white),
            };
            texture.bind(gl, unit as u32);
            set_int(gl, pass.program, &format!("g_Texture{unit}"), unit as i32);
            // .xy is the texture, .zw the image inside it; shaders divide one
            // by the other to find the part that is not padding. A buffer of
            // ours says the size Wallpaper Engine's would be, for the sake of
            // every effect that reaches a number of texels.
            let as_big = match handed {
                Some(_) => object.view.scene_per_pixel,
                None => 1.0,
            };
            // How many of the scene's pixels one of the picture's stands for, as
            // big as the line above says it is. `blend` divides an offset given
            // in the scene's pixels by this and then by that size: left at
            // nought it threw the texture off to infinity, and counted in the
            // buffer's real pixels it fell short by the same factor the smaller
            // the screen, which took Dark Angels' light off its cathedral.
            if unit == 0 {
                let reduction = match handed {
                    Some(_) => object.size[0] / (texture.image_size.0.max(1) as f32 * as_big),
                    None => 1.0,
                };
                set_float(gl, pass.program, "g_TextureReductionScale", reduction);
            }
            set_vec4(
                gl,
                pass.program,
                &format!("g_Texture{unit}Resolution"),
                [
                    texture.width as f32 * as_big,
                    texture.height as f32 * as_big,
                    texture.image_size.0 as f32 * as_big,
                    texture.image_size.1 as f32 * as_big,
                ],
            );
        }

        // Shaders multiply as `mul(position, matrix)`, a row vector on the
        // left, so the matrix goes up transposed to mean the same thing.
        if let Some(l) = gl.get_uniform_location(pass.program, "g_ModelViewProjectionMatrix") {
            gl.uniform_matrix_4_f32_slice(Some(&l), true, mvp);
        }
        // A lit shader takes its corners into the scene and from there to
        // wherever this pass draws, which for an object with effects is its
        // own buffer: what the two come to has to be `mvp` whichever it is.
        if let Some(l) = gl.get_uniform_location(pass.program, "g_ModelMatrix") {
            let view = &object.view;
            gl.uniform_matrix_4_f32_slice(Some(&l), true, &view.model);
            if let Some(l) = gl.get_uniform_location(pass.program, "g_ViewProjectionMatrix") {
                let back = transform::inverse_flat(&view.model).unwrap_or(transform::IDENTITY);
                gl.uniform_matrix_4_f32_slice(Some(&l), true, &transform::multiply(mvp, &back));
            }
            if let Some(l) = gl.get_uniform_location(pass.program, "g_NormalModelMatrix") {
                gl.uniform_matrix_3_f32_slice(Some(&l), false, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
            }
            let lights = &view.lights;
            set_vec3(gl, pass.program, "g_LightAmbientColor", lights.ambient);
            if let Some(l) = gl.get_uniform_location(pass.program, "g_LightsPosition") {
                gl.uniform_3_f32_slice(Some(&l), lights.positions.as_flattened());
            }
            if let Some(l) = gl.get_uniform_location(pass.program, "g_LightsColorPremultiplied") {
                gl.uniform_4_f32_slice(Some(&l), lights.colors.as_flattened());
            }
        }
        set_vec2(gl, pass.program, "g_PointerPosition", object.view.pointer);
        if let Some(l) = gl.get_uniform_location(pass.program, "g_EffectTextureProjectionMatrixInverse") {
            gl.uniform_matrix_4_f32_slice(Some(&l), true, &object.view.from_clip_to_unit);
        }
        if let Some(l) = gl.get_uniform_location(pass.program, "g_ModelViewProjectionMatrixInverse") {
            gl.uniform_matrix_4_f32_slice(Some(&l), true, &object.view.from_clip);
        }
        set_float(gl, pass.program, "g_Time", time);
        set_float(gl, pass.program, "g_Alpha", object.alpha);
        set_float(gl, pass.program, "g_UserAlpha", object.alpha);
        set_float(gl, pass.program, "g_Brightness", object.brightness);
        set_vec4(
            gl,
            pass.program,
            "g_Color4",
            [
                object.color[0],
                object.color[1],
                object.color[2],
                object.alpha,
            ],
        );
        set_vec3(gl, pass.program, "g_Color", object.color);

        if let Some((_, still)) = still {
            set_vec4(
                gl,
                pass.program,
                "g_Texture0Rotation",
                [still.size[0], 0.0, 0.0, still.size[1]],
            );
            set_vec2(gl, pass.program, "g_Texture0Translation", still.offset);
        }

        for (name, value) in &pass.constants {
            set_json(gl, pass.program, name, value);
        }
    }
}

fn build_pass(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    pass: &wpe_format::material::Pass,
    kind: Kind,
    over: Option<&wpe_format::scene::PassOverride>,
) -> Result<Pass, String> {
    if pass.shader.is_empty() {
        return Err("pass names no shader".into());
    }

    // A pass draws the texture in its first slot, so one that wants a render
    // target there is compositing something this renderer has not produced.
    // Drawing it anyway would put a full-screen rectangle of nothing over the
    // scene. A pass of an effect is the exception: a render target is exactly
    // what it is handed, and the chain it belongs to says which one.
    //
    // The later slots were once let through on the grounds that they only
    // modulate and an unbound one stands in as white. They do not: `blend`
    // blends *towards* the texture in slot one, so a composite buffer nobody
    // made turned a Space Marine's armour white at 57 per cent. Whatever the
    // slot, a render target this renderer cannot produce means the pass cannot
    // be drawn as it was meant, and not drawing it loses an effect rather than
    // painting one over the wallpaper.
    let asked = match over.map(|o| &o.textures) {
        Some(theirs) if !theirs.is_empty() => theirs,
        _ => &pass.textures,
    };
    for (slot, name) in asked.iter().enumerate() {
        let Some(name) = name.as_deref().filter(|n| !n.is_empty()) else {
            continue;
        };
        // The one target this renderer does make for an image: what is
        // already on the screen, for a layer to read.
        let frame = slot == 0 && kind == Kind::Image && name == FULL_FRAME;
        if wpe_format::effect::is_render_target(name) && (slot > 0 || kind != Kind::Effect) && !frame {
            return Err(format!("needs the render target {name}"));
        }
    }

    // Textures are loaded before the shaders are compiled, because whether
    // the first one is a sprite sheet decides how the shaders are built.
    let mut textures = Vec::new();
    let mut sheet = None;
    let mut film = None;
    let mut padded = false;
    let mut covered = [1.0, 1.0];
    let mut video = None;
    // What the scene says about this pass stands over what the material says.
    let named_textures = match over.map(|o| &o.textures) {
        Some(theirs) if !theirs.is_empty() => theirs,
        _ => &pass.textures,
    };
    for (slot, name) in named_textures.iter().enumerate() {
        let named = name.as_deref().filter(|name| !name.is_empty());
        let source = match named {
            Some(name) if !wpe_format::effect::is_render_target(name) => resolve
                .get(&format!("materials/{name}.tex"))
                .and_then(|bytes| wpe_format::Texture::parse(&bytes).ok()),
            _ => None,
        };
        // A pass whose own image is a video is a video wallpaper wearing a
        // scene's clothes, and there is nothing to draw for it yet.
        if slot == 0 {
            match &source {
                // An image's video is played into the slot by the scene,
                // which owns one player for every pass that names it.
                Some(texture) if texture.is_video() && kind == Kind::Image => {
                    video = named.map(str::to_owned);
                }
                Some(texture) if texture.is_video() => {
                    return Err("its texture holds a video".into());
                }
                // The screen so far, which a layer is handed when it is drawn.
                None if named == Some(FULL_FRAME) => {}
                None if named.is_some() => {
                    return Err(format!("texture {} could not be read", named.unwrap()));
                }
                _ => {}
            }
            // How much of the texture is image rather than padding.
            if let Some(texture) = source.as_ref().filter(|t| !t.is_video())
                && let Some(page) = texture.images.first().and_then(|mips| mips.first())
            {
                covered = coverage(texture.image_size, (page.width, page.height));
            }
            match kind {
                Kind::Particles(_) => {
                    sheet = source
                        .as_ref()
                        .and_then(|source| Sheet::of(source, named, resolve));
                    padded = source.as_ref().is_some_and(Sheet::padded);
                }
                Kind::Image => film = source.as_ref().and_then(|source| Film::of(gl, source)),
                // An effect's first texture is the object's own picture,
                // handed over at drawing time rather than loaded from a file.
                Kind::Effect => {}
            }
        }
        textures.push(
            source
                .as_ref()
                .filter(|texture| !texture.is_video())
                .and_then(|source| {
                    Texture::upload_page(gl, source, 0, source.repeats()).ok()
                }),
        );
    }

    // Effect materials name no textures at all -- 45 of the 47 in the library
    // leave the list out -- but their shaders still sample a noise and a mask
    // beside the picture. Leaving those unbound reads whatever was last on the
    // unit, so they get the white stand-in.
    if kind == Kind::Effect {
        while textures.len() < 3 {
            textures.push(None);
        }
    }

    let mut combos: BTreeMap<String, String> = pass
        .defines()
        .map(|(k, v)| (k, v.to_string().trim_matches('"').to_owned()))
        .collect();
    // An effect read only from its own file looks nothing like the one the
    // wallpaper set up: this is where its options are actually turned on.
    if let Some(over) = over {
        for (name, value) in &over.combos {
            combos.insert(
                name.to_ascii_uppercase(),
                value.to_string().trim_matches('"').to_owned(),
            );
        }
    }
    // Reflection samples a mip-mapped copy of the frame that is never made, and
    // a particle's shader would want lights this renderer gives images alone.
    if kind != Kind::Effect {
        combos.insert("REFLECTION".into(), "0".into());
    }
    if matches!(kind, Kind::Particles(_)) {
        combos.insert("LIGHTING".into(), "0".into());
    }
    // What was actually uploaded wins over anything the material says, since
    // this is the texture the shader will be sampling.
    for (slot, texture) in textures.iter().enumerate() {
        if let Some(texture) = texture {
            combos.insert(format!("TEX{slot}FORMAT"), texture.format.to_string());
        }
    }
    if let Kind::Particles(form) = kind {
        let on = |yes: bool| if yes { "1" } else { "0" }.to_owned();
        // No geometry shader, so the vertex shader builds the corner itself.
        combos.insert("GS_ENABLED".into(), on(false));
        // The wide vertex layout, which carries velocity and lifetime. The
        // sprite sheet path reads the lifetime, so it is always sent.
        combos.insert("THICKFORMAT".into(), on(true));
        combos.insert("TRAILRENDERER".into(), on(form == Form::Trail));
        // A ribbon's own shader has no sprite sheet path at all, and its
        // texture coordinate is a plain pair.
        if form == Form::Rope {
            sheet = None;
        }
        // Whether a system animates is the texture's business, not the
        // particle file's: most sheets in the library are used by a system
        // that names no animation mode at all, and walking the sheet is what
        // they expect. The mode only picks how it is walked.
        let sheeted = sheet.is_some();
        combos.insert("SPRITESHEET".into(), on(sheeted));
        combos.insert("SPRITESHEETBLENDNPOT".into(), on(sheeted && padded));
    }
    if film.is_some() {
        // The image shaders put their texture matrix behind this, and without
        // it they stretch the whole sheet across the object. A frame states
        // its own rectangle, so the quad is read whole and the padding of the
        // page is already accounted for.
        combos.insert("SPRITESHEET".into(), "1".into());
        covered = [1.0, 1.0];
    }

    // Every particle material in the library names `genericparticle`, whatever
    // its system is drawn as, so the shader is chosen by the renderer and not
    // by the material. A ribbon's geometry has nothing in common with a quad's
    // and reads an entirely different set of attributes. A material that names
    // something else is left alone, since then it is not the stock shader
    // being stood in for.
    let shader = match (kind, pass.shader.as_str()) {
        (Kind::Particles(Form::Rope), "genericparticle") => "genericropeparticle",
        _ => pass.shader.as_str(),
    };

    // A sampler may declare a switch that turns on when something is bound to
    // it -- a mask, a normal map -- and the shader keeps the code that reads
    // it behind that switch. Without this an effect that was given a mask
    // applies everywhere instead of where the mask allows. The annotations are
    // read from the files as they are, since the switch has to be settled
    // before the shaders can be built with it.
    let declared: Vec<_> = [(Stage::Vertex, "vert"), (Stage::Fragment, "frag")]
        .iter()
        .filter_map(|(_, ext)| resolve.shader(shader, ext))
        .flat_map(|source| uniforms(&String::from_utf8_lossy(&source)))
        .filter(|uniform| uniform.sampler)
        .collect();
    for uniform in &declared {
        let Some(slot) = texture_slot(&uniform.name) else {
            continue;
        };
        // Only where the wallpaper itself bound something. A sampler's own
        // default is the blank that stands for nothing being bound.
        let bound = named_textures
            .get(slot)
            .and_then(|name| name.as_deref())
            .is_some_and(|name| !name.is_empty());
        if bound && let Some(switch) = &uniform.combo {
            combos.insert(switch.to_ascii_uppercase(), "1".into());
        }
    }

    // An option declared in only one of the two stages still has to be defined
    // in both, or they disagree with each other: a blur whose vertex shader was
    // told the kernel size and whose fragment shader was not writes a different
    // number of texture coordinates than it reads, and the two will not link.
    // What the material or the scene said always wins over a shader's default.
    for (_, ext) in [(Stage::Vertex, "vert"), (Stage::Fragment, "frag")] {
        let Some(source) = resolve.shader(shader, ext) else {
            continue;
        };
        for (name, value) in combo_defaults(&String::from_utf8_lossy(&source)) {
            combos.entry(name).or_insert(value);
        }
    }

    let mut stages = Vec::new();
    for (stage, ext) in [(Stage::Vertex, "vert"), (Stage::Fragment, "frag")] {
        let source = resolve
            .shader(shader, ext)
            .ok_or_else(|| format!("no {ext} shader"))?;
        let (full, missing) = preprocess(&String::from_utf8_lossy(&source), stage, &combos, |i| {
            resolve
                .get(&format!("shaders/{i}"))
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        });
        if let Some(first) = missing.first() {
            return Err(first.to_string());
        }
        stages.push((stage, full));
    }
    // The two stages have to agree on how wide a varying is before they can be
    // linked, and a wallpaper's own shaders do not always say the same thing.
    if let [(_, vertex), (_, fragment)] = stages.as_mut_slice() {
        let fragment = fragment.clone();
        wpe_format::shader::widen_varyings(vertex, &fragment);
    }
    // A workshop shader may lean on HLSL shortening a wider value to fit, which
    // GLSL refuses. Those are written out and tried again, only for a shader
    // that needs it, so one that compiles stays exactly as it was.
    let program = match unsafe { link(gl, &stages) } {
        Ok(program) => program,
        Err(why) => {
            for (_, source) in &mut stages {
                *source = wpe_format::shader::spell_out_conversions(source);
            }
            unsafe { link(gl, &stages) }.map_err(|_| why)?
        }
    };
    let hears = ["g_AudioSpectrum16Left", "g_AudioSpectrum16Right"].iter().any(|name| unsafe {
        gl.get_uniform_location(program, name).is_some()
            || gl.get_uniform_location(program, &format!("{name}[0]")).is_some()
    });

    // A uniform nobody sets reads as zero, so the defaults a shader declares
    // for itself go in first and the material's own values over them.
    let mut constants = BTreeMap::new();
    let mut named_by = BTreeMap::new();
    let mut wanted_textures = Vec::new();
    for (_, source) in &stages {
        for uniform in uniforms(source) {
            if uniform.sampler {
                // A sampler's default names a texture to load rather than a
                // value to set, and a shader given none of it draws with
                // whatever stands in -- a cloud layer of flat white.
                if let Some(path) = uniform.default.as_ref().and_then(|d| d.as_str())
                    && let Some(slot) = texture_slot(&uniform.name)
                {
                    wanted_textures.push((slot, path.to_owned()));
                }
                continue;
            }
            if let Some(default) = uniform.default {
                constants.insert(uniform.name.clone(), default);
            }
            if let Some(label) = uniform.label {
                named_by.insert(label, uniform.name);
            }
        }
    }

    // Only where the material named nothing: what it states always wins.
    let mut screen_slots = Vec::new();
    for (slot, path) in wanted_textures {
        if path == FULL_FRAME
            && textures.get(slot).is_none_or(Option::is_none)
            && unsafe { gl.get_uniform_location(program, &format!("g_Texture{slot}")) }.is_some()
        {
            screen_slots.push(slot);
        }
        if wpe_format::effect::is_render_target(&path) || path.starts_with("_alias_") {
            continue;
        }
        while textures.len() <= slot {
            textures.push(None);
        }
        if textures[slot].is_some() {
            continue;
        }
        textures[slot] = resolve
            .get(&format!("materials/{path}.tex"))
            .and_then(|bytes| wpe_format::Texture::parse(&bytes).ok())
            .filter(|texture| !texture.is_video())
            .and_then(|source| {
                Texture::upload_page(gl, &source, 0, source.repeats()).ok()
            });
    }
    let stated = pass
        .constantshadervalues
        .iter()
        .map(|(k, v)| (k, v.value.clone()))
        .chain(over.into_iter().flat_map(|o| {
            o.constantshadervalues
                .iter()
                .map(|(k, v)| (k, v.value.clone()))
        }));
    for (key, value) in stated {
        // Materials name a constant by the label the shader gave it, so the
        // label is translated back before it can reach a uniform at all.
        let name = named_by.get(key).unwrap_or(key);
        constants.insert(name.clone(), value);
    }

    Ok(Pass {
        program,
        textures,
        blending: Blending::parse(&pass.blending),
        constants,
        sheet,
        film,
        covered,
        hears,
        labels: named_by,
        video,
        screen_slots,
    })
}

impl Sheet {
    /// The layout of a texture's sprite sheet, if it has one this renderer can
    /// use.
    ///
    /// The layout comes from the texture's own sprite block where that states
    /// a frame size, and otherwise from the `.tex-json` beside it. A texture
    /// that says neither is drawn whole.
    fn of(
        texture: &wpe_format::Texture,
        name: Option<&str>,
        resolve: &Resolver<'_>,
    ) -> Option<Self> {
        let own = texture
            .sprite
            .as_ref()
            .and_then(|sprite| Some((sprite.frame_size?, sprite.frames.len() as u32)));
        // A built-in texture whose sprite block is the older version states
        // its layout in the sidecar instead, and some state it only there.
        let (frame_size, frames) = own.or_else(|| {
            let sheet = resolve
                .get(&format!("materials/{}.tex-json", name?))
                .and_then(|bytes| wpe_format::tex::Sidecar::parse(&bytes).ok())?
                .sheet()?;
            Some(((sheet.width, sheet.height), sheet.frames))
        })?;

        let (frame_w, frame_h) = frame_size;
        let (texture_w, texture_h) = texture.texture_size;
        if frame_w == 0 || frame_h == 0 || texture_w == 0 || texture_h == 0 || frames == 0 {
            return None;
        }
        Some(Self {
            // Against the whole texture, padding included, because that is the
            // space the shader steps a frame through.
            frame_width: frame_w as f32 / texture_w as f32,
            frame_height: frame_h as f32 / texture_h as f32,
            frames: frames as f32,
            ratio: frame_h as f32 / frame_w as f32,
        })
    }

    /// Whether the sheet sits in a texture wider than its image.
    ///
    /// A sheet one frame wide in a texture padded to a power of two is the
    /// usual case in the library, and stepping it as if the frames filled the
    /// texture walks straight into the padding. The shaders have a separate
    /// path for it.
    fn padded(texture: &wpe_format::Texture) -> bool {
        texture.image_size != texture.texture_size
    }
}

unsafe fn link(gl: &glow::Context, stages: &[(Stage, String)]) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (stage, source) in stages {
            let kind = match stage {
                Stage::Vertex => glow::VERTEX_SHADER,
                Stage::Fragment => glow::FRAGMENT_SHADER,
            };
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                gl.delete_program(program);
                return Err(first_line(&log));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        // The attribute names are fixed by the format, so they are bound by
        // name before linking rather than looked up after.
        gl.bind_attrib_location(program, 0, "a_Position");
        gl.bind_attrib_location(program, 1, "a_TexCoord");
        // The particle layout. Binding a name a program does not declare is
        // ignored, so images are unaffected by these.
        gl.bind_attrib_location(program, 2, "a_TexCoordVec4");
        gl.bind_attrib_location(program, 3, "a_Color");
        gl.bind_attrib_location(program, 4, "a_TexCoordC2");
        gl.bind_attrib_location(program, 5, "a_TexCoordVec4C1");
        // The ribbon layout, whose vertices carry a whole segment.
        gl.bind_attrib_location(program, 6, "a_PositionVec4");
        gl.bind_attrib_location(program, 7, "a_TexCoordVec4C2");
        gl.bind_attrib_location(program, 8, "a_TexCoordVec4C3");
        gl.bind_attrib_location(program, 9, "a_TexCoordC4");
        gl.link_program(program);
        let ok = gl.get_program_link_status(program);
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        if !ok {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(first_line(&log));
        }
        Ok(program)
    }
}

fn first_line(log: &str) -> String {
    log.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("unknown error")
        .to_owned()
}

/// How much of a stored texture its image actually fills.
///
/// Textures are often stored at the next power of two with the image in a
/// corner and the rest left empty, and nothing in the shaders corrects for it:
/// the texture coordinates a pass is given have to stop where the image does.
fn coverage(image: (u32, u32), stored: (u32, u32)) -> [f32; 2] {
    if stored.0 == 0 || stored.1 == 0 || image.0 == 0 || image.1 == 0 {
        return [1.0, 1.0];
    }
    [
        (image.0 as f32 / stored.0 as f32).min(1.0),
        (image.1 as f32 / stored.1 as f32).min(1.0),
    ]
}

/// Writes the quad for a pass, reading `covered` of its texture.
///
/// Texture coordinates run from the top left, which is the corner an image
/// starts at and the corner it keeps when the rest of the texture is padding.
///
/// `reach` is how far the corners sit from the middle: half a unit for a quad
/// that a matrix is going to place, and a whole one for a pass that fills what
/// it is drawn into without a matrix at all. The blur and god ray shaders are
/// the second kind -- they use the vertex position exactly as it is given.
unsafe fn write_quad(gl: &glow::Context, buffer: glow::Buffer, covered: [f32; 2], reach: f32) {
    let (u, v) = (covered[0], covered[1]);
    let r = reach;
    #[rustfmt::skip]
    let vertices: [f32; 20] = [
        -r, -r, 0.0,   0.0, 0.0,
         r, -r, 0.0,     u, 0.0,
        -r,  r, 0.0,   0.0,   v,
         r,  r, 0.0,     u,   v,
    ];
    unsafe {
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
        let bytes: &[u8] = std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            std::mem::size_of_val(&vertices),
        );
        gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);
    }
}

/// A unit quad centred on the origin, with texture coordinates from the top
/// left, which is the corner an image starts at.
unsafe fn make_quad(gl: &glow::Context) -> Result<(glow::VertexArray, glow::Buffer), String> {
    #[rustfmt::skip]
    const VERTICES: [f32; 20] = [
        -0.5, -0.5, 0.0,   0.0, 0.0,
         0.5, -0.5, 0.0,   1.0, 0.0,
        -0.5,  0.5, 0.0,   0.0, 1.0,
         0.5,  0.5, 0.0,   1.0, 1.0,
    ];
    unsafe {
        let vao = gl.create_vertex_array()?;
        let vbo = gl.create_buffer()?;
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let bytes: &[u8] = std::slice::from_raw_parts(
            VERTICES.as_ptr() as *const u8,
            std::mem::size_of_val(&VERTICES),
        );
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
        let stride = 5 * size_of::<f32>() as i32;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            2,
            glow::FLOAT,
            false,
            stride,
            3 * size_of::<f32>() as i32,
        );
        Ok((vao, vbo))
    }
}

/// How long a slice of a scene one simulation step covers.
const STEP: f32 = 1.0 / 60.0;

/// The most steps one frame will simulate before it gives up and lets the
/// scene run slow. Four is a fifteenth of a second, which no light scene ever
/// reaches and which keeps a heavy one moving instead of seizing.
const STEPS_PER_FRAME: usize = 4;

/// Far enough that the direction to it is the z axis for every particle in a
/// scene, whose own coordinates run to a few thousand.
const FAR_AWAY: f32 = 1.0e6;

/// Two triangles per particle.
const INDICES_PER_PARTICLE: usize = 6;

/// The buffers a particle system is drawn through.
///
/// The vertex buffer is written every frame and the index buffer never, so the
/// indices are built once for the largest system in the scene and the shorter
/// ones simply use less of them.
unsafe fn make_particle_buffers(gl: &glow::Context, capacity: usize) -> Result<Buffers, String> {
    let mut indices: Vec<u32> = Vec::with_capacity(capacity * INDICES_PER_PARTICLE);
    for particle in 0..capacity as u32 {
        let corner = particle * 4;
        indices.extend_from_slice(&[
            corner,
            corner + 1,
            corner + 2,
            corner + 2,
            corner + 1,
            corner + 3,
        ]);
    }

    unsafe {
        let vao = gl.create_vertex_array()?;
        let vertices = gl.create_buffer()?;
        let index_buffer = gl.create_buffer()?;
        gl.bind_vertex_array(Some(vao));

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
        let size = capacity * 4 * FLOATS_PER_VERTEX * size_of::<f32>();
        gl.buffer_data_size(glow::ARRAY_BUFFER, size as i32, glow::DYNAMIC_DRAW);

        // Position, then corner with rotation and size, then colour, then the
        // other two rotation axes, then velocity with lifetime -- the layout
        // `particles` writes and the particle shaders declare.
        let stride = (FLOATS_PER_VERTEX * size_of::<f32>()) as i32;
        for (location, components, offset) in [
            (0u32, 3i32, 0usize),
            (2, 4, 3),
            (3, 4, 7),
            (4, 2, 11),
            (5, 4, 13),
        ] {
            gl.enable_vertex_attrib_array(location);
            gl.vertex_attrib_pointer_f32(
                location,
                components,
                glow::FLOAT,
                false,
                stride,
                (offset * size_of::<f32>()) as i32,
            );
        }

        // The ribbon shares the indices: both draw four vertices a piece.
        let rope = gl.create_vertex_array()?;
        let rope_vertices = gl.create_buffer()?;
        gl.bind_vertex_array(Some(rope));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(rope_vertices));
        let rope_size = capacity * 4 * FLOATS_PER_ROPE_VERTEX * size_of::<f32>();
        gl.buffer_data_size(glow::ARRAY_BUFFER, rope_size as i32, glow::DYNAMIC_DRAW);
        // The start with its size, the end with the rope's length, a control
        // point with the segment's place, a second control point with the
        // end's size, the end's colour, the corner, and the start's colour --
        // the layout `particles` writes for a ribbon.
        let rope_stride = (FLOATS_PER_ROPE_VERTEX * size_of::<f32>()) as i32;
        for (location, components, offset) in [
            (6u32, 4i32, 0usize),
            (2, 4, 4),
            (5, 4, 8),
            (7, 4, 12),
            (8, 4, 16),
            (9, 2, 20),
            (3, 4, 22),
        ] {
            gl.enable_vertex_attrib_array(location);
            gl.vertex_attrib_pointer_f32(
                location,
                components,
                glow::FLOAT,
                false,
                rope_stride,
                (offset * size_of::<f32>()) as i32,
            );
        }
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        let bytes: &[u8] = std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            std::mem::size_of_val(indices.as_slice()),
        );
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        let bytes: &[u8] = std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            std::mem::size_of_val(indices.as_slice()),
        );
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

        Ok(Buffers {
            vao,
            vertices,
            rope,
            rope_vertices,
            indices: index_buffer,
        })
    }
}

fn set_int(gl: &glow::Context, p: glow::Program, name: &str, v: i32) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            gl.uniform_1_i32(Some(&l), v);
        }
    }
}

fn set_float(gl: &glow::Context, p: glow::Program, name: &str, v: f32) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            gl.uniform_1_f32(Some(&l), v);
        }
    }
}

fn set_matrix(gl: &glow::Context, p: glow::Program, name: &str, m: &Matrix) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            // Transposed, for the same reason the model-view-projection is.
            gl.uniform_matrix_4_f32_slice(Some(&l), true, m);
        }
    }
}

fn set_vec2(gl: &glow::Context, p: glow::Program, name: &str, v: [f32; 2]) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            gl.uniform_2_f32(Some(&l), v[0], v[1]);
        }
    }
}

fn set_vec3(gl: &glow::Context, p: glow::Program, name: &str, v: [f32; 3]) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            gl.uniform_3_f32(Some(&l), v[0], v[1], v[2]);
        }
    }
}

fn set_vec4(gl: &glow::Context, p: glow::Program, name: &str, v: [f32; 4]) {
    unsafe {
        if let Some(l) = gl.get_uniform_location(p, name) {
            gl.uniform_4_f32(Some(&l), v[0], v[1], v[2], v[3]);
        }
    }
}

/// Fills an array uniform, such as the spectrum a shader listens to.
///
/// A driver may name the array by its first element rather than by the array,
/// so both spellings are tried before giving up on it.
fn set_floats(gl: &glow::Context, p: glow::Program, name: &str, values: &[f32]) {
    unsafe {
        let found = gl
            .get_uniform_location(p, name)
            .or_else(|| gl.get_uniform_location(p, &format!("{name}[0]")));
        if let Some(l) = found {
            gl.uniform_1_f32_slice(Some(&l), values);
        }
    }
}

/// Sets a uniform from a material's stored value.
///
/// Materials write vectors as space-separated strings, the same way scenes do,
/// so the number of components decides which call to make.
fn set_json(gl: &glow::Context, p: glow::Program, name: &str, value: &serde_json::Value) {
    match value {
        serde_json::Value::Number(n) => {
            set_float(gl, p, name, n.as_f64().unwrap_or(0.0) as f32);
        }
        serde_json::Value::Bool(b) => set_float(gl, p, name, if *b { 1.0 } else { 0.0 }),
        serde_json::Value::String(s) => {
            let parts: Vec<f32> = s
                .split_whitespace()
                .filter_map(|p| p.parse().ok())
                .collect();
            match parts.len() {
                1 => set_float(gl, p, name, parts[0]),
                2 => unsafe {
                    if let Some(l) = gl.get_uniform_location(p, name) {
                        gl.uniform_2_f32(Some(&l), parts[0], parts[1]);
                    }
                },
                3 => set_vec3(gl, p, name, [parts[0], parts[1], parts[2]]),
                4 => set_vec4(gl, p, name, [parts[0], parts[1], parts[2], parts[3]]),
                _ => {}
            }
        }
        _ => {}
    }
}

/// The file inside the package that is the scene: what `project.json` names
/// as the wallpaper's `file`, which is `scene.json` almost always and not
/// always -- SCP Terminal Lockdown is `gifscene.json` in `gifscene.pkg`.
pub(crate) fn scene_entry(project: Option<&[u8]>) -> String {
    project
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .and_then(|json| json.get("file")?.as_str().map(str::to_owned))
        .filter(|file| file.ends_with(".json"))
        .unwrap_or_else(|| "scene.json".into())
}

/// Objects left out by the user's decision although Wallpaper Engine draws
/// them, as (the wallpaper's folder, the object's id).
const LEFT_OUT: &[(&str, i64)] = &[
    // Alien Romulus' "explosion on cursor": small dark specks round the pointer on
    // Windows, splats three or four times as large here, and none at all was
    // preferred to those. Its visibility is bound to a `mausverfolgung` that the
    // published project.json does not offer, so there is no setting to turn it off.
    ("3315524934", 162),
];

/// A wallpaper's files: its package, or the folder itself where there is
/// none, which is how Wallpaper Engine's own Deep Space and the rest come.
pub(crate) fn open_package(dir: &std::path::Path) -> Result<Package, String> {
    match find_package(dir) {
        Ok(path) => Package::read(&path).map_err(|e| format!("reading package: {e}")),
        Err(none) if !dir.join("project.json").exists() => Err(none),
        Err(_) => Package::from_folder(dir).map_err(|e| format!("reading {}: {e}", dir.display())),
    }
}

pub(crate) fn find_package(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let mut found: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "pkg"))
        .collect();
    found.sort();
    found
        .into_iter()
        .next()
        .ok_or_else(|| format!("no .pkg in {}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::{middle_of, anchor_of, coverage, divisor_of, other_layer, scene_entry, showing};

    #[test]
    fn the_scene_is_the_file_the_project_names() {
        assert_eq!(scene_entry(Some(br#"{"file":"gifscene.json","type":"scene"}"#)), "gifscene.json");
        assert_eq!(scene_entry(Some(br#"{"type":"scene"}"#)), "scene.json");
        assert_eq!(scene_entry(Some(b"not json")), "scene.json");
        assert_eq!(scene_entry(None), "scene.json");
    }

    /// The scene's y runs up, so an origin on the bottom edge has the middle
    /// of the image above it -- half an image the other way from the top.
    /// A hand standing on its origin, turned a half turn, hangs below it.
    #[test]
    fn an_object_turns_about_its_origin() {
        let anchor = anchor_of("bottom");
        let upright = middle_of([100.0, 100.0, 0.0], anchor, [10.0, 40.0], [1.0; 3], 0.0);
        assert_eq!(&upright[..2], &[100.0, 120.0]);
        let turned =
            middle_of([100.0, 100.0, 0.0], anchor, [10.0, 40.0], [1.0; 3], std::f32::consts::PI);
        assert!((turned[0] - 100.0).abs() < 1e-3 && (turned[1] - 80.0).abs() < 1e-3, "{turned:?}");
        let quarter =
            middle_of([100.0, 100.0, 0.0], anchor, [10.0, 40.0], [1.0; 3], std::f32::consts::FRAC_PI_2);
        assert!((quarter[0] - 80.0).abs() < 1e-3, "{quarter:?}");
    }

    #[test]
    fn an_alignment_puts_the_origin_on_that_edge() {
        assert_eq!(anchor_of("center"), [0.0, 0.0]);
        assert_eq!(anchor_of("bottom"), [0.0, -0.5]);
        assert_eq!(anchor_of("topleft"), [-0.5, 0.5]);
        assert_eq!(anchor_of("right"), [0.5, 0.0]);
    }

    /// Getting one of these wrong blurs at the wrong size: a half-sized buffer
    /// taken for a full-sized one reaches half as far across the picture.
    #[test]
    fn the_engines_buffer_names_say_how_big_they_are() {
        assert_eq!(divisor_of("_rt_FullCompoBuffer1"), 1);
        assert_eq!(divisor_of("_rt_HalfCompoBuffer2"), 2);
        assert_eq!(divisor_of("_rt_QuarterCompoBuffer1"), 4);
        assert_eq!(divisor_of("_rt_imageLayerComposite_12_a"), 1);
    }

    #[test]
    fn another_layers_picture_is_known_by_its_number() {
        assert_eq!(other_layer("_rt_imageLayerComposite_151_a"), Some(151));
        assert_eq!(other_layer("_rt_imageLayerComposite_151_b"), None);
        assert_eq!(other_layer("_rt_FullCompoBuffer1"), None);
        assert_eq!(other_layer("masks/blend_mask_7be32257"), None);
    }

    #[test]
    fn a_texture_stored_at_its_own_size_is_read_whole() {
        assert_eq!(coverage((1920, 1080), (1920, 1080)), [1.0, 1.0]);
    }

    #[test]
    fn a_padded_texture_is_read_only_as_far_as_its_image() {
        // The background of one wallpaper in the library, 3840 by 2160 stored
        // in a 4096 square. Reading it whole samples the empty part.
        assert_eq!(coverage((3840, 2160), (4096, 4096)), [0.9375, 0.52734375]);
    }

    #[test]
    fn thinning_takes_from_the_largest_and_leaves_the_small_alone() {
        assert_eq!(super::most_for_each(vec![45_000, 41, 41, 30_000], 24_000), 11_959);
        assert_eq!(super::most_for_each(vec![10, 20], 30), usize::MAX);
        assert_eq!(super::most_for_each(vec![30, 30], 30), 15);
    }

    #[test]
    fn a_texture_that_states_nothing_sensible_is_read_whole() {
        assert_eq!(coverage((0, 0), (512, 512)), [1.0, 1.0]);
        assert_eq!(coverage((512, 512), (0, 0)), [1.0, 1.0]);
        // An image larger than what holds it cannot be read past the edge.
        assert_eq!(coverage((600, 600), (512, 512)), [1.0, 1.0]);
    }

    #[test]
    fn a_sheet_steps_a_frame_at_a_time_and_then_starts_over() {
        // Fifty-one frames held a 14th of a second each, as one wallpaper's
        // glitter states.
        let (frames, interval) = (51, 0.07);
        assert_eq!(showing(frames, interval, 0.0), 0);
        assert_eq!(showing(frames, interval, 0.06), 0);
        assert_eq!(showing(frames, interval, 0.08), 1);
        assert_eq!(showing(frames, interval, 0.07 * 50.5), 50);
        // Round the loop and back to the start.
        assert_eq!(showing(frames, interval, 0.07 * 51.0), 0);
        assert_eq!(showing(frames, interval, 0.07 * 52.5), 1);
    }

    #[test]
    fn a_sheet_never_asks_for_a_frame_it_does_not_have() {
        for time in [0.0, 1.0, 1e6, -1.0, f32::NAN] {
            assert!(showing(2, 0.12, time) < 2, "picked a frame past the end");
        }
        // Nothing to show and no interval to show it for.
        assert_eq!(showing(0, 0.04, 1.0), 0);
        assert_eq!(showing(10, 0.0, 1.0), 0);
    }
}
