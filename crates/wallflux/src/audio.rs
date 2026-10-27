// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Listening to what the speakers are playing, as the sixteen bands a
//! wallpaper asks for.
//!
//! Wallpapers name bands by index and ask for the loudness across a range of
//! them, so the whole job here is to turn a stream of samples into sixteen
//! numbers from zero to one, often enough that a visualiser keeps up.
//!
//! The sound is read from the output's own monitor rather than a microphone,
//! which is what makes a wallpaper answer to the music the viewer is playing.
//! `@DEFAULT_MONITOR@` is PulseAudio's name for whichever output is current,
//! so nothing has to be chosen or followed when the viewer switches devices.
//!
//! Nothing here is required: a machine with no sound server, or one where the
//! connection drops, leaves the bands at zero, and a wallpaper reads that as
//! silence and runs at the quiet end of what it stated.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use wpe_format::particle::{AUDIO_BANDS, Spectrum};

/// Samples read at a time. At 44 100 Hz this is 23 ms, so the bands are
/// renewed about forty times a second, which is faster than the frames that
/// read them.
pub const READ: usize = 1024;

/// Samples each reading is measured over, which is the last two mouthfuls
/// rather than only the newest. Twice the samples halves the width of a bin,
/// and the bottom bands need that: a kick drum and a bass note are only forty
/// hertz apart, and a window of a thousand samples cannot tell them apart.
const WINDOW: usize = 2048;
pub const RATE: u32 = 44_100;

/// The lowest and highest frequency the bands are spread between. Below the
/// first is rumble no music carries, and above the last is air.
const LOWEST: f32 = 40.0;
const HIGHEST: f32 = 16_000.0;

/// How quickly a band may fall. A band that followed the sound exactly would
/// flicker on every drum hit; rising at once and falling slowly is what makes
/// a visualiser read as a beat rather than as noise.
const FALL: f32 = 0.12;

/// What a band's magnitude is divided by before it is squared down into zero
/// to one. This is the knob: too small and every band sits at the top however
/// quiet the room, too large and nothing ever reaches it.
///
/// Settled by ear against three pieces of music the wallpapers carry
/// themselves. The first reading of 0.02 was judged a fifth too weak; because
/// the magnitude is square-rooted, turning the answer up by a fifth means
/// dividing this by 1.2 squared.
const LOUD: f32 = 0.02 / (1.2 * 1.2);

/// The sound the speakers are playing, split into bands.
pub struct Audio {
    bands: &'static Mutex<Spectrum>,
    listening: &'static AtomicBool,
}

impl Audio {
    /// Starts listening, or returns `None` where there is nothing to listen
    /// to. A caller that gets `None` runs the scene as it would in silence.
    pub fn start() -> Option<Self> {
        // Leaked rather than shared through an Arc: there is one of these for
        // the life of the process, and the thread outlives any handle to it.
        let bands: &'static Mutex<Spectrum> = Box::leak(Box::new(Mutex::new(Spectrum::SILENT)));
        let listening: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(true)));

        let stream = connect()?;
        std::thread::Builder::new()
            .name("wallflux-audio".into())
            .spawn(move || listen(stream, bands, listening))
            .ok()?;
        Some(Self { bands, listening })
    }

    /// The loudness of each band in each ear, zero to one. Silence where the
    /// sound stopped being readable, which is what a wallpaper reads as a
    /// quiet room.
    pub fn bands(&self) -> Spectrum {
        match self.listening.load(Ordering::Relaxed) {
            true => self.bands.lock().map(|b| *b).unwrap_or(Spectrum::SILENT),
            false => Spectrum::SILENT,
        }
    }
}

fn connect() -> Option<Simple> {
    // Both ears: the stock shaders ask for one channel or the other, and a
    // wallpaper that pans a sound from side to side means it.
    let spec = Spec {
        format: Format::F32le,
        channels: 2,
        rate: RATE,
    };
    Simple::new(
        None,
        "WallFlux",
        Direction::Record,
        // Whichever output is playing, followed as the viewer switches
        // devices, which a named monitor would not be.
        Some("@DEFAULT_MONITOR@"),
        "wallpaper",
        &spec,
        None,
        None,
    )
    .ok()
}

/// Turns a stream of samples into the bands a wallpaper asks for.
///
/// Kept apart from the listening so that a tool rendering a clip from a sound
/// file reads it exactly as the renderer reads the speakers: a clip that
/// measured the sound its own way would be showing something nobody gets.
pub struct Meter {
    edges: [usize; AUDIO_BANDS + 1],
    samples: [f32; WINDOW],
    held: [f32; AUDIO_BANDS],
}

impl Meter {
    pub fn new() -> Self {
        Self {
            edges: band_edges(),
            samples: [0.0; WINDOW],
            held: [0.0; AUDIO_BANDS],
        }
    }

    /// Takes the next [`READ`] samples and gives the bands as they now stand.
    ///
    /// The newest mouthful goes at the end and what was there slides back, so
    /// each reading is measured over a window twice as long as it waited.
    pub fn feed(&mut self, fresh: &[f32; READ]) -> [f32; AUDIO_BANDS] {
        self.samples.copy_within(READ.., 0);
        self.samples[WINDOW - READ..].copy_from_slice(fresh);
        let measured = spectrum(&self.samples, &self.edges);
        for (held, measured) in self.held.iter_mut().zip(measured) {
            *held = match measured > *held {
                true => measured,
                false => *held + (measured - *held) * FALL,
            };
        }
        self.held
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}

/// A meter for each ear, fed from the woven pair of channels a stream carries.
pub struct Ears {
    left: Meter,
    right: Meter,
}

impl Ears {
    pub fn new() -> Self {
        Self {
            left: Meter::new(),
            right: Meter::new(),
        }
    }

    /// Takes `READ` frames of two woven channels and gives both sets of bands.
    pub fn feed(&mut self, woven: &[f32; READ * 2]) -> Spectrum {
        let mut left = [0f32; READ];
        let mut right = [0f32; READ];
        for (at, frame) in woven.as_chunks::<2>().0.iter().enumerate() {
            left[at] = frame[0];
            right[at] = frame[1];
        }
        Spectrum {
            left: self.left.feed(&left),
            right: self.right.feed(&right),
        }
    }
}

impl Default for Ears {
    fn default() -> Self {
        Self::new()
    }
}

fn listen(stream: Simple, bands: &Mutex<Spectrum>, listening: &AtomicBool) {
    let mut ears = Ears::new();
    // The two channels arrive woven together, a sample of each in turn.
    let mut woven = [0f32; READ * 2];
    loop {
        let raw: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(woven.as_mut_ptr() as *mut u8, size_of_val(&woven))
        };
        if stream.read(raw).is_err() {
            // The server went away or the device changed under us. Saying so
            // is better than a wallpaper silently freezing at its last beat.
            listening.store(false, Ordering::Relaxed);
            return;
        }
        let measured = ears.feed(&woven);
        if let Ok(mut out) = bands.lock() {
            *out = measured;
        }
    }
}

/// Which bin each band starts at, spread so that each band covers the same
/// span in pitch rather than in hertz -- the bottom four bands are the bass a
/// listener hears as separate notes, which an even spread would lump together.
fn band_edges() -> [usize; AUDIO_BANDS + 1] {
    let bins = WINDOW / 2;
    let per_bin = RATE as f32 / WINDOW as f32;
    let step = (HIGHEST / LOWEST).powf(1.0 / AUDIO_BANDS as f32);
    let mut edges = [0usize; AUDIO_BANDS + 1];
    for (band, edge) in edges.iter_mut().enumerate() {
        let frequency = LOWEST * step.powi(band as i32);
        *edge = ((frequency / per_bin).round() as usize).min(bins - 1);
    }
    // Two bands landing on one bin would leave the lower of them always empty,
    // so each is pushed up to at least one bin of its own.
    for band in 1..edges.len() {
        edges[band] = edges[band].max(edges[band - 1] + 1).min(bins);
    }
    edges
}

/// The loudness of each band in one window of samples, zero to one.
fn spectrum(samples: &[f32; WINDOW], edges: &[usize; AUDIO_BANDS + 1]) -> [f32; AUDIO_BANDS] {
    // A window that cut the samples off square would smear every tone across
    // the whole spectrum, which reads as every band lighting up at once.
    let mut real = [0f32; WINDOW];
    let mut imaginary = [0f32; WINDOW];
    for (at, sample) in samples.iter().enumerate() {
        let turn = std::f32::consts::TAU * at as f32 / WINDOW as f32;
        real[at] = sample * 0.5 * (1.0 - turn.cos());
    }
    transform(&mut real, &mut imaginary);

    let mut out = [0f32; AUDIO_BANDS];
    for (band, out) in out.iter_mut().enumerate() {
        let (from, to) = (edges[band], edges[band + 1]);
        let mut loudest = 0f32;
        for bin in from..to {
            let magnitude = (real[bin] * real[bin] + imaginary[bin] * imaginary[bin]).sqrt();
            loudest = loudest.max(magnitude / WINDOW as f32);
        }
        *out = (loudest / LOUD).sqrt().min(1.0);
    }
    out
}

/// An in-place radix-2 fast Fourier transform.
///
/// Hand-written rather than brought in: the whole of it is this, it only ever
/// runs on a power-of-two window, and a dependency would be more to carry than
/// to read.
fn transform(real: &mut [f32], imaginary: &mut [f32]) {
    let n = real.len();
    debug_assert!(n.is_power_of_two());

    // Reorder into bit-reversed positions, which is what lets the butterflies
    // below read their pairs from neighbouring slots.
    let mut target = 0usize;
    for at in 1..n {
        let mut bit = n >> 1;
        while target & bit != 0 {
            target ^= bit;
            bit >>= 1;
        }
        target |= bit;
        if at < target {
            real.swap(at, target);
            imaginary.swap(at, target);
        }
    }

    let mut span = 2;
    while span <= n {
        let turn = -std::f32::consts::TAU / span as f32;
        for start in (0..n).step_by(span) {
            for at in 0..span / 2 {
                let angle = turn * at as f32;
                let (sin, cos) = angle.sin_cos();
                let (here, there) = (start + at, start + at + span / 2);
                let re = real[there] * cos - imaginary[there] * sin;
                let im = real[there] * sin + imaginary[there] * cos;
                real[there] = real[here] - re;
                imaginary[there] = imaginary[here] - im;
                real[here] += re;
                imaginary[here] += im;
            }
        }
        span <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single tone has to land in one bin and nowhere else, or every band
    /// lights up together and a visualiser answers to nothing in particular.
    #[test]
    fn a_tone_lands_in_the_bin_its_pitch_belongs_to() {
        let wanted = 40;
        let mut real = [0f32; WINDOW];
        let mut imaginary = [0f32; WINDOW];
        for (at, real) in real.iter_mut().enumerate() {
            *real = (std::f32::consts::TAU * wanted as f32 * at as f32 / WINDOW as f32).sin();
        }
        transform(&mut real, &mut imaginary);

        let magnitude =
            |bin: usize| (real[bin] * real[bin] + imaginary[bin] * imaginary[bin]).sqrt();
        let loudest = (1..WINDOW / 2).max_by(|a, b| magnitude(*a).total_cmp(&magnitude(*b)));
        assert_eq!(loudest, Some(wanted));
        assert!(magnitude(wanted) > 100.0 * magnitude(wanted + 3));
    }

    /// Every band needs a bin of its own: two sharing one would leave the
    /// lower of them dark whatever was played.
    #[test]
    fn the_bands_rise_and_never_share_a_bin() {
        let edges = band_edges();
        for band in 1..edges.len() {
            assert!(
                edges[band] > edges[band - 1],
                "band {band} starts where {} did: {edges:?}",
                band - 1
            );
        }
        assert!(*edges.last().unwrap() <= WINDOW / 2);
    }

    /// Silence has to read as silence rather than as a floor of noise.
    #[test]
    fn silence_reads_as_nothing_in_every_band() {
        let bands = spectrum(&[0.0; WINDOW], &band_edges());
        assert_eq!(bands, [0.0; AUDIO_BANDS]);
    }

    /// A bass note belongs in the bottom bands and a hiss in the top ones.
    #[test]
    fn a_tone_shows_up_in_the_band_that_covers_it() {
        let edges = band_edges();
        let tone = |hz: f32| {
            let mut samples = [0f32; WINDOW];
            for (at, sample) in samples.iter_mut().enumerate() {
                *sample = (std::f32::consts::TAU * hz * at as f32 / RATE as f32).sin();
            }
            let bands = spectrum(&samples, &edges);
            bands
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(band, _)| band)
                .unwrap()
        };
        assert!(tone(60.0) <= 2, "60 Hz landed in band {}", tone(60.0));
        assert!(tone(8000.0) >= 13, "8 kHz landed in band {}", tone(8000.0));
        assert!(tone(60.0) < tone(1000.0) && tone(1000.0) < tone(8000.0));
    }
}
