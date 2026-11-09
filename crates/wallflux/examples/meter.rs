// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Reads a raw stereo f32le file at 48 000 Hz through the renderer's own meter
//! and prints every reading: its time, then the 64 entries of each ear.
//!
//! ```text
//! cargo run -p wallflux --example meter -- <sound.f32> > readings.txt
//! ```

#![allow(dead_code)]

#[path = "../src/audio.rs"]
mod audio;

use std::io::Write;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: meter <stereo f32le at 44100 Hz>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        std::process::exit(1);
    });
    let samples: Vec<f32> = bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let mut ears = audio::Ears::new();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for (at, chunk) in samples.chunks_exact(audio::READ * 2).enumerate() {
        let sound = ears.feed(chunk.try_into().unwrap());
        let t = ((at + 1) * audio::READ) as f32 / audio::RATE as f32;
        write!(out, "{t:.5}").unwrap();
        for v in sound.left.iter().chain(sound.right.iter()) {
            write!(out, " {v:.5}").unwrap();
        }
        writeln!(out).unwrap();
    }
}
