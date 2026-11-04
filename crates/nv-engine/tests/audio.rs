//! The playback path, measured rather than listened to.
//!
//! "The sound is weird" was twice diagnosed by reasoning and twice wrong,
//! so the resampler is checked here against what it is supposed to do:
//! keep the pitch, and not fold energy into other frequencies.

use nv_engine::player::fill;

const SRC: f64 = 44_100.0;
const DST: f64 = 48_000.0;

/// Peak bin of a real DFT, in Hz, plus how much energy sits away from it.
fn spectrum_peak(x: &[f32], rate: f64) -> (f64, f64) {
    let n = 1 << 14;
    let x: Vec<f64> = x.iter().take(n).map(|v| *v as f64).collect();
    let mut mag = vec![0.0; n / 2];
    // a plain DFT over the few bins that matter is enough and keeps this
    // test free of an FFT dependency
    for (k, m) in mag.iter_mut().enumerate().skip(1) {
        let (mut re, mut im) = (0.0, 0.0);
        let w = std::f64::consts::TAU * k as f64 / n as f64;
        for (i, v) in x.iter().enumerate() {
            // Hann, so the tone does not smear across neighbours
            let win = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
            re += v * win * (w * i as f64).cos();
            im -= v * win * (w * i as f64).sin();
        }
        *m = (re * re + im * im).sqrt();
    }
    let peak = mag.iter().skip(1).enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0 + 1;
    let total: f64 = mag.iter().map(|m| m * m).sum();
    // energy outside the peak and its two neighbours
    let near: f64 = mag[peak - 1..=peak + 1].iter().map(|m| m * m).sum();
    (peak as f64 * rate / n as f64, ((total - near) / total).sqrt())
}

fn sine(freq: f64, rate: f64, frames: usize) -> Vec<f32> {
    (0..frames)
        .flat_map(|i| {
            let v = (std::f64::consts::TAU * freq * i as f64 / rate).sin() as f32 * 0.8;
            [v, v]
        })
        .collect()
}

#[test]
fn resampling_keeps_the_pitch() {
    let frames = 1 << 16;
    let src = sine(440.0, SRC, frames);
    let mut out = vec![0.0f32; (1 << 15) * 2];
    let (_, peak) = fill(&mut out, 2, &src, 2, frames, 0.0, SRC / DST);

    let left: Vec<f32> = out.chunks(2).map(|c| c[0]).collect();
    let (hz, stray) = spectrum_peak(&left, DST);
    println!(
        "440 Hz at 44.1k -> 48k: peak {hz:.1} Hz, stray energy {:.2}%, peak {peak:.2}",
        stray * 100.0
    );

    assert!((hz - 440.0).abs() < 5.0, "pitch moved: {hz:.1} Hz, expected 440");
    assert!(stray < 0.05, "{:.1}% of the energy is not the tone", stray * 100.0);
    assert!((peak - 0.8).abs() < 0.02, "level changed: {peak}");
}

#[test]
fn a_high_tone_survives_resampling() {
    // the rate is where nearest-neighbour resampling does its worst
    let frames = 1 << 16;
    let src = sine(8_000.0, SRC, frames);
    let mut out = vec![0.0f32; (1 << 15) * 2];
    fill(&mut out, 2, &src, 2, frames, 0.0, SRC / DST);
    let left: Vec<f32> = out.chunks(2).map(|c| c[0]).collect();
    let (hz, stray) = spectrum_peak(&left, DST);
    println!("8 kHz at 44.1k -> 48k: peak {hz:.1} Hz, stray {:.2}%", stray * 100.0);
    assert!((hz - 8_000.0).abs() < 20.0, "pitch moved: {hz:.1} Hz");
    assert!(stray < 0.15, "{:.1}% stray energy at 8 kHz", stray * 100.0);
}

#[test]
fn playing_at_the_same_rate_is_a_copy() {
    let frames = 4096;
    let src = sine(1_000.0, DST, frames);
    let mut out = vec![0.0f32; 2048 * 2];
    fill(&mut out, 2, &src, 2, frames, 0.0, 1.0);
    for (i, c) in out.chunks(2).enumerate() {
        assert!((c[0] - src[i * 2]).abs() < 1e-6, "sample {i} changed at a 1:1 rate");
    }
}

#[test]
fn mono_material_reaches_both_ears() {
    let frames = 512;
    let src: Vec<f32> = (0..frames).map(|i| (i as f32 / 64.0).sin() * 0.5).collect();
    let mut out = vec![0.0f32; 256 * 2];
    fill(&mut out, 2, &src, 1, frames, 0.0, 1.0);
    assert!(out.chunks(2).all(|c| c[0] == c[1]), "a mono file must feed both channels");
    assert!(out.iter().any(|v| *v != 0.0), "silence from a mono file");
}
