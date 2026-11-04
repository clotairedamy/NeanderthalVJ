//! Pitch-preserving time stretch, matching `librosa.effects.time_stretch`.

use rustfft::num_complex::Complex;

use crate::spectrum::{Spectrogram, Stft};

/// `librosa.phase_vocoder(D, rate)`.
///
/// Resamples the spectrogram along time, accumulating the frame-to-frame
/// phase differences so partials stay coherent. Note this follows librosa
/// 1.0, which advances the phase by the *measured* difference rather than
/// correcting against an expected advance -- reproducing its output
/// matters more here than reproducing the textbook algorithm.
pub fn phase_vocoder(d: &Spectrogram, rate: f64) -> Spectrogram {
    let n_frames = d.n_frames;
    if n_frames == 0 || rate <= 0.0 {
        return d.clone();
    }
    let n_out = ((n_frames as f64) / rate).ceil() as usize;
    let n_bins = d.n_bins;

    let mut phase = vec![0.0f64; n_bins];
    let mut out = Vec::with_capacity(n_out);
    for step in 0..n_out {
        let t = step as f64 * rate;
        if t >= n_frames as f64 {
            break;
        }
        if step == 0 {
            let i0 = t.floor() as usize;
            for (ph, c) in phase.iter_mut().zip(&d.frames[i0]) {
                *ph = c.arg() as f64;
            }
        } else {
            // the previous step's measured advance, accumulated
            let p = (step - 1) as f64 * rate;
            let j0 = p.floor() as usize;
            let j1 = (j0 + 1).min(n_frames - 1);
            for (b, ph) in phase.iter_mut().enumerate() {
                *ph += (d.frames[j1][b].arg() - d.frames[j0][b].arg()) as f64;
            }
        }

        // `interp1d(kind="linear", fill_value="extrapolate")` on |D|:
        // clamped to the last whole segment so the tail extrapolates
        let k = (t.floor() as usize).min(n_frames.saturating_sub(2));
        let frac = t - k as f64;
        let frame: Vec<Complex<f32>> = (0..n_bins)
            .map(|b| {
                let m0 = d.frames[k][b].norm() as f64;
                let m1 = d.frames[(k + 1).min(n_frames - 1)][b].norm() as f64;
                let mag = m0 + frac * (m1 - m0);
                Complex::from_polar(mag as f32, phase[b] as f32)
            })
            .collect();
        out.push(frame);
    }
    Spectrogram { n_bins, n_frames: out.len(), frames: out }
}

/// `librosa.effects.time_stretch(y, rate)`.
///
/// `rate > 1` makes the signal shorter without moving its pitch.
pub fn time_stretch(y: &[f32], rate: f64, n_fft: usize, hop: usize) -> Vec<f32> {
    if y.is_empty() || rate <= 0.0 {
        return y.to_vec();
    }
    let stft = Stft::new(n_fft, hop);
    let d = stft.forward(y);
    let stretched = phase_vocoder(&d, rate);
    let len = (y.len() as f64 / rate).round() as usize;
    stft.inverse(&stretched, Some(len))
}
