//! Windowed FFT analysis of the mix and of each stem.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::beat::BeatDetector;
use crate::ema::{Ema, EmaN};
use crate::frame::{
    AnalysisFrame, BAND_EDGES, FFT_SIZE, N_BANDS, SPECTRUM_BINS, STEMS, StemReading,
};
use crate::num::{clip, geomspace_unique_int, hanning, mean, rfftfreq};
use crate::settings::AnalysisSettings;
use crate::timbre::{self, WobbleTracker};

pub struct AudioAnalyzer {
    sr: f64,
    pub settings: AnalysisSettings,
    window: Vec<f64>,
    freqs: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
    scratch: Vec<Complex<f64>>,

    pub beat: BeatDetector,
    band_ema: EmaN<N_BANDS>,
    stem_emas: [EmaN<N_BANDS>; 4],
    rms_ema: Ema,
    centroid_ema: Ema,
    /// Flatness is a slow, steady read.
    flat_ema: Ema,

    // synth timbre: pulse width, LFO wobble, how fast the tone morphs
    pwm_ema: Ema,
    reso_ema: Ema,
    morph_ema: Ema,
    prev_spec: Option<[f64; SPECTRUM_BINS]>,
    wob_centroid: WobbleTracker,
    wob_rms: WobbleTracker,
    wob_pwm: WobbleTracker,

    // Adaptive per-band normalization: track ceiling AND floor so real
    // music (which hovers in a narrow loudness window) still spans 0..1.
    band_max: [f64; N_BANDS],
    band_min: [f64; N_BANDS],
    prev_norm: [f64; N_BANDS],
    rms_max: f64,
    rms_min: f64,

    /// Which bins fall in each of the seven bands.
    band_bins: [Vec<usize>; N_BANDS],
    /// The range flatness is measured over.
    flat_bins: Vec<usize>,
}

impl AudioAnalyzer {
    pub fn new(sr: u32, settings: AnalysisSettings) -> Self {
        let sr = sr as f64;
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let sm = settings.smoothing;
        let mut me = Self {
            sr,
            settings,
            window: hanning(FFT_SIZE),
            freqs: rfftfreq(FFT_SIZE, sr),
            scratch: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            fft,
            beat: BeatDetector::default(),
            band_ema: EmaN::new(sm),
            stem_emas: std::array::from_fn(|_| EmaN::new(sm)),
            rms_ema: Ema::new(sm),
            centroid_ema: Ema::new(sm),
            flat_ema: Ema::new(0.9),
            pwm_ema: Ema::new(0.88),
            reso_ema: Ema::new(0.85),
            morph_ema: Ema::new(0.8),
            prev_spec: None,
            wob_centroid: WobbleTracker::default(),
            wob_rms: WobbleTracker::default(),
            wob_pwm: WobbleTracker::default(),
            band_max: [1e-3; N_BANDS],
            band_min: [0.0; N_BANDS],
            prev_norm: [0.0; N_BANDS],
            rms_max: 1e-3,
            rms_min: 0.0,
            band_bins: std::array::from_fn(|_| Vec::new()),
            flat_bins: Vec::new(),
        };
        me.refresh_fft_range();
        me
    }

    fn precompute_band_bins(&mut self) {
        let (lo, hi) = (self.settings.fft_min_hz, self.settings.fft_max_hz);
        for (b, (f0, f1)) in BAND_EDGES.iter().enumerate() {
            let f0 = f0.max(lo);
            let f1 = f1.min(hi);
            let bins: Vec<usize> = (0..self.freqs.len())
                .filter(|&i| self.freqs[i] >= f0 && self.freqs[i] < f1)
                .collect();
            // an empty band would read as silence; keep one bin alive
            self.band_bins[b] = if bins.is_empty() { vec![1] } else { bins };
        }
        // Skip DC/rumble and the very top, where an empty spectrum would
        // read as "flat" on its own.
        let (flo, fhi) = (lo.max(120.0), hi.min(12_000.0));
        self.flat_bins = (0..self.freqs.len())
            .filter(|&i| self.freqs[i] >= flo && self.freqs[i] <= fhi)
            .collect();
        if self.flat_bins.is_empty() {
            self.flat_bins = (0..self.freqs.len()).filter(|&i| self.freqs[i] > 0.0).collect();
        }
    }

    pub fn refresh_fft_range(&mut self) {
        self.precompute_band_bins();
    }

    pub fn set_smoothing(&mut self, factor: f64) {
        let f = clip(factor, 0.0, 0.98);
        self.band_ema.factor = f;
        self.rms_ema.factor = f;
        self.centroid_ema.factor = f;
        for e in &mut self.stem_emas {
            e.factor = f;
        }
    }

    /// `|rfft(mono[..FFT_SIZE] * window)|`, zero-padded if short.
    fn magnitude(&mut self, mono: &[f64]) -> Vec<f64> {
        for i in 0..FFT_SIZE {
            let s = mono.get(i).copied().unwrap_or(0.0);
            self.scratch[i] = Complex::new(s * self.window[i], 0.0);
        }
        self.fft.process(&mut self.scratch);
        // the real FFT's non-redundant half
        self.scratch[..=FFT_SIZE / 2].iter().map(|c| c.norm()).collect()
    }

    fn band_energies(&mut self, mag: &[f64], adapt: bool) -> [f64; N_BANDS] {
        let mut vals = [0.0; N_BANDS];
        for (val, bins) in vals.iter_mut().zip(&self.band_bins) {
            let m: f64 = bins.iter().map(|&i| mag[i]).sum::<f64>() / bins.len() as f64;
            *val = (1.0 + 8.0 * m).log10();
        }
        let mut out = [0.0; N_BANDS];
        for b in 0..N_BANDS {
            if adapt {
                // The ceiling decays fairly fast and the floor drifts up
                // toward it, so the usable window keeps hugging the current
                // music's dynamic range.
                self.band_max[b] = (self.band_max[b] * 0.993).max(vals[b]);
                self.band_min[b] =
                    (self.band_min[b] + (self.band_max[b] - self.band_min[b]) * 0.008).min(vals[b]);
                // denom guard: a near-constant band must not amplify noise to 1
                let denom =
                    (self.band_max[b] - self.band_min[b]).max(0.25 * self.band_max[b]) + 1e-9;
                out[b] = (vals[b] - self.band_min[b]) / denom;
            } else {
                out[b] = vals[b] / (self.band_max[b] + 1e-9);
            }
            // gamma: punchier contrast
            out[b] = clip(clip(out[b], 0.0, 1.0).powf(1.25) * self.settings.sensitivity, 0.0, 1.0);
        }
        out
    }

    pub fn analyze(
        &mut self,
        t: f64,
        mono: &[f64],
        stem_windows: Option<&[(&str, Vec<f64>); 4]>,
    ) -> AnalysisFrame {
        let mag = self.magnitude(mono);

        let raw_bands = self.band_energies(&mag, true);
        // Per-band transient (attack) plus low-band punch: fires on every
        // kick / bass hit even when the beat detector is unsure.
        let mut attack = [0.0; N_BANDS];
        for b in 0..N_BANDS {
            attack[b] = clip((raw_bands[b] - self.prev_norm[b]) * 5.0, 0.0, 1.0);
        }
        self.prev_norm = raw_bands;
        let punch = clip(attack[..3].iter().copied().fold(f64::MIN, f64::max) * 1.3, 0.0, 1.0);
        let bands = self.band_ema.push(&raw_bands);

        let raw_rms = (mono.iter().map(|v| v * v).sum::<f64>() / mono.len() as f64).sqrt();
        self.rms_max = (self.rms_max * 0.993).max(raw_rms);
        self.rms_min = (self.rms_min + (self.rms_max - self.rms_min) * 0.008).min(raw_rms);
        let norm_rms = (raw_rms - self.rms_min)
            / (self.rms_max - self.rms_min).max(0.25 * self.rms_max).max(1e-9);
        let rms = self.rms_ema.push(clip(norm_rms * self.settings.sensitivity, 0.0, 1.0));

        let total: f64 = mag.iter().sum::<f64>() + 1e-9;
        let centroid_hz: f64 = self.freqs.iter().zip(&mag).map(|(f, m)| f * m).sum::<f64>() / total;
        let centroid = self.centroid_ema.push(clip(centroid_hz / (self.sr / 4.0), 0.0, 1.0).sqrt());

        // Spectral flatness: geometric over arithmetic mean of the
        // spectrum. A tone concentrates its energy in a few bins (flatness
        // near 0); white noise spreads it evenly (near 1). Hiss, cymbals,
        // applause and a detuned wash all read high, which is what "noise"
        // means here.
        let band: Vec<f64> = self.flat_bins.iter().map(|&i| mag[i]).collect();
        let band_max = band.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let flatness = if band.len() > 8 && band_max > 1e-7 {
            let geo =
                (band.iter().map(|v| (v + 1e-12).ln()).sum::<f64>() / band.len() as f64).exp();
            let ari = mean(&band) + 1e-12;
            self.flat_ema.push(clip((geo / ari).sqrt(), 0.0, 1.0))
        } else {
            0.0
        };

        // A filter sweep or a PWM move changes the tone enormously while
        // the level barely moves; see `timbre`.
        let f0 = timbre::fundamental(&mag, &self.freqs);
        let harm = timbre::harmonicity(&mag, &self.freqs, f0);
        // trust the pulse-width read only on a tonal, non-noisy signal
        let gate = clip((harm - 0.06) / 0.10, 0.0, 1.0) * clip((0.72 - flatness) / 0.25, 0.0, 1.0);
        let pwm = self.pwm_ema.push(timbre::pulse_width(&mag, &self.freqs, f0) * gate);
        let reso = self.reso_ema.push(timbre::resonance(&mag, &self.freqs, f0) * gate);

        let tick = self.beat.process(&mag, t);

        let mut stems = [StemReading::default(); 4];
        match stem_windows {
            Some(windows) => {
                for (si, (name, w)) in windows.iter().enumerate() {
                    debug_assert_eq!(*name, STEMS[si]);
                    let smag = self.magnitude(w);
                    let sb = self.band_energies(&smag, false);
                    stems[si].bands = self.stem_emas[si].push(&sb);
                    let n = w.len().min(FFT_SIZE);
                    let energy = (w[..n].iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
                    stems[si].energy = clip(energy * 3.0 * self.settings.sensitivity, 0.0, 1.0);
                }
            }
            None => {
                for s in &mut stems {
                    s.bands = bands;
                    s.energy = clip(rms * 3.0, 0.0, 1.0);
                }
            }
        }

        // compact display spectrum (64 log-spaced bins)
        let edges = geomspace_unique_int(1.0, (mag.len() - 1) as f64, SPECTRUM_BINS + 1);
        let mut spec_v: Vec<f64> = edges
            .windows(2)
            .map(|w| if w[1] > w[0] { mean(&mag[w[0]..w[1]]) } else { mag[w[0]] })
            .collect();
        for v in &mut spec_v {
            *v = (1.0 + 8.0 * *v).log10();
        }
        let peak = spec_v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        for v in &mut spec_v {
            *v /= peak + 1e-9;
        }
        let mut spectrum = [0.0; SPECTRUM_BINS];
        for (i, v) in spec_v.iter().take(SPECTRUM_BINS).enumerate() {
            spectrum[i] = *v;
        }

        let morph = self
            .morph_ema
            .push(timbre::spectral_morph(&spectrum, self.prev_spec.as_ref().map(|s| s.as_slice())));
        self.prev_spec = Some(spectrum);

        // An LFO can ride on brightness (a filter sweep), on level
        // (tremolo) or on the pulse width itself, which barely moves the
        // other two.
        let cands = [
            (self.wob_centroid.push(centroid), 1.0),
            (self.wob_rms.push(rms), 0.85),
            (self.wob_pwm.push(pwm), 1.15),
        ];
        let (mut depth, mut wobble_hz) = (0.0, 0.0);
        for ((d, hz), weight) in cands {
            if d * weight > depth {
                depth = d * weight;
                wobble_hz = hz;
            }
        }

        let (beat_phase, bar_phase) = self.beat.phase_at(t, tick.bpm);
        let waveform: Vec<f64> =
            (0..FFT_SIZE).map(|i| mono.get(i).copied().unwrap_or(0.0)).collect();

        AnalysisFrame {
            time: t,
            bands,
            stems,
            rms: clip(rms, 0.0, 1.0),
            attack,
            punch,
            beat: tick.is_beat,
            beat_strength: tick.strength,
            bpm: tick.bpm,
            flatness,
            pwm,
            pitch_hz: f0,
            resonance: reso,
            wobble: clip(depth, 0.0, 1.0),
            wobble_hz,
            morph,
            beat_phase,
            bar_phase,
            centroid,
            spectrum,
            waveform,
        }
    }
}
