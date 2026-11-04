//! The head's movement, against the Python it is ported from.
//!
//! This exists because the movement was first written from memory instead
//! of ported: it shook on every beat, never settled, and deformed the head
//! far harder than the original. Nothing about that is a compile error, so
//! it is pinned here.
//!
//! The random draws cannot match -- the Python picks the slice seed, the
//! band count and the shake's random walk from NumPy's generator -- so
//! this checks everything else, which is the part that decides how the
//! head reads.

use nv_audio::AnalysisFrame;
use nv_render::{BustMotion, Knobs, Uniforms};
use serde_json::Value;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/motion.json");

#[test]
fn movement_matches_the_python() {
    let raw = std::fs::read_to_string(FIXTURE).expect(
        "missing fixture -- they are checked in under tests/fixtures; restore them with git",
    );
    let doc: Value = serde_json::from_str(&raw).unwrap();
    let dt = doc["dt"].as_f64().unwrap() as f32;
    let inputs = doc["inputs"].as_array().unwrap();
    let outputs = doc["outputs"].as_array().unwrap();

    let mut motion = BustMotion::default();
    let mut u = Uniforms::default();
    // The pose (yaw, pitch, swell) departs from the Python on purpose --
    // see `motion.rs` -- and is checked by `the_pose_stays_calm` instead.
    let fields = [
        "glitch",
        "smear",
        "dust",
        "dissipate",
        "diss_t",
        "explode",
        "low",
        "mid",
        "high",
        "bright",
        "time",
    ];
    let mut worst = vec![(0.0f64, 0usize); fields.len()];

    for (i, inp) in inputs.iter().enumerate() {
        let mut f = AnalysisFrame { rms: inp["rms"].as_f64().unwrap(), ..Default::default() };
        for (b, v) in inp["bands"].as_array().unwrap().iter().enumerate() {
            f.bands[b] = v.as_f64().unwrap();
        }
        for (b, v) in inp["attack"].as_array().unwrap().iter().enumerate() {
            f.attack[b] = v.as_f64().unwrap();
        }
        let beat = inp["beat"].as_bool().unwrap();
        f.beat = beat;
        f.beat_strength = inp["beat_strength"].as_f64().unwrap();
        f.flatness = inp["flatness"].as_f64().unwrap();
        f.bar_phase = inp["bar_phase"].as_f64().unwrap();

        // the Python's own value, which the app turns down by default
        let k = Knobs { drum_glitch: 1.0, ..Knobs::default() };
        motion.update(&f, dt, beat, k, &mut u);

        let got = |name: &str| -> f64 {
            (match name {
                "glitch" => u.glitch,
                "smear" => u.smear,
                "dust" => u.dust,
                "dissipate" => u.dissipate,
                "diss_t" => u.diss_t,
                "explode" => u.explode,
                "low" => u.low,
                "mid" => u.mid,
                "high" => u.high,
                "bright" => u.bright,
                "time" => u.time,
                _ => unreachable!(),
            }) as f64
        };
        for (k, name) in fields.iter().enumerate() {
            let want = outputs[i][name].as_f64().unwrap();
            let d = (got(name) - want).abs();
            if d > worst[k].0 {
                worst[k] = (d, i);
            }
        }
    }

    println!("worst deviation from the Python over {} frames:", inputs.len());
    for (k, name) in fields.iter().enumerate() {
        println!("  {name:<10} {:.3e}  (frame {})", worst[k].0, worst[k].1);
    }
    let bad: Vec<String> = fields
        .iter()
        .enumerate()
        .filter(|(k, _)| worst[*k].0 > 1e-4)
        .map(|(k, n)| format!("{n}: {:.3e} at frame {}", worst[k].0, worst[k].1))
        .collect();
    assert!(bad.is_empty(), "movement differs from the Python:\n  {}", bad.join("\n  "));
}

#[test]
fn the_head_is_still_in_silence() {
    // silence should be perfectly still: the clock stops, the dust settles
    let mut motion = BustMotion::default();
    let mut u = Uniforms::default();
    let quiet = AnalysisFrame::default();
    for _ in 0..600 {
        motion.update(&quiet, 1.0 / 60.0, false, Knobs::default(), &mut u);
    }
    assert!(u.glitch < 1e-3, "the head is still slicing in silence: {}", u.glitch);
    assert!(u.smear < 1e-3, "still smearing in silence: {}", u.smear);
    assert!(u.dust < 0.05, "still shedding dust in silence: {}", u.dust);
    assert!(u.shake.iter().all(|s| s.abs() < 1e-4), "still shaking in silence");
    assert!(u.explode < 1e-3, "still exploding in silence");
}

#[test]
fn the_pose_stays_calm() {
    // loud synth with a deep LFO and a beat every half second: the Python
    // swung the head through ~75 degrees here
    let mut motion = BustMotion::default();
    let mut u = Uniforms::default();
    let k = Knobs { synth: true, ..Knobs::default() };
    let (mut prev, mut worst_step) = ((u.yaw, u.pitch), 0.0f32);
    for i in 0..1200 {
        let f = AnalysisFrame {
            rms: 0.9,
            bands: [0.9; 7],
            wobble: 1.0,
            wobble_hz: 2.0,
            beat_strength: 1.0,
            ..Default::default()
        };
        motion.update(&f, 1.0 / 60.0, i % 30 == 0, k, &mut u);
        assert!((u.yaw - nv_render::motion::REST_YAW).abs() < 0.2, "yaw {}", u.yaw);
        assert!((u.pitch - nv_render::motion::REST_PITCH).abs() < 0.1, "pitch {}", u.pitch);
        if i > 0 {
            worst_step = worst_step.max((u.yaw - prev.0).abs()).max((u.pitch - prev.1).abs());
        }
        prev = (u.yaw, u.pitch);
    }
    assert!(worst_step < 0.012, "the head snapped {worst_step} rad in one frame");
}

#[test]
fn a_breakdown_dims_and_the_drop_bursts() {
    let mut motion = BustMotion::default();
    let mut u = Uniforms::default();
    let mut step = |rms: f64, secs: f32, motion: &mut BustMotion| {
        let mut dropped = false;
        for _ in 0..(secs * 60.0) as usize {
            let f = AnalysisFrame { rms, ..Default::default() };
            motion.update(&f, 1.0 / 60.0, false, Knobs::default(), &mut u);
            dropped |= motion.dropped();
        }
        dropped
    };
    step(0.6, 20.0, &mut motion);
    assert!(motion.intensity() > 0.95, "steady music should be at full strength");
    assert!(!step(0.6, 5.0, &mut motion), "steady music is not a drop");
    step(0.12, 6.0, &mut motion);
    assert!(motion.intensity() < 0.8, "a breakdown should dim: {}", motion.intensity());
    assert!(step(0.8, 1.0, &mut motion), "the music coming back in is a drop");
    assert!(motion.intensity() > 0.95);
}
