//! Real-time audio analysis: FFT, 7 frequency bands, beat/BPM, energy, EMA.
//!
//! Ported from `visualizer/audio/` in the Python tree. The port is checked
//! numerically rather than by eye: `tests/parity.rs` replays vectors
//! captured from the Python implementation and asserts this code agrees
//! with it, so the two cannot drift apart while the port is in progress.

pub mod analyzer;
pub mod beat;
pub mod ema;
pub mod frame;
pub mod num;
pub mod settings;
pub mod timbre;

pub use analyzer::AudioAnalyzer;
pub use beat::{BeatDetector, BeatTick};
pub use frame::{AnalysisFrame, BAND_EDGES, BAND_NAMES, FFT_SIZE, N_BANDS, STEMS, StemReading};
pub use settings::AnalysisSettings;
