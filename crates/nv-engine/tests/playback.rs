//! Speed, looping and the pitch-kept copy, without a sound card.

use nv_engine::engine::stretch;
use nv_engine::player::{advance, Stretched};

/// A ramp, so the frame a sample came from can be read off its value.
fn ramp(frames: usize) -> Vec<f32> {
    (0..frames).flat_map(|i| [i as f32, i as f32]).collect()
}

#[test]
fn speed_moves_the_cursor_faster_without_a_stretched_copy() {
    let src = ramp(1000);
    let mut out = vec![0.0; 2 * 100];
    let (pos, _, ended) = advance(&mut out, 2, &src, None, 2, 1000, 0.0, 1.0, 2.0, false);
    assert!((pos - 200.0).abs() < 1e-9, "100 frames at 2x should cover 200, got {pos}");
    assert!(!ended);
    // and what it played came from twice as far along
    assert!((out[2 * 50] - 100.0).abs() < 1e-3);
}

#[test]
fn a_matching_stretched_copy_is_played_and_the_cursor_stays_in_the_originals_time() {
    let src = ramp(1000);
    // a stand-in for the stretch: half as long, every value marked
    let copy = Stretched { samples: vec![-1.0; 2 * 500], speed: 2.0, frames: 500 };
    let mut out = vec![0.0; 2 * 100];
    let (pos, _, _) = advance(&mut out, 2, &src, Some(&copy), 2, 1000, 400.0, 1.0, 2.0, false);
    assert!(out.iter().all(|s| *s == -1.0), "the stretched copy was not the one played");
    assert!((pos - 600.0).abs() < 1e-9, "100 stretched frames are 200 original ones: {pos}");

    // a copy made for another speed is ignored
    let mut out = vec![0.0; 2 * 10];
    advance(&mut out, 2, &src, Some(&copy), 2, 1000, 0.0, 1.0, 1.5, false);
    assert!(out.iter().all(|s| *s >= 0.0));
}

#[test]
fn looping_wraps_to_the_start_instead_of_ending() {
    let src = ramp(100);
    let mut out = vec![0.0; 2 * 30];
    let (pos, _, ended) = advance(&mut out, 2, &src, None, 2, 100, 90.0, 1.0, 1.0, true);
    assert!(!ended);
    assert!((pos - 20.0).abs() < 1e-9, "should have wrapped to frame 20, got {pos}");
    let (_, _, ended) = advance(&mut out, 2, &src, None, 2, 100, 90.0, 1.0, 1.0, false);
    assert!(ended);
}

#[test]
fn stretching_keeps_the_pitch_and_changes_the_length() {
    let sr = 44_100.0f32;
    let frames = 44_100 * 2;
    let tone: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let v = (std::f32::consts::TAU * 440.0 * i as f32 / sr).sin() * 0.5;
            [v, v]
        })
        .collect();
    let s = stretch(&tone, 2, 1.25);
    let expected = (frames as f64 / 1.25) as usize;
    assert!(s.frames.abs_diff(expected) < 2048, "{} frames, expected about {expected}", s.frames);

    // count zero crossings over the middle second: the same 440 Hz
    let left: Vec<f32> = s.samples.iter().step_by(2).copied().collect();
    let mid = &left[left.len() / 4..left.len() / 4 + 44_100];
    let crossings = mid.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    assert!((crossings as i64 - 880).abs() < 20, "pitch moved: {crossings} crossings a second");
}
