//! Tempo estimation, matching `librosa.feature.tempo`.
//!
//! An autocorrelation tempogram of the onset envelope, averaged over time
//! and weighted by a log-normal prior centred on 120 BPM, which is what
//! stops the estimate from locking onto a half- or double-time peak.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::window::hann_periodic;

/// `scipy.fft.next_fast_len(n, real=True)`: the next 5-smooth size.
pub fn next_fast_len(n: usize) -> usize {
    if n <= 6 {
        return n.max(1);
    }
    let mut best = usize::MAX;
    let mut p5 = 1usize;
    while p5 < n * 2 {
        let mut p35 = p5;
        while p35 < n * 2 {
            // round p35 up to the next power of two multiple
            let mut v = p35;
            while v < n {
                v *= 2;
            }
            best = best.min(v);
            if p35 > usize::MAX / 3 {
                break;
            }
            p35 *= 3;
        }
        if p5 > usize::MAX / 5 {
            break;
        }
        p5 *= 5;
    }
    best
}

/// `librosa.tempo_frequencies`: the BPM each tempogram lag corresponds to.
/// Lag 0 is an infinite period; the caller gives it `-inf` prior weight.
pub fn tempo_frequencies(n_bins: usize, sr: f64, hop: usize) -> Vec<f64> {
    (0..n_bins)
        .map(|k| if k == 0 { f64::INFINITY } else { 60.0 * sr / (hop as f64 * k as f64) })
        .collect()
}

/// `np.pad(x, w, mode="linear_ramp", end_values=[0, 0])`.
fn pad_linear_ramp(x: &[f32], w: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(x.len() + 2 * w);
    // `linspace(0, x[0], num=w, endpoint=False)`
    let first = *x.first().unwrap_or(&0.0);
    for i in 0..w {
        out.push(first * i as f32 / w as f32);
    }
    out.extend_from_slice(x);
    // the mirror of the head: ramps from the last sample back down to 0
    let last = *x.last().unwrap_or(&0.0);
    for i in 0..w {
        out.push(last * (w - 1 - i) as f32 / w as f32);
    }
    out
}

struct AutoCorr {
    n_pad: usize,
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    buf: Vec<Complex<f32>>,
}

impl AutoCorr {
    fn new(n: usize) -> Self {
        let n_pad = next_fast_len(2 * n - 1);
        let mut planner = FftPlanner::new();
        Self {
            n_pad,
            fwd: planner.plan_fft_forward(n_pad),
            inv: planner.plan_fft_inverse(n_pad),
            buf: vec![Complex::new(0.0, 0.0); n_pad],
        }
    }

    /// `librosa.autocorrelate(x)[..len]`, via the power spectrum.
    fn run(&mut self, x: &[f32], out: &mut [f32]) {
        self.buf.iter_mut().for_each(|c| *c = Complex::new(0.0, 0.0));
        for (b, v) in self.buf.iter_mut().zip(x) {
            b.re = *v;
        }
        self.fwd.process(&mut self.buf);
        // |X|^2 is real and even, so the inverse is real by construction
        for c in self.buf.iter_mut() {
            *c = Complex::new(c.norm_sqr(), 0.0);
        }
        self.inv.process(&mut self.buf);
        let scale = 1.0 / self.n_pad as f32;
        for (o, c) in out.iter_mut().zip(&self.buf) {
            *o = c.re * scale;
        }
    }
}

/// `librosa.feature.tempogram(onset_envelope, sr, hop_length, win_length)`.
///
/// Returns `win_length` lags by `env.len()` frames, column-major.
pub fn tempogram(env: &[f32], win_length: usize) -> Vec<Vec<f32>> {
    let n = env.len();
    let w = win_length / 2;
    let padded = pad_linear_ramp(env, w);
    let ac_window = hann_periodic(win_length);
    let mut ac = AutoCorr::new(win_length);

    let mut cols = Vec::with_capacity(n);
    let mut windowed = vec![0.0f32; win_length];
    let mut corr = vec![0.0f32; win_length];
    for k in 0..n {
        if k + win_length > padded.len() {
            break;
        }
        for i in 0..win_length {
            windowed[i] = padded[k + i] * ac_window[i];
        }
        ac.run(&windowed, &mut corr);
        // `util.normalize(norm=inf)`: scale each lag column by its peak,
        // leaving a column that is all but silent untouched
        let peak = corr.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        let scale = if peak < f32::MIN_POSITIVE { 1.0 } else { peak };
        cols.push(corr.iter().map(|v| v / scale).collect());
    }
    cols
}

/// `librosa.feature.tempo(onset_envelope, sr, hop_length)` with librosa's
/// defaults: an 8-second window, a prior centred on 120 BPM, 320 BPM cap.
pub fn estimate_tempo(env: &[f32], sr: f64, hop: usize) -> f64 {
    const START_BPM: f64 = 120.0;
    const STD_BPM: f64 = 1.0;
    const MAX_TEMPO: f64 = 320.0;
    const AC_SIZE: f64 = 8.0;

    // librosa's `time_to_frames`: seconds to samples, then integer-divided
    // by the hop -- not rounded
    let win_length = ((AC_SIZE * sr) as usize) / hop;
    let tg = tempogram(env, win_length);
    if tg.is_empty() {
        return 0.0;
    }
    // average the tempogram over time
    let mut tg_agg = vec![0.0f64; win_length];
    for col in &tg {
        for (a, v) in tg_agg.iter_mut().zip(col) {
            *a += *v as f64;
        }
    }
    for a in &mut tg_agg {
        *a /= tg.len() as f64;
    }

    let bpms = tempo_frequencies(win_length, sr, hop);
    // everything at or above the cap is ruled out outright
    let max_idx = bpms.iter().position(|b| *b < MAX_TEMPO).unwrap_or(0);

    let mut best = (f64::NEG_INFINITY, 0usize);
    for (k, &bpm) in bpms.iter().enumerate() {
        if k < max_idx || !bpm.is_finite() {
            continue;
        }
        let logprior = -0.5 * ((bpm.log2() - START_BPM.log2()) / STD_BPM).powi(2);
        let score = (1e6 * tg_agg[k]).ln_1p() + logprior;
        if score > best.0 {
            best = (score, k);
        }
    }
    bpms[best.1]
}
