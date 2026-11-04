//! Onset strength envelopes, matching `librosa.onset.onset_strength`.

use crate::mel::{apply_filterbank, mel_filterbank, power_to_db};
use crate::spectrum::Stft;

/// How the per-mel-band onsets are collapsed into one envelope.
///
/// librosa defaults to the mean; the beat grid asks for the median,
/// because broadband noise -- storm rumble, room tone, tape hiss --
/// otherwise swamps the onsets and drags the tempo estimate with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregate {
    Mean,
    Median,
}

impl Aggregate {
    fn apply(self, values: &mut [f32]) -> f32 {
        match self {
            Self::Mean => {
                values.iter().map(|v| *v as f64).sum::<f64>() as f32 / values.len() as f32
            }
            Self::Median => {
                values.sort_by(|a, b| a.total_cmp(b));
                let n = values.len();
                if n % 2 == 1 { values[n / 2] } else { 0.5 * (values[n / 2 - 1] + values[n / 2]) }
            }
        }
    }
}

pub const N_MELS: usize = 128;

/// `librosa.onset.onset_strength(y, sr, hop_length, aggregate)`.
///
/// A mel spectrogram in dB, differenced by one frame, negatives discarded,
/// collapsed across the mel bands, then shifted to undo the framing delay.
pub fn onset_strength(y: &[f32], sr: f64, n_fft: usize, hop: usize, agg: Aggregate) -> Vec<f32> {
    let stft = Stft::new(n_fft, hop);
    let s = stft.forward(y);
    let basis = mel_filterbank(sr, n_fft, N_MELS);
    let mut mel = apply_filterbank(&basis, &s.power(2.0));
    power_to_db(&mut mel);

    let n_frames = mel.len();
    if n_frames < 2 {
        return vec![0.0; n_frames];
    }
    // lag=1: the rise since the previous frame, in each band
    let mut env: Vec<f32> = Vec::with_capacity(n_frames - 1);
    let mut scratch = vec![0.0f32; N_MELS];
    for t in 1..n_frames {
        for m in 0..N_MELS {
            scratch[m] = (mel[t][m] - mel[t - 1][m]).max(0.0);
        }
        env.push(agg.apply(&mut scratch));
    }

    // Counter-act the framing delay: one frame for the lag, plus the half
    // window `center=True` put in front of the signal.
    let pad = 1 + n_fft / (2 * hop);
    let mut out = vec![0.0f32; pad];
    out.extend_from_slice(&env);
    out.truncate(n_frames);
    out.resize(n_frames, 0.0);
    out
}

/// `librosa.util.peak_pick` in its greedy mode, with the window sizes
/// `onset_detect` chooses for a given sample rate and hop.
///
/// A frame is an onset when it is the largest in a short window around it,
/// stands `delta` above the local average, and is not inside the refractory
/// `wait` after the previous one.
pub struct PeakPicker {
    pub pre_max: usize,
    pub post_max: usize,
    pub pre_avg: usize,
    pub post_avg: usize,
    pub wait: usize,
    pub delta: f32,
}

impl PeakPicker {
    /// The settings `librosa.onset.onset_detect` uses: 30 ms either side
    /// for the peak test, 100 ms for the average, 30 ms refractory.
    pub fn for_onset_detect(sr: f64, hop: usize) -> Self {
        let frames = |secs: f64| ((secs * sr).floor() / hop as f64).floor();
        Self {
            pre_max: frames(0.03).ceil() as usize,
            post_max: (frames(0.0) + 1.0).ceil() as usize,
            pre_avg: frames(0.10).ceil() as usize,
            post_avg: (frames(0.10) + 1.0).ceil() as usize,
            wait: frames(0.03).ceil() as usize,
            delta: 0.07,
        }
    }

    pub fn pick(&self, x: &[f32]) -> Vec<usize> {
        let len = x.len();
        if len == 0 {
            return Vec::new();
        }
        let max_of =
            |lo: usize, hi: usize| x[lo..hi].iter().fold(f32::NEG_INFINITY, |a, b| a.max(*b));
        let mean_of = |lo: usize, hi: usize| {
            (x[lo..hi].iter().map(|v| *v as f64).sum::<f64>() / (hi - lo) as f64) as f32
        };

        let mut out = Vec::new();
        // the first frame is special-cased: it has no history to look back on
        let head = self.post_max.min(len);
        let head_avg = self.post_avg.min(len);
        let mut n = if x[0] >= max_of(0, head) && x[0] >= mean_of(0, head_avg) + self.delta {
            out.push(0);
            self.wait + 1
        } else {
            1
        };

        while n < len {
            let lo = n.saturating_sub(self.pre_max);
            let hi = (n + self.post_max).min(len);
            if x[n] != max_of(lo, hi) {
                n += 1;
                continue;
            }
            let alo = n.saturating_sub(self.pre_avg);
            let ahi = (n + self.post_avg).min(len);
            if x[n] < mean_of(alo, ahi) + self.delta {
                n += 1;
                continue;
            }
            out.push(n);
            n += self.wait + 1;
        }
        out
    }
}

/// `librosa.onset.onset_detect(onset_envelope, sr, hop_length)`.
///
/// The envelope is shifted to zero and max-scaled first, which is what
/// makes the fixed `delta` threshold meaningful across material.
pub fn onset_detect(env: &[f32], sr: f64, hop: usize) -> Vec<usize> {
    if env.is_empty() || env.iter().all(|v| *v == 0.0) || !env.iter().all(|v| v.is_finite()) {
        return Vec::new();
    }
    let lo = env.iter().fold(f32::INFINITY, |a, b| a.min(*b));
    let shifted: Vec<f32> = env.iter().map(|v| v - lo).collect();
    let peak = shifted.iter().fold(f32::NEG_INFINITY, |a, b| a.max(*b));
    let scaled: Vec<f32> = shifted.iter().map(|v| v / (peak + f32::MIN_POSITIVE)).collect();
    PeakPicker::for_onset_detect(sr, hop).pick(&scaled)
}
