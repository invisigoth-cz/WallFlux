// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Renders a wallpaper to a PNG without touching the screen.
//!
//! Runs the same loading and drawing as the on-screen renderer against an
//! off-screen buffer, so the result is a file that can be compared with the
//! wallpaper's own preview. Takes a time in seconds, because an animated
//! wallpaper looks different at different moments.
//!
//! With a frame count it also draws the scene that many times and reports the
//! middle one's time, which is the only honest way to answer what a change to
//! the renderer costs.
//!
//! The last argument stands in for sound, nought to one across every band, so
//! a wallpaper that answers to music can be looked at as if some were playing.
//! Left out, the scene is drawn in silence, which is what makes the same
//! wallpaper give the same picture twice.
//!
//! `WALLFLUX_POINTER=x,y` puts the mouse over the wallpaper, as fractions of
//! the picture across and down from its top-left corner. Left out, there is
//! no mouse, for the same reason there is no sound.
//!
//! ```text
//! cargo run -p wallflux --example shot -- <wallpaper-dir> <out.png> [width] [height] [seconds] [particle-budget] [frames] [loudness]
//! ```

use std::path::PathBuf;

use glow::HasContext;

mod offscreen;

#[path = "../src/particles.rs"]
mod particles;
#[path = "../src/render.rs"]
mod render;
#[path = "../src/script.rs"]
mod script;
#[path = "../src/text.rs"]
mod text;
#[path = "../src/texture.rs"]
mod texture;
#[path = "../src/transform.rs"]
mod transform;
#[path = "../src/video.rs"]
mod video;

use render::Scene;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, out, rest @ ..] = args.as_slice() else {
        eprintln!(
            "usage: shot <wallpaper-dir> <out.png> [width] [height] [seconds] [particle-budget] [frames] [loudness]"
        );
        std::process::exit(2);
    };
    let width: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(720);
    let time: f32 = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let (_egl, gl) = offscreen::headless(width, height).unwrap_or_else(|e| {
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
        "scene {}x{}, {} objects{}: {}",
        scene.scene_size.0,
        scene.scene_size.1,
        scene.len(),
        if scene.listens() { ", follows the sound" } else { "" },
        scene.object_names().collect::<Vec<_>>().join(", ")
    );
    for skipped in &scene.skipped {
        eprintln!("  skipped {skipped}");
    }

    let frames: usize = rest.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    let loudness: f32 = rest.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let loudness = loudness.clamp(0.0, 1.0);
    if let Ok(at) = std::env::var("WALLFLUX_POINTER") {
        let parsed = at
            .split_once(',')
            .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)));
        let Some(pointer) = parsed else {
            eprintln!("WALLFLUX_POINTER wants x,y as fractions, got {at:?}");
            std::process::exit(2);
        };
        scene.point_at(Some(pointer), (width, height));
    }
    scene.hear(wpe_format::particle::Spectrum {
        left: [loudness; wpe_format::particle::AUDIO_BANDS],
        right: [loudness; wpe_format::particle::AUDIO_BANDS],
    });
    let pixels = unsafe { render_to_image(&gl, &mut scene, width, height, time, frames) }
        .unwrap_or_else(|e| {
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
    frames: usize,
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
        // Each frame is a step further on, so the particle systems are running
        // as they would be rather than being asked for the same moment twice.
        let mut taken = Vec::new();
        for frame in 0..frames.max(1) {
            let at = time + frame as f32 / 60.0;
            let started = std::time::Instant::now();
            // As the program does every frame: a layer that is not opaque
            // would otherwise pile up on the frame before.
            gl.clear(glow::COLOR_BUFFER_BIT);
            scene.draw(gl, (width, height), at);
            gl.finish();
            taken.push(started.elapsed());
        }
        if frames > 1 {
            taken.sort();
            println!(
                "{frames} frames, middle one {:.1} ms",
                taken[taken.len() / 2].as_secs_f32() * 1000.0
            );
        }

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
