//! The Slaney mel scale and filterbank, matching `librosa.filters.mel`.

/// `librosa.fft_frequencies`: the centre frequency of each rfft bin.
pub fn fft_frequencies(sr: f64, n_fft: usize) -> Vec<f64> {
    let n = n_fft / 2 + 1;
    (0..n).map(|i| i as f64 * (sr / 2.0) / (n - 1) as f64).collect()
}

// The Slaney scale (`htk=False`): linear below 1 kHz, logarithmic above.
const F_SP: f64 = 200.0 / 3.0;
const MIN_LOG_HZ: f64 = 1000.0;
const MIN_LOG_MEL: f64 = MIN_LOG_HZ / F_SP; // 15.0

fn logstep() -> f64 {
    (6.4f64).ln() / 27.0
}

pub fn hz_to_mel(f: f64) -> f64 {
    if f >= MIN_LOG_HZ { MIN_LOG_MEL + (f / MIN_LOG_HZ).ln() / logstep() } else { f / F_SP }
}

pub fn mel_to_hz(m: f64) -> f64 {
    if m >= MIN_LOG_MEL { MIN_LOG_HZ * (logstep() * (m - MIN_LOG_MEL)).exp() } else { F_SP * m }
}

/// `librosa.mel_frequencies(n, fmin, fmax, htk=False)`.
pub fn mel_frequencies(n: usize, fmin: f64, fmax: f64) -> Vec<f64> {
    let (lo, hi) = (hz_to_mel(fmin), hz_to_mel(fmax));
    (0..n)
        .map(|i| {
            let t = if n == 1 { 0.0 } else { i as f64 / (n - 1) as f64 };
            mel_to_hz(lo + t * (hi - lo))
        })
        .collect()
}

/// `librosa.filters.mel(sr, n_fft, n_mels, norm="slaney")`.
///
/// Returned as f32 because librosa's is: its `dtype` defaults to float32,
/// so the whole mel spectrogram downstream is float32 too, and computing
/// this in f64 would quietly disagree with the implementation being
/// replaced.
pub fn mel_filterbank(sr: f64, n_fft: usize, n_mels: usize) -> Vec<Vec<f32>> {
    let fft_freqs = fft_frequencies(sr, n_fft);
    let mel_f = mel_frequencies(n_mels + 2, 0.0, sr / 2.0);
    let fdiff: Vec<f64> = mel_f.windows(2).map(|w| w[1] - w[0]).collect();

    let mut weights = vec![vec![0.0f32; fft_freqs.len()]; n_mels];
    for (i, row) in weights.iter_mut().enumerate() {
        // Slaney normalization: each filter is scaled to unit *area*, so
        // the wide high filters do not swamp the narrow low ones.
        let enorm = 2.0 / (mel_f[i + 2] - mel_f[i]);
        for (j, &f) in fft_freqs.iter().enumerate() {
            let lower = (f - mel_f[i]) / fdiff[i];
            let upper = (mel_f[i + 2] - f) / fdiff[i + 1];
            row[j] = (lower.min(upper).max(0.0) * enorm) as f32;
        }
    }
    weights
}

/// `librosa.power_to_db(S, ref=1.0, amin=1e-10, top_db=80.0)`.
///
/// `top_db` floors the result against the **global** peak of the whole
/// spectrogram, not each frame's, so quiet frames stay quiet.
pub fn power_to_db(frames: &mut [Vec<f32>]) {
    const AMIN: f32 = 1e-10;
    const TOP_DB: f32 = 80.0;
    let mut peak = f32::NEG_INFINITY;
    for frame in frames.iter_mut() {
        for v in frame.iter_mut() {
            *v = 10.0 * v.max(AMIN).log10();
            peak = peak.max(*v);
        }
    }
    let floor = peak - TOP_DB;
    for frame in frames.iter_mut() {
        for v in frame.iter_mut() {
            *v = v.max(floor);
        }
    }
}

/// `mel_basis @ |S|^power`, one column per frame.
///
/// Accumulated in f64 and rounded once, rather than summing 1025 f32
/// products: the result lands within a float32 ulp of librosa's BLAS
/// either way, and this does not depend on a summation order.
pub fn apply_filterbank(basis: &[Vec<f32>], power: &[Vec<f32>]) -> Vec<Vec<f32>> {
    power
        .iter()
        .map(|col| {
            basis
                .iter()
                .map(|row| {
                    row.iter().zip(col).map(|(w, p)| *w as f64 * *p as f64).sum::<f64>() as f32
                })
                .collect()
        })
        .collect()
}
