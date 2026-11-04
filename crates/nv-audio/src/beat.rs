//! Spectral-flux onset detection with an adaptive threshold, plus BPM.

use crate::num::{clip, median, std_population};

/// Never faster than 240 BPM.
const REFRACTORY: f64 = 60.0 / 240.0;

#[derive(Debug, Clone)]
pub struct BeatDetector {
    history: usize,
    flux_hist: Vec<f64>,
    prev_mag: Option<Vec<f64>>,
    pub last_beat_t: f64,
    beat_intervals: Vec<f64>,
    pub beat_count: u64,
}

impl Default for BeatDetector {
    fn default() -> Self {
        Self::new(43)
    }
}

/// What a tick of the detector saw.
#[derive(Debug, Clone, Copy, Default)]
pub struct BeatTick {
    pub is_beat: bool,
    pub strength: f64,
    pub bpm: f64,
}

impl BeatDetector {
    pub fn new(history: usize) -> Self {
        Self {
            history,
            flux_hist: Vec::new(),
            prev_mag: None,
            last_beat_t: -10.0,
            beat_intervals: Vec::new(),
            beat_count: 0,
        }
    }

    pub fn process(&mut self, mag: &[f64], t: f64) -> BeatTick {
        let stale = !matches!(&self.prev_mag, Some(p) if p.len() == mag.len());
        if stale {
            self.prev_mag = Some(mag.to_vec());
            return BeatTick::default();
        }
        // positive spectral flux, weighted toward low/mid where beats live
        let prev = self.prev_mag.as_ref().unwrap();
        let quarter = mag.len() / 4;
        let flux: f64 = mag
            .iter()
            .zip(prev)
            .enumerate()
            .map(|(i, (m, p))| (m - p).max(0.0) * if i < quarter { 2.0 } else { 1.0 })
            .sum();
        self.prev_mag = Some(mag.to_vec());

        self.flux_hist.push(flux);
        if self.flux_hist.len() > self.history {
            self.flux_hist.remove(0);
        }

        let mut tick = BeatTick::default();
        if self.flux_hist.len() >= 12 {
            let mean = self.flux_hist.iter().sum::<f64>() / self.flux_hist.len() as f64;
            let std = std_population(&self.flux_hist);
            let thresh = mean + 1.4 * std + 1e-6;
            if flux > thresh && (t - self.last_beat_t) > REFRACTORY {
                tick.is_beat = true;
                tick.strength = clip((flux - mean) / (std + 1e-9) / 4.0, 0.2, 1.0);
                let interval = t - self.last_beat_t;
                if (60.0 / 220.0..=60.0 / 50.0).contains(&interval) {
                    self.beat_intervals.push(interval);
                    if self.beat_intervals.len() > 16 {
                        self.beat_intervals.remove(0);
                    }
                }
                self.last_beat_t = t;
                self.beat_count += 1;
            }
        }

        if self.beat_intervals.len() >= 4 {
            let mut bpm = 60.0 / median(&self.beat_intervals);
            while bpm > 180.0 {
                bpm /= 2.0;
            }
            while bpm < 60.0 {
                bpm *= 2.0;
            }
            tick.bpm = bpm;
        }
        tick
    }

    /// Live `(beat_phase, bar_phase)`, extrapolated from the last detected
    /// beat at the running tempo. There is no downbeat to find live, so
    /// bars simply count detected beats.
    pub fn phase_at(&self, t: f64, bpm: f64) -> (f64, f64) {
        if bpm <= 0.0 || self.last_beat_t < 0.0 {
            return (0.0, 0.0);
        }
        let n = ((t - self.last_beat_t) * bpm / 60.0).max(0.0);
        let beat_phase = n.rem_euclid(1.0);
        let bars = (self.beat_count as i64 - 1).rem_euclid(4) as f64;
        let bar_phase = ((bars + n.min(3.999)) / 4.0).rem_euclid(1.0);
        (beat_phase, bar_phase)
    }
}
