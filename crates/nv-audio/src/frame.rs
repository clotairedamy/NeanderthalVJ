//! One analysis snapshot, as handed to the visuals.

/// The seven bands the whole application is built around.
pub const BAND_NAMES: [&str; 7] =
    ["sub_bass", "bass", "low_mid", "mid", "high_mid", "treble", "presence"];
pub const BAND_EDGES: [(f64, f64); 7] = [
    (20.0, 60.0),
    (60.0, 250.0),
    (250.0, 500.0),
    (500.0, 2000.0),
    (2000.0, 4000.0),
    (4000.0, 8000.0),
    (8000.0, 16000.0),
];
pub const STEMS: [&str; 4] = ["vocals", "drums", "bass", "other"];
pub const FFT_SIZE: usize = 2048;
pub const N_BANDS: usize = 7;
pub const SPECTRUM_BINS: usize = 64;

/// A stem's contribution to one tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct StemReading {
    pub bands: [f64; N_BANDS],
    pub energy: f64,
}

/// One 30 Hz analysis snapshot consumed by the visualizations.
#[derive(Debug, Clone)]
pub struct AnalysisFrame {
    pub time: f64,
    pub bands: [f64; N_BANDS],
    /// Indexed by [`STEMS`].
    pub stems: [StemReading; 4],
    pub rms: f64,
    pub attack: [f64; N_BANDS],
    /// Low-band transient (kick/bass hit), 0..1.
    pub punch: f64,
    pub beat: bool,
    pub beat_strength: f64,
    pub bpm: f64,
    /// 0 = a clean tone, 1 = white noise.
    pub flatness: f64,
    /// Pulse width: 0 = hollow square, 1 = narrow.
    pub pwm: f64,
    /// The oscillator's fundamental, 0 if none.
    pub pitch_hz: f64,
    /// 0 damped, 1 a narrow resonant peak.
    pub resonance: f64,
    /// Depth of a slow modulation (LFO, tremolo), and its rate.
    pub wobble: f64,
    pub wobble_hz: f64,
    /// How fast the spectrum's shape is changing.
    pub morph: f64,
    /// 0 on the beat -> 1 just before the next.
    pub beat_phase: f64,
    /// The same across a 4-beat bar.
    pub bar_phase: f64,
    /// Normalized 0..1.
    pub centroid: f64,
    pub spectrum: [f64; SPECTRUM_BINS],
    pub waveform: Vec<f64>,
}

impl Default for AnalysisFrame {
    fn default() -> Self {
        Self {
            time: 0.0,
            bands: [0.0; N_BANDS],
            stems: [StemReading::default(); 4],
            rms: 0.0,
            attack: [0.0; N_BANDS],
            punch: 0.0,
            beat: false,
            beat_strength: 0.0,
            bpm: 0.0,
            flatness: 0.0,
            pwm: 0.0,
            pitch_hz: 0.0,
            resonance: 0.0,
            wobble: 0.0,
            wobble_hz: 0.0,
            morph: 0.0,
            beat_phase: 0.0,
            bar_phase: 0.0,
            centroid: 0.0,
            spectrum: [0.0; SPECTRUM_BINS],
            waveform: vec![0.0; FFT_SIZE],
        }
    }
}

impl AnalysisFrame {
    /// The reading for a named stem, or `None` if the name is not a stem.
    pub fn stem(&self, name: &str) -> Option<&StemReading> {
        STEMS.iter().position(|s| *s == name).map(|i| &self.stems[i])
    }
}
