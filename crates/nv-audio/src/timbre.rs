//! Timbre features for synth material: pulse width, wobble and morphing.
//!
//! A synth line (a Crave, a mono bass, any oscillator through a filter) can
//! move enormously while its level barely changes: an LFO opens the filter,
//! pulse-width modulation hollows the tone out and fills it back in, the
//! resonance whistles. Band energies and RMS -- what the rest of the
//! analysis reads -- barely notice any of that, which is why such a part
//! can look static on screen while it sounds anything but.
//!
//! Three cheap measures, all pure functions of the magnitude spectrum:
//!
//! - [`pulse_width`]: the balance of even against odd harmonics. A 50%
//!   square has almost no even harmonics; narrowing the pulse fills them
//!   in. So this rises and falls exactly as a PWM sweep does, without
//!   needing the pitch.
//! - [`WobbleTracker`]: depth and rate of any slow (0.2-12 Hz) modulation
//!   of a signal -- an LFO on the filter, a tremolo, a hand on a knob.
//! - [`spectral_morph`]: how fast the *shape* of the spectrum is changing,
//!   ignoring how loud it is. A filter sweep reads high, a steady chord low.

use crate::num::{clip, median, percentile_sorted, round_half_even};

/// Where a synth's fundamental lives.
pub const F0_MIN: f64 = 40.0;
pub const F0_MAX: f64 = 1000.0;
pub const N_HARMONICS: usize = 10;

/// The oscillator's pitch: the lowest strong partial, 0 if none.
///
/// Not simply the loudest bin -- with the filter resonating, the whistle at
/// the cutoff can be louder than the fundamental, and locking onto it would
/// report the wrong pitch (and wreck the pulse-width reading, which counts
/// harmonics of it). Among the peaks that stand out, the lowest one wins.
pub fn fundamental(mag: &[f64], freqs: &[f64]) -> f64 {
    let in_band = |i: usize| freqs[i] >= F0_MIN && freqs[i] <= F0_MAX;
    let band: Vec<f64> = (0..mag.len()).filter(|&i| in_band(i)).map(|i| mag[i]).collect();
    if band.is_empty() {
        return 0.0;
    }
    // the spectrum with everything outside the band zeroed
    let sub: Vec<f64> = (0..mag.len()).map(|i| if in_band(i) { mag[i] } else { 0.0 }).collect();
    let top = sub.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if top <= 0.0 || top < 2.2 * (median(&band) + 1e-12) {
        return 0.0; // no clear tone
    }
    // local maxima standing within 9 dB of the strongest; the lowest wins
    let lowest_peak = (1..sub.len().saturating_sub(1))
        .find(|&i| sub[i] > sub[i - 1] && sub[i] >= sub[i + 1] && sub[i] > top * 0.35);
    let i = lowest_peak.unwrap_or_else(|| {
        // `np.argmax` takes the first maximum on a tie
        let mut best = 0usize;
        for k in 1..sub.len() {
            if sub[k] > sub[best] {
                best = k;
            }
        }
        best
    });
    // parabolic interpolation across the peak, for sub-bin precision
    let shift = if i > 0 && i < mag.len() - 1 {
        let (a, b, c) = (mag[i - 1], mag[i], mag[i + 1]);
        let denom = a - 2.0 * b + c;
        if denom.abs() > 1e-12 { 0.5 * (a - c) / denom } else { 0.0 }
    } else {
        0.0
    };
    let step = freqs[1] - freqs[0];
    (freqs[i] + shift * step).max(0.0)
}

/// Energy at each harmonic of `f0`, peak-picked over a small window.
pub fn harmonic_series(mag: &[f64], freqs: &[f64], f0: f64, n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n];
    if f0 <= 0.0 {
        return out;
    }
    let step = freqs[1] - freqs[0];
    for k in 1..=n {
        let b = round_half_even(k as f64 * f0 / step);
        if b < 0 || b as usize >= mag.len() - 1 {
            break;
        }
        let b = b as usize;
        let lo = b.saturating_sub(2);
        let hi = (b + 3).min(mag.len());
        out[k - 1] = mag[lo..hi].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

/// 0 for a hollow 50% square, rising as the pulse narrows.
///
/// Even harmonics vanish at a 50% duty cycle and grow as the width moves
/// away from it, so the even/odd balance follows a PWM sweep directly.
pub fn pulse_width(mag: &[f64], freqs: &[f64], f0: f64) -> f64 {
    let f0 = if f0 != 0.0 { f0 } else { fundamental(mag, freqs) };
    let h = harmonic_series(mag, freqs, f0, N_HARMONICS);
    let total: f64 = h.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let odd: f64 = h.iter().step_by(2).sum(); // 1st, 3rd, 5th...
    let even: f64 = h.iter().skip(1).step_by(2).sum(); // 2nd, 4th, 6th...
    clip(2.0 * even / (odd + even + 1e-12), 0.0, 1.0)
}

/// Share of the spectrum that sits on a harmonic series (0..1).
pub fn harmonicity(mag: &[f64], freqs: &[f64], f0: f64) -> f64 {
    let f0 = if f0 != 0.0 { f0 } else { fundamental(mag, freqs) };
    if f0 <= 0.0 {
        return 0.0;
    }
    let h: f64 = harmonic_series(mag, freqs, f0, N_HARMONICS).iter().sum();
    clip(h / (mag.iter().sum::<f64>() + 1e-12), 0.0, 1.0)
}

/// How much the spectrum's *shape* changed, level removed.
pub fn spectral_morph(spec: &[f64], prev: Option<&[f64]>) -> f64 {
    let Some(prev) = prev else { return 0.0 };
    if prev.len() != spec.len() {
        return 0.0;
    }
    let norm = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt() + 1e-12;
    let (na, nb) = (norm(spec), norm(prev));
    let sum: f64 = spec.iter().zip(prev).map(|(a, b)| (a / na - b / nb).abs()).sum();
    clip(sum * 1.5, 0.0, 1.0)
}

/// 0 for a plain tone, 1 when a filter is ringing.
///
/// A pulse wave's harmonics are narrow peaks too, so sharpness alone says
/// nothing. What resonance does is lift one region *above* the natural
/// rolloff: with a Crave's filter open and resonant, a harmonic near the
/// cutoff stands far louder than the 1/k slope predicts. That excess is
/// what this measures, so an unfiltered oscillator reads 0 however bright.
pub fn resonance(mag: &[f64], freqs: &[f64], f0: f64) -> f64 {
    let f0 = if f0 != 0.0 { f0 } else { fundamental(mag, freqs) };
    if f0 <= 0.0 {
        return 0.0;
    }
    let h = harmonic_series(mag, freqs, f0, 12);
    if h[0] <= 1e-9 || h.iter().filter(|&&v| v > 0.0).count() < 4 {
        return 0.0;
    }
    let mut boost = f64::NEG_INFINITY;
    for (idx, &hk) in h.iter().enumerate().skip(1) {
        let k = (idx + 1) as f64;
        let expected = (h[0] / k).max(1e-12); // the 1/k slope of a pulse train
        boost = boost.max(hk / expected);
    }
    clip((boost - 1.3) / 2.2, 0.0, 1.0)
}

/// Depth and rate of a slow modulation (LFO, tremolo, a knob turning).
///
/// Fed one value per analysis tick; keeps a few seconds of history and
/// reports how deeply it swings and how fast.
#[derive(Debug, Clone)]
pub struct WobbleTracker {
    rate: f64,
    buf: Vec<f64>,
    i: usize,
    filled: usize,
    pub depth: f64,
    pub hz: f64,
}

impl Default for WobbleTracker {
    fn default() -> Self {
        Self::new(30.0, 2.5)
    }
}

impl WobbleTracker {
    pub fn new(rate_hz: f64, seconds: f64) -> Self {
        let n = ((rate_hz * seconds) as usize).max(16);
        Self { rate: rate_hz, buf: vec![0.0; n], i: 0, filled: 0, depth: 0.0, hz: 0.0 }
    }

    pub fn push(&mut self, value: f64) -> (f64, f64) {
        let n = self.buf.len();
        self.buf[self.i] = value;
        self.i = (self.i + 1) % n;
        self.filled = (self.filled + 1).min(n);
        if self.filled < n / 2 {
            return (self.depth, self.hz);
        }
        // oldest-first, then the most recent `filled` of them
        let mut hist: Vec<f64> =
            self.buf[self.i..].iter().chain(&self.buf[..self.i]).copied().collect();
        hist.drain(..hist.len() - self.filled);

        let mean = hist.iter().sum::<f64>() / hist.len() as f64;
        let mut sorted = hist.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let swing = percentile_sorted(&sorted, 90.0) - percentile_sorted(&sorted, 10.0);
        let depth = swing / (mean.abs() + 0.15);

        // rate from zero crossings of the detrended history
        let signs: Vec<bool> = hist.iter().map(|v| (v - mean).is_sign_negative()).collect();
        let crossings = signs.windows(2).filter(|w| w[0] != w[1]).count();
        let hz = crossings as f64 / 2.0 / (hist.len() as f64 / self.rate);

        self.depth += (clip(depth, 0.0, 1.0) - self.depth) * 0.25;
        if swing > 0.02 {
            // only trust the rate if it actually moves
            self.hz += (clip(hz, 0.0, 12.0) - self.hz) * 0.25;
        }
        (self.depth, self.hz)
    }
}
