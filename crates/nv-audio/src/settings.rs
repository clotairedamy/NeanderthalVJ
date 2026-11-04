//! The analysis-facing slice of the application's settings.

/// Mirrors the fields of the Python `Settings` that the analyzer reads
/// (see `visualizer/config.py`); the rest of that struct belongs to the
/// visuals and the UI and is ported alongside them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisSettings {
    /// EMA factor for the smoothed readings, 0..0.98.
    pub smoothing: f64,
    /// Scales the band energies and the RMS.
    pub sensitivity: f64,
    pub fft_min_hz: f64,
    pub fft_max_hz: f64,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self { smoothing: 0.8, sensitivity: 1.0, fft_min_hz: 20.0, fft_max_hz: 16_000.0 }
    }
}
