//! Prints the bands the renderer hears, one meter per ear, for a few seconds.
//!
//! ```text
//! cargo run -p wallflux --example listen -- [seconds]
//! ```

#![allow(dead_code)]

#[path = "../src/audio.rs"]
mod audio;

use wpe_format::particle::AUDIO_BANDS;

fn main() {
    let seconds: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5.0);
    let Some(ears) = audio::Audio::start() else {
        eprintln!("nothing to listen to");
        std::process::exit(1);
    };
    let started = std::time::Instant::now();
    let mut peak = [[0f32; AUDIO_BANDS]; 2];
    while started.elapsed().as_secs_f32() < seconds {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let sound = ears.bands();
        for (peak, band) in peak[0].iter_mut().zip(sound.left) {
            *peak = peak.max(band);
        }
        for (peak, band) in peak[1].iter_mut().zip(sound.right) {
            *peak = peak.max(band);
        }
        println!("L[{}]  P[{}]", meter(&sound.left), meter(&sound.right));
    }
    println!("\nnejhlasitejsi na pasmo (levy / pravy):");
    for (band, (left, right)) in peak[0].iter().zip(peak[1]).enumerate() {
        println!("  pasmo {band:>2}: {left:.3} / {right:.3}");
    }
}

fn meter(bands: &[f32; AUDIO_BANDS]) -> String {
    bands
        .iter()
        .map(|b| {
            " .:-=+*#@"
                .chars()
                .nth((b * 8.0).round() as usize)
                .unwrap_or('@')
        })
        .collect()
}
