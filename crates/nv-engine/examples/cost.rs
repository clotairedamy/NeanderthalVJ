fn main() {
    let path = std::env::args().nth(1).expect("usage: cost <audio file>");
    let mut e = nv_engine::Engine::new();
    let t0 = std::time::Instant::now();
    if let Err(err) = e.load(&path) {
        eprintln!("note: {err}");
    }
    println!("load (decode + beat grid): {:?}", t0.elapsed());

    for _ in 0..20 {
        let _ = e.tick();
    }
    let n = 400;
    let t = std::time::Instant::now();
    for _ in 0..n {
        let _ = e.tick();
    }
    let per = t.elapsed().as_secs_f64() / n as f64;
    println!(
        "engine.tick(): {:.3} ms  ({:.1}% of a 60 fps frame)",
        per * 1e3,
        per / 0.0166 * 100.0
    );
}
