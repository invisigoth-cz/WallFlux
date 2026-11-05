// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Renders a wallpaper to a video, driven by a piece of music, with that
//! music on the soundtrack.
//!
//! A picture cannot answer whether a wallpaper pumps too hard or too little:
//! that takes watching it with the sound that drove it. The bands are read
//! here exactly as the renderer reads the speakers, so what the clip shows is
//! what a viewer sitting in front of it gets.
//!
//! The sound is named either as a file on disk or as something the wallpaper
//! carries itself -- many of them ship the very track they were built around.
//!
//! ```text
//! cargo run -p wallflux --example clip -- <wallpaper-dir> <out.mp4> <sound> [seconds] [width] [height]
//! ```

// The audio module carries the listening side too, which a tool reading a
// file never touches.
#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use glow::HasContext;

#[path = "../src/audio.rs"]
mod audio;
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

/// Frames a second. Enough to see a beat land without making a file nobody
/// wants to send anywhere.
const FPS: u32 = 30;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, out, sound, rest @ ..] = args.as_slice() else {
        eprintln!("usage: clip <wallpaper-dir> <out.mp4> <sound> [seconds] [width] [height]");
        std::process::exit(2);
    };
    let seconds: f32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(15.0);
    let width: u32 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(1280);
    let height: u32 = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(720);

    let dir = PathBuf::from(dir);
    let track = find_sound(&dir, sound).unwrap_or_else(|| {
        eprintln!("no sound called {sound}, on disk or in the wallpaper");
        std::process::exit(1);
    });
    let samples = decode(&track, seconds).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {e}", track.display());
        std::process::exit(1);
    });

    let (_egl, gl) = offscreen::headless(width, height).unwrap_or_else(|e| {
        eprintln!("cannot get a GL context: {e}");
        std::process::exit(1);
    });
    // WALLFLUX_BUDGET draws with another particle budget than the program's,
    // to see what thinning a scene costs it.
    let budget = std::env::var("WALLFLUX_BUDGET")
        .ok()
        .and_then(|b| b.parse().ok())
        .unwrap_or(render::DEFAULT_BUDGET);
    let mut scene = Scene::load_within(&gl, &dir, budget).unwrap_or_else(|e| {
        eprintln!("cannot load wallpaper: {e}");
        std::process::exit(1);
    });

    println!(
        "scene {}x{}, {} objects",
        scene.scene_size.0,
        scene.scene_size.1,
        scene.len()
    );
    for skipped in &scene.skipped {
        eprintln!("  skipped {skipped}");
    }

    // Frames go straight into the encoder: as PNGs on the disk, a minute at
    // 2560x1440 is ten gigabytes, which a tmpfs /tmp does not have.
    let mut encoder = encode(&track, out, seconds, width, height).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let mut ears = audio::Ears::new();
    let mut fed = 0usize;
    // How the bands sat over the whole piece, which is the only way to tell a
    // response that follows the music from one pinned at the top of its range.
    let mut loudest = 0f32;
    let mut total = 0f64;
    let mut readings = 0usize;
    let mut pegged = 0usize;
    let frames_wanted = (seconds * FPS as f32) as u32;
    for frame in 0..frames_wanted {
        let at = frame as f32 / FPS as f32;
        // Feed the meter up to this moment, so the bands a frame is drawn
        // with are the sound a viewer has heard by then and no more.
        // Two samples a frame, one for each ear, woven together.
        let wanted = (at * audio::RATE as f32) as usize * 2;
        while fed + audio::READ * 2 <= wanted.min(samples.len()) {
            let mut mouthful = [0f32; audio::READ * 2];
            mouthful.copy_from_slice(&samples[fed..fed + audio::READ * 2]);
            let sound = ears.feed(&mouthful);
            for heard in [&sound.left, &sound.right] {
                loudest = loudest.max(heard.iter().cloned().fold(0.0, f32::max));
                total += heard.iter().map(|b| *b as f64).sum::<f64>();
                pegged += heard.iter().filter(|b| **b > 0.95).count();
                readings += heard.len();
            }
            scene.hear(sound);
            fed += audio::READ * 2;
        }
        let pixels = unsafe { render_frame(&gl, &mut scene, width, height, at) }
            .expect("drawing a frame failed");
        use std::io::Write;
        if let Err(e) = encoder.stdin.as_mut().expect("piped").write_all(&pixels) {
            eprintln!("ffmpeg stopped taking frames: {e}");
            std::process::exit(1);
        }
        if frame % FPS == 0 {
            print!("\r{}s of {seconds}s", frame / FPS);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    }
    println!("\rdrew {frames_wanted} frames   ");

    match finish(encoder) {
        Ok(()) => println!(
            "wrote {out} ({width}x{height}, {seconds}s, sound from {})",
            track.display()
        ),
        Err(e) => eprintln!("ffmpeg: {e}"),
    }
    let readings = readings.max(1);
    println!(
        "bands over the clip: average {:.2}, loudest {loudest:.2}, {:.0}% of readings at the top",
        total / readings as f64,
        100.0 * pegged as f64 / readings as f64
    );
}

/// The sound to play: a file on disk, or something the wallpaper carries.
///
/// A wallpaper's own track is the one it was built around, so it is the
/// fairest thing to judge the response against.
fn find_sound(dir: &Path, name: &str) -> Option<PathBuf> {
    let on_disk = PathBuf::from(name);
    if on_disk.is_file() {
        return Some(on_disk);
    }
    let package = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "pkg"))?;
    let package = wpe_format::Package::read(&package).ok()?;
    let (found, bytes) = package
        .iter()
        .find(|(held, _)| *held == name || held.ends_with(name))?;
    let out = std::env::temp_dir().join(Path::new(found).file_name()?);
    std::fs::write(&out, bytes).ok()?;
    Some(out)
}

/// The track as woven stereo samples at the rate the meter expects, which is
/// what ffmpeg is for: it reads whatever the wallpaper happened to ship.
fn decode(track: &Path, seconds: f32) -> Result<Vec<f32>, String> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(track)
        .args([
            "-t",
            &format!("{}", seconds + 1.0),
            "-f",
            "f32le",
            "-ac",
            "2",
            "-ar",
            &audio::RATE.to_string(),
            "-",
        ])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err("ffmpeg would not decode it".into());
    }
    Ok(out
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}

unsafe fn render_frame(
    gl: &glow::Context,
    scene: &mut Scene,
    width: u32,
    height: u32,
    at: f32,
) -> Result<Vec<u8>, String> {
    unsafe {
        gl.viewport(0, 0, width as i32, height as i32);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        scene.draw(gl, (width, height), at);
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
        let stride = (width * 4) as usize;
        Ok(pixels
            .chunks_exact(stride)
            .rev()
            .flatten()
            .copied()
            .collect())
    }
}

/// ffmpeg, waiting for raw RGBA frames on its stdin and the track beside them.
fn encode(
    track: &Path,
    out: &str,
    seconds: f32,
    width: u32,
    height: u32,
) -> Result<std::process::Child, String> {
    Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{width}x{height}")])
        .args(["-framerate", &FPS.to_string(), "-i", "-", "-i"])
        .arg(track)
        .args([
            "-t",
            &seconds.to_string(),
            "-c:v",
            "libx264",
            "-crf",
            "14",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            out,
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run ffmpeg: {e}"))
}

fn finish(mut child: std::process::Child) -> Result<(), String> {
    drop(child.stdin.take());
    let mut complaint = String::new();
    if let Some(err) = child.stderr.as_mut() {
        let _ = err.read_to_string(&mut complaint);
    }
    match child.wait().map_err(|e| e.to_string())?.success() {
        true => Ok(()),
        false => Err(complaint),
    }
}
