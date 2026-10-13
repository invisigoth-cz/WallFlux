//! Renders a wallpaper to a PNG without touching the screen.
//!
//! The on-screen renderer can only be checked by looking at the desktop, which
//! is awkward to do while working on it and impossible to do automatically.
//! This runs the same loading and drawing against an off-screen buffer, so the
//! result is a file that can be compared with the wallpaper's own preview.
//!
//! ```text
//! cargo run -p wallflux --example shot -- <wallpaper-dir> <out.png> [width] [height]
//! ```

use std::path::PathBuf;

use glow::HasContext;
use khronos_egl as egl;

#[path = "../src/texture.rs"]
mod texture;
#[path = "../src/wallpaper.rs"]
mod wallpaper;

use wallpaper::Wallpaper;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, out, rest @ ..] = args.as_slice() else {
        eprintln!("usage: shot <wallpaper-dir> <out.png> [width] [height]");
        std::process::exit(2);
    };
    let width: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(720);

    let (_egl, gl) = headless(width, height).unwrap_or_else(|e| {
        eprintln!("cannot get a GL context: {e}");
        std::process::exit(1);
    });

    let wallpaper = Wallpaper::load(&gl, &PathBuf::from(dir)).unwrap_or_else(|e| {
        eprintln!("cannot load wallpaper: {e}");
        std::process::exit(1);
    });
    println!(
        "object {:?}: texture {}x{}, {} mip levels, scene {}x{}",
        wallpaper.name,
        wallpaper.texture.width,
        wallpaper.texture.height,
        wallpaper.texture.levels,
        wallpaper.scene_size.0,
        wallpaper.scene_size.1,
    );

    let pixels = unsafe { render(&gl, &wallpaper, width, height) }.unwrap_or_else(|e| {
        eprintln!("render failed: {e}");
        std::process::exit(1);
    });

    let file = std::fs::File::create(out).expect("cannot create output file");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&pixels))
        .expect("cannot write PNG");
    println!("wrote {out} ({width}x{height})");
}

unsafe fn render(
    gl: &glow::Context,
    wallpaper: &Wallpaper,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    unsafe {
        let colour = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(colour));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        let fbo = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(colour),
            0,
        );
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            return Err("off-screen buffer is not complete".into());
        }

        let program = build(gl)?;
        let vao = gl.create_vertex_array()?;
        gl.viewport(0, 0, width as i32, height as i32);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.use_program(Some(program));
        wallpaper.texture.bind(gl, 0);
        let ((sx, sy), (ox, oy)) = wallpaper.texture.cover((width, height));
        if let Some(l) = gl.get_uniform_location(program, "u_image") {
            gl.uniform_1_i32(Some(&l), 0);
        }
        if let Some(l) = gl.get_uniform_location(program, "u_uvScale") {
            gl.uniform_2_f32(Some(&l), sx, sy);
        }
        if let Some(l) = gl.get_uniform_location(program, "u_uvOffset") {
            gl.uniform_2_f32(Some(&l), ox, oy);
        }
        gl.bind_vertex_array(Some(vao));
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        gl.finish();

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        gl.read_pixels(
            0,
            0,
            width as i32,
            height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        );
        // GL hands back the bottom row first; PNG wants the top row first.
        let stride = (width * 4) as usize;
        let flipped = pixels
            .chunks_exact(stride)
            .rev()
            .flatten()
            .copied()
            .collect();
        Ok(flipped)
    }
}

const VERTEX: &str = r#"#version 330 core
out vec2 v_uv;
void main() {
    vec2 corner = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
    v_uv = vec2(corner.x, 1.0 - corner.y);
    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 fragColor;
uniform sampler2D u_image;
uniform vec2 u_uvScale;
uniform vec2 u_uvOffset;
void main() {
    fragColor = texture(u_image, v_uv * u_uvScale + u_uvOffset);
}
"#;

unsafe fn build(gl: &glow::Context) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        for (kind, src) in [
            (glow::VERTEX_SHADER, VERTEX),
            (glow::FRAGMENT_SHADER, FRAGMENT),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, src);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }
        Ok(program)
    }
}

type Egl = egl::DynamicInstance<egl::EGL1_4>;

fn headless(width: u32, height: u32) -> Result<(Egl, glow::Context), String> {
    let egl: Egl = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required() }
        .map_err(|e| format!("no usable libEGL: {e}"))?;
    let display = unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }.ok_or("no default display")?;
    egl.initialize(display).map_err(|e| e.to_string())?;
    egl.bind_api(egl::OPENGL_API).map_err(|e| e.to_string())?;
    let config = egl
        .choose_first_config(
            display,
            &[
                egl::SURFACE_TYPE,
                egl::PBUFFER_BIT,
                egl::RENDERABLE_TYPE,
                egl::OPENGL_BIT,
                egl::NONE,
            ],
        )
        .map_err(|e| e.to_string())?
        .ok_or("no desktop GL config")?;
    let context = egl
        .create_context(
            display,
            config,
            None,
            &[0x3098, 3, 0x30FB, 3, 0x30FD, 0x0001, egl::NONE],
        )
        .map_err(|e| e.to_string())?;
    let surface = egl
        .create_pbuffer_surface(
            display,
            config,
            &[
                egl::WIDTH,
                width as i32,
                egl::HEIGHT,
                height as i32,
                egl::NONE,
            ],
        )
        .map_err(|e| e.to_string())?;
    egl.make_current(display, Some(surface), Some(surface), Some(context))
        .map_err(|e| e.to_string())?;
    let gl = unsafe {
        glow::Context::from_loader_function(|name| {
            egl.get_proc_address(name)
                .map_or(std::ptr::null(), |p| p as *const _)
        })
    };
    Ok((egl, gl))
}
