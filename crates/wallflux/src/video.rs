// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! A video a scene uses as a texture, played into it as the scene runs.
//!
//! A scene can hand an image an MP4 instead of a picture: the `.tex` says so
//! in its flags and carries the file whole as its only surface. Dark Core
//! lays a network of moving lines over itself that way, four layers sharing
//! one clip, and Diablo IV Lilith is nothing but such an image.
//!
//! The decoding is `ffmpeg`'s, run as a program: it is on every machine that
//! plays video wallpapers, since the media player those use is built on it,
//! and a program of its own rather than a library linked in keeps a codec
//! that misbehaves out of the desktop's render thread. It is handed the file
//! written out into the cache -- an MP4 cannot be read through a pipe, since
//! finding its index means seeking -- and gives back raw RGBA frames down
//! one, read on a thread of their own. The pipe is the clock's brake: a
//! decoder ahead of the scene waits on a full pipe and costs nothing.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError, sync_channel};
use std::time::Duration;

use glow::HasContext;

use crate::texture::Texture;

/// The longest side a frame is decoded at. A clip as big as a 4K screen is
/// 33 MB a frame down the pipe; a texture this size is drawn into a layer
/// with nobody able to tell.
// ponytail: fixed cap, frames sized to the layer on screen if one ever shows
const LONGEST: u32 = 1920;

/// How far behind the scene a clip may fall before it is started again from
/// where the scene is rather than caught up frame by frame.
const MOST_BEHIND: f64 = 2.0;

pub struct Video {
    path: PathBuf,
    size: (u32, u32),
    fps: f64,
    /// Seconds the clip runs before it loops. Nought where it could not be
    /// read, in which case the clip is only ever started from its beginning.
    length: f64,
    texture: glow::Texture,
    decoder: Option<Decoder>,
    /// The number of the next frame the decoder will hand over, counted from
    /// the scene's start: a looping clip keeps counting.
    next: u64,
}

struct Decoder {
    child: Child,
    frames: Receiver<Vec<u8>>,
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // The reading thread ends by itself once the pipe closes.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// What to do to show frame `wanted` when the decoder will hand over `next`.
#[derive(Debug, PartialEq)]
enum Plan {
    /// Take this many frames off the decoder, showing the last.
    Take(u64),
    /// Start the decoder again at `wanted`: the scene has gone back, or so
    /// far ahead that catching up would be slower than seeking.
    Restart,
}

fn plan(wanted: u64, next: u64, fps: f64) -> Plan {
    if wanted + 1 < next || (wanted - next.min(wanted)) as f64 > MOST_BEHIND * fps {
        Plan::Restart
    } else {
        Plan::Take((wanted + 1).saturating_sub(next))
    }
}

impl Video {
    /// Writes the clip out under `cache` as `name` and makes the texture it
    /// will be played into. `ffprobe` is asked what the clip is; a file it
    /// cannot read is refused here, at load, rather than drawn as nothing.
    pub fn open(gl: &glow::Context, mp4: &[u8], cache: &Path, name: &str) -> Result<Self, String> {
        std::fs::create_dir_all(cache).map_err(|e| format!("{}: {e}", cache.display()))?;
        let path = cache.join(file_name(name));
        // The same clip at the same size is left alone, as the sounds are.
        if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(mp4.len() as u64) {
            std::fs::write(&path, mp4).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let (size, fps, length) = probe(&path)?;
        let texture = unsafe {
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
                glow::PixelUnpackData::Slice(Some(&vec![0; (size.0 * size.1 * 4) as usize])),
            );
            for (name, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
            }
            texture
        };
        Ok(Self {
            path,
            size,
            fps,
            length,
            texture,
            decoder: None,
            next: 0,
        })
    }

    /// The texture the clip plays into, for a pass to bind, saying it is
    /// `as_big_as` whatever size it is decoded at.
    pub fn texture(&self, as_big_as: (u32, u32)) -> Texture {
        Texture::borrowed(self.texture, as_big_as.0.max(1), as_big_as.1.max(1))
    }

    /// Puts the frame showing at `time` into the texture. Waits for it only
    /// when the decoder has just been started, which is the first frame and
    /// any jump; otherwise a decoder that is behind leaves the last frame up.
    pub fn show(&mut self, gl: &glow::Context, time: f32) {
        let wanted = (time.max(0.0) as f64 * self.fps) as u64;
        let (mut take, fresh) = match (&self.decoder, plan(wanted, self.next, self.fps)) {
            (Some(_), Plan::Take(n)) => (n, false),
            _ => {
                self.restart(wanted);
                (1, true)
            }
        };
        let mut last = None;
        while take > 0 {
            let Some(decoder) = &self.decoder else { return };
            let frame = match fresh {
                true => decoder.frames.recv_timeout(Duration::from_secs(2)).map_err(|e| e == RecvTimeoutError::Disconnected),
                false => decoder.frames.try_recv().map_err(|e| e == TryRecvError::Disconnected),
            };
            match frame {
                Ok(frame) => {
                    last = Some(frame);
                    self.next += 1;
                    take -= 1;
                }
                // Gone: ffmpeg stopped, which a loop never does. Started
                // again on the next frame rather than spun on here.
                Err(true) => {
                    eprintln!("wallflux scene: the video {} stopped", self.path.display());
                    self.decoder = None;
                    break;
                }
                Err(false) => break,
            }
        }
        if let Some(frame) = last {
            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    self.size.0 as i32,
                    self.size.1 as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&frame)),
                );
            }
        }
    }

    fn restart(&mut self, at: u64) {
        self.decoder = None;
        self.next = at;
        let seconds = at as f64 / self.fps;
        let into = match self.length > 0.0 {
            true => seconds % self.length,
            false => 0.0,
        };
        match decode(&self.path, self.size, into) {
            Ok(decoder) => self.decoder = Some(decoder),
            Err(e) => eprintln!("wallflux scene: {}: {e}", self.path.display()),
        }
    }
}

/// A name for the clip's file that cannot climb out of the cache, whatever
/// the package called the texture.
fn file_name(name: &str) -> String {
    let plain: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    format!("{plain}.mp4")
}

/// The clip's size as it will be decoded, its frame rate and its length.
fn probe(path: &Path) -> Result<((u32, u32), f64, f64), String> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
        .arg("stream=width,height,avg_frame_rate:format=duration")
        .args(["-of", "default=noprint_wrappers=1"])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("ffprobe could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!("ffprobe: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    read_probe(&String::from_utf8_lossy(&out.stdout))
}

fn read_probe(text: &str) -> Result<((u32, u32), f64, f64), String> {
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(str::trim)
    };
    let number = |key: &str| field(key).and_then(|v| v.parse::<f64>().ok());
    let (width, height) = match (number("width"), number("height")) {
        (Some(w), Some(h)) if w >= 1.0 && h >= 1.0 => (w, h),
        _ => return Err("the clip has no picture".into()),
    };
    // A rate is a fraction, "30000/1001".
    let fps = field("avg_frame_rate")
        .and_then(|rate| {
            let (n, d) = rate.split_once('/').unwrap_or((rate, "1"));
            Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?)
        })
        .filter(|fps| fps.is_finite() && *fps > 0.0)
        .unwrap_or(30.0);
    let length = number("duration").filter(|d| d.is_finite() && *d > 0.0).unwrap_or(0.0);
    let scale = (LONGEST as f64 / width.max(height)).min(1.0);
    // Even sizes, which every pixel format ffmpeg might go through can hold.
    let even = |v: f64| (((v * scale) as u32) & !1).max(2);
    Ok(((even(width), even(height)), fps, length))
}

/// Starts ffmpeg on the clip `into` seconds in, looping for ever.
fn decode(path: &Path, size: (u32, u32), into: f64) -> Result<Decoder, String> {
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-ss"])
        .arg(format!("{into:.3}"))
        .args(["-stream_loop", "-1", "-i"])
        .arg(path)
        .args(["-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
        .arg(format!("{}x{}", size.0, size.1))
        .arg("pipe:1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("ffmpeg could not be run: {e}"))?;
    let stdout = child.stdout.take().ok_or("ffmpeg gave no output")?;
    // Two frames in hand: one to show, one on its way.
    let (send, frames) = sync_channel(2);
    let bytes = (size.0 * size.1 * 4) as usize;
    std::thread::Builder::new()
        .name("wallflux video".into())
        .spawn(move || read_frames(stdout, bytes, send))
        .map_err(|e| e.to_string())?;
    Ok(Decoder { child, frames })
}

fn read_frames(mut from: impl Read, bytes: usize, to: SyncSender<Vec<u8>>) {
    loop {
        let mut frame = vec![0; bytes];
        if from.read_exact(&mut frame).is_err() || to.send(frame).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_running_on_takes_the_frames_it_has_reached() {
        assert_eq!(plan(10, 10, 30.0), Plan::Take(1));
        assert_eq!(plan(12, 10, 30.0), Plan::Take(3));
        // Asked again for the frame already showing.
        assert_eq!(plan(9, 10, 30.0), Plan::Take(0));
    }

    #[test]
    fn a_jump_either_way_starts_the_clip_again_where_the_scene_is() {
        assert_eq!(plan(3, 10, 30.0), Plan::Restart);
        // Back from three minutes under a window.
        assert_eq!(plan(10 + 5400, 10, 30.0), Plan::Restart);
        assert_eq!(plan(10 + 50, 10, 30.0), Plan::Take(51));
    }

    #[test]
    fn a_clip_is_read_off_what_ffprobe_says() {
        let probed = "width=3840\nheight=2160\navg_frame_rate=30000/1001\nduration=60.060000\n";
        let (size, fps, length) = read_probe(probed).unwrap();
        assert_eq!(size, (1920, 1080));
        assert!((fps - 29.97).abs() < 0.01, "{fps}");
        assert!((length - 60.06).abs() < 1e-6);
        assert!(read_probe("duration=3\n").is_err());
        // A rate ffprobe could not tell is taken as thirty.
        assert_eq!(read_probe("width=640\nheight=360\navg_frame_rate=0/0\n").unwrap().1, 30.0);
    }

    #[test]
    fn a_name_cannot_leave_the_cache() {
        assert_eq!(file_name("../../etc/passwd"), "______etc_passwd.mp4");
        assert_eq!(file_name("Plexus_60_sec_Loop2"), "Plexus_60_sec_Loop2.mp4");
    }
}
