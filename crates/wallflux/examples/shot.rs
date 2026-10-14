//! Renders a wallpaper to a PNG without touching the screen.
//!
//! Runs the same loading and drawing as the on-screen renderer against an
//! off-screen buffer, so the result is a file that can be compared with the
//! wallpaper's own preview. Takes a time in seconds, because an animated
//! wallpaper looks different at different moments.
//!
//! ```text
//! cargo run -p wallflux --example shot -- <wallpaper-dir> <out.png> [width] [height] [seconds] [particle-budget]
//! ```

use std::path::PathBuf;

use glow::HasContext;
use khronos_egl as egl;

#[path = "../src/particles.rs"]
mod particles;
#[path = "../src/render.rs"]
mod render;
#[path = "../src/texture.rs"]
mod texture;
#[path = "../src/transform.rs"]
mod transform;

use render::Scene;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, out, rest @ ..] = args.as_slice() else {
        eprintln!(
            "usage: shot <wallpaper-dir> <out.png> [width] [height] [seconds] [particle-budget]"
        );
        std::process::exit(2);
    };
    let width: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(720);
    let time: f32 = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let (_egl, gl) = headless(width, height).unwrap_or_else(|e| {
        eprintln!("cannot get a GL context: {e}");
        std::process::exit(1);
    });

    let budget = rest
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(render::DEFAULT_BUDGET);
    let mut scene = Scene::load_within(&gl, &PathBuf::from(dir), budget).unwrap_or_else(|e| {
        eprintln!("cannot load wallpaper: {e}");
        std::process::exit(1);
    });
    println!(
        "scene {}x{}, {} objects: {}",
        scene.scene_size.0,
        scene.scene_size.1,
        scene.len(),
        scene.object_names().collect::<Vec<_>>().join(", ")
    );
    for skipped in &scene.skipped {
        eprintln!("  skipped {skipped}");
    }

    let pixels =
        unsafe { render_to_image(&gl, &mut scene, width, height, time) }.unwrap_or_else(|e| {
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
    println!(
        "wrote {out} ({width}x{height}) at t={time}s, {} particles alive",
        scene.particle_count()
    );
}

unsafe fn render_to_image(
    gl: &glow::Context,
    scene: &mut Scene,
    width: u32,
    height: u32,
    time: f32,
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

        gl.viewport(0, 0, width as i32, height as i32);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        // The scene as it would be at that moment, not as a frame could
        // catch up to it.
        scene.settle(time);
        scene.draw(gl, (width, height), time);
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
        Ok(pixels
            .chunks_exact(stride)
            .rev()
            .flatten()
            .copied()
            .collect())
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
