//! Short-time Fourier transform, matching `librosa.stft` / `librosa.istft`.
//!
//! librosa's defaults are assumed throughout, because they are what the
//! Python tree calls with: a periodic Hann window, `center=True`, and
//! `pad_mode="constant"` (zeros). The spectral path carries f32 because
//! librosa's does -- its mel basis is float32 and the callers pass float32
//! audio, so computing in f64 here would not be "more accurate", it would
//! simply disagree with the implementation being replaced.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::window::hann_periodic;

/// A complex spectrogram, `n_bins` rows by `n_frames` columns, stored
/// column-major (one frame contiguous) because every consumer walks frames.
#[derive(Debug, Clone)]
pub struct Spectrogram {
    pub n_bins: usize,
    pub n_frames: usize,
    /// `frames[f][b]` is bin `b` of frame `f`.
    pub frames: Vec<Vec<Complex<f32>>>,
}

impl Spectrogram {
    pub fn magnitude(&self) -> Vec<Vec<f32>> {
        self.frames.iter().map(|f| f.iter().map(|c| c.norm()).collect()).collect()
    }

    /// `|S|^power`, as `librosa.feature.melspectrogram` takes it.
    pub fn power(&self, power: f32) -> Vec<Vec<f32>> {
        self.frames
            .iter()
            .map(|f| {
                f.iter()
                    .map(|c| if power == 2.0 { c.norm_sqr() } else { c.norm().powf(power) })
                    .collect()
            })
            .collect()
    }
}

pub struct Stft {
    pub n_fft: usize,
    pub hop_length: usize,
    window: Vec<f32>,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
}

impl Stft {
    pub fn new(n_fft: usize, hop_length: usize) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            n_fft,
            hop_length,
            window: hann_periodic(n_fft),
            fft: planner.plan_fft_forward(n_fft),
            ifft: planner.plan_fft_inverse(n_fft),
        }
    }

    fn n_bins(&self) -> usize {
        self.n_fft / 2 + 1
    }

    /// `librosa.stft(y, n_fft, hop_length)` with `center=True`.
    pub fn forward(&self, y: &[f32]) -> Spectrogram {
        let pad = self.n_fft / 2;
        // center=True pads both ends with zeros (pad_mode="constant")
        let mut padded = vec![0.0f32; y.len() + 2 * pad];
        padded[pad..pad + y.len()].copy_from_slice(y);

        let n_frames = if padded.len() < self.n_fft {
            0
        } else {
            1 + (padded.len() - self.n_fft) / self.hop_length
        };
        let mut buf = vec![Complex::new(0.0f32, 0.0); self.n_fft];
        let mut frames = Vec::with_capacity(n_frames);
        for f in 0..n_frames {
            let off = f * self.hop_length;
            for i in 0..self.n_fft {
                buf[i] = Complex::new(padded[off + i] * self.window[i], 0.0);
            }
            self.fft.process(&mut buf);
            frames.push(buf[..self.n_bins()].to_vec());
        }
        Spectrogram { n_bins: self.n_bins(), n_frames, frames }
    }

    /// `librosa.istft(S, hop_length, length)` with `center=True`.
    ///
    /// Overlap-add of the windowed inverse transforms, divided through by
    /// the summed square of the window so that a forward/inverse round
    /// trip is the identity wherever the window sum is non-zero.
    ///
    /// The `center` padding is taken out by *offsetting* where each frame
    /// lands rather than by trimming the result: the final frames reach
    /// past the end of the un-padded signal, so trimming instead would
    /// zero the last `n_fft / 2` samples.
    pub fn inverse(&self, s: &Spectrogram, length: Option<usize>) -> Vec<f32> {
        let pad = self.n_fft / 2;
        let n_frames = match length {
            // librosa stops early when the requested length cannot need
            // the remaining frames
            Some(n) => s.n_frames.min((n + 2 * pad).div_ceil(self.hop_length)),
            None => s.n_frames,
        };
        let out_len = length.unwrap_or_else(|| {
            (self.n_fft + self.hop_length * n_frames.saturating_sub(1)).saturating_sub(2 * pad)
        });

        let mut out = vec![0.0f32; out_len];
        let mut wsum = vec![0.0f32; out_len];
        let mut buf = vec![Complex::new(0.0f32, 0.0); self.n_fft];
        let scale = 1.0 / self.n_fft as f32; // rustfft leaves the 1/N out

        for (f, frame) in s.frames.iter().take(n_frames).enumerate() {
            // rebuild the full spectrum from its non-redundant half
            for b in 0..self.n_fft {
                buf[b] = if b < s.n_bins { frame[b] } else { frame[self.n_fft - b].conj() };
            }
            self.ifft.process(&mut buf);
            let base = (f * self.hop_length) as isize - pad as isize;
            for (i, (sample, w)) in buf.iter().zip(&self.window).enumerate() {
                let idx = base + i as isize;
                if idx < 0 || idx as usize >= out_len {
                    continue;
                }
                let idx = idx as usize;
                out[idx] += sample.re * scale * w;
                wsum[idx] += w * w;
            }
        }
        for (v, w) in out.iter_mut().zip(&wsum) {
            if *w > f32::MIN_POSITIVE {
                *v /= *w;
            }
        }
        out
    }
}
