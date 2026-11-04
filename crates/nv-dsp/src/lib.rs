//! The DSP librosa and SciPy provided for the Python tree.
//!
//! Only the surface the application actually calls is ported, and each
//! piece is checked numerically against librosa (see `tests/parity.rs`)
//! rather than against a reading of the papers -- librosa's conventions
//! (periodic windows, Slaney mel scaling, float32 filterbanks, its
//! particular beat-tracking dynamic program) are what the visuals were
//! tuned against, so matching the library matters more than matching the
//! literature.

pub mod beat;
pub mod filter;
pub mod mel;
pub mod onset;
pub mod spectrum;
pub mod stretch;
pub mod tempo;
pub mod window;

pub use beat::{beat_track, frames_to_time};
pub use filter::{BandType, butter, median_filter_1d, sosfiltfilt};
pub use onset::{Aggregate, PeakPicker, onset_detect, onset_strength};
pub use spectrum::{Spectrogram, Stft};
pub use stretch::{phase_vocoder, time_stretch};
pub use tempo::estimate_tempo;
