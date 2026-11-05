//! Faults that only the GPU's validation layer can see.
//!
//! Both of these shipped: one took the whole app down going fullscreen, the
//! other kept the Hiding Grid from ever appearing. Neither is visible to
//! the compiler or to any pure test, so they are checked here against a
//! real device, with validation errors captured instead of panicking.

use nv_audio::AnalysisFrame;
use nv_render::{
    bust::BustModel, BustRenderer, Camera, GenKnobs, GridKnobs, GridRenderer, GridUniforms,
    HeadEffect, HidingGrid, Knobs, Params, PostKnobs, Scene, Uniforms,
};

const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default(), None)).ok()?;
    Some(Gpu { device, queue })
}

fn target(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn model() -> BustModel {
    BustModel::build(format!("{ASSETS}/LeePerrySmith.glb"), None, 5_000, 3.6, 7)
        .expect("the scan in assets/")
}

/// Run `f`, returning the first validation error it raised.
fn validation<F: FnOnce()>(g: &Gpu, f: F) -> Option<String> {
    g.device.push_error_scope(wgpu::ErrorFilter::Validation);
    f();
    g.device.poll(wgpu::Maintain::Wait);
    pollster::block_on(g.device.pop_error_scope()).map(|e| e.to_string())
}

#[test]
fn a_viewport_larger_than_the_target_does_not_fault() {
    let Some(g) = gpu() else { return };
    let (w, h) = (640, 400);
    let view = target(&g.device, w, h);
    let mut r = BustRenderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &model(), (w, h));
    // Going fullscreen, the page reported a stage wider than the surface
    // still was -- this exact shape of rectangle took the app down.
    r.set_viewport(0.0, 60.0, w as f32 * 1.13, h as f32 * 0.78);
    let err = validation(&g, || r.render(&g.device, &g.queue, &view, &Uniforms::default()));
    assert!(err.is_none(), "an oversized viewport faulted:\n{}", err.unwrap());
    // and one that starts outside the target entirely
    r.set_viewport(w as f32 + 50.0, 10.0, 200.0, 100.0);
    let err = validation(&g, || r.render(&g.device, &g.queue, &view, &Uniforms::default()));
    assert!(err.is_none(), "an off-target viewport faulted:\n{}", err.unwrap());
}

#[test]
fn the_grid_draws_inside_the_heads_pass() {
    let Some(g) = gpu() else { return };
    let (w, h) = (480, 480);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let view = target(&g.device, w, h);
    let bust = BustRenderer::new(&g.device, format, &model(), (w, h));
    let mut gr = GridRenderer::new(&g.device, format);
    let mut grid = HidingGrid::default();
    let f = AnalysisFrame { rms: 0.4, ..Default::default() };
    grid.update(&f, 1.0 / 60.0, false, GridKnobs::default());
    gr.upload(&g.queue, &grid, &GridUniforms::default());
    // The grid's pipelines once declared no depth format while drawing in
    // a pass that has one, which WebGPU rejects -- so it never appeared.
    let err = validation(&g, || {
        bust.render_with(&g.device, &g.queue, &view, &Uniforms::default(), |pass| gr.draw(pass))
    });
    assert!(err.is_none(), "the grid's pipelines do not fit the pass:\n{}", err.unwrap());
}

#[test]
fn the_generative_geometry_composites_inside_the_heads_pass() {
    let Some(g) = gpu() else { return };
    let (w, h) = (360, 640);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let view = target(&g.device, w, h);
    let mut scene = Scene::new(&g.device, format, &model(), (w, h));
    let params = Params { generative: Some(GenKnobs::default()), ..Default::default() };
    let f = AnalysisFrame { bands: [0.6; 7], rms: 0.5, beat_strength: 0.8, ..Default::default() };
    // its composite draws in a pass with a depth attachment, as the grid's
    // does; and a stage that changes size remakes its trail images
    let err = validation(&g, || {
        for i in 0..12 {
            if i == 6 {
                scene.set_viewport(20.0, 40.0, 300.0, 500.0);
            }
            let cam = Camera::default();
            scene.render(&g.device, &g.queue, &view, &f, i % 4 == 0, 1.0 / 60.0, &params, cam);
        }
    });
    assert!(err.is_none(), "the generative geometry does not fit the pass:\n{}", err.unwrap());
}

#[test]
fn every_effect_composites_in_every_mode() {
    let Some(g) = gpu() else { return };
    let (w, h) = (400, 300);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let view = target(&g.device, w, h);
    let mut scene = Scene::new(&g.device, format, &model(), (w, h));
    // the stage is smaller than the target, as under the control column
    scene.set_viewport(120.0, 20.0, 260.0, 260.0);
    let all = PostKnobs {
        bloom: 1.0,
        threshold: 0.5,
        flash: 1.0,
        grain: 1.0,
        chroma: 1.0,
        vignette: 1.0,
        ghost: 1.0,
    };
    let mut modes = vec![
        Params { post: all, ..Default::default() },
        Params { post: all, band_axis: 4, band_push: 1.0, perspective: 1.0, ..Default::default() },
        Params { post: all, grid: Some(GridKnobs::default()), ..Default::default() },
        Params { post: all, generative: Some(GenKnobs::default()), ..Default::default() },
    ];
    // and every head effect, at full strength
    for effect in HeadEffect::ALL {
        let knobs = Knobs { effect, effect_amount: 2.0, ..Knobs::default() };
        modes.push(Params { post: all, knobs, ..Default::default() });
    }
    let mut f =
        AnalysisFrame { bands: [0.7; 7], rms: 0.6, beat_strength: 1.0, ..Default::default() };
    f.attack = [0.9; 7];
    for params in modes {
        let err = validation(&g, || {
            for i in 0..6 {
                let cam = Camera::default();
                scene.render(&g.device, &g.queue, &view, &f, i % 2 == 0, 1.0 / 60.0, &params, cam);
            }
        });
        assert!(err.is_none(), "the effects failed validation:\n{}", err.unwrap());
    }
}
