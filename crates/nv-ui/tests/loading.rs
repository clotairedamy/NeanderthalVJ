//! Loading a track, without the file dialog.
//!
//! The app crashed the first time a song was opened: a `borrow_mut()` in a
//! `match` scrutinee outlived the arm that borrowed the engine again.
//! Nothing about that is visible to the compiler, so it is pinned here.

use std::cell::RefCell;
use std::rc::Rc;

use nv_engine::Engine;
use nv_ui::engine_link::load_track;

/// A two-second 100 BPM kick pattern, written as a 16-bit wav.
fn test_wav() -> std::path::PathBuf {
    let path = std::env::temp_dir().join("nv_ui_loading_test.wav");
    let (sr, secs) = (44_100u32, 2.0);
    let n = (sr as f64 * secs) as usize;
    let mut pcm: Vec<u8> = Vec::with_capacity(n * 4 + 44);

    let data_len = (n * 4) as u32;
    pcm.extend_from_slice(b"RIFF");
    pcm.extend_from_slice(&(36 + data_len).to_le_bytes());
    pcm.extend_from_slice(b"WAVEfmt ");
    pcm.extend_from_slice(&16u32.to_le_bytes());
    pcm.extend_from_slice(&1u16.to_le_bytes()); // PCM
    pcm.extend_from_slice(&2u16.to_le_bytes()); // stereo
    pcm.extend_from_slice(&sr.to_le_bytes());
    pcm.extend_from_slice(&(sr * 4).to_le_bytes());
    pcm.extend_from_slice(&4u16.to_le_bytes());
    pcm.extend_from_slice(&16u16.to_le_bytes());
    pcm.extend_from_slice(b"data");
    pcm.extend_from_slice(&data_len.to_le_bytes());

    for i in 0..n {
        let t = i as f64 / sr as f64;
        let d = t % 0.6; // 100 BPM
        let env = (-d * 36.0).exp() * if d < 0.3 { 1.0 } else { 0.0 };
        let v = (0.8 * env * (std::f64::consts::TAU * 52.0 * d).sin() * 32_000.0) as i16;
        pcm.extend_from_slice(&v.to_le_bytes());
        pcm.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(&path, pcm).expect("write test wav");
    path
}

#[test]
fn loading_a_track_does_not_panic_and_reports_what_it_found() {
    let wav = test_wav();
    let engine: Rc<RefCell<Engine>> = Rc::new(RefCell::new(Engine::new()));

    let (duration, bpm) = match load_track(&engine, wav.to_str().unwrap()) {
        Ok(v) => v,
        // the machine running the tests may have no output device; the
        // decode and the grid are what this is really checking
        Err(e) if e.contains("audio output") || e.contains("output device") => return,
        Err(e) => panic!("loading failed: {e}"),
    };

    assert!((duration - 2.0).abs() < 0.05, "duration was {duration}");
    assert!(bpm > 0.0, "no tempo found");
    // a track waits to be played: loading one never starts the sound
    assert!(!engine.borrow().is_playing(), "loading a track started playing it");
    // the engine stays usable afterwards -- the crash was on the second borrow
    assert!(engine.borrow().is_loaded());
    assert!(engine.borrow_mut().tick().is_some());
}

#[test]
fn a_file_that_is_not_audio_is_reported_not_fatal() {
    let junk = std::env::temp_dir().join("nv_ui_not_audio.wav");
    std::fs::write(&junk, b"this is not a wav file at all").unwrap();
    let engine: Rc<RefCell<Engine>> = Rc::new(RefCell::new(Engine::new()));

    let err = load_track(&engine, junk.to_str().unwrap()).unwrap_err();
    assert!(!err.is_empty(), "a failed load must say why");
    // and the engine is still there to try again with
    assert!(!engine.borrow().is_loaded());
}
