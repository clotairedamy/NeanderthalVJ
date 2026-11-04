//! Live input: a line in, a microphone or a loopback device, in stereo.
//!
//! The device's first two channels are kept as a pair (a mono device is
//! heard in both). Nothing is played back, so there is no feedback loop to
//! worry about. As in the player, the callback runs on a realtime thread
//! and must not lock or allocate: it writes into a ring of atomics, and the
//! analysis reads the newest window back out of it.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Frames the ring holds: several analysis windows, so a slow tick never
/// reads a window the callback is halfway through replacing.
const RING: usize = 1 << 14;

/// The newest stereo frames, written by the callback, read by the analysis.
pub struct Ring {
    left: Vec<AtomicU32>,
    right: Vec<AtomicU32>,
    /// Frames written since the stream started.
    written: AtomicUsize,
}

impl Default for Ring {
    fn default() -> Self {
        let zeros = || (0..RING).map(|_| AtomicU32::new(0)).collect();
        Self { left: zeros(), right: zeros(), written: AtomicUsize::new(0) }
    }
}

impl Ring {
    pub fn push(&self, l: f32, r: f32) {
        let n = self.written.load(Ordering::Relaxed);
        self.left[n % RING].store(l.to_bits(), Ordering::Relaxed);
        self.right[n % RING].store(r.to_bits(), Ordering::Relaxed);
        self.written.store(n + 1, Ordering::Release);
    }

    /// The newest `left.len()` frames, oldest first; silence before the
    /// stream has filled them.
    pub fn latest(&self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len()).min(RING);
        let end = self.written.load(Ordering::Acquire);
        for i in 0..n {
            let (l, r) = match (end + i).checked_sub(n) {
                Some(at) => (
                    f32::from_bits(self.left[at % RING].load(Ordering::Relaxed)),
                    f32::from_bits(self.right[at % RING].load(Ordering::Relaxed)),
                ),
                None => (0.0, 0.0),
            };
            left[i] = l;
            right[i] = r;
        }
    }
}

/// Apply the gain to both channels and close the gate below its level.
///
/// Returns the peak after the gain, and whether the gate is open. A closed
/// gate zeroes the window: room tone and hum should leave the visuals
/// still, not twitching.
pub fn condition(left: &mut [f32], right: &mut [f32], gain_db: f64, gate_db: f64) -> (f32, bool) {
    let gain = 10f64.powf(gain_db / 20.0) as f32;
    let mut peak = 0.0f32;
    let mut energy = 0.0f64;
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
        *l *= gain;
        *r *= gain;
        peak = peak.max(l.abs()).max(r.abs());
        energy += (*l as f64).powi(2) + (*r as f64).powi(2);
    }
    let frames = left.len().max(1) as f64;
    let rms_db = 10.0 * (energy / (2.0 * frames) + 1e-20).log10();
    let open = rms_db >= gate_db;
    if !open {
        left.fill(0.0);
        right.fill(0.0);
    }
    (peak, open)
}

/// The input devices this machine offers, by name.
pub fn input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// An open input stream feeding a [`Ring`].
pub struct LiveInput {
    _stream: cpal::Stream,
    ring: Arc<Ring>,
    sample_rate: u32,
    device: String,
}

impl LiveInput {
    /// Open `device` by name, or the default input when it is empty or no
    /// longer there.
    pub fn open(device: &str) -> Result<Self, String> {
        let host = cpal::default_host();
        let chosen = host.input_devices().ok().and_then(|mut all| {
            all.find(|d| !device.is_empty() && d.name().ok().as_deref() == Some(device))
        });
        let device = match chosen {
            Some(d) => d,
            None => host.default_input_device().ok_or("there is no audio input on this machine")?,
        };
        let name = device.name().unwrap_or_else(|_| "audio input".into());
        let supported = device.default_input_config().map_err(|e| e.to_string())?;
        let channels = supported.channels() as usize;
        let sample_rate = supported.sample_rate().0;
        let config: cpal::StreamConfig = supported.clone().into();
        let ring = Arc::new(Ring::default());

        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, &config, channels, &ring),
            SampleFormat::I16 => build::<i16>(&device, &config, channels, &ring),
            SampleFormat::U16 => build::<u16>(&device, &config, channels, &ring),
            SampleFormat::I32 => build::<i32>(&device, &config, channels, &ring),
            other => return Err(format!("{name}: unsupported sample format {other:?}")),
        }
        .map_err(|e| format!("{name}: {e}"))?;
        stream.play().map_err(|e| format!("{name}: {e}"))?;
        Ok(Self { _stream: stream, ring, sample_rate, device: name })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    pub fn ring(&self) -> &Ring {
        &self.ring
    }
}

/// An input stream of sample type `T` that writes the first channel pair
/// into `ring`.
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    ring: &Arc<Ring>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let ring = Arc::clone(ring);
    let channels = channels.max(1);
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            for frame in data.chunks_exact(channels) {
                let l = frame[0].to_sample::<f32>();
                // a mono device is heard in both channels
                let r = if channels > 1 { frame[1].to_sample::<f32>() } else { l };
                ring.push(l, r);
            }
        },
        |e| eprintln!("live input error: {e}"),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_returns_the_newest_frames_in_order() {
        let ring = Ring::default();
        let (mut l, mut r) = (vec![9.0; 4], vec![9.0; 4]);
        ring.latest(&mut l, &mut r);
        assert_eq!(l, [0.0; 4], "an empty ring is silence");
        for i in 0..(RING + 6) {
            ring.push(i as f32, -(i as f32));
        }
        ring.latest(&mut l, &mut r);
        let last = (RING + 5) as f32;
        assert_eq!(l, [last - 3.0, last - 2.0, last - 1.0, last]);
        assert_eq!(r, [-(last - 3.0), -(last - 2.0), -(last - 1.0), -last]);
    }

    #[test]
    fn the_channels_stay_apart() {
        let ring = Ring::default();
        for _ in 0..8 {
            ring.push(0.5, -0.25);
        }
        let (mut l, mut r) = (vec![0.0; 8], vec![0.0; 8]);
        ring.latest(&mut l, &mut r);
        assert!(l.iter().all(|&s| s == 0.5) && r.iter().all(|&s| s == -0.25));
    }

    #[test]
    fn gain_scales_both_channels_and_the_gate_silences_hum() {
        let (mut l, mut r) = (vec![0.1f32; 64], vec![-0.1f32; 64]);
        let (peak, open) = condition(&mut l, &mut r, 6.0, -55.0);
        assert!(open);
        assert!((peak - 0.1995).abs() < 1e-3, "6 dB is about twice: {peak}");
        assert!((r[0] + 0.1995).abs() < 1e-3);

        // -80 dB of hum under a -55 dB gate
        let (mut l, mut r) = (vec![1e-4f32; 64], vec![1e-4f32; 64]);
        let (_, open) = condition(&mut l, &mut r, 0.0, -55.0);
        assert!(!open);
        assert!(l.iter().chain(&r).all(|&s| s == 0.0));
    }
}
