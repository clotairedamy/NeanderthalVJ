//! Decode a file, build its beat grid, and print what the engine sees.
//!
//!     cargo run -p nv-engine --example probe -- /path/to/track.wav

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: probe <audio file>");
        std::process::exit(2);
    };
    let t0 = std::time::Instant::now();
    let track = match nv_engine::decode::load(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    println!(
        "decoded {:.2}s, {} Hz, {} ch, {} frames in {:?}",
        track.duration(),
        track.sample_rate,
        track.channels,
        track.frames(),
        t0.elapsed()
    );

    let t1 = std::time::Instant::now();
    let mut engine = nv_engine::Engine::new();
    match engine.load(&path) {
        Ok(()) => {}
        Err(e) => {
            // the output device may be unavailable in a headless run; the
            // decode and the grid are still worth reporting
            eprintln!("note: {e}");
        }
    }
    println!(
        "beat grid: {:.2} BPM, {} beats, {} onsets ({:?})",
        engine.grid.bpm,
        engine.grid.beats.len(),
        engine.grid.onsets.len(),
        t1.elapsed()
    );
    if let Some(b) = engine.grid.beats.get(..6) {
        let s: Vec<String> = b.iter().map(|t| format!("{t:.2}")).collect();
        println!("first beats at: {}s", s.join(", "));
    }
}
