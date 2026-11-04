//! What the interface talks to: load a track, play it, and read back what
//! the music is doing right now.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use nv_audio::{AnalysisFrame, AnalysisSettings, AudioAnalyzer, FFT_SIZE};
use nv_dsp::Aggregate;

use crate::decode::{self, Track};
use crate::live::{self, LiveInput};
use crate::player::{Player, Shared, Stretched};

/// A decoded track with its beat grid, ready to install.
pub struct Prepared {
    track: Track,
    mono: Vec<f32>,
    grid: BeatGrid,
    path: String,
}

impl Prepared {
    pub fn duration(&self) -> f64 {
        self.track.duration()
    }
    pub fn bpm(&self) -> f64 {
        self.grid.bpm
    }
    /// The mono mix the analysis reads.
    pub fn mono(&self) -> &[f32] {
        &self.mono
    }
    pub fn sample_rate(&self) -> u32 {
        self.track.sample_rate
    }
    pub fn grid(&self) -> &BeatGrid {
        &self.grid
    }
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// The beat grid, computed once when a track loads.
///
/// Knowing the whole song lets the visuals be *on* the beat instead of a
/// frame behind it: the engine fires an impulse when the playhead crosses
/// a grid entry, rather than reacting to one it has just heard.
#[derive(Debug, Clone, Default)]
pub struct BeatGrid {
    pub beats: Vec<f64>,
    pub onsets: Vec<f64>,
    pub bpm: f64,
}

impl BeatGrid {
    /// Whether a beat or an onset falls in `(t0, t1]`.
    pub fn events_between(&self, t0: f64, t1: f64) -> (bool, bool) {
        let any = |v: &[f64]| v.iter().any(|t| *t > t0 && *t <= t1);
        (any(&self.beats), any(&self.onsets))
    }
}

pub struct Engine {
    track: Option<Track>,
    player: Option<Player>,
    shared: Option<Arc<Shared>>,
    analyzer: AudioAnalyzer,
    mono: Vec<f32>,
    pub grid: BeatGrid,
    pub path: Option<String>,
    last_position: f64,
    /// Live input, when it is on; it takes the place of the track.
    live: Option<LiveInput>,
    live_gain_db: f64,
    live_gate_db: f64,
    live_started: std::time::Instant,
    /// How the track plays: speed, whether the pitch is kept, looping.
    speed: f64,
    keep_pitch: bool,
    looping: bool,
    /// Seconds the visuals trail the sound by.
    av_offset: f64,
    /// Bumped by every speed change, so a stretch that finishes after a
    /// newer one was asked for is thrown away rather than installed.
    stretch_gen: Arc<AtomicU64>,
}

/// A snapshot for the interface: everything it needs for one repaint.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub frame: AnalysisFrame,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub peak_db: f64,
    pub bpm: f64,
    /// A grid beat/onset was crossed since the last snapshot.
    pub beat: bool,
    pub punch: bool,
    /// From the live input rather than the track.
    pub live: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            track: None,
            player: None,
            shared: None,
            analyzer: AudioAnalyzer::new(44_100, AnalysisSettings::default()),
            mono: Vec::new(),
            grid: BeatGrid::default(),
            path: None,
            last_position: 0.0,
            live: None,
            live_gain_db: 0.0,
            live_gate_db: -55.0,
            live_started: std::time::Instant::now(),
            speed: 1.0,
            keep_pitch: true,
            looping: false,
            av_offset: 0.0,
            stretch_gen: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.track.is_some()
    }

    /// Everything decoding a file produces. Built off the UI thread.
    ///
    /// Decoding and tracking a four-minute song takes seconds; doing it on
    /// the thread that runs the window froze the whole app behind a
    /// spinning cursor, so [`Engine::prepare`] does the work anywhere and
    /// [`Engine::adopt`] installs the result.
    pub fn prepare(path: &str) -> Result<Prepared, String> {
        let track = decode::load(path).map_err(|e| e.to_string())?;
        let mono = track.mono();
        let grid = Self::build_grid(&mono, track.sample_rate);
        Ok(Prepared { track, mono, grid, path: path.to_string() })
    }

    /// Install a decoded track and open the output for it.
    pub fn adopt(&mut self, p: Prepared) -> Result<(), String> {
        let Prepared { track, mono, grid, path } = p;
        self.mono = mono;
        self.analyzer = AudioAnalyzer::new(track.sample_rate, AnalysisSettings::default());
        self.grid = grid;
        let player = Player::new(&track).map_err(|e| e.to_string())?;
        self.shared = Some(player.shared());
        self.player = Some(player);
        self.track = Some(track);
        self.path = Some(path);
        self.last_position = 0.0;
        self.apply_playback(true);
        Ok(())
    }

    /// Decode, track and open in one go. Blocks; prefer `prepare`/`adopt`
    /// anywhere the UI is waiting.
    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let track = decode::load(path).map_err(|e| e.to_string())?;
        let sr = track.sample_rate;
        self.mono = track.mono();
        self.analyzer = AudioAnalyzer::new(sr, AnalysisSettings::default());
        self.grid = Self::build_grid(&self.mono, sr);

        let player = Player::new(&track).map_err(|e| e.to_string())?;
        self.shared = Some(player.shared());
        self.player = Some(player);
        self.track = Some(track);
        self.path = Some(path.to_string());
        self.last_position = 0.0;
        self.apply_playback(true);
        Ok(())
    }

    /// The beat grid, from the ported DSP.
    fn build_grid(mono: &[f32], sr: u32) -> BeatGrid {
        const HOP: usize = 512;
        const N_FFT: usize = 2048;
        let sr_f = sr as f64;
        // the median aggregate, as the Python beat grid asks for: broadband
        // noise otherwise swamps the onsets and drags the tempo with it
        let env = nv_dsp::onset_strength(mono, sr_f, N_FFT, HOP, Aggregate::Median);
        let (bpm, beat_frames) = nv_dsp::beat_track(&env, sr_f, HOP);
        let env_mean = nv_dsp::onset_strength(mono, sr_f, N_FFT, HOP, Aggregate::Mean);
        let onset_frames = nv_dsp::onset_detect(&env_mean, sr_f, HOP);
        BeatGrid {
            beats: nv_dsp::frames_to_time(&beat_frames, sr_f, HOP),
            onsets: nv_dsp::frames_to_time(&onset_frames, sr_f, HOP),
            bpm,
        }
    }

    pub fn toggle_play(&mut self) {
        let on = !self.is_playing();
        self.set_playing(on);
    }

    /// Playing the track takes over from the live input.
    pub fn set_playing(&mut self, on: bool) {
        if on {
            self.stop_live();
        }
        if let Some(p) = &self.player {
            p.set_playing(on);
        }
    }

    /// Listen to `device` (the default input if empty) instead of the
    /// track, which pauses. Returns the device's name.
    pub fn start_live(&mut self, device: &str) -> Result<String, String> {
        let input = LiveInput::open(device)?;
        if let Some(p) = &self.player {
            p.set_playing(false);
        }
        self.analyzer = AudioAnalyzer::new(input.sample_rate(), self.analyzer.settings);
        self.live_started = std::time::Instant::now();
        let name = input.device().to_string();
        self.live = Some(input);
        Ok(name)
    }

    /// Close the live input; the track is what the visuals follow again.
    pub fn stop_live(&mut self) {
        if self.live.take().is_some() {
            let sr = self.track.as_ref().map_or(44_100, |t| t.sample_rate);
            self.analyzer = AudioAnalyzer::new(sr, self.analyzer.settings);
        }
    }

    /// How the track should play. A speed other than 1 with the pitch
    /// kept needs the track time-stretched, which takes a moment, so it is
    /// done off this thread; until it is ready the track plays at the new
    /// speed with its pitch moved, as a turntable would.
    pub fn set_playback(&mut self, speed: f64, keep_pitch: bool, looping: bool) {
        let speed = speed.clamp(0.25, 4.0);
        let changed = (speed - self.speed).abs() > 1e-9 || keep_pitch != self.keep_pitch;
        self.speed = speed;
        self.keep_pitch = keep_pitch;
        self.looping = looping;
        self.apply_playback(changed);
    }

    /// Visuals later (positive) or earlier than the sound, in seconds:
    /// Bluetooth and some interfaces report less output latency than
    /// they have.
    pub fn set_av_offset(&mut self, seconds: f64) {
        self.av_offset = seconds;
    }

    fn apply_playback(&mut self, restretch: bool) {
        let (Some(shared), Some(player)) = (&self.shared, &self.player) else { return };
        shared.speed.store(self.speed.to_bits(), Ordering::Relaxed);
        shared.looping.store(self.looping, Ordering::Relaxed);
        shared.keep_pitch.store(self.keep_pitch, Ordering::Relaxed);
        if !restretch {
            return;
        }
        let generation = self.stretch_gen.fetch_add(1, Ordering::Relaxed) + 1;
        if !self.keep_pitch || (self.speed - 1.0).abs() < 1e-3 {
            shared.stretched.store(None);
            return;
        }
        let (samples, channels) = player.samples();
        let (shared, speed, latest) =
            (Arc::clone(shared), self.speed, Arc::clone(&self.stretch_gen));
        std::thread::spawn(move || {
            let stretched = stretch(&samples, channels, speed);
            // a newer speed was asked for while this one was being made
            if latest.load(Ordering::Relaxed) == generation {
                shared.stretched.store(Some(Arc::new(stretched)));
            }
        });
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// The live input's gain, and the level below which it counts as
    /// silence, both in dB.
    pub fn set_live_levels(&mut self, gain_db: f64, gate_db: f64) {
        self.live_gain_db = gain_db;
        self.live_gate_db = gate_db;
    }

    pub fn seek(&mut self, seconds: f64) {
        let (Some(shared), Some(track)) = (&self.shared, &self.track) else { return };
        let frame = (seconds.max(0.0) * track.sample_rate as f64) as usize;
        let frame = frame.min(track.frames());
        // hand it to the callback, which owns the read position
        shared.seek_to.store(frame as i64, Ordering::Relaxed);
        shared.cursor.store(frame, Ordering::Relaxed);
        self.last_position = seconds.max(0.0);
    }

    /// Analyse the window around the playhead and report it.
    pub fn tick(&mut self) -> Option<Snapshot> {
        if self.live.is_some() {
            return self.tick_live();
        }
        let track = self.track.as_ref()?;
        let player = self.player.as_ref()?;
        let shared = self.shared.as_ref()?;

        let position = player.position();
        // what the listener is hearing now, less any offset the panel set
        let heard = (position - self.av_offset).max(0.0);
        let sr = track.sample_rate as usize;
        let start = (heard * sr as f64) as usize;
        let mut window = vec![0.0f64; FFT_SIZE];
        for (i, w) in window.iter_mut().enumerate() {
            *w = self.mono.get(start + i).copied().unwrap_or(0.0) as f64;
        }

        let frame = self.analyzer.analyze(heard, &window, None);
        let (beat, punch) = self.grid.events_between(self.last_position, heard);
        self.last_position = heard;

        let peak = shared.peak_level().max(1e-6);
        Some(Snapshot {
            frame,
            position,
            duration: player.duration(),
            playing: player.is_playing(),
            peak_db: 20.0 * (peak as f64).log10(),
            bpm: if self.grid.bpm > 0.0 { self.grid.bpm } else { 0.0 },
            beat,
            punch,
            live: false,
        })
    }

    /// Analyse the newest window of the live input. There is no beat grid
    /// for sound that has not happened yet, so the beat and the tempo come
    /// from the analyzer's own realtime detector.
    fn tick_live(&mut self) -> Option<Snapshot> {
        let input = self.live.as_ref()?;
        let (mut left, mut right) = (vec![0.0f32; FFT_SIZE], vec![0.0f32; FFT_SIZE]);
        input.ring().latest(&mut left, &mut right);
        let (peak, _) =
            live::condition(&mut left, &mut right, self.live_gain_db, self.live_gate_db);
        // the pair stays stereo up to here; the spectrum is of its mid
        let window: Vec<f64> =
            left.iter().zip(&right).map(|(l, r)| (*l as f64 + *r as f64) * 0.5).collect();
        let t = self.live_started.elapsed().as_secs_f64();
        let frame = self.analyzer.analyze(t, &window, None);
        Some(Snapshot {
            beat: frame.beat,
            punch: frame.punch > 0.5,
            bpm: frame.bpm,
            position: 0.0,
            duration: 0.0,
            playing: false,
            peak_db: 20.0 * (peak.max(1e-6) as f64).log10(),
            live: true,
            frame,
        })
    }
}

impl Engine {
    /// Push the analysis settings the interface edits.
    pub fn set_analysis(&mut self, smoothing: f64, sensitivity: f64, min_hz: f64, max_hz: f64) {
        self.analyzer.settings.sensitivity = sensitivity;
        self.analyzer.settings.fft_min_hz = min_hz;
        self.analyzer.settings.fft_max_hz = max_hz;
        self.analyzer.set_smoothing(smoothing);
        self.analyzer.refresh_fft_range();
    }

    pub fn duration(&self) -> f64 {
        self.player.as_ref().map_or(0.0, |p| p.duration())
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.is_playing())
    }
}

/// The interleaved `samples` time-stretched to `speed`, pitch kept, one
/// channel at a time.
pub fn stretch(samples: &[f32], channels: usize, speed: f64) -> Stretched {
    let channels = channels.max(1);
    let split: Vec<Vec<f32>> = (0..channels)
        .map(|c| samples.iter().skip(c).step_by(channels).copied().collect())
        .collect();
    let done: Vec<Vec<f32>> =
        split.iter().map(|ch| nv_dsp::time_stretch(ch, speed, 2048, 512)).collect();
    let frames = done.iter().map(Vec::len).min().unwrap_or(0);
    let mut out = Vec::with_capacity(frames * channels);
    for i in 0..frames {
        for ch in &done {
            out.push(ch[i]);
        }
    }
    Stretched { samples: out, speed, frames }
}
