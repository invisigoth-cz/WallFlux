//! Drawing a scene with the shaders the wallpaper brought with it.
//!
//! Each visible image object becomes a quad placed in the scene's own pixel
//! coordinates, drawn with the program its material names and the textures its
//! pass binds. Objects are drawn in the order the scene lists them, which is
//! the order they are meant to stack.

use std::collections::BTreeMap;

use glow::HasContext;
use wpe_format::particle::Renderer as ParticleRenderer;
use wpe_format::scene::ObjectKind;
use wpe_format::shader::{Stage, preprocess, uniforms};
use wpe_format::{Assets, Material, Model, Package, ParticleSystem, Resolver, Scene as SceneFile};

use crate::particles::{self, FLOATS_PER_VERTEX};
use crate::texture::Texture;
use crate::transform::{self, Matrix};

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
            "translucent" => Self::Translucent,
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

impl Pass {
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
    /// How much of the first texture the image fills.
    ///
    /// Of the 384 textures in the library, 98 are stored at the next power of
    /// two with the image in a corner and the rest left empty. Reading one of
    /// those from corner to corner samples the empty part, which clamping then
    /// smears across the whole object.
    covered: [f32; 2],
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
    Particles,
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
}

pub struct Scene {
    objects: Vec<Object>,
    quad: glow::VertexArray,
    buffer: glow::Buffer,
    /// Allocated only if the scene has a particle system to draw.
    particles: Option<Buffers>,
    /// Stands in for a texture slot a pass leaves empty.
    white: Texture,
    /// How far the particle systems have been simulated.
    simulated: f32,
    pub scene_size: (u32, u32),
    /// Passes that would not compile, named so they can be looked at.
    pub skipped: Vec<String>,
}

/// The buffers every particle system in the scene is drawn through, sized for
/// the largest of them and refilled per system per frame.
struct Buffers {
    vao: glow::VertexArray,
    vertices: glow::Buffer,
    #[allow(dead_code)]
    indices: glow::Buffer,
}

impl Scene {
    pub fn load(gl: &glow::Context, dir: &std::path::Path) -> Result<Self, String> {
        let path = find_package(dir)?;
        let package = Package::read(&path).map_err(|e| format!("reading package: {e}"))?;
        let assets = Assets::locate();
        let resolve = Resolver::new(&package, assets.as_ref());

        let bytes = package
            .get("scene.json")
            .ok_or("no scene.json in this wallpaper")?;
        let file = SceneFile::parse(bytes).map_err(|e| format!("scene.json: {e}"))?;
        let scene_size = (
            file.general.orthogonalprojection.width,
            file.general.orthogonalprojection.height,
        );

        let mut objects = Vec::new();
        let mut skipped = Vec::new();
        for object in &file.objects {
            if !object.visible.value {
                continue;
            }
            match object.kind() {
                ObjectKind::Particle(file) => {
                    match load_particles(gl, &resolve, object, &file, &mut skipped) {
                        Some(built) => objects.push(built),
                        None => continue,
                    }
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

            let passes = build_passes(gl, &resolve, &material, Kind::Image, object, &mut skipped);
            if passes.is_empty() {
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
            });
        }

        if objects.is_empty() {
            return Err("nothing in this wallpaper could be drawn".into());
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
        Ok(Self {
            objects,
            quad,
            buffer,
            particles,
            white: Texture::white(gl)?,
            simulated: 0.0,
            scene_size,
            skipped,
        })
    }

    pub fn object_names(&self) -> impl Iterator<Item = &str> {
        self.objects.iter().map(|o| o.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.objects.len()
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
        self.advance_to(time);
        let projection = transform::orthographic_cover(
            (self.scene_size.0 as f32, self.scene_size.1 as f32),
            (output.0 as f32, output.1 as f32),
        );
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
        }
        for object in &self.objects {
            let model = transform::model(object.origin, object.size, object.scale, object.angles);
            let mvp = transform::multiply(&projection, &model);
            match (&object.drawing, &self.particles) {
                (Drawing::Image, _) => unsafe {
                    gl.bind_vertex_array(Some(self.quad));
                    for pass in &object.passes {
                        write_quad(gl, self.buffer, pass.covered);
                        draw_pass(gl, pass, object, &mvp, time, &self.white);
                    }
                },
                (Drawing::Particles(system), Some(buffers)) => unsafe {
                    draw_particles(gl, object, system, buffers, &mvp, time, &self.white);
                },
                // Unreachable: the buffers are allocated whenever a particle
                // system survived loading.
                (Drawing::Particles(_), None) => {}
            }
        }
    }

    /// Steps every particle system up to `time`.
    ///
    /// Fixed slices rather than whatever the last frame took, so a system
    /// looks the same whether the compositor asked for 60 frames or 144, and
    /// so the screenshot tool can seek to a moment by stepping there.
    fn advance_to(&mut self, time: f32) {
        const STEP: f32 = 1.0 / 60.0;
        // Half a minute of catching up is already more than any system needs
        // to reach its steady state; beyond that the rest is skipped rather
        // than stalling the frame that resumes.
        const MOST: usize = 1800;

        let mut steps = ((time - self.simulated).max(0.0) / STEP) as usize;
        if steps > MOST {
            self.simulated = time - MOST as f32 * STEP;
            steps = MOST;
        }
        for _ in 0..steps {
            for object in &mut self.objects {
                if let Drawing::Particles(system) = &mut object.drawing {
                    system.advance(STEP);
                }
            }
        }
        self.simulated += steps as f32 * STEP;
    }
}

/// Builds the particle object a scene names, or nothing if this renderer
/// cannot draw it yet.
fn load_particles(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    object: &wpe_format::scene::Object,
    path: &str,
    skipped: &mut Vec<String>,
) -> Option<Object> {
    let bytes = resolve.get(path)?;
    let definition = match ParticleSystem::parse(&bytes) {
        Ok(d) => d,
        Err(e) => {
            skipped.push(format!("{}: {path}: {e}", object.name));
            return None;
        }
    };
    if definition.renderer != ParticleRenderer::Sprite {
        skipped.push(format!(
            "{}: {:?} particles are not drawn yet",
            object.name, definition.renderer
        ));
        return None;
    }
    if definition.emitters.is_empty() || definition.maxcount == 0 {
        return None;
    }
    for step in &definition.unsupported {
        skipped.push(format!("{}: {step} is not applied yet", object.name));
    }

    let material_bytes = resolve.get(&definition.material)?;
    let material = Material::parse(&material_bytes).ok()?;
    let passes = build_passes(gl, resolve, &material, Kind::Particles, object, skipped);
    if passes.is_empty() {
        return None;
    }

    // Seeded from the object's own id so two systems in one scene do not
    // emit in step, and so a screenshot of a wallpaper is reproducible.
    let system = particles::System::new(&definition, object.id as u32);
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
    })
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
        match build_pass(gl, resolve, pass, kind) {
            Ok(built) => passes.push(built),
            Err(e) => skipped.push(format!("{}: {}: {e}", object.name, pass.shader)),
        }
    }
    passes
}

unsafe fn draw_pass(
    gl: &glow::Context,
    pass: &Pass,
    object: &Object,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
) {
    unsafe {
        set_common(gl, pass, object, mvp, time, white);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}

/// Uploads a system's particles and draws them in one call.
unsafe fn draw_particles(
    gl: &glow::Context,
    object: &Object,
    system: &particles::System,
    buffers: &Buffers,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
) {
    let vertices = system.vertices();
    if vertices.is_empty() {
        return;
    }
    unsafe {
        gl.bind_vertex_array(Some(buffers.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffers.vertices));
        let bytes: &[u8] = std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            std::mem::size_of_val(vertices),
        );
        gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);

        for pass in &object.passes {
            set_common(gl, pass, object, mvp, time, white);
            // A flat wallpaper's particles face the screen, so the frame the
            // shader turns a particle in is the scene's own axes.
            set_vec3(gl, pass.program, "g_OrientationRight", [1.0, 0.0, 0.0]);
            set_vec3(gl, pass.program, "g_OrientationUp", [0.0, 1.0, 0.0]);
            set_vec3(gl, pass.program, "g_OrientationForward", [0.0, 0.0, 1.0]);
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
                (system.count() * INDICES_PER_PARTICLE) as i32,
                glow::UNSIGNED_INT,
                0,
            );
        }
    }
}

/// The uniforms every pass is given, whichever way it is drawn.
unsafe fn set_common(
    gl: &glow::Context,
    pass: &Pass,
    object: &Object,
    mvp: &Matrix,
    time: f32,
    white: &Texture,
) {
    unsafe {
        gl.use_program(Some(pass.program));
        pass.blending.apply(gl);

        for (unit, slot) in pass.textures.iter().enumerate() {
            // An empty slot still has to be given something, or the pass reads
            // whatever texture was last left on that unit -- another object's.
            let texture = slot.as_ref().unwrap_or(white);
            texture.bind(gl, unit as u32);
            set_int(gl, pass.program, &format!("g_Texture{unit}"), unit as i32);
            // .xy is the texture, .zw the image inside it; shaders divide one
            // by the other to find the part that is not padding.
            set_vec4(
                gl,
                pass.program,
                &format!("g_Texture{unit}Resolution"),
                [
                    texture.width as f32,
                    texture.height as f32,
                    texture.image_size.0 as f32,
                    texture.image_size.1 as f32,
                ],
            );
        }

        // Shaders multiply as `mul(position, matrix)`, a row vector on the
        // left, so the matrix goes up transposed to mean the same thing.
        if let Some(l) = gl.get_uniform_location(pass.program, "g_ModelViewProjectionMatrix") {
            gl.uniform_matrix_4_f32_slice(Some(&l), true, mvp);
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
) -> Result<Pass, String> {
    if pass.shader.is_empty() {
        return Err("pass names no shader".into());
    }

    // A pass draws the texture in its first slot, so one that wants a render
    // target there is compositing something this renderer has not produced.
    // Drawing it anyway would put a full-screen rectangle of nothing over the
    // scene; the later slots only modulate, and white leaves those alone.
    if let Some(Some(name)) = pass.textures.first()
        && wpe_format::effect::is_render_target(name)
    {
        return Err(format!("needs the render target {name}"));
    }

    // Textures are loaded before the shaders are compiled, because whether
    // the first one is a sprite sheet decides how the shaders are built.
    let mut textures = Vec::new();
    let mut sheet = None;
    let mut padded = false;
    let mut covered = [1.0, 1.0];
    for (slot, name) in pass.textures.iter().enumerate() {
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
                Some(texture) if texture.is_video() => {
                    return Err("its texture holds a video".into());
                }
                None if named.is_some() => {
                    return Err(format!("texture {} could not be read", named.unwrap()));
                }
                _ => {}
            }
            sheet = source
                .as_ref()
                .and_then(|source| Sheet::of(source, named, resolve));
            padded = source.as_ref().is_some_and(Sheet::padded);
            if let Some(texture) = &source
                && let Some(page) = texture.images.first().and_then(|mips| mips.first())
            {
                covered = coverage(texture.image_size, (page.width, page.height));
            }
        }
        textures.push(
            source
                .as_ref()
                .filter(|texture| !texture.is_video())
                .and_then(|source| Texture::upload(gl, source).ok()),
        );
    }

    let mut combos: BTreeMap<String, String> = pass
        .defines()
        .map(|(k, v)| (k, v.to_string().trim_matches('"').to_owned()))
        .collect();
    // What was actually uploaded wins over anything the material says, since
    // this is the texture the shader will be sampling.
    for (slot, texture) in textures.iter().enumerate() {
        if let Some(texture) = texture {
            combos.insert(format!("TEX{slot}FORMAT"), texture.format.to_string());
        }
    }
    if kind == Kind::Particles {
        let on = |yes: bool| if yes { "1" } else { "0" }.to_owned();
        // No geometry shader, so the vertex shader builds the corner itself.
        combos.insert("GS_ENABLED".into(), on(false));
        // The wide vertex layout, which carries velocity and lifetime. The
        // sprite sheet path reads the lifetime, so it is always sent.
        combos.insert("THICKFORMAT".into(), on(true));
        combos.insert("TRAILRENDERER".into(), on(false));
        // Whether a system animates is the texture's business, not the
        // particle file's: most sheets in the library are used by a system
        // that names no animation mode at all, and walking the sheet is what
        // they expect. The mode only picks how it is walked.
        let sheeted = sheet.is_some();
        combos.insert("SPRITESHEET".into(), on(sheeted));
        combos.insert("SPRITESHEETBLENDNPOT".into(), on(sheeted && padded));
    }
    if kind == Kind::Image {
        sheet = None;
    }

    let mut stages = Vec::new();
    for (stage, ext) in [(Stage::Vertex, "vert"), (Stage::Fragment, "frag")] {
        let source = resolve
            .shader(&pass.shader, ext)
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
    let program = unsafe { link(gl, &stages)? };

    // A uniform nobody sets reads as zero, so the defaults a shader declares
    // for itself go in first and the material's own values over them.
    let mut constants = BTreeMap::new();
    let mut named_by = BTreeMap::new();
    for (_, source) in &stages {
        for uniform in uniforms(source) {
            if let Some(default) = uniform.default {
                constants.insert(uniform.name.clone(), default);
            }
            if let Some(label) = uniform.label {
                named_by.insert(label, uniform.name);
            }
        }
    }
    for (key, value) in &pass.constantshadervalues {
        // Materials name a constant by the label the shader gave it, so the
        // label is translated back before it can reach a uniform at all.
        let name = named_by.get(key).unwrap_or(key);
        constants.insert(name.clone(), value.value.clone());
    }

    Ok(Pass {
        program,
        textures,
        blending: Blending::parse(&pass.blending),
        constants,
        sheet,
        covered,
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

/// Writes the quad a pass draws, reading `covered` of its texture.
unsafe fn write_quad(gl: &glow::Context, buffer: glow::Buffer, covered: [f32; 2]) {
    let (u, v) = (covered[0], covered[1]);
    #[rustfmt::skip]
    let vertices: [f32; 20] = [
        -0.5, -0.5, 0.0,   0.0, 0.0,
         0.5, -0.5, 0.0,     u, 0.0,
        -0.5,  0.5, 0.0,   0.0,   v,
         0.5,  0.5, 0.0,     u,   v,
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

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        let bytes: &[u8] = std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            std::mem::size_of_val(indices.as_slice()),
        );
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

        Ok(Buffers {
            vao,
            vertices,
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

fn find_package(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
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
    use super::coverage;

    #[test]
    fn a_texture_stored_at_its_own_size_is_read_whole() {
        assert_eq!(coverage((1920, 1080), (1920, 1080)), [1.0, 1.0]);
    }

    #[test]
    fn a_padded_texture_is_read_only_as_far_as_its_image() {
        // The background of one wallpaper in the library: 3840 by 2160 stored
        // in a 4096 square. Reading it whole samples the empty part.
        assert_eq!(coverage((3840, 2160), (4096, 4096)), [0.9375, 0.52734375]);
    }

    #[test]
    fn a_texture_that_states_nothing_sensible_is_read_whole() {
        assert_eq!(coverage((0, 0), (512, 512)), [1.0, 1.0]);
        assert_eq!(coverage((512, 512), (0, 0)), [1.0, 1.0]);
        // An image larger than what holds it cannot be read past the edge.
        assert_eq!(coverage((600, 600), (512, 512)), [1.0, 1.0]);
    }
}
