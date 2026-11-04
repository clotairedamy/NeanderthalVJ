//! Playback through the default output device.
//!
//! The audio callback runs on a realtime thread: it must not allocate,
//! lock, or block. So it owns nothing but an `Arc` to the samples and a
//! few atomics, and everything the rest of the app wants to know is read
//! back out of those. The one thing handed to it while it runs -- the
//! track stretched to a new speed -- arrives through an `ArcSwap`, which
//! the callback reads without taking a lock.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwapOption;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::decode::Track;

/// Fill one callback's worth of output from the track, advancing `pos`.
///
/// The device almost never runs at the file's rate -- 44.1 kHz material on
/// a 48 kHz output is the usual case -- so the read cursor moves by a
/// fraction and the samples either side are blended. Returns the new
/// position and the peak seen, and is a free function so it can be tested
/// without an audio device.
pub fn fill(
    out: &mut [f32],
    out_channels: usize,
    samples: &[f32],
    src_channels: usize,
    frames: usize,
    mut pos: f64,
    ratio: f64,
) -> (f64, f32) {
    let mut peak = 0.0f32;
    for frame in out.chunks_mut(out_channels) {
        if pos >= frames as f64 {
            frame.iter_mut().for_each(|s| *s = 0.0);
            continue;
        }
        let i = pos as usize;
        let t = (pos - i as f64) as f32;
        for (ch, s) in frame.iter_mut().enumerate() {
            let c = ch.min(src_channels - 1);
            let at = |k: isize| -> f32 {
                let idx = i as isize + k;
                if idx < 0 {
                    return samples.get(c).copied().unwrap_or(0.0);
                }
                samples.get(idx as usize * src_channels + c).copied().unwrap_or(0.0)
            };
            // Catmull-Rom through four samples. Straight-line interpolation
            // rolls the top end off and folds some of it back as aliasing;
            // a cubic through the neighbours keeps the treble where it was.
            let (p0, p1, p2, p3) = (at(-1), at(0), at(1), at(2));
            let v = p1
                + 0.5
                    * t
                    * ((p2 - p0)
                        + t * ((2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3)
                            + t * (3.0 * (p1 - p2) + p3 - p0)));
            *s = v;
            peak = peak.max(v.abs());
        }
        pos += ratio;
    }
    (pos, peak)
}

/// The track time-stretched to `speed` with its pitch kept: interleaved
/// like the original, `1 / speed` times as long.
pub struct Stretched {
    pub samples: Vec<f32>,
    pub speed: f64,
    pub frames: usize,
}

/// One callback's worth of playback at `speed`, from the original or, when
/// it matches the speed, from the stretched copy.
///
/// `pos` is always in the original's frames, whichever buffer is read, so
/// seeking, the clock and the analysis never need to know which one is
/// playing. Returns the new position, the peak, and whether the track
/// ended (it never ends while `looping`: it wraps to the start).
#[allow(clippy::too_many_arguments)]
pub fn advance(
    out: &mut [f32],
    out_channels: usize,
    samples: &[f32],
    stretched: Option<&Stretched>,
    src_channels: usize,
    frames: usize,
    pos: f64,
    ratio: f64,
    speed: f64,
    looping: bool,
) -> (f64, f32, bool) {
    let stretched = stretched.filter(|s| (s.speed - speed).abs() < 1e-9);
    // the buffer read, its length, how far one output frame moves through
    // it, and how its frames convert to the original's
    let (buf, len, step, scale) = match stretched {
        Some(s) => (s.samples.as_slice(), s.frames, ratio, s.speed),
        None => (samples, frames, ratio * speed, 1.0),
    };
    let mut at = pos / scale;
    let mut peak = 0.0f32;
    let mut done = 0;
    let total = out.len() / out_channels.max(1);
    let mut ended = false;
    while done < total {
        // play up to the end of the track, then wrap and carry on in the
        // same callback, so a loop has no gap at the seam
        let left = ((len as f64 - at) / step).ceil().max(0.0) as usize;
        let n = left.min(total - done);
        let chunk = &mut out[done * out_channels..(done + n) * out_channels];
        let (p, pk) = fill(chunk, out_channels, buf, src_channels, len, at, step);
        at = p;
        peak = peak.max(pk);
        done += n;
        if at >= len as f64 {
            if looping && len > 0 {
                at %= len as f64;
            } else {
                out[done * out_channels..].iter_mut().for_each(|s| *s = 0.0);
                ended = true;
                break;
            }
        }
    }
    (at * scale, peak, ended)
}

/// What the callback and the rest of the app share.
///
/// `cursor` is in frames and is the one piece of truth about where
/// playback is; the UI derives its clock from it rather than running a
/// timer of its own, so the readout cannot drift from what is heard.
pub struct Shared {
    pub cursor: AtomicUsize,
    pub playing: AtomicBool,
    pub finished: AtomicBool,
    /// Peak level of the last callback, as f32 bits, for the meter.
    pub peak: AtomicU64,
    /// A seek the callback has not applied yet, in frames; -1 for none.
    /// The callback owns the read position, so a seek has to be handed to
    /// it rather than written over the cursor it publishes.
    pub seek_to: AtomicI64,
    /// Playback speed as f64 bits; 1 is normal.
    pub speed: AtomicU64,
    /// Start again from the top instead of stopping at the end.
    pub looping: AtomicBool,
    /// Play the stretched copy (pitch kept) when it matches the speed.
    pub keep_pitch: AtomicBool,
    pub stretched: ArcSwapOption<Stretched>,
}

impl Shared {
    fn new() -> Self {
        Self {
            cursor: AtomicUsize::new(0),
            playing: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            peak: AtomicU64::new(0),
            seek_to: AtomicI64::new(-1),
            speed: AtomicU64::new(1.0f64.to_bits()),
            looping: AtomicBool::new(false),
            keep_pitch: AtomicBool::new(true),
            stretched: ArcSwapOption::empty(),
        }
    }

    pub fn speed(&self) -> f64 {
        f64::from_bits(self.speed.load(Ordering::Relaxed))
    }

    pub fn peak_level(&self) -> f32 {
        f32::from_bits(self.peak.load(Ordering::Relaxed) as u32)
    }
}

pub struct Player {
    _stream: cpal::Stream,
    shared: Arc<Shared>,
    samples: Arc<Vec<f32>>,
    channels: usize,
    sample_rate: u32,
    frames: usize,
}

#[derive(Debug)]
pub struct PlayError(pub String);

impl std::fmt::Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audio output: {}", self.0)
    }
}

impl std::error::Error for PlayError {}

impl Player {
    /// Open the default output and start a (paused) stream for this track.
    pub fn new(track: &Track) -> Result<Self, PlayError> {
        let host = cpal::default_host();
        let device =
            host.default_output_device().ok_or_else(|| PlayError("no output device".into()))?;
        let default = device.default_output_config().map_err(|e| PlayError(e.to_string()))?;

        // Ask the device to run at the file's own rate. Almost every Mac
        // output supports both 44.1 and 48 kHz, and matching the file means
        // no resampling at all -- the only way to be certain nothing is
        // coloured on the way out. Resampling is the fallback, not the plan.
        let wanted = cpal::SampleRate(track.sample_rate);
        let exact = device.supported_output_configs().ok().and_then(|cfgs| {
            cfgs.filter(|c| {
                c.sample_format() == default.sample_format()
                    && c.channels() == default.channels()
                    && c.min_sample_rate() <= wanted
                    && c.max_sample_rate() >= wanted
            })
            .map(|c| c.with_sample_rate(wanted))
            .next()
        });
        let shared = Arc::new(Shared::new());
        let samples: Arc<Vec<f32>> = Arc::new(track.samples.clone());
        // Windows' shared mode can list a rate and then refuse to open it,
        // so the file's rate is the first try and the device's own the
        // second, rather than the end of playback.
        let stream = match exact.map(|c| open_stream(&device, c, track, &shared, &samples)) {
            Some(Ok(stream)) => stream,
            Some(Err(e)) => {
                eprintln!("audio: the file's rate would not open ({e}); using the device's");
                open_stream(&device, default, track, &shared, &samples)?
            }
            None => open_stream(&device, default, track, &shared, &samples)?,
        };

        stream.play().map_err(|e| PlayError(e.to_string()))?;
        Ok(Self {
            _stream: stream,
            shared,
            samples,
            channels: track.channels.max(1),
            sample_rate: track.sample_rate,
            frames: track.frames(),
        })
    }

    pub fn shared(&self) -> Arc<Shared> {
        Arc::clone(&self.shared)
    }

    /// The interleaved samples, and how many channels they carry.
    pub fn samples(&self) -> (Arc<Vec<f32>>, usize) {
        (Arc::clone(&self.samples), self.channels)
    }

    pub fn set_playing(&self, on: bool) {
        if on && self.shared.finished.load(Ordering::Relaxed) {
            self.shared.finished.store(false, Ordering::Relaxed);
            self.shared.cursor.store(0, Ordering::Relaxed);
        }
        self.shared.playing.store(on, Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    /// Where playback has reached, in seconds.
    pub fn position(&self) -> f64 {
        self.shared.cursor.load(Ordering::Relaxed) as f64 / self.sample_rate.max(1) as f64
    }

    pub fn duration(&self) -> f64 {
        self.frames as f64 / self.sample_rate.max(1) as f64
    }
}

/// Start a (paused) output stream for `track` on `device` in `supported`.
fn open_stream(
    device: &cpal::Device,
    supported: cpal::SupportedStreamConfig,
    track: &Track,
    shared: &Arc<Shared>,
    samples: &Arc<Vec<f32>>,
) -> Result<cpal::Stream, PlayError> {
    let out_channels = supported.channels() as usize;
    let out_rate = supported.sample_rate().0;
    let config: StreamConfig = supported.clone().into();
    if out_rate == track.sample_rate {
        eprintln!("audio: {out_rate} Hz, matching the file -- no resampling");
    } else {
        eprintln!("audio: device {out_rate} Hz, file {} Hz -- resampling", track.sample_rate);
    }

    let src_channels = track.channels.max(1);
    let frames = track.frames();
    // The device rarely runs at the file's rate -- 44.1k files on a
    // 48k output is the common case -- so the read cursor advances by
    // a fraction and the samples either side of it are blended. Taking
    // the nearest one instead drops and repeats samples, which is
    // audible as a grainy edge on everything.
    let ratio = track.sample_rate as f64 / out_rate as f64;

    let cb_shared = Arc::clone(shared);
    let cb_samples = Arc::clone(samples);
    let mut pos = 0.0f64;

    let err_fn = |e| eprintln!("audio stream error: {e}");
    match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &config,
            move |out: &mut [f32], _| {
                let playing = cb_shared.playing.load(Ordering::Relaxed);
                // a seek asked for since the last callback
                let want = cb_shared.seek_to.swap(-1, Ordering::Relaxed);
                if want >= 0 {
                    pos = want as f64;
                }
                let peak;
                if playing {
                    let speed = cb_shared.speed();
                    let looping = cb_shared.looping.load(Ordering::Relaxed);
                    let keep = cb_shared.keep_pitch.load(Ordering::Relaxed);
                    let stretched = cb_shared.stretched.load();
                    let stretched = stretched.as_deref().filter(|_| keep);
                    let (p, pk, ended) = advance(
                        out,
                        out_channels,
                        &cb_samples,
                        stretched,
                        src_channels,
                        frames,
                        pos,
                        ratio,
                        speed,
                        looping,
                    );
                    pos = p;
                    peak = pk;
                    cb_shared.cursor.store(pos as usize, Ordering::Relaxed);
                    if ended {
                        cb_shared.finished.store(true, Ordering::Relaxed);
                        cb_shared.playing.store(false, Ordering::Relaxed);
                    }
                } else {
                    out.iter_mut().for_each(|s| *s = 0.0);
                    peak = 0.0;
                }
                cb_shared.peak.store(peak.to_bits() as u64, Ordering::Relaxed);
            },
            err_fn,
            None,
        ),
        other => return Err(PlayError(format!("unsupported sample format {other:?}"))),
    }
    .map_err(|e| PlayError(e.to_string()))
}
