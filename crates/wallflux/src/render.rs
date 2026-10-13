//! Drawing a scene with the shaders the wallpaper brought with it.
//!
//! Each visible image object becomes a quad placed in the scene's own pixel
//! coordinates, drawn with the program its material names and the textures its
//! pass binds. Objects are drawn in the order the scene lists them, which is
//! the order they are meant to stack.

use std::collections::BTreeMap;

use glow::HasContext;
use wpe_format::scene::ObjectKind;
use wpe_format::shader::{Stage, preprocess};
use wpe_format::{Assets, Material, Model, Package, Resolver, Scene as SceneFile};

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
}

pub struct Scene {
    objects: Vec<Object>,
    quad: glow::VertexArray,
    #[allow(dead_code)]
    buffer: glow::Buffer,
    pub scene_size: (u32, u32),
    /// Passes that would not compile, named so they can be looked at.
    pub skipped: Vec<String>,
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
            let ObjectKind::Image(model_path) = object.kind() else {
                continue;
            };
            if !object.visible.value {
                continue;
            }
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

            let mut passes = Vec::new();
            for pass in &material.passes {
                match build_pass(gl, &resolve, pass) {
                    Ok(built) => passes.push(built),
                    Err(e) => skipped.push(format!("{}: {}: {e}", object.name, pass.shader)),
                }
            }
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
            });
        }

        if objects.is_empty() {
            return Err("no image object in this wallpaper could be drawn".into());
        }

        let (quad, buffer) = unsafe { make_quad(gl)? };
        Ok(Self {
            objects,
            quad,
            buffer,
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

    /// Draws the scene onto an output of the given size.
    pub fn draw(&self, gl: &glow::Context, output: (u32, u32), time: f32) {
        let projection = transform::orthographic_cover(
            (self.scene_size.0 as f32, self.scene_size.1 as f32),
            (output.0 as f32, output.1 as f32),
        );
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.bind_vertex_array(Some(self.quad));
        }
        for object in &self.objects {
            let model = transform::model(object.origin, object.size, object.scale, object.angles);
            let mvp = transform::multiply(&projection, &model);
            for pass in &object.passes {
                unsafe { draw_pass(gl, pass, object, &mvp, time) };
            }
        }
    }
}

unsafe fn draw_pass(gl: &glow::Context, pass: &Pass, object: &Object, mvp: &Matrix, time: f32) {
    unsafe {
        gl.use_program(Some(pass.program));
        pass.blending.apply(gl);

        for (unit, slot) in pass.textures.iter().enumerate() {
            let Some(texture) = slot else { continue };
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

        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}

fn build_pass(
    gl: &glow::Context,
    resolve: &Resolver<'_>,
    pass: &wpe_format::material::Pass,
) -> Result<Pass, String> {
    if pass.shader.is_empty() {
        return Err("pass names no shader".into());
    }
    let combos: BTreeMap<String, String> = pass
        .defines()
        .map(|(k, v)| (k, v.to_string().trim_matches('"').to_owned()))
        .collect();

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

    let mut textures = Vec::new();
    for name in &pass.textures {
        textures.push(match name {
            Some(name) if !name.is_empty() && !wpe_format::effect::is_render_target(name) => {
                resolve
                    .get(&format!("materials/{name}.tex"))
                    .and_then(|bytes| wpe_format::Texture::parse(&bytes).ok())
                    .filter(|t| !t.is_video())
                    .and_then(|source| Texture::upload(gl, &source).ok())
            }
            _ => None,
        });
    }

    Ok(Pass {
        program,
        textures,
        blending: Blending::parse(&pass.blending),
        constants: pass
            .constantshadervalues
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect(),
    })
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
