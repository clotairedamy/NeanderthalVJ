//! Export a track as an Instagram clip from the command line.
//!
//!     cargo run --release -p nv-export --example export -- track.wav out.mp4 [reel|square] [head|grid|generative]
use std::sync::Arc;

fn main() {
    let mut a = std::env::args().skip(1);
    let (Some(audio), Some(out)) = (a.next(), a.next()) else {
        eprintln!("usage: export <audio> <out.mp4> [reel|square] [head|grid|generative]");
        std::process::exit(2);
    };
    let format = match a.next().as_deref() {
        Some("square") => nv_export::Format::Square,
        _ => nv_export::Format::Reel,
    };
    let params = match a.next().as_deref() {
        Some("grid") => nv_render::Params { grid: Some(Default::default()), ..Default::default() },
        Some("generative") => {
            nv_render::Params { generative: Some(Default::default()), ..Default::default() }
        }
        _ => Default::default(),
    };
    let job = nv_export::Job {
        audio: audio.into(),
        out: out.into(),
        format,
        fps: 60,
        params,
        camera: Default::default(),
        assets: std::env::var("NV_ASSETS").unwrap_or_else(|_| "assets".into()).into(),
        start: 0.0,
        length: 0.0,
        bitrate_mbps: 24,
        orbit: 0.0,
    };
    let progress = Arc::new(nv_export::Progress::default());
    let t0 = std::time::Instant::now();
    let handle = nv_export::spawn(job, progress.clone());
    while !handle.is_finished() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        eprint!("\r{:5.1}%", progress.fraction() * 100.0);
    }
    eprintln!();
    match handle.join().unwrap() {
        Ok(p) => {
            let frames = progress.done.load(std::sync::atomic::Ordering::Relaxed);
            let secs = t0.elapsed().as_secs_f64();
            println!(
                "wrote {} — {frames} frames in {secs:.1}s ({:.0} fps)",
                p.display(),
                frames as f64 / secs
            );
        }
        Err(e) => {
            eprintln!("export failed: {e}");
            std::process::exit(1);
        }
    }
}
