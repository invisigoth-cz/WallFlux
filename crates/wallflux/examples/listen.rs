//! Prints the sixteen bands the renderer hears, as a meter, for a few seconds.
//!
//! ```text
//! cargo run -p wallflux --example listen -- [seconds]
//! ```

#[path = "../src/audio.rs"]
mod audio;

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
    let mut peak = [0f32; 16];
    while started.elapsed().as_secs_f32() < seconds {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let bands = ears.bands();
        for (peak, band) in peak.iter_mut().zip(bands) {
            *peak = peak.max(band);
        }
        let bar: String = bands
            .iter()
            .map(|b| {
                " .:-=+*#@"
                    .chars()
                    .nth((b * 8.0).round() as usize)
                    .unwrap_or('@')
            })
            .collect();
        println!("[{bar}]  {:.2}", bands.iter().sum::<f32>() / 16.0);
    }
    println!("\nnejhlasitejsi na pasmo:");
    for (band, peak) in peak.iter().enumerate() {
        println!("  pasmo {band:>2}: {peak:.3}");
    }
}
