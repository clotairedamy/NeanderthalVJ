//! Window functions, matching `scipy.signal.get_window`.

/// `scipy.signal.get_window("hann", n, fftbins=True)`: the **periodic**
/// window, which is what librosa asks for. NumPy's `np.hanning` is the
/// symmetric one and differs in every sample -- see `nv_audio::num::hanning`
/// for that one, which the real-time analyzer uses.
pub fn hann_periodic(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()) as f32)
        .collect()
}

/// The symmetric variant, as `np.hanning(n)`.
pub fn hann_symmetric(n: usize) -> Vec<f32> {
    if n == 1 {
        return vec![1.0];
    }
    (0..n)
        .map(|i| {
            (0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos()) as f32
        })
        .collect()
}
