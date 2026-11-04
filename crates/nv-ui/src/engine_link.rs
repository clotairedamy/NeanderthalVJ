//! The bridge between the interface and the engine.
//!
//! There is no bridge in the old sense — no codegen, no serialization, no
//! process boundary. The engine is a Rust struct this crate owns. What
//! lives here is only the part Dioxus needs: where the engine is kept, and
//! how its state reaches the signals the components read.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use nv_engine::Engine;

use crate::stage::StageHandle;
use crate::state::{Readout, Settings, Sources, SCOPE_BARS};

/// Holds each hit for the Damping control: the bands and the level rise
/// at once and fall back no faster than `damping` per 60th of a second.
#[derive(Default)]
pub struct Hold {
    bands: [f64; 7],
    rms: f64,
}

impl Hold {
    pub fn apply(&mut self, frame: &mut nv_audio::AnalysisFrame, damping: f64, dt: f64) {
        let keep = damping.clamp(0.0, 0.99).powf(dt * 60.0);
        for (held, b) in self.bands.iter_mut().zip(frame.bands.iter_mut()) {
            *held = b.max(*held * keep);
            *b = *held;
        }
        self.rms = frame.rms.max(self.rms * keep);
        frame.rms = self.rms;
    }
}

/// Moves to the next mode every so many bars, and at a drop.
#[derive(Default)]
pub struct AutoSwitch {
    last_bar: f64,
    bars: u32,
    fast: f64,
    slow: f64,
    since: f64,
}

impl AutoSwitch {
    /// Whether to switch now, given this frame.
    pub fn due(&mut self, frame: &nv_audio::AnalysisFrame, s: &Settings, dt: f64) -> bool {
        self.since += dt;
        // a bar is crossed when its phase wraps
        let wrapped = frame.bar_phase < self.last_bar - 0.5;
        self.last_bar = frame.bar_phase;
        if wrapped {
            self.bars += 1;
        }
        // the level over a moment against the level over a phrase: a drop
        // is the first jumping well above the second
        self.fast += (frame.rms - self.fast) * (dt / 0.3).min(1.0);
        self.slow += (frame.rms - self.slow) * (dt / 6.0).min(1.0);
        let drop = s.auto_switch_drops && self.fast > 0.35 && self.fast > self.slow * 1.6;
        let every = wrapped && self.bars >= s.auto_switch_bars.max(1);
        // never twice in a few seconds, however busy the music
        if (every || drop) && self.since > 4.0 {
            self.bars = 0;
            self.since = 0.0;
            self.slow = self.fast;
            return true;
        }
        false
    }
}

/// The 64-bin spectrum folded into the scope's bars.
pub fn scope_bars(spectrum: &[f64]) -> [f32; SCOPE_BARS] {
    let per = (spectrum.len() / SCOPE_BARS).max(1);
    std::array::from_fn(|i| {
        spectrum[i * per..((i + 1) * per).min(spectrum.len())]
            .iter()
            .fold(0.0f64, |a, b| a.max(*b))
            .clamp(0.0, 1.0) as f32
    })
}

/// The engine, shared with the components that drive it.
///
/// `Rc<RefCell<…>>` rather than an `Arc<Mutex<…>>`: the audio stream is
/// not `Send` on every platform, and all of this runs on the UI thread
/// anyway — the realtime work happens inside the audio callback, which
/// owns nothing here.
pub type EngineHandle = Rc<RefCell<Engine>>;

/// How often the interface asks the engine what the music is doing.
/// 60 Hz: the analysis is cheap and the readout should not visibly lag the
/// sound.
const TICK: std::time::Duration = std::time::Duration::from_millis(16);

/// Install the engine and keep the readout fed from it.
pub fn use_engine(
    mut readout: Signal<Readout>,
    mut settings: Signal<Settings>,
    mut sources: Signal<Sources>,
    stage: StageHandle,
) -> EngineHandle {
    let engine: EngineHandle = use_hook(|| Rc::new(RefCell::new(Engine::new())));

    let poll = engine.clone();
    use_future(move || {
        let poll = poll.clone();
        let stage = stage.clone();
        async move {
            let mut tick = 0u64;
            let mut was_playing = false;
            // `NV_PROFILE`: how promptly the UI thread wakes. The worst gap
            // between wake-ups is what a click or a slider drag feels.
            let profile = std::env::var("NV_PROFILE").is_ok();
            if std::env::var("NV_PROFILE_FRONT").is_ok() {
                // measure as a user would see it: frontmost, not napping
                dioxus::desktop::window().set_focus();
            }
            let (mut prev, mut worst, mut at) =
                (std::time::Instant::now(), 0.0f64, std::time::Instant::now());
            let mut gaps: Vec<f64> = Vec::new();
            let begun = std::time::Instant::now();
            let mut hold = Hold::default();
            let mut auto = AutoSwitch::default();
            let dt = TICK.as_secs_f64();
            loop {
                tick += 1;
                if profile {
                    let gap = prev.elapsed().as_secs_f64();
                    worst = worst.max(gap);
                    // skip start-up, which is not what a user drags sliders in
                    if begun.elapsed().as_secs_f64() > 3.0 {
                        gaps.push(gap * 1e3);
                    }
                    if gaps.len() == 900 {
                        let mut g = gaps.clone();
                        g.sort_by(f64::total_cmp);
                        let q = |p: f64| g[((g.len() - 1) as f64 * p) as usize];
                        let over = g.iter().filter(|x| **x > 33.4).count();
                        eprintln!(
                            "gaps: p50 {:.1} p95 {:.1} p99 {:.1} max {:.1} ms | {:.1}% miss two frames",
                            q(0.5), q(0.95), q(0.99), g[g.len() - 1],
                            over as f64 / g.len() as f64 * 100.0
                        );
                        gaps.clear();
                    }
                    prev = std::time::Instant::now();
                    if at.elapsed().as_secs_f64() >= 2.0 {
                        eprintln!(
                            "profile: UI thread worst wake-up gap {:.1} ms (ideal 16)",
                            worst * 1e3
                        );
                        worst = 0.0;
                        at = std::time::Instant::now();
                    }
                }
                let snapshot = poll.borrow_mut().tick();
                // pressing play hands the visuals back to the track, so the
                // live switch follows the engine rather than the click
                let live = snapshot.as_ref().is_some_and(|s| s.live);
                if sources.peek().mic_live != live {
                    sources.write().mic_live = live;
                    if !live {
                        // the meter would otherwise hold the last live level
                        let mut r = readout.write();
                        r.level_db = Readout::default().level_db;
                        r.velocity = 0.0;
                    }
                }
                if let Some(mut snap) = snapshot {
                    let (damping, switching) = {
                        let s = settings.peek();
                        (s.damping, s.auto_switch_sections)
                    };
                    hold.apply(&mut snap.frame, damping, dt);
                    // only while there is music to follow
                    let moving = snap.playing || snap.live;
                    if switching && moving && auto.due(&snap.frame, &settings.peek(), dt) {
                        let next = settings.peek().viz_mode.next();
                        settings.write().viz_mode = next;
                    }
                    {
                        let mut p = stage.pulse.lock().unwrap();
                        p.frame = snap.frame.clone();
                        // never clear it here: the render thread does that
                        // when it has drawn the beat
                        p.beat |= snap.beat;
                    }
                    // The analysis runs at 60 Hz for the visuals, but the
                    // readout only needs 15: every write re-renders the
                    // status bar, the scope and the transport, and nobody
                    // reads a clock faster than that. A play/pause change
                    // goes through at once, so the button never lags.
                    if tick.is_multiple_of(4) || snap.playing != was_playing {
                        was_playing = snap.playing;
                        let mut r = readout.write();
                        r.bpm = snap.bpm;
                        // the frame rate the picture is actually drawn at
                        r.fps = stage.fps() as f64;
                        r.velocity = snap.frame.rms;
                        r.level_db = snap.peak_db;
                        r.position = snap.position;
                        r.duration = snap.duration;
                        r.playing = snap.playing;
                        r.spectrum = scope_bars(&snap.frame.spectrum);
                    }
                } else if tick.is_multiple_of(15) {
                    readout.write().fps = stage.fps() as f64;
                }
                tokio::time::sleep(TICK).await;
            }
        }
    });

    // keep the analysis settings the panels edit in step with the engine
    let sync = engine.clone();
    use_effect(move || {
        let s = settings.read();
        let mut e = sync.borrow_mut();
        e.set_analysis(s.smoothing, s.sensitivity, s.fft_min_hz, s.fft_max_hz);
        e.set_live_levels(s.input_gain_db, s.input_gate_db);
        e.set_playback(s.speed, s.preserve_pitch, s.looping);
        e.set_av_offset(s.av_offset_ms / 1000.0);
    });

    engine
}

/// The audio files the decoder reads.
pub const AUDIO_EXTENSIONS: [&str; 6] = ["wav", "mp3", "flac", "m4a", "aac", "mp4"];

/// Whether `path` looks like a track the decoder reads.
pub fn is_audio(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Ask for a file, then load it.
///
/// The dialog is asked for from a task rather than straight from the
/// click: on macOS a modal dialog runs the event loop inside the call, the
/// interface keeps redrawing meanwhile, and redrawing a component whose
/// event handler is still on the stack aborts the app.
pub fn pick_and_load(
    engine: EngineHandle,
    sources: Signal<Sources>,
    readout: Signal<Readout>,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
) {
    spawn(async move {
        let Some(picked) = rfd::AsyncFileDialog::new()
            .add_filter("Audio", &AUDIO_EXTENSIONS)
            .set_title("Load a track")
            .pick_file()
            .await
        else {
            return;
        };
        let path = picked.path().to_string_lossy().to_string();
        load_path(engine, path, sources, readout, busy, error);
    });
}

/// Decode and track a file **off this thread**, then hand it to the engine.
///
/// Decoding and beat-tracking a four-minute song is seconds of work. Doing
/// it here froze the window behind a spinning cursor until it finished, so
/// only the hand-over happens on the UI thread.
pub fn load_path(
    engine: EngineHandle,
    path: String,
    mut sources: Signal<Sources>,
    mut readout: Signal<Readout>,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
) {
    busy.set(true);
    error.set(None);

    let (tx, rx) = std::sync::mpsc::channel();
    let worker_path = path.clone();
    std::thread::spawn(move || {
        let _ = tx.send(nv_engine::Engine::prepare(&worker_path));
    });

    spawn(async move {
        // the worker is not async, so wait on it without holding the thread
        let prepared = loop {
            match rx.try_recv() {
                Ok(v) => break v,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    busy.set(false);
                    error.set(Some("loading stopped unexpectedly".into()));
                    return;
                }
            }
        };
        busy.set(false);
        match prepared.and_then(|p| {
            let (d, b) = (p.duration(), p.bpm());
            engine.borrow_mut().adopt(p).map(|()| (d, b))
        }) {
            Ok((duration, bpm)) => {
                sources.write().audio = Some(path);
                let mut r = readout.write();
                r.duration = duration;
                r.bpm = bpm;
                r.position = 0.0;
            }
            Err(e) => error.set(Some(e)),
        }
    });
}

/// Load a file into the engine and read back what it found.
///
/// Separate from the dialog so it can be tested: this is where the window
/// once died. A `borrow_mut()` written straight into a `match` scrutinee
/// stays alive for the whole match, so borrowing the engine again inside
/// an arm panicked with "RefCell already mutably borrowed" -- a runtime
/// fault the compiler cannot see. Each borrow here is its own statement.
pub fn load_track(engine: &EngineHandle, path: &str) -> Result<(f64, f64), String> {
    engine.borrow_mut().load(path)?;
    let e = engine.borrow();
    Ok((e.duration(), e.grid.bpm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nv_audio::AnalysisFrame;

    #[test]
    fn damping_holds_a_hit_and_lets_it_go() {
        let hit = AnalysisFrame { bands: [1.0; 7], rms: 1.0, ..Default::default() };
        let quiet = AnalysisFrame::default();
        let after = |damping: f64| {
            let mut h = Hold::default();
            h.apply(&mut hit.clone(), damping, 1.0 / 60.0);
            let mut f = quiet.clone();
            for _ in 0..6 {
                f = quiet.clone();
                h.apply(&mut f, damping, 1.0 / 60.0);
            }
            f.bands[0]
        };
        assert!(after(0.95) > after(0.70), "more damping should hold the hit longer");
        assert!(after(0.70) < 0.2, "low damping should let go quickly");
    }

    #[test]
    fn auto_switch_moves_on_after_the_bars_it_was_given() {
        let s = Settings {
            auto_switch_sections: true,
            auto_switch_bars: 4,
            auto_switch_drops: false,
            ..Settings::default()
        };
        let mut a = AutoSwitch::default();
        let mut switched = 0;
        // 120 BPM: a bar every two seconds, for twenty seconds
        for i in 0..1200 {
            let t = i as f64 / 60.0;
            let f = AnalysisFrame { bar_phase: (t / 2.0).fract(), rms: 0.4, ..Default::default() };
            if a.due(&f, &s, 1.0 / 60.0) {
                switched += 1;
            }
        }
        assert_eq!(switched, 2, "ten bars at four a switch is two switches");
    }

    #[test]
    fn auto_switch_catches_a_drop() {
        let s =
            Settings { auto_switch_sections: true, auto_switch_bars: 32, ..Settings::default() };
        let mut a = AutoSwitch::default();
        let quiet = AnalysisFrame { rms: 0.15, ..Default::default() };
        let loud = AnalysisFrame { rms: 0.9, ..Default::default() };
        for _ in 0..600 {
            assert!(!a.due(&quiet, &s, 1.0 / 60.0));
        }
        let fired = (0..60).any(|_| a.due(&loud, &s, 1.0 / 60.0));
        assert!(fired, "the drop went by unnoticed");
    }

    #[test]
    fn the_scope_folds_the_spectrum_into_its_bars() {
        let mut spectrum = [0.0f64; 64];
        spectrum[63] = 1.0;
        let bars = scope_bars(&spectrum);
        assert_eq!(bars[SCOPE_BARS - 1], 1.0);
        assert_eq!(bars[0], 0.0);
    }
}
