//! Beat tracking, matching `librosa.beat.beat_track`.
//!
//! A dynamic program over the onset envelope: each frame scores the best
//! predecessor a beat-period behind it, penalised by how far that spacing
//! strays from the estimated tempo, and the beats are read back along the
//! winning chain.

use crate::tempo::estimate_tempo;
use crate::window::hann_symmetric;

/// How closely the tracker holds to the estimated tempo.
pub const TIGHTNESS: f32 = 100.0;

/// `int(round(x))` with Python's half-to-even tie rule.
fn round_half_even(x: f64) -> i64 {
    let r = x.round();
    let is_tie = (x - x.trunc()).abs() == 0.5;
    let adjusted = if is_tie && (r as i64) % 2 != 0 { r - x.signum() } else { r };
    adjusted as i64
}

/// `librosa.util.localmax`: strictly above the left neighbour, at least
/// equal to the right one. The first frame is never a peak.
fn localmax(x: &[f32]) -> Vec<bool> {
    let n = x.len();
    let mut out = vec![false; n];
    for i in 1..n.saturating_sub(1) {
        out[i] = x[i] > x[i - 1] && x[i] >= x[i + 1];
    }
    if n >= 2 {
        out[n - 1] = x[n - 1] > x[n - 2];
    }
    out
}

/// `onsets / (std(ddof=1) + tiny)`.
fn normalize_onsets(env: &[f32]) -> Vec<f32> {
    let n = env.len();
    if n < 2 {
        return env.to_vec();
    }
    let mean = env.iter().map(|v| *v as f64).sum::<f64>() / n as f64;
    // ddof=1: the sample standard deviation, not the population one
    let var = env.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let norm = var.sqrt() as f32 + f32::MIN_POSITIVE;
    env.iter().map(|v| v / norm).collect()
}

/// A smoothed, tempo-matched version of the onset envelope: the same-mode
/// convolution with a Gaussian one beat period wide.
fn local_score(env: &[f32], frames_per_beat: f64) -> Vec<f32> {
    let n = env.len();
    let fpb = frames_per_beat;
    let k_half = fpb as usize;
    let k = 2 * k_half + 1;
    let window: Vec<f32> = (0..k)
        .map(|i| {
            let x = (i as f64 - fpb) * 32.0 / fpb;
            (-0.5 * x * x).exp() as f32
        })
        .collect();

    let mut out = vec![0.0f32; n];
    for (i, o) in out.iter_mut().enumerate() {
        let lo = (i + k_half + 1).saturating_sub(n);
        let hi = (i + k_half + 1).min(k);
        let mut acc = 0.0f64;
        for kk in lo..hi {
            acc += window[kk] as f64 * env[i + k_half - kk] as f64;
        }
        *o = acc as f32;
    }
    out
}

/// The core dynamic program; returns `(backlink, cumscore)`.
fn track_dp(localscore: &[f32], frames_per_beat: f64, tightness: f32) -> (Vec<i64>, Vec<f32>) {
    let n = localscore.len();
    let mut backlink = vec![-1i64; n];
    let mut cumscore = vec![0.0f32; n];
    let peak = localscore.iter().fold(f32::NEG_INFINITY, |a, b| a.max(*b));
    let score_thresh = 0.01 * peak;
    let mut first_beat = true;

    for i in 0..n {
        let score_i = localscore[i];
        let mut best_score = f32::NEG_INFINITY;
        let mut beat_location: i64 = -1;

        // search backwards over plausible predecessors: from half a beat
        // behind out to two beats behind
        let start = i as i64 - round_half_even(frames_per_beat / 2.0);
        let stop = (i as f64 - 2.0 * frames_per_beat - 1.0) as i64;
        let mut loc = start;
        while loc > stop {
            if loc < 0 {
                break;
            }
            let gap = (i as i64 - loc) as f64;
            let dev = gap.ln() - frames_per_beat.ln();
            let score = cumscore[loc as usize] - tightness * (dev * dev) as f32;
            if score > best_score {
                best_score = score;
                beat_location = loc;
            }
            loc -= 1;
        }

        cumscore[i] = if beat_location >= 0 { score_i + best_score } else { score_i };
        // hold off until the envelope actually has something in it
        if first_beat && score_i < score_thresh {
            backlink[i] = -1;
        } else {
            backlink[i] = beat_location;
            first_beat = false;
        }
    }
    (backlink, cumscore)
}

/// The last frame that is both a local peak and strong enough to be a beat.
fn last_beat(cumscore: &[f32]) -> usize {
    let peaks = localmax(cumscore);
    let mut vals: Vec<f32> =
        cumscore.iter().zip(&peaks).filter(|(_, p)| **p).map(|(c, _)| *c).collect();
    if vals.is_empty() {
        return cumscore.len().saturating_sub(1);
    }
    vals.sort_by(|a, b| a.total_cmp(b));
    let m = vals.len();
    let median = if m % 2 == 1 { vals[m / 2] } else { 0.5 * (vals[m / 2 - 1] + vals[m / 2]) };
    let threshold = 0.5 * median;
    for n in (0..cumscore.len()).rev() {
        if peaks[n] && cumscore[n] >= threshold {
            return n;
        }
    }
    cumscore.len().saturating_sub(1)
}

/// Discard leading and trailing beats sitting on a near-silent envelope.
fn trim_beats(localscore: &[f32], beats: &mut [bool], trim: bool) {
    let n = localscore.len();
    let threshold = if trim {
        let picked: Vec<f32> =
            localscore.iter().zip(beats.iter()).filter(|(_, b)| **b).map(|(s, _)| *s).collect();
        let w = hann_symmetric(5);
        // full convolution, then sliced to emulate a same-mode one
        let full_len = picked.len() + w.len() - 1;
        let mut full = vec![0.0f32; full_len];
        for (i, p) in picked.iter().enumerate() {
            for (j, wv) in w.iter().enumerate() {
                full[i + j] += p * wv;
            }
        }
        let lo = (w.len() / 2).min(full_len);
        let hi = (n + w.len() / 2).min(full_len);
        let smooth = &full[lo..hi];
        if smooth.is_empty() {
            0.0
        } else {
            let ms =
                smooth.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>() / smooth.len() as f64;
            0.5 * ms.sqrt() as f32
        }
    } else {
        0.0
    };

    let mut i = 0;
    while i < n && localscore[i] <= threshold {
        beats[i] = false;
        i += 1;
    }
    let mut j = n;
    while j > 0 && localscore[j - 1] <= threshold {
        beats[j - 1] = false;
        j -= 1;
    }
}

/// `librosa.beat.beat_track(onset_envelope, sr, hop_length)`.
///
/// Returns `(tempo_bpm, beat_frames)`.
pub fn beat_track(env: &[f32], sr: f64, hop: usize) -> (f64, Vec<usize>) {
    if env.is_empty() || env.iter().all(|v| *v == 0.0) {
        return (0.0, Vec::new());
    }
    let bpm = estimate_tempo(env, sr, hop);
    if bpm <= 0.0 {
        return (0.0, Vec::new());
    }
    let frame_rate = sr / hop as f64;
    let frames_per_beat = round_half_even(frame_rate * 60.0 / bpm) as f64;

    let localscore = local_score(&normalize_onsets(env), frames_per_beat);
    let (backlink, cumscore) = track_dp(&localscore, frames_per_beat, TIGHTNESS);

    // read the beat chain back from the last beat
    let mut beats = vec![false; env.len()];
    let mut n = last_beat(&cumscore) as i64;
    while n >= 0 {
        beats[n as usize] = true;
        n = backlink[n as usize];
    }
    trim_beats(&localscore, &mut beats, true);

    let frames = beats.iter().enumerate().filter(|(_, b)| **b).map(|(i, _)| i).collect();
    (bpm, frames)
}

/// `librosa.frames_to_time`.
pub fn frames_to_time(frames: &[usize], sr: f64, hop: usize) -> Vec<f64> {
    frames.iter().map(|f| *f as f64 * hop as f64 / sr).collect()
}
