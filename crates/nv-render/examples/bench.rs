//! How long a frame of the head takes on the GPU, with no window.
//!
//!     cargo run --release -p nv-render --example bench -- [points] [frames] [assets]
//!
//! Renders through the same `Scene` the app and the exporter use, fed a
//! synthetic 120 BPM pulse so every effect is live. Two numbers:
//!
//! - **latency**: wait for the GPU after each frame. Honest about one
//!   frame, but an idle GPU clocks down between frames, so it is noisy.
//! - **throughput**: submit every frame back to back and wait once. The
//!   GPU stays at full clock, so this is the steadier measure of cost.

use nv_audio::AnalysisFrame;
use nv_render::{bust::BustModel, Camera, Params, Scene};

const W: u32 = 1920;
const H: u32 = 1200;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(420_000);
    let frames: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(600);
    let assets = std::env::args().nth(3).unwrap_or_else(|| "assets".into());
    let synth = std::env::args().any(|a| a == "--synth");

    let model = BustModel::build(format!("{assets}/LeePerrySmith.glb"), None, n, 3.6, 7)
        .expect("build model");

    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).expect("no GPU adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default(), None)).expect("no device");
    println!("adapter: {}", adapter.get_info().name);

    let format = wgpu::TextureFormat::Bgra8Unorm;
    let view = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let mut scene = Scene::new(&device, format, &model, (W, H));
    let mut params = Params::default();
    params.knobs.synth = synth;

    let dt = 1.0 / 60.0;
    let mut times = Vec::with_capacity(frames);
    let mut frame = AnalysisFrame::default();
    let mut started = std::time::Instant::now();
    for i in 0..2 * frames + 30 {
        let latency = i < frames + 30;
        if i == frames + 30 {
            device.poll(wgpu::Maintain::Wait);
            started = std::time::Instant::now();
        }
        let t = i as f32 * dt;
        let phase = (t * 2.0).fract();
        frame.rms = (0.55 + 0.3 * (1.0 - phase)) as f64;
        for b in 0..7 {
            frame.bands[b] = (0.3 + 0.5 * ((t * (1.0 + b as f32)).sin() * 0.5 + 0.5)) as f64;
        }
        for (k, s) in frame.spectrum.iter_mut().enumerate() {
            *s = (0.5 + 0.5 * (t * 3.0 + k as f32 * 0.3).sin()) as f64;
        }
        frame.beat_phase = phase as f64;
        frame.bar_phase = ((t * 0.5).fract()) as f64;
        frame.bpm = 120.0;
        frame.pwm = 0.4;
        frame.wobble = 0.6;
        frame.wobble_hz = 0.5;
        frame.pitch_hz = 110.0;
        let beat = i % 30 == 0;
        frame.beat_strength = if beat { 0.9 } else { 0.0 };

        let t0 = std::time::Instant::now();
        scene.render(&device, &queue, &view, &frame, beat, dt, &params, Camera::default());
        if latency {
            device.poll(wgpu::Maintain::Wait);
            if i >= 30 {
                times.push(t0.elapsed().as_secs_f64() * 1e3);
            }
        }
    }
    device.poll(wgpu::Maintain::Wait);
    let throughput = started.elapsed().as_secs_f64() * 1e3 / frames as f64;
    times.sort_by(f64::total_cmp);
    let q = |p: f64| times[((times.len() - 1) as f64 * p) as usize];
    println!(
        "{n} particles, {W}x{H}: throughput {throughput:.3} ms/frame | latency p50 {:.2} ms  p95 {:.2} ms",
        q(0.5),
        q(0.95),
    );
}
