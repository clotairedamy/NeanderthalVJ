//! Render a mode to a PNG, through the same `Scene` the app draws with,
//! driven by a synthetic 120 BPM track.
//!
//!     cargo run --release -p nv-render --example genshot -- out.png [seconds] [WxH] [generative|grid|head|head:<effect>] [fx]
//!
//! `fx` turns every effect on, as the FX tab would.
use std::f32::consts::TAU;

use nv_audio::AnalysisFrame;
use nv_render::{
    BustModel, Camera, GenKnobs, GridKnobs, HeadEffect, Impact, Knobs, Params, PostKnobs, Scene,
};

/// Music without a file: a kick on every beat, a bassline, mids that swell
/// across the bar and hats on the off-beats.
fn music(t: f32) -> (AnalysisFrame, bool) {
    let beat_len = 0.5;
    let phase = (t / beat_len).fract();
    let bar = (t / (beat_len * 4.0)).fract();
    let kick = (-phase * 9.0).exp();
    let hat = (-((phase + 0.5).fract()) * 14.0).exp();
    let swell = 0.5 - 0.5 * (bar * TAU).cos();
    let bands = [
        0.35 + 0.65 * kick,
        0.3 + 0.4 * kick + 0.2 * (t * 1.3).sin().abs(),
        0.25 + 0.5 * swell,
        0.2 + 0.6 * swell * (t * 0.7).sin().abs(),
        0.15 + 0.4 * swell,
        0.1 + 0.7 * hat,
        0.08 + 0.6 * hat,
    ];
    let mut f = AnalysisFrame {
        bands: bands.map(|b| b as f64),
        rms: (0.3 + 0.5 * kick) as f64,
        beat_strength: 0.9,
        bar_phase: bar as f64,
        beat_phase: phase as f64,
        ..Default::default()
    };
    f.attack[0] = (kick * 0.9) as f64;
    f.attack[1] = (kick * 0.6) as f64;
    f.attack[5] = (hat * 0.8) as f64;
    (f, phase < 1.0 / 30.0)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "generative.png".into());
    let seconds: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(6.0);
    let (w, h) = args
        .next()
        .and_then(|s| {
            let (a, b) = s.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((1080u32, 1080u32));
    let mode = args.next().unwrap_or_else(|| "generative".into());
    let fx = args.next().as_deref() == Some("fx");

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("a GPU");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default(), None)).expect("a device");
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    // the head at the app's full density, with its normal map, so a shot of
    // it looks like the window; the other modes only need the model's shape
    let head = mode.starts_with("head");
    let model = BustModel::build(
        "assets/LeePerrySmith.glb",
        head.then_some(std::path::Path::new("assets/Infinite-Level_02_Tangent_SmoothUV.jpg")),
        if head { 420_000 } else { 2_000 },
        3.6,
        7,
    )
    .expect("assets/");
    let mut scene = Scene::new(&device, format, &model, (w, h));
    let post = if fx {
        PostKnobs {
            bloom: 0.8,
            threshold: 0.55,
            flash: 1.0,
            grain: 0.4,
            chroma: 0.6,
            vignette: 0.6,
            ghost: 0.5,
        }
    } else {
        PostKnobs::default()
    };
    let params = match mode.as_str() {
        "grid" => Params {
            grid: Some(GridKnobs { tilt: 0.55, impact: Impact::Scattered, ..GridKnobs::default() }),
            post,
            ..Default::default()
        },
        m if m.starts_with("head") => {
            // `head:orbits` and the like pick a head effect
            let effect =
                HeadEffect::from_name(m.trim_start_matches("head").trim_start_matches(':'));
            Params { post, knobs: Knobs { effect, ..Knobs::default() }, ..Default::default() }
        }
        _ => Params { generative: Some(GenKnobs::default()), post, ..Default::default() },
    };

    let dt = 1.0 / 60.0;
    let frames = (seconds / dt) as u32;
    for i in 0..frames {
        let (f, beat) = music(i as f32 * dt);
        scene.render(&device, &queue, &view, &f, beat, dt, &params, Camera::default());
    }

    let row = w * 4;
    let padded = row.next_multiple_of(256);
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer: &buf,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    queue.submit([enc.finish()]);
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = buf.slice(..).get_mapped_range();
    let mut px = Vec::with_capacity((row * h) as usize);
    for y in 0..h {
        let start = (y * padded) as usize;
        px.extend_from_slice(&data[start..start + row as usize]);
    }
    image::save_buffer(&out, &px, w, h, image::ColorType::Rgba8).expect("write the png");
    println!("wrote {out}: {frames} frames, {w}x{h}");
}
