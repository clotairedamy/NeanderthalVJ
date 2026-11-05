//! How a visualization listens: the shaping between the analysis and the
//! picture, set per visualization from the Motion tab, and the choices of
//! what in the music drives what, which each mode offers in its own panel.
//!
//! `Reactor` reshapes the frame every mode is handed:
//!
//! - **Auto level** follows the song's running peak and scales the bands
//!   to it, so a quiet master and a loud one both fill the picture.
//! - **Dynamics** bends the levels: below 0 it lifts the quiet parts (the
//!   picture is always busy), above 0 it keeps only the peaks (it waits
//!   for the hits).
//! - **Rise** and **Fall** are how long the levels take to climb to a hit
//!   and to let go of it.
//! - **Hits on** thins the beat: every beat, every other one (half-time),
//!   or the one of the bar.
//!
//! At their defaults every one of them passes the frame through untouched.

use nv_audio::AnalysisFrame;

/// Which beats a visualization answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Division {
    #[default]
    Every,
    /// Beats one and three of the bar: half-time.
    Half,
    /// Only the one of the bar.
    Bar,
}

impl Division {
    pub fn from_name(name: &str) -> Self {
        match name {
            "half" => Division::Half,
            "bar" => Division::Bar,
            _ => Division::Every,
        }
    }
}

/// A visualization's listening, from the Motion tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReactKnobs {
    pub auto_level: bool,
    /// -1 lifts the quiet parts, 0 leaves the levels alone, 1 keeps only
    /// the peaks.
    pub dynamics: f32,
    /// 0..1: how long a level takes to climb, up to 0.4 s.
    pub rise: f32,
    /// 0..1: how long it takes to let go, up to 1.5 s.
    pub fall: f32,
    pub division: Division,
}

impl Default for ReactKnobs {
    fn default() -> Self {
        Self { auto_level: false, dynamics: 0.0, rise: 0.0, fall: 0.0, division: Division::Every }
    }
}

impl ReactKnobs {
    /// Whether these settings change anything at all.
    pub fn is_neutral(&self) -> bool {
        *self == Self::default()
    }
}

/// What fires an event -- a shock wave, a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    /// The beat grid.
    #[default]
    Beat,
    /// A hit in the lows.
    Kick,
    /// A hit in the low mids and mids, where the snare and claps sit.
    Snare,
    /// A hit in the highs: hats and cymbals.
    Hats,
    /// A hit anywhere in the spectrum.
    Any,
}

impl Source {
    pub fn from_name(name: &str) -> Self {
        match name {
            "kick" => Source::Kick,
            "snare" => Source::Snare,
            "hats" => Source::Hats,
            "any" => Source::Any,
            _ => Source::Beat,
        }
    }

    /// How hard this source is hitting in `frame`, 0..1-ish.
    pub fn level(self, frame: &AnalysisFrame, beat: bool) -> f32 {
        let peak =
            |lo: usize, hi: usize| frame.attack[lo..hi].iter().fold(0.0f64, |a, b| a.max(*b));
        let v = match self {
            Source::Beat => {
                if beat {
                    frame.beat_strength
                } else {
                    0.0
                }
            }
            Source::Kick => peak(0, 2),
            Source::Snare => peak(2, 5),
            Source::Hats => peak(5, 7),
            Source::Any => peak(0, 7),
        };
        v as f32
    }
}

/// A transient source turned into separate hits: one per attack, not one
/// per frame of it.
#[derive(Debug, Clone, Copy)]
pub struct HitDetector {
    since: f32,
}

impl Default for HitDetector {
    /// Ready to fire at once: the first hit of a track counts.
    fn default() -> Self {
        Self { since: 1.0 }
    }
}

impl HitDetector {
    /// The strength of a hit that starts this frame, if one does.
    pub fn hit(
        &mut self,
        source: Source,
        frame: &AnalysisFrame,
        beat: bool,
        dt: f32,
    ) -> Option<f32> {
        self.since += dt;
        if source == Source::Beat {
            return (beat && frame.beat_strength > 0.0).then_some(frame.beat_strength as f32);
        }
        let level = source.level(frame, beat);
        if level > 0.3 && self.since > 0.11 {
            self.since = 0.0;
            return Some(level.min(1.0));
        }
        None
    }
}

/// What part of the music drives a movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    Lows,
    Mids,
    Highs,
    /// The overall loudness.
    Loudness,
    Off,
}

impl Drive {
    /// The drive stored under `name`, or `default` for an unknown one.
    pub fn from_name(name: &str, default: Drive) -> Self {
        match name {
            "lows" => Drive::Lows,
            "mids" => Drive::Mids,
            "highs" => Drive::Highs,
            "loudness" => Drive::Loudness,
            "off" => Drive::Off,
            _ => default,
        }
    }

    /// Its level in `frame`, before any gain.
    pub fn level(self, frame: &AnalysisFrame) -> f32 {
        let mean =
            |lo: usize, hi: usize| frame.bands[lo..hi].iter().sum::<f64>() / (hi - lo) as f64;
        let v = match self {
            Drive::Lows => mean(0, 2),
            Drive::Mids => mean(2, 5),
            Drive::Highs => mean(5, 7),
            Drive::Loudness => frame.rms,
            Drive::Off => 0.0,
        };
        v as f32
    }
}

/// Reshapes the analysis for one visualization.
#[derive(Debug, Clone)]
pub struct Reactor {
    peak: f32,
    bands: [f32; 7],
    rms: f32,
    beats: u32,
}

impl Default for Reactor {
    fn default() -> Self {
        Self { peak: 0.6, bands: [0.0; 7], rms: 0.0, beats: 0 }
    }
}

/// One step of a smoothing that takes `tau` seconds; 0 is instant.
fn follow(x: &mut f32, target: f32, tau: f32, dt: f32) {
    if tau <= 1e-4 {
        *x = target;
    } else {
        *x += (target - *x) * (1.0 - (-dt / tau).exp());
    }
}

impl Reactor {
    /// The frame and the beat as the visualization should see them.
    pub fn shape(
        &mut self,
        frame: &AnalysisFrame,
        beat: bool,
        dt: f32,
        k: &ReactKnobs,
    ) -> (AnalysisFrame, bool) {
        let dt = dt.clamp(0.0, 0.1);

        // the song's running peak, always followed so switching auto level
        // on mid-song starts from the right place
        let now = frame.bands.iter().fold(0.0f64, |a, b| a.max(*b)) as f32;
        self.peak = now.max(self.peak * (-dt / 4.0).exp()).max(0.05);

        let beat = beat && self.keep_beat(frame, k.division);
        if k.is_neutral() {
            return (frame.clone(), beat);
        }

        let gain = if k.auto_level { (0.85 / self.peak).clamp(0.6, 4.0) } else { 1.0 };
        // 2^(1.5 d): 0.35 at -1 lifts the quiet parts, 2.8 at 1 keeps peaks
        let exponent = 2f32.powf(1.5 * k.dynamics.clamp(-1.0, 1.0));
        let bend = |v: f32| {
            let v = (v * gain).max(0.0);
            if v <= 1.0 {
                v.powf(exponent)
            } else {
                v
            }
        };
        let (rise, fall) = (k.rise.clamp(0.0, 1.0) * 0.4, k.fall.clamp(0.0, 1.0) * 1.5);

        let mut out = frame.clone();
        for (i, b) in out.bands.iter_mut().enumerate() {
            let target = bend(*b as f32);
            let tau = if target > self.bands[i] { rise } else { fall };
            follow(&mut self.bands[i], target, tau, dt);
            *b = self.bands[i] as f64;
        }
        let target = bend(frame.rms as f32);
        let tau = if target > self.rms { rise } else { fall };
        follow(&mut self.rms, target, tau, dt);
        out.rms = self.rms as f64;
        // transients are hits: scaled with the level, never smoothed, or a
        // slow rise would swallow every one of them
        for a in out.attack.iter_mut() {
            *a = (*a * gain as f64).min(1.5);
        }
        out.punch = (out.punch * gain as f64).min(1.5);
        for s in out.spectrum.iter_mut() {
            *s = bend(*s as f32) as f64;
        }
        (out, beat)
    }

    /// Whether this beat is one the division keeps.
    fn keep_beat(&mut self, frame: &AnalysisFrame, division: Division) -> bool {
        if division == Division::Every {
            return true;
        }
        // which beat of the bar this is, from the bar clock when it has
        // one, else by counting
        let index = if frame.bar_phase > 0.0 || frame.bpm > 0.0 {
            ((frame.bar_phase * 4.0).round() as u32) % 4
        } else {
            self.beats = self.beats.wrapping_add(1);
            (self.beats - 1) % 4
        };
        match division {
            Division::Every => true,
            Division::Half => index % 2 == 0,
            Division::Bar => index == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(level: f64) -> AnalysisFrame {
        AnalysisFrame { bands: [level; 7], rms: level, beat_strength: 1.0, ..Default::default() }
    }

    #[test]
    fn the_defaults_change_nothing() {
        let mut r = Reactor::default();
        let mut f = frame(0.37);
        f.attack = [0.2; 7];
        let (out, beat) = r.shape(&f, true, 1.0 / 60.0, &ReactKnobs::default());
        assert_eq!(out.bands, f.bands);
        assert_eq!(out.attack, f.attack);
        assert!(beat);
    }

    #[test]
    fn auto_level_fills_a_quiet_song() {
        let mut r = Reactor::default();
        let k = ReactKnobs { auto_level: true, ..Default::default() };
        let mut out = frame(0.0);
        for _ in 0..600 {
            out = r.shape(&frame(0.15), false, 1.0 / 60.0, &k).0;
        }
        assert!(out.bands[0] > 0.5, "a quiet song stayed quiet: {}", out.bands[0]);
    }

    #[test]
    fn dynamics_lift_the_quiet_parts_or_keep_only_the_peaks() {
        let at = |d: f32| {
            let k = ReactKnobs { dynamics: d, ..Default::default() };
            Reactor::default().shape(&frame(0.3), false, 1.0 / 60.0, &k).0.bands[3]
        };
        assert!(at(-1.0) > 0.5);
        assert!(at(1.0) < 0.1);
    }

    #[test]
    fn a_slow_fall_lets_a_hit_ring_out() {
        let k = ReactKnobs { fall: 1.0, ..Default::default() };
        let mut r = Reactor::default();
        r.shape(&frame(1.0), false, 1.0 / 60.0, &k);
        let after = r.shape(&frame(0.0), false, 0.25, &k).0.bands[0];
        assert!(after > 0.7, "it let go at once: {after}");
    }

    #[test]
    fn half_time_and_bar_keep_the_right_beats() {
        let kept = |division: Division| {
            let k = ReactKnobs { division, ..Default::default() };
            let mut r = Reactor::default();
            (0..8)
                .filter(|i| {
                    let mut f = frame(0.5);
                    f.bpm = 120.0;
                    f.bar_phase = (i % 4) as f64 / 4.0;
                    r.shape(&f, true, 0.5, &k).1
                })
                .count()
        };
        assert_eq!(kept(Division::Every), 8);
        assert_eq!(kept(Division::Half), 4);
        assert_eq!(kept(Division::Bar), 2);
    }

    #[test]
    fn a_hit_fires_once_per_attack() {
        let mut d = HitDetector::default();
        let mut f = frame(0.5);
        f.attack[3] = 0.8;
        let hits = (0..5).filter(|_| d.hit(Source::Snare, &f, false, 1.0 / 60.0).is_some()).count();
        assert_eq!(hits, 1);
        assert!(d.hit(Source::Kick, &f, false, 1.0).is_none(), "a snare fired the kick");
    }
}
