//! Load a file, play it, and print what the analysis sees as it goes.
//!
//!     cargo run --release -p nv-engine --example play -- track.wav [seconds]

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: play <audio file> [seconds]");
        std::process::exit(2);
    };
    let secs: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3.0);

    let mut engine = nv_engine::Engine::new();
    if let Err(e) = engine.load(&path) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    println!(
        "loaded {:.2}s, {:.1} BPM, {} beats, {} onsets",
        engine.duration(),
        engine.grid.bpm,
        engine.grid.beats.len(),
        engine.grid.onsets.len()
    );

    engine.set_playing(true);
    println!("\n   time   pos    rms  bands(low..high)            peak dB  beat");
    let start = std::time::Instant::now();
    let mut beats_seen = 0;
    while start.elapsed().as_secs_f64() < secs {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(s) = engine.tick() else { continue };
        if s.beat {
            beats_seen += 1;
        }
        let bands: String =
            s.frame.bands.iter().map(|b| format!("{:.2} ", b)).collect::<Vec<_>>().join("");
        println!(
            "  {:5.2}  {:5.2}  {:.3}  {bands} {:7.1}  {}",
            start.elapsed().as_secs_f64(),
            s.position,
            s.frame.rms,
            s.peak_db,
            if s.beat { "●" } else { "" }
        );
    }
    engine.set_playing(false);
    println!(
        "\nplayback advanced to {:.2}s, {beats_seen} grid beats crossed",
        engine.tick().map_or(0.0, |s| s.position)
    );
}
