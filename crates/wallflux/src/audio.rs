// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Listening to what the speakers are playing, as the 64 entries per ear
//! Wallpaper Engine hands its wallpapers.
//!
//! Wallpapers name entries by index, sixteen, 32 or 64 of them, and ask for
//! the loudness across a range, so the whole job here is to turn a stream of
//! samples into the same numbers Wallpaper Engine would, often enough that a
//! visualiser keeps up.
//!
//! The sound is read from the output's own monitor rather than a microphone,
//! which is what makes a wallpaper answer to the music the viewer is playing.
//! `@DEFAULT_MONITOR@` is PulseAudio's name for whichever output is current,
//! so nothing has to be chosen or followed when the viewer switches devices.
//!
//! Nothing here is required: a machine with no sound server, or one where the
//! connection drops, leaves the bands at zero, and a wallpaper reads that as
//! silence and runs at the quiet end of what it stated.
//!
//! **Listening is only done while somebody is asking.** Plasma counts a
//! recording of the speakers as using the microphone and puts its microphone
//! icon in the tray for as long as one is open -- measured. It hides only the
//! volume mixers' own meters, by name. So the stream is closed once the bands
//! have gone unread for `QUIET`, which is what a wallpaper under a window, a
//! scene that answers to nothing, and no scene at all look like from here.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use wpe_format::particle::{AUDIO_ENTRIES, Spectrum};

/// Samples read at a time. At 44 100 Hz this is 23 ms, so the bands are
/// renewed about forty times a second, which is faster than the frames that
/// read them.
pub const READ: usize = 1024;

/// Samples kept, the last two mouthfuls; each reading is measured over the
/// newest 1920 of them, as Wallpaper Engine's.
const WINDOW: usize = 2048;
/// Wallpaper Engine's rate on Windows, where the device mixes at 48 kHz, and
/// PipeWire's own here, so nothing is resampled on the way in. Its bins are
/// 25 Hz apart at this rate, and where a bass note falls between them is what
/// the bass bands hear: read at 44.1 kHz, DOOM's 35 Hz drone filled band 0 and
/// lit the D and the M, which on Windows stay dark through it (they followed
/// Windows at 0.64; 0.76 at 48 kHz).
pub const RATE: u32 = 48_000;

/// Wallpaper Engine's own reading of the sound: a
/// discrete Fourier transform of 1920 samples, no window, of which bins 1 to
/// 639 are read. Each bin's power is weighted by `TILT - cos(pi x)(1 - TILT)`,
/// `x` how far up the 639 it is, so the bass counts a five-hundredth as much as
/// the top and music comes out about level; its root goes into one of 64
/// entries, the loudest bin in each winning. An entry is placed by `x` to the
/// `PLACING` power and is never more than one ahead of the last, so the first
/// thirty each hold a single bin of 25 Hz. The whole is multiplied by the
/// samples' scale of 127, by 0.001, by 640 over half the window and by the
/// recording volume, 50 out of 200 by default, which is one.
///
/// **Nothing is held, smoothed or capped**: every reading starts from nought,
/// and music that is loud reads above one. A script's own smoothing is all the
/// smoothing there is, which is why one that is held already stands still.
const ITS_WINDOW: f32 = 1920.0;
const ITS_BINS: usize = 640;
const TILT: f32 = 0.501;
const PLACING: f32 = 0.25;
const SCALE: f32 = 127.0 * 0.001 * ITS_BINS as f32 / (ITS_WINDOW / 2.0);

/// Wallpaper Engine's recording volume as a multiplier on `SCALE`: one at
/// its default of 50, which is where `SCALE` already stands. Settings ›
/// Performance moves it, as the user's "sensitivity to music". Kept as the
/// bits of an f32, since the meter reads it on its own thread.
static GAIN: AtomicU32 = AtomicU32::new(0x3f80_0000);

/// Sets the recording volume, one being Wallpaper Engine's default.
pub fn set_gain(gain: f32) {
    GAIN.store(gain.clamp(0.0, 8.0).to_bits(), Ordering::Relaxed);
}

fn gain() -> f32 {
    f32::from_bits(GAIN.load(Ordering::Relaxed))
}

/// How long the bands may go unread before the stream is closed.
const QUIET: Duration = Duration::from_secs(1);

/// How often a closed stream looks to see whether it is wanted again, and
/// how long one that could not be opened waits before trying once more.
const IDLE: Duration = Duration::from_millis(50);
const RETRY: Duration = Duration::from_secs(1);

/// The sound the speakers are playing, split into bands.
pub struct Audio {
    bands: &'static Mutex<Spectrum>,
    /// When the bands were last read, in milliseconds after `epoch`.
    asked: &'static AtomicU64,
    epoch: Instant,
}

impl Audio {
    /// Starts listening, or returns `None` where there is nothing to listen
    /// to. A caller that gets `None` runs the scene as it would in silence.
    pub fn start() -> Option<Self> {
        // Leaked rather than shared through an Arc: there is one of these for
        // the life of the process, and the thread outlives any handle to it.
        let bands: &'static Mutex<Spectrum> = Box::leak(Box::new(Mutex::new(Spectrum::SILENT)));
        let asked: &'static AtomicU64 = Box::leak(Box::new(AtomicU64::new(0)));
        let epoch = Instant::now();

        let stream = connect()?;
        std::thread::Builder::new()
            .name("wallflux-audio".into())
            .spawn(move || listen(stream, bands, asked, epoch))
            .ok()?;
        Some(Self { bands, asked, epoch })
    }

    /// The loudness of each band in each ear, zero to one. Silence where the
    /// sound stopped being readable, which is what a wallpaper reads as a
    /// quiet room. Asking is what keeps the stream open.
    pub fn bands(&self) -> Spectrum {
        self.asked.store(since(self.epoch), Ordering::Relaxed);
        self.bands.lock().map(|b| *b).unwrap_or(Spectrum::SILENT)
    }
}

fn since(epoch: Instant) -> u64 {
    epoch.elapsed().as_millis() as u64
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
        // A fragment as long as one reading. Left to the server, pipewire-pulse
        // hands a recording over in fragments of two seconds -- measured,
        // `node.latency` 88200/44100 -- so a wallpaper moved up to two seconds
        // behind its own music: DOOM's letters, seen by the user.
        Some(&BufferAttr {
            maxlength: u32::MAX,
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
            fragsize: (READ * 2 * size_of::<f32>()) as u32,
        }),
    )
    .ok()
}

/// Turns a stream of samples into the bands a wallpaper asks for.
///
/// Kept apart from the listening so that a tool rendering a clip from a sound
/// file reads it exactly as the renderer reads the speakers: a clip that
/// measured the sound its own way would be showing something nobody gets.
pub struct Meter {
    samples: [f32; WINDOW],
}

impl Meter {
    pub fn new() -> Self {
        Self { samples: [0.0; WINDOW] }
    }

    /// Takes the next [`READ`] samples and gives the entries as they now stand.
    ///
    /// The newest mouthful goes at the end and what was there slides back, so
    /// each reading is measured over a window twice as long as it waited.
    pub fn feed(&mut self, fresh: &[f32; READ]) -> [f32; AUDIO_ENTRIES] {
        self.samples.copy_within(READ.., 0);
        self.samples[WINDOW - READ..].copy_from_slice(fresh);
        spectrum(&self.samples)
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

fn listen(stream: Simple, bands: &Mutex<Spectrum>, asked: &AtomicU64, epoch: Instant) {
    let mut stream = Some(stream);
    let mut ears = Ears::new();
    let silence = || {
        if let Ok(mut out) = bands.lock() {
            *out = Spectrum::SILENT;
        }
    };
    // The two channels arrive woven together, a sample of each in turn.
    let mut woven = [0f32; READ * 2];
    loop {
        let unread = since(epoch).saturating_sub(asked.load(Ordering::Relaxed));
        if unread > QUIET.as_millis() as u64 {
            if stream.take().is_some() {
                silence();
            }
            std::thread::sleep(IDLE);
            continue;
        }
        let Some(open) = &stream else {
            stream = connect();
            ears = Ears::new();
            if stream.is_none() {
                std::thread::sleep(RETRY);
            }
            continue;
        };
        let raw: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(woven.as_mut_ptr() as *mut u8, size_of_val(&woven))
        };
        if open.read(raw).is_err() {
            // The server went away or the device changed under us: silence
            // rather than a wallpaper frozen at its last beat, and another
            // try on the next turn.
            stream = None;
            silence();
            continue;
        }
        let measured = ears.feed(&woven);
        if let Ok(mut out) = bands.lock() {
            *out = measured;
        }
    }
}

/// The 64 entries of one window of samples, as Wallpaper Engine reads them:
/// its transform of the newest 1920, bin for bin.
fn spectrum(samples: &[f32; WINDOW]) -> [f32; AUDIO_ENTRIES] {
    heard_at(samples, gain())
}

/// The same at a given recording volume, one being Wallpaper Engine's
/// default -- apart from the setting, so it can be tested without it.
fn heard_at(samples: &[f32; WINDOW], gain: f32) -> [f32; AUDIO_ENTRIES] {
    let (real, imaginary) = its_transform(&samples[WINDOW - ITS_LENGTH..]);
    let mut out = [0f32; AUDIO_ENTRIES];
    let mut entry = 0;
    for bin in 1..ITS_BINS {
        let x = (bin - 1) as f32 / (ITS_BINS - 1) as f32;
        let power = real[bin] * real[bin] + imaginary[bin] * imaginary[bin];
        let weight = TILT - (std::f32::consts::PI * x).cos() * (1.0 - TILT);
        let placed = ((x.powf(PLACING) * AUDIO_ENTRIES as f32) as usize).min(AUDIO_ENTRIES - 1);
        entry = placed.min(if bin == 1 { 0 } else { entry + 1 });
        out[entry] = out[entry].max((weight * power).max(0.0).sqrt());
    }
    let scale = SCALE * gain;
    out.map(|level| level * scale)
}

const ITS_LENGTH: usize = ITS_WINDOW as usize;
/// 1920 is fifteen times 128.
const SPLIT: usize = 15;
const PART: usize = ITS_LENGTH / SPLIT;

/// The first [`ITS_BINS`] bins of the discrete Fourier transform of 1920
/// samples: fifteen transforms of 128, every fifteenth sample in each, put
/// together for the bins wanted (Cooley-Tukey with a radix of 15).
fn its_transform(samples: &[f32]) -> ([f32; ITS_BINS], [f32; ITS_BINS]) {
    debug_assert_eq!(samples.len(), ITS_LENGTH);
    let mut parts = [[[0f32; PART]; 2]; SPLIT];
    for (offset, [real, imaginary]) in parts.iter_mut().enumerate() {
        for (at, real) in real.iter_mut().enumerate() {
            *real = samples[at * SPLIT + offset];
        }
        transform(real, imaginary);
    }
    let mut real = [0f32; ITS_BINS];
    let mut imaginary = [0f32; ITS_BINS];
    for bin in 0..ITS_BINS {
        let (mut re, mut im) = (0f32, 0f32);
        for (offset, [part_re, part_im]) in parts.iter().enumerate() {
            let turn = -std::f32::consts::TAU * (offset * bin) as f32 / ITS_LENGTH as f32;
            let (sin, cos) = turn.sin_cos();
            let (a, b) = (part_re[bin % PART], part_im[bin % PART]);
            re += a * cos - b * sin;
            im += a * sin + b * cos;
        }
        real[bin] = re;
        imaginary[bin] = im;
    }
    (real, imaginary)
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

    /// Silence has to read as silence rather than as a floor of noise.
    /// Sensitivity to music: twice the recording volume hears every entry
    /// twice as loud, and nought hears nothing.
    #[test]
    fn the_recording_volume_scales_what_is_heard() {
        let mut samples = [0f32; WINDOW];
        for (i, s) in samples.iter_mut().enumerate() {
            *s = (i as f32 * 0.3).sin() * 0.2;
        }
        let once = heard_at(&samples, 1.0);
        let twice = heard_at(&samples, 2.0);
        assert!(once.iter().any(|&v| v > 0.0));
        for (a, b) in once.iter().zip(twice) {
            assert!((b - 2.0 * a).abs() <= 1e-4 * a.max(1.0), "{a} {b}");
        }
        assert_eq!(heard_at(&samples, 0.0), [0.0; AUDIO_ENTRIES]);
    }

    #[test]
    fn silence_reads_as_nothing_in_every_entry() {
        assert_eq!(spectrum(&[0.0; WINDOW]), [0.0; AUDIO_ENTRIES]);
    }

    fn tone(hz: f32, loudness: f32) -> [f32; AUDIO_ENTRIES] {
        let mut samples = [0f32; WINDOW];
        for (at, sample) in samples.iter_mut().enumerate() {
            *sample = loudness * (std::f32::consts::TAU * hz * at as f32 / RATE as f32).sin();
        }
        spectrum(&samples)
    }

    fn loudest(entries: [f32; AUDIO_ENTRIES]) -> usize {
        (0..AUDIO_ENTRIES).max_by(|a, b| entries[*a].total_cmp(&entries[*b])).unwrap()
    }

    /// The first thirty entries are a bin of 25 Hz each and the rest spread
    /// up to 16 kHz: a bass note near the bottom, a kilohertz about half way.
    #[test]
    fn a_tone_lands_in_the_entry_that_covers_it() {
        assert!(loudest(tone(60.0, 0.5)) <= 2, "{}", loudest(tone(60.0, 0.5)));
        assert!((31..=34).contains(&loudest(tone(1000.0, 0.5))), "{}", loudest(tone(1000.0, 0.5)));
        assert!(loudest(tone(8000.0, 0.5)) >= 52, "{}", loudest(tone(8000.0, 0.5)));
    }

    /// A tenth of full scale at 10 kHz, on a bin so none of it leaks: 127 x
    /// 0.1 x 960 for the transform, the root of its weight of 0.69, and 0.001
    /// x 2/3 -- about seven. **Not one**: nothing is capped.
    #[test]
    fn a_loud_treble_tone_reads_well_above_one() {
        let hz = 400.0 * RATE as f32 / ITS_WINDOW;
        let level = tone(hz, 0.1)[loudest(tone(hz, 0.1))];
        assert!((6.0..8.0).contains(&level), "{level}");
    }

    /// The transform of 1920 is the discrete Fourier transform, bin for bin.
    #[test]
    fn its_transform_is_the_plain_one() {
        let samples: Vec<f32> = (0..ITS_LENGTH).map(|n| ((n * 7919) % 101) as f32 / 50.0 - 1.0).collect();
        let (real, imaginary) = its_transform(&samples);
        for bin in [0, 1, 2, 37, 127, 128, 129, 400, 639] {
            let (mut re, mut im) = (0f64, 0f64);
            for (n, s) in samples.iter().enumerate() {
                let turn = -std::f64::consts::TAU * (n * bin) as f64 / ITS_LENGTH as f64;
                re += *s as f64 * turn.cos();
                im += *s as f64 * turn.sin();
            }
            assert!((real[bin] as f64 - re).abs() < 0.05 && (imaginary[bin] as f64 - im).abs() < 0.05,
                "bin {bin}: {} {} against {re} {im}", real[bin], imaginary[bin]);
        }
    }

    /// The bass is weighed down on purpose: the same tone at 50 Hz reads a
    /// small part of what it does at 10 kHz.
    #[test]
    fn the_bass_counts_far_less_than_the_treble() {
        let low = tone(50.0, 0.1).iter().cloned().fold(0.0, f32::max);
        let high = tone(10_000.0, 0.1).iter().cloned().fold(0.0, f32::max);
        assert!(low < high / 10.0, "{low} against {high}");
    }
}
