//! Check the Syphon output without Resolume: find the "NeanderthalVJ"
//! source, receive frames from it for a few seconds, and save the last one.
//!
//!     cargo run --release -p nv-ui --features syphon --example syphon_check -- out.png
//!
//! Start NeanderthalVJ (built with `--features syphon`) and tick "Share the
//! visuals through Syphon" in its Export tab first.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "syphon.png".into());
    let servers = syphon_rs::discover_servers(Duration::from_secs(3));
    for s in &servers {
        println!("found Syphon source \"{}\" from {}", s.name, s.app_name);
    }
    let Some(source) = servers.iter().find(|s| s.name == nv_ui::syphon::SERVER_NAME) else {
        eprintln!(
            "no \"{}\" source: is the app running with Syphon ticked?",
            nv_ui::syphon::SERVER_NAME
        );
        std::process::exit(1);
    };

    let arrived = Arc::new(AtomicU32::new(0));
    let count = arrived.clone();
    let client = syphon_rs::Client::new(source, move || {
        count.fetch_add(1, Ordering::Relaxed);
    })
    .expect("could not connect to the source");

    // frames arrive on Syphon's own threads; count them for a few seconds
    let start = Instant::now();
    let mut last = None;
    while start.elapsed() < Duration::from_secs(4) {
        if let Some(frame) = client.new_frame() {
            last = Some(frame);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let n = arrived.load(Ordering::Relaxed);
    let Some(frame) = last else {
        eprintln!("connected, but no frame arrived in 4 s");
        std::process::exit(1);
    };
    let (w, h) = (frame.width(), frame.height());
    let rgba = frame.read_rgba_pixels();
    let lit = rgba.chunks(4).filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 60).count();
    println!(
        "received {n} frames in 4 s (~{:.0} fps), {w}x{h}, {:.1}% of pixels lit",
        n as f32 / 4.0,
        lit as f32 * 100.0 / (w * h).max(1) as f32
    );
    save_png(&out, w, h, &rgba);
    println!("saved the last frame to {out}");
}

/// A plain RGBA PNG, without pulling in an image crate.
fn save_png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    fn crc(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn adler(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &x in data {
            a = (a + x as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    let mut raw = Vec::with_capacity((w * 4 + 1) as usize * h as usize);
    for row in rgba.chunks((w * 4) as usize) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    // zlib with stored (uncompressed) blocks
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        let len = block.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler(&raw).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    std::fs::write(path, png).expect("could not write the picture");
}
