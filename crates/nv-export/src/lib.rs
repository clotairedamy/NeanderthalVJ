//! Offline video export, after `visualizer/export.py`.
//!
//! Live recording would be bounded by how fast frames can be read back
//! while the screen is also being drawn. This steps time in exact `1/fps`
//! increments instead, so the result is a true constant-rate file with
//! every frame rendered, whatever the machine. The picture comes from the
//! same [`nv_render::Scene`] the window draws, so the clip looks like what
//! was on screen.
//!
//! Frames are rendered straight at the target size -- 1080x1920 for a
//! Reel, 1080x1080 for a square post -- and piped to ffmpeg as raw RGBA,
//! which encodes H.264 and muxes the original audio file in as AAC.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use nv_audio::{AnalysisSettings, AudioAnalyzer, FFT_SIZE};
use nv_engine::Engine;
use nv_render::{bust::BustModel, Camera, Params, Scene};

/// Where the clip is going, which decides its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Instagram Reel, 9:16.
    Reel,
    /// Instagram square post, 1:1.
    Square,
}

impl Format {
    pub fn size(self) -> (u32, u32) {
        match self {
            Format::Reel => (1080, 1920),
            Format::Square => (1080, 1080),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Reel => "Reel",
            Format::Square => "Square",
        }
    }
}

pub struct Job {
    pub audio: PathBuf,
    pub out: PathBuf,
    pub format: Format,
    pub fps: u32,
    pub params: Params,
    pub camera: Camera,
    /// The folder holding the scan and its normal map.
    pub assets: PathBuf,
    /// Where the clip starts in the track, in seconds.
    pub start: f64,
    /// How long it runs, in seconds; 0 for to the end of the track.
    pub length: f64,
    /// The video bitrate for the hardware encoders, in Mbit/s; the
    /// software one takes a matching quality instead.
    pub bitrate_mbps: u32,
    /// The camera's orbit, in radians a second, as the auto camera turns
    /// it on screen; 0 for a still camera.
    pub orbit: f32,
}

impl Job {
    /// The stretch of the track to render, in seconds, for a track
    /// `duration` long.
    pub fn span(&self, duration: f64) -> (f64, f64) {
        let begin = self.start.clamp(0.0, duration.max(0.0));
        let end = if self.length > 0.0 { (begin + self.length).min(duration) } else { duration };
        (begin, end.max(begin))
    }

    /// The encoder's arguments at this job's quality.
    fn codec_args(&self, codec: &[&str]) -> Vec<String> {
        let rate = format!("{}M", self.bitrate_mbps.max(1));
        // the software encoder: lower CRF is higher quality, 16 against the
        // usual 18 for the high setting
        let crf = if self.bitrate_mbps >= 16 { "16" } else { "20" };
        let software = codec.contains(&"libx264");
        codec
            .iter()
            .map(|a| match *a {
                "16M" => rate.clone(),
                "18" if software => crf.to_string(),
                other => other.to_string(),
            })
            .collect()
    }
}

/// Progress and cancellation, shared with whoever started the job.
#[derive(Default)]
pub struct Progress {
    /// Frames rendered so far, and the total.
    pub done: AtomicU32,
    pub total: AtomicU32,
    pub cancel: AtomicBool,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        let t = self.total.load(Ordering::Relaxed).max(1);
        self.done.load(Ordering::Relaxed) as f32 / t as f32
    }
}

/// ffmpeg's file name on this platform.
const FFMPEG: &str = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };

/// Find an ffmpeg to encode with.
///
/// `NV_FFMPEG` wins; then anything on `PATH`; then where the usual
/// package managers put it, searched by name because an app started from
/// Finder or Explorer can get a `PATH` without them.
pub fn find_ffmpeg() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("NV_FFMPEG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).map(|d| d.join(FFMPEG)).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        candidates.push("/opt/homebrew/bin/ffmpeg".into());
        candidates.push("/usr/local/bin/ffmpeg".into());
    }
    if cfg!(windows) {
        // winget, Scoop and Chocolatey
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(Path::new(&local).join(r"Microsoft\WinGet\Links\ffmpeg.exe"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            candidates.push(Path::new(&home).join(r"scoop\shims\ffmpeg.exe"));
        }
        candidates.push(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe".into());
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// The encoders to try, best first, each with the pixel format it takes.
/// All of them give what Instagram asks for: H.264, 8-bit 4:2:0. The
/// hardware ones keep encoding up with rendering; the bitrate is generous
/// because Instagram re-encodes, and particle grain is the first detail
/// every generation loses.
const ENCODERS: &[&[&str]] = &[
    // the Mac's
    &["-c:v", "h264_videotoolbox", "-b:v", "16M", "-profile:v", "high", "-pix_fmt", "yuv420p"],
    // NVIDIA's, Intel's and AMD's
    &[
        "-c:v",
        "h264_nvenc",
        "-preset",
        "p5",
        "-b:v",
        "16M",
        "-profile:v",
        "high",
        "-pix_fmt",
        "yuv420p",
    ],
    &["-c:v", "h264_qsv", "-b:v", "16M", "-profile:v", "high", "-pix_fmt", "nv12"],
    &["-c:v", "h264_amf", "-b:v", "16M", "-profile:v", "high", "-pix_fmt", "yuv420p"],
];

/// Software encoding, the Python's settings: always there, just slower.
const SOFTWARE: &[&str] =
    &["-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p"];

/// The best encoder that actually works here. A build of ffmpeg lists
/// NVENC whether or not there is an NVIDIA card in the machine, so each
/// listed hardware encoder is tried on one small frame before it is
/// trusted with an export.
fn video_codec(ffmpeg: &Path) -> &'static [&'static str] {
    let listed = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    ENCODERS
        .iter()
        .find(|args| listed.contains(args[1]) && encodes(ffmpeg, args))
        .copied()
        .unwrap_or(SOFTWARE)
}

/// Whether ffmpeg can encode one frame with `args`.
fn encodes(ffmpeg: &Path, args: &[&str]) -> bool {
    Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error"])
        .args(["-f", "lavfi", "-i", "color=c=black:s=256x256:r=30", "-frames:v", "1"])
        .args(args)
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Render and encode the whole track. Blocks; run it on a worker thread.
/// Returns the output path, or why it failed.
pub fn run(job: &Job, progress: &Progress) -> Result<PathBuf, String> {
    let ffmpeg = find_ffmpeg().ok_or(if cfg!(windows) {
        "ffmpeg was not found. Install it with `winget install Gyan.FFmpeg`, or set NV_FFMPEG."
    } else {
        "ffmpeg was not found. Install it with `brew install ffmpeg`, or set NV_FFMPEG."
    })?;

    // decode and track the song exactly as the live view does
    let prepared = Engine::prepare(&job.audio.to_string_lossy())?;
    let sr = prepared.sample_rate();
    let mono = prepared.mono();
    let grid = prepared.grid();
    let duration = prepared.duration();
    let (begin, end) = job.span(duration);
    let fps = job.fps.max(1);
    let total = ((end - begin) * fps as f64).ceil() as u32;
    progress.total.store(total, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);

    let (w, h) = job.format.size();
    let model = BustModel::build(
        job.assets.join("LeePerrySmith.glb"),
        Some(&job.assets.join("Infinite-Level_02_Tangent_SmoothUV.jpg")),
        420_000,
        3.6,
        7,
    )
    .map_err(|e| e.to_string())?;

    // a GPU of our own, separate from the window's
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .ok_or("no GPU adapter for the export")?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None))
        .map_err(|e| format!("no GPU device for the export: {e}"))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("export frame"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut scene = Scene::new(&device, format, &model, (w, h));
    scene.set_viewport(0.0, 0.0, w as f32, h as f32);

    // rows are padded to 256 bytes on the GPU side; ffmpeg wants them tight
    let row = w * 4;
    let padded = row.next_multiple_of(256);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("export readback"),
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let tmp = job.out.with_extension("partial.mp4");
    let mut child = Command::new(&ffmpeg)
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}")])
        .args(["-r", &fps.to_string(), "-i", "-"])
        // only the stretch being rendered
        .args(["-ss", &format!("{begin:.3}"), "-t", &format!("{:.3}", end - begin)])
        .arg("-i")
        .arg(&job.audio)
        .args(["-map", "0:v", "-map", "1:a"])
        // the encoder brings its own pixel format; with AAC and the moov
        // atom up front, it is what Instagram asks for
        .args(job.codec_args(video_codec(&ffmpeg)))
        .args(["-c:a", "aac", "-b:a", "192k"])
        .args(["-shortest", "-movflags", "+faststart"])
        .arg(&tmp)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start ffmpeg: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg has no input")?;

    let mut analyzer = AudioAnalyzer::new(sr, AnalysisSettings::default());
    let analysis_dt = 1.0 / 30.0; // the Python's analysis rate
    let mut next_analysis = begin;
    let mut frame = nv_audio::AnalysisFrame::default();
    let mut window = vec![0.0f64; FFT_SIZE];
    let mut last_t = begin;
    let mut tight = vec![0u8; (row * h) as usize];
    let dt = 1.0 / fps as f32;

    let result = (|| -> Result<(), String> {
        for i in 0..total {
            if progress.cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            let t = begin + i as f64 / fps as f64;
            if t >= next_analysis {
                next_analysis = t + analysis_dt;
                let start = (t * sr as f64) as usize;
                for (k, v) in window.iter_mut().enumerate() {
                    *v = mono.get(start + k).copied().unwrap_or(0.0) as f64;
                }
                frame = analyzer.analyze(t, &window, None);
            }
            // a grid beat crossed since the last video frame
            let (beat, _) = grid.events_between(last_t, t);
            last_t = t;

            let mut camera = job.camera;
            camera.azimuth += job.orbit * (t - begin) as f32;
            scene.render(&device, &queue, &view, &frame, beat, dt, &job.params, camera);

            let mut enc = device.create_command_encoder(&Default::default());
            enc.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &readback,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            queue.submit([enc.finish()]);
            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            device.poll(wgpu::Maintain::Wait);
            {
                let data = slice.get_mapped_range();
                for r in 0..h as usize {
                    let src = r * padded as usize;
                    tight[r * row as usize..(r + 1) * row as usize]
                        .copy_from_slice(&data[src..src + row as usize]);
                }
            }
            readback.unmap();
            stdin.write_all(&tight).map_err(|e| format!("ffmpeg stopped taking frames: {e}"))?;
            progress.done.store(i + 1, Ordering::Relaxed);
        }
        Ok(())
    })();

    drop(stdin); // end of input: let ffmpeg finish the file
    let out = child.wait_with_output().map_err(|e| format!("ffmpeg: {e}"))?;
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffmpeg failed: {}", why.trim()));
    }
    std::fs::rename(&tmp, &job.out)
        .map_err(|e| format!("could not move the file into place: {e}"))?;
    Ok(job.out.clone())
}

/// Run a job on its own thread, reporting through `progress`.
pub fn spawn(
    job: Job,
    progress: Arc<Progress>,
) -> std::thread::JoinHandle<Result<PathBuf, String>> {
    std::thread::Builder::new()
        .name("nv-export".into())
        .spawn(move || run(&job, &progress))
        .expect("spawn the export thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    // These need an ffmpeg on the machine, and say so rather than fail.
    fn ffmpeg() -> Option<PathBuf> {
        let f = find_ffmpeg();
        if f.is_none() {
            eprintln!("skipped: no ffmpeg here");
        }
        f
    }

    #[test]
    fn an_encoder_this_machine_lacks_is_not_chosen() {
        let Some(ff) = ffmpeg() else { return };
        // listed by most builds, usable only with an NVIDIA card -- which
        // no Mac has
        if cfg!(target_os = "macos") {
            assert!(!encodes(&ff, ENCODERS[1]), "NVENC passed its probe on a Mac");
        }
        assert!(encodes(&ff, video_codec(&ff)), "the chosen encoder cannot encode");
    }

    #[test]
    fn the_mac_uses_its_hardware_encoder() {
        let Some(ff) = ffmpeg() else { return };
        if cfg!(target_os = "macos") {
            assert_eq!(video_codec(&ff)[1], "h264_videotoolbox");
        }
    }
}

#[cfg(test)]
mod job_tests {
    use super::*;

    fn job(start: f64, length: f64, mbps: u32) -> Job {
        Job {
            audio: PathBuf::new(),
            out: PathBuf::new(),
            format: Format::Reel,
            fps: 60,
            params: Params::default(),
            camera: Camera::default(),
            assets: PathBuf::new(),
            start,
            length,
            bitrate_mbps: mbps,
            orbit: 0.0,
        }
    }

    #[test]
    fn the_span_stays_inside_the_track() {
        assert_eq!(job(0.0, 0.0, 16).span(200.0), (0.0, 200.0));
        assert_eq!(job(30.0, 15.0, 16).span(200.0), (30.0, 45.0));
        assert_eq!(
            job(190.0, 60.0, 16).span(200.0),
            (190.0, 200.0),
            "runs to the end, not past it"
        );
        assert_eq!(job(500.0, 10.0, 16).span(200.0), (200.0, 200.0));
    }

    #[test]
    fn quality_sets_the_bitrate_or_the_crf() {
        let hw = ["-c:v", "h264_videotoolbox", "-b:v", "16M"];
        assert_eq!(job(0.0, 0.0, 24).codec_args(&hw)[3], "24M");
        let sw = ["-c:v", "libx264", "-preset", "medium", "-crf", "18"];
        assert_eq!(job(0.0, 0.0, 24).codec_args(&sw)[5], "16");
        assert_eq!(job(0.0, 0.0, 10).codec_args(&sw)[5], "20");
    }
}
