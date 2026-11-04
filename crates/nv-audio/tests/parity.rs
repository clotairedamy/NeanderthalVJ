//! Numerical parity with the Python implementation this crate replaces.
//!
//! `tools/capture_parity.py` runs a deterministic signal through
//! `visualizer/audio/analyzer.py` and writes the samples plus every output
//! field. This replays the same bytes and asserts the Rust agrees, so the
//! port cannot drift from the original while the two trees coexist.

use std::fs;

use nv_audio::frame::SPECTRUM_BINS;
use nv_audio::{AnalysisSettings, AudioAnalyzer, N_BANDS};
use serde_json::Value;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// The worst absolute deviation seen for one output field.
#[derive(Default)]
struct Worst {
    value: f64,
    frame: usize,
}

impl Worst {
    fn see(&mut self, got: f64, want: f64, frame: usize) {
        let d = (got - want).abs();
        if d > self.value {
            self.value = d;
            self.frame = frame;
        }
    }
}

/// The continuous waveform, at the float32 precision the real audio path
/// carries. The caller cuts it into windows exactly as the engine does.
fn read_input() -> Vec<f64> {
    let bytes = fs::read(format!("{FIXTURES}/input.f32")).expect(
        "missing fixtures -- they are checked in under tests/fixtures; restore them with git",
    );
    bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b) as f64).collect()
}

#[test]
fn matches_the_python_analyzer() {
    let raw = fs::read_to_string(format!("{FIXTURES}/expected.json")).expect(
        "missing fixtures -- they are checked in under tests/fixtures; restore them with git",
    );
    let doc: Value = serde_json::from_str(&raw).unwrap();
    let sr = doc["sr"].as_u64().unwrap() as u32;
    let hop = doc["hop"].as_f64().unwrap();
    let n = doc["n_frames"].as_u64().unwrap() as usize;
    let fft_size = doc["fft_size"].as_u64().unwrap() as usize;
    let hop_samples = doc["hop_samples"].as_u64().unwrap() as usize;
    let want = doc["frames"].as_array().unwrap();
    let signal = read_input();
    assert!(signal.len() >= hop_samples * n + fft_size, "fixture too short");

    let mut an = AudioAnalyzer::new(sr, AnalysisSettings::default());

    let scalars = [
        "rms",
        "punch",
        "beat_strength",
        "bpm",
        "flatness",
        "pwm",
        "pitch_hz",
        "resonance",
        "wobble",
        "wobble_hz",
        "morph",
        "beat_phase",
        "bar_phase",
        "centroid",
        "stem_energy_vocals",
    ];
    let mut worst: Vec<(String, Worst)> =
        scalars.iter().map(|s| (s.to_string(), Worst::default())).collect();
    let mut worst_bands = Worst::default();
    let mut worst_attack = Worst::default();
    let mut worst_spec = Worst::default();
    let mut worst_stem_bands = Worst::default();
    let mut beat_mismatches = Vec::new();

    for i in 0..n {
        let win = &signal[i * hop_samples..i * hop_samples + fft_size];
        let got = an.analyze(i as f64 * hop, win, None);
        let w = &want[i];

        let got_scalars = [
            got.rms,
            got.punch,
            got.beat_strength,
            got.bpm,
            got.flatness,
            got.pwm,
            got.pitch_hz,
            got.resonance,
            got.wobble,
            got.wobble_hz,
            got.morph,
            got.beat_phase,
            got.bar_phase,
            got.centroid,
            got.stems[0].energy,
        ];
        for (k, name) in scalars.iter().enumerate() {
            worst[k].1.see(got_scalars[k], w[name].as_f64().unwrap(), i);
        }
        if got.beat != w["beat"].as_bool().unwrap() {
            beat_mismatches.push(i);
        }
        for b in 0..N_BANDS {
            worst_bands.see(got.bands[b], w["bands"][b].as_f64().unwrap(), i);
            worst_attack.see(got.attack[b], w["attack"][b].as_f64().unwrap(), i);
            worst_stem_bands.see(
                got.stems[0].bands[b],
                w["stem_bands_vocals"][b].as_f64().unwrap(),
                i,
            );
        }
        for b in 0..SPECTRUM_BINS {
            worst_spec.see(got.spectrum[b], w["spectrum"][b].as_f64().unwrap(), i);
        }
    }

    println!("\nworst absolute deviation over {n} frames:");
    for (name, wr) in &worst {
        println!("  {name:<20} {:.3e}  (frame {})", wr.value, wr.frame);
    }
    for (name, wr) in [
        ("bands", &worst_bands),
        ("attack", &worst_attack),
        ("spectrum", &worst_spec),
        ("stem_bands", &worst_stem_bands),
    ] {
        println!("  {name:<20} {:.3e}  (frame {})", wr.value, wr.frame);
    }
    println!("  beat flag mismatches: {}", beat_mismatches.len());

    // The two use different FFT kernels, so agreement is to rounding, not
    // to the bit. Anything above this is a real difference in the port.
    const TOL: f64 = 1e-9;
    let mut failures = Vec::new();
    for (name, wr) in &worst {
        if wr.value > TOL {
            failures.push(format!("{name}: {:.3e} at frame {}", wr.value, wr.frame));
        }
    }
    for (name, wr) in [
        ("bands", &worst_bands),
        ("attack", &worst_attack),
        ("spectrum", &worst_spec),
        ("stem_bands", &worst_stem_bands),
    ] {
        if wr.value > TOL {
            failures.push(format!("{name}: {:.3e} at frame {}", wr.value, wr.frame));
        }
    }
    if !beat_mismatches.is_empty() {
        failures.push(format!("beat flag differs at frames {beat_mismatches:?}"));
    }
    assert!(failures.is_empty(), "port differs from Python:\n  {}", failures.join("\n  "));
}
