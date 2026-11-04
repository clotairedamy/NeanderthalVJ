//! Small numeric helpers that reproduce NumPy's exact conventions.
//!
//! The analysis is ported function for function from NumPy, and a few of
//! its defaults differ from Rust's: ties, interpolation and the population
//! denominator all shift results enough to fail a comparison against the
//! Python, so they are spelled out here once.

/// `int(round(x))` in Python: ties go to the even integer, where Rust's
/// `f64::round` takes them away from zero.
pub fn round_half_even(x: f64) -> i64 {
    let r = x.round(); // Rust takes ties away from zero
    let is_tie = (x - x.trunc()).abs() == 0.5;
    let adjusted = if is_tie && (r as i64) % 2 != 0 { r - x.signum() } else { r };
    adjusted as i64
}

/// `np.percentile(a, q)` with the default linear interpolation.
/// `sorted` must already be ascending.
pub fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let pos = (q / 100.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

/// `np.median`: the mean of the two middle values for an even count.
pub fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    percentile_sorted(&v, 50.0)
}

/// `np.std` with the default `ddof=0` (population, not sample).
pub fn std_population(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt()
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

pub fn clip(x: f64, lo: f64, hi: f64) -> f64 {
    x.max(lo).min(hi)
}

/// `np.hanning(m)`: the *symmetric* window, which is not the periodic one
/// most DSP crates default to.
pub fn hanning(m: usize) -> Vec<f64> {
    if m == 1 {
        return vec![1.0];
    }
    (0..m)
        .map(|n| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / (m - 1) as f64).cos())
        .collect()
}

/// `np.fft.rfftfreq(n, 1/sr)`.
pub fn rfftfreq(n: usize, sr: f64) -> Vec<f64> {
    (0..=n / 2).map(|i| i as f64 * sr / n as f64).collect()
}

/// `np.geomspace(start, stop, num).astype(int)`, then `np.unique`.
pub fn geomspace_unique_int(start: f64, stop: f64, num: usize) -> Vec<usize> {
    if num == 0 {
        return Vec::new();
    }
    let (ls, le) = (start.ln(), stop.ln());
    let mut out: Vec<usize> = (0..num)
        .map(|i| {
            let t = if num == 1 { 0.0 } else { i as f64 / (num - 1) as f64 };
            // NumPy computes the endpoints exactly rather than via exp/ln
            let v = if i == 0 {
                start
            } else if i == num - 1 {
                stop
            } else {
                (ls + t * (le - ls)).exp()
            };
            v as usize // `astype(int)` truncates toward zero
        })
        .collect();
    out.dedup();
    out
}
