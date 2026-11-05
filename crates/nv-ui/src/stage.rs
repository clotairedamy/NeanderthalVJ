//! The visualization, drawn behind the chrome on a thread of its own.
//!
//! ADS renders HTML, so the interface lives in a webview. The visuals
//! cannot: 420,000 points at the display's refresh rate is a GPU surface.
//! So the window carries both -- a GPU surface on the window itself, with
//! the webview transparent on top of it. Every element the page leaves
//! transparent becomes a window onto the visuals; that is why `.nv-stage`
//! sets no background.
//!
//! The surface is drawn from a **dedicated render thread**. Drawing it from
//! the UI thread held that thread a quarter of the time -- mostly blocked
//! waiting for the next vsync drawable -- and every click and slider drag
//! had to squeeze between those waits. Now the UI thread only publishes
//! what to draw (the latest analysis frame, the panels' settings, the
//! camera and the stage's rectangle) and the render thread vsyncs on its
//! own.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use dioxus::desktop::window;
use dioxus::prelude::*;
use nv_render::{
    bust::BustModel, motion::Knobs, palette, Camera, Division, Drive, GenKnobs, GridKnobs,
    HeadEffect, Impact, Params, PostKnobs, ReactKnobs, Scene, Source,
};

use crate::state::{Mode, Settings};

/// How many particles the head is sampled into.
const POINTS: usize = 420_000;

/// The latest thing the engine heard, for the visuals to draw.
///
/// The engine is ticked in exactly one place. When the renderer ticked it
/// too, each loop advanced the playhead past some of the grid beats and
/// the other never saw them.
#[derive(Default)]
pub struct Pulse {
    pub frame: nv_audio::AnalysisFrame,
    /// Set when a grid beat is crossed, cleared by the render thread when
    /// it has drawn it, so a beat fires once however the two interleave.
    pub beat: bool,
}

/// Where to draw, in physical pixels: the whole surface, and the stage
/// rectangle inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub surface: (u32, u32),
    pub stage: (f32, f32, f32, f32),
}

/// Everything the UI thread hands the render thread.
#[derive(Default)]
pub struct StageShared {
    pub pulse: Mutex<Pulse>,
    pub camera: Mutex<Camera>,
    pub params: Mutex<Params>,
    pub layout: Mutex<Option<Layout>>,
    /// The render thread's frame rate, as f32 bits, for the status bar.
    pub fps: AtomicU32,
    /// The rate to draw at: 60, or the display's full rate at 120.
    pub target_fps: AtomicU32,
    /// How fast the camera orbits on its own, as f32 bits; 0 for not.
    pub orbit: AtomicU32,
    pub stop: AtomicBool,
}

impl StageShared {
    pub fn fps(&self) -> f32 {
        f32::from_bits(self.fps.load(Ordering::Relaxed))
    }

    pub fn orbit(&self) -> f32 {
        f32::from_bits(self.orbit.load(Ordering::Relaxed))
    }
}

pub type StageHandle = Arc<StageShared>;

/// The colour stops of the palette the current mode wears.
pub fn palette_stops(g: &Settings) -> Vec<[f32; 3]> {
    match g.active_palette() {
        "image" if !g.image_palette.is_empty() => g.image_palette.clone(),
        "custom" => {
            let picked: Vec<[f32; 3]> =
                g.custom_palette.iter().filter_map(|h| palette::parse_hex(h)).collect();
            if picked.is_empty() {
                palette::stops("nsku")
            } else {
                picked
            }
        }
        // an image palette before any picture has been chosen
        "image" => palette::stops("nsku"),
        name => palette::stops(name),
    }
}

/// A number that differs from call to call, for picking at random.
pub fn seed_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0)
}

/// How fast the camera turns on its own, in radians a second.
pub fn orbit_rate(g: &Settings) -> f32 {
    if g.auto_camera {
        0.35 * g.auto_camera_speed as f32
    } else {
        0.0
    }
}

/// The panels' settings as the renderer wants them.
pub fn params_from(g: &Settings) -> Params {
    let stops = palette_stops(g);
    let at = |t: f32| palette::sample_stops(&stops, t);
    // The palette tints the grey. The head is a spot-lit photograph, so the
    // tint is mixed in rather than replacing the light, and "mono" means
    // no tint at all.
    let tint = at(0.62);
    let tint_mix = if g.active_palette() == "mono" { 0.0 } else { g.tint_strength as f32 };
    let mode = g.viz_mode;
    Params {
        knobs: Knobs {
            reactivity: g.pc_reactivity as f32,
            drum_glitch: g.pc_drum_glitch as f32,
            point_size: g.pc_point_size as f32,
            beat_impulse: g.beat_impulse as f32,
            // Synth Cloud is the point cloud played by an oscillator
            synth: g.pc_synth || mode == Mode::SynthCloud,
            rings: g.pc_brainwaves,
            mid_gain: g.pc_mid_gain as f32,
            high_gain: g.pc_high_gain as f32,
            ring_band: match g.pc_ring_band.as_str() {
                "mids" => 1,
                "highs" => 2,
                _ => 0,
            },
            // the head effect is the Depth Point Cloud's alone: Synth Cloud
            // is the head played by an oscillator, and keeps its sheet
            effect: if mode == Mode::PointCloud {
                HeadEffect::from_name(&g.pc_effect)
            } else {
                HeadEffect::None
            },
            effect_amount: g.pc_effect_amount as f32,
            punch: g.pc_camera_punch as f32,
            tint,
            tint_mix,
        },
        post: PostKnobs {
            bloom: g.fx_bloom as f32,
            threshold: g.fx_threshold as f32,
            flash: g.fx_flash as f32,
            grain: g.fx_grain as f32,
            chroma: g.fx_chroma as f32,
            vignette: g.fx_vignette as f32,
            ghost: g.fx_ghost as f32,
        },
        perspective: g.pc_perspective as f32,
        band_axis: match g.pc_band_axis.as_str() {
            "vertical" => 1,
            "depth" => 2,
            "horizontal" => 3,
            "radial" => 4,
            _ => 0,
        },
        band_push: g.pc_band_push as f32,
        grid: (mode == Mode::HidingGrid).then(|| GridKnobs {
            cols: g.grid_cols,
            noise: g.grid_noise as f32,
            warp: g.grid_warp as f32,
            flow: g.grid_flow as f32,
            beat: g.grid_beat as f32,
            beat_impulse: g.beat_impulse as f32,
            sensitivity: g.sensitivity as f32,
            waves: g.grid_waves as f32,
            wave_speed: g.grid_wave_speed as f32,
            echo: g.grid_echo as f32,
            impact: match g.grid_impact.as_str() {
                "scattered" => Impact::Scattered,
                "floor" => Impact::Floor,
                "sides" => Impact::Sides,
                _ => Impact::Centre,
            },
            twist: g.grid_twist as f32,
            sparkle: g.grid_sparkle as f32,
            analyser: g.grid_analyser,
            tilt: g.grid_tilt as f32,
            color: g.grid_color as f32,
            colors: [at(0.15), at(0.55), at(0.95)],
            // the scene sets the frame's real shape as it draws
            aspect: 1.0,
            wave_source: Source::from_name(&g.grid_wave_source),
            twist_by: Drive::from_name(&g.grid_twist_by, Drive::Mids),
            sparks_by: Drive::from_name(&g.grid_sparks_by, Drive::Highs),
            warp_by: Drive::from_name(&g.grid_warp_by, Drive::Lows),
        }),
        generative: (mode == Mode::Generative).then(|| GenKnobs {
            symmetry: g.gen_symmetry,
            complexity: g.gen_complexity as f32,
            trails: g.gen_trails as f32,
            evolve: g.gen_evolve as f32,
            // the whole palette, low to high, across the rings
            colors: [0.0, 0.25, 0.5, 0.75, 1.0].map(at),
            mid_gain: g.pc_mid_gain as f32,
            high_gain: g.pc_high_gain as f32,
            width: g.gen_width as f32,
            spin: g.gen_spin as f32,
            flow: g.gen_flow as f32,
            glow: g.gen_glow as f32,
            polygon_source: Source::from_name(&g.gen_polygon_source),
            spin_by: Drive::from_name(&g.gen_spin_by, Drive::Mids),
            shimmer_by: Drive::from_name(&g.gen_shimmer_by, Drive::Highs),
            unison: g.gen_unison,
            mutate_bars: g.gen_mutate_bars.max(1),
        }),
        react: {
            let r = g.reaction();
            ReactKnobs {
                auto_level: r.auto_level,
                dynamics: r.dynamics as f32,
                rise: r.rise as f32,
                fall: r.fall as f32,
                division: Division::from_name(&r.hits),
            }
        },
    }
}

/// The render thread: owns the surface and the scene, draws at vsync.
fn render_loop(
    shared: StageHandle,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    initial: (u32, u32),
    assets: String,
) {
    let fail = |what: &str| eprintln!("visualizer: {what}; the chrome will run without it");
    let Some(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
    else {
        return fail("no GPU adapter");
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
    else {
        return fail("no GPU device");
    };
    // A validation fault must not take the whole app down -- the chrome is
    // still useful without the visuals. Report it instead of panicking.
    device.on_uncaptured_error(Box::new(|e| eprintln!("visualizer: GPU error: {e}")));

    let model = match BustModel::build(
        format!("{assets}/LeePerrySmith.glb"),
        Some(std::path::Path::new(&format!("{assets}/Infinite-Level_02_Tangent_SmoothUV.jpg"))),
        POINTS,
        3.6,
        7,
    ) {
        Ok(m) => m,
        Err(e) => {
            return fail(&format!("{e}; set NV_ASSETS to the folder holding LeePerrySmith.glb"))
        }
    };

    let caps = surface.get_capabilities(&adapter);
    // linear, not sRGB: the shader writes the values the GL original wrote
    // straight to a plain framebuffer
    let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: initial.0.max(1),
        height: initial.1.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);
    let mut scene = Scene::new(&device, format, &model, (config.width, config.height));
    eprintln!("visualizer: {} — {POINTS} particles, own render thread", adapter.get_info().name);

    let mut last = std::time::Instant::now();
    let (mut frames, mut fps_at) = (0u32, std::time::Instant::now());
    let mut stale = false;
    let profile = std::env::var("NV_PROFILE").is_ok();
    let mut busy = 0.0f64;
    let mut next_frame = std::time::Instant::now();

    while !shared.stop.load(Ordering::Relaxed) {
        // follow the window and the stage
        if let Some(l) = *shared.layout.lock().unwrap() {
            let (w, h) = (l.surface.0.max(1), l.surface.1.max(1));
            if stale || (w, h) != (config.width, config.height) {
                config.width = w;
                config.height = h;
                surface.configure(&device, &config);
                scene.resize(&device, (w, h));
                stale = false;
            }
            let (x, y, sw, sh) = l.stage;
            scene.set_viewport(x, y, sw, sh);
        }

        // Pace to the chosen rate. Presenting at the display's full 120 Hz
        // under a transparent webview cost the UI thread real time -- its
        // worst stall went from ~25 ms to ~45 ms -- so 60 is the default,
        // as it was in the Python. The schedule is absolute (each frame's
        // deadline is the last one plus a period), because the drawable
        // usually comes back at once rather than on a vsync, and timing
        // from it drifted to ~71 fps.
        let target = shared.target_fps.load(Ordering::Relaxed).max(1) as f64;
        if target < 100.0 {
            let period = std::time::Duration::from_secs_f64(1.0 / target);
            let now = std::time::Instant::now();
            // fall behind by more than a frame and the schedule restarts,
            // rather than racing to catch up
            next_frame = (next_frame + period).max(now - period);
            if next_frame > now {
                std::thread::sleep(next_frame - now);
            }
        }
        let tex = match surface.get_current_texture() {
            Ok(t) => t,
            // the window changed under us (fullscreen, a resize): configure
            // again and try the next frame rather than faulting
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                stale = true;
                continue;
            }
            Err(wgpu::SurfaceError::Timeout) => continue,
            Err(wgpu::SurfaceError::OutOfMemory) => return fail("out of GPU memory"),
        };
        if tex.suboptimal {
            stale = true;
        }

        let dt = last.elapsed().as_secs_f32();
        last = std::time::Instant::now();
        // take the beat, so one grid beat drives exactly one response
        let (frame, beat) = {
            let mut p = shared.pulse.lock().unwrap();
            (p.frame.clone(), std::mem::take(&mut p.beat))
        };
        let params = *shared.params.lock().unwrap();
        let camera = {
            let mut cam = shared.camera.lock().unwrap();
            // the auto camera turns it a little every frame; a drag still
            // works on top, and it carries on from wherever it was left
            let rate = shared.orbit();
            if rate != 0.0 {
                cam.azimuth += rate * dt.min(0.1);
            }
            *cam
        };

        let t_work = std::time::Instant::now();
        let view = tex.texture.create_view(&Default::default());
        scene.render(&device, &queue, &view, &frame, beat, dt, &params, camera);
        tex.present();
        busy += t_work.elapsed().as_secs_f64();

        frames += 1;
        if fps_at.elapsed().as_secs_f32() >= 1.0 {
            let fps = frames as f32 / fps_at.elapsed().as_secs_f32();
            shared.fps.store(fps.to_bits(), Ordering::Relaxed);
            if profile {
                eprintln!(
                    "profile: render thread {fps:.0} fps, {:.2} ms of work per frame, surface {}x{}",
                    busy / frames as f64 * 1e3,
                    config.width,
                    config.height
                );
            }
            busy = 0.0;
            frames = 0;
            fps_at = std::time::Instant::now();
        }
    }
}

/// Where the head model lives: `NV_ASSETS` if set; else an `assets`
/// folder beside the program, which is how a packaged build ships it;
/// else the workspace's own, found from where this crate was built, so a
/// build from source runs whatever folder it is started from.
pub fn assets_dir() -> String {
    if let Ok(dir) = std::env::var("NV_ASSETS") {
        return dir;
    }
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("assets")))
        .filter(|dir| dir.is_dir());
    match beside {
        Some(dir) => dir.to_string_lossy().into_owned(),
        None => concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into(),
    }
}

/// Start the visualizer behind the page.
pub fn use_stage(shared: StageHandle, settings: Signal<Settings>) {
    // the panels' settings, published whenever they change -- the render
    // thread never reads a signal
    let pub_params = shared.clone();
    use_effect(move || {
        let g = settings.read();
        *pub_params.params.lock().unwrap() = params_from(&g);
        pub_params.orbit.store(orbit_rate(&g).to_bits(), Ordering::Relaxed);
        let fps = std::env::var("NV_FPS").ok().and_then(|v| v.parse().ok()).unwrap_or(g.frame_rate);
        pub_params.target_fps.store(fps, Ordering::Relaxed);
    });

    // stop the render thread with the component
    let on_drop = shared.clone();
    use_drop(move || on_drop.stop.store(true, Ordering::Relaxed));

    use_future(move || {
        let shared = shared.clone();
        async move {
            let win = window().window.clone();
            let assets = assets_dir();

            // The surface has to be made on the main thread, where the
            // window lives. Everything after that happens on the render
            // thread.
            let instance = wgpu::Instance::default();
            let Ok(surface) = instance.create_surface(win.clone()) else {
                eprintln!("visualizer: could not make a GPU surface");
                return;
            };
            let size = win.inner_size();
            let thread_shared = shared.clone();
            if let Err(e) = std::thread::Builder::new().name("nv-render".into()).spawn(move || {
                render_loop(thread_shared, instance, surface, (size.width, size.height), assets)
            }) {
                eprintln!("visualizer: could not start the render thread: {e}");
                return;
            }

            // `NV_TEST_FULLSCREEN=1` toggles real fullscreen every few
            // seconds: the transition that used to fault the whole app.
            if std::env::var("NV_TEST_FULLSCREEN").is_ok() {
                spawn(async move {
                    let mut on = false;
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                        on = !on;
                        eprintln!("test: fullscreen {}", if on { "on" } else { "off" });
                        window().set_fullscreen(on);
                    }
                });
            }

            // Publish the layout. Both sizes come from the page, which
            // fills the window. tao's `inner_size()` is not trustworthy for
            // this: going fullscreen it kept reporting the windowed size
            // (2960x1880) while the window was far larger, so the surface
            // was configured too small and the page's correct stage
            // rectangle overflowed it -- the fault that took the app down.
            loop {
                let measured = document::eval(
                    "const r = document.querySelector('.nv-stage')?.getBoundingClientRect(); \
                     return r ? [r.x, r.y, r.width, r.height, window.devicePixelRatio, \
                                 window.innerWidth, window.innerHeight] : null;",
                )
                .await
                .ok()
                .and_then(|v| {
                    let a = v.as_array()?;
                    let g = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let dpr = g(4).max(1.0);
                    let surface = ((g(5) * dpr).round() as u32, (g(6) * dpr).round() as u32);
                    let stage = (
                        (g(0) * dpr) as f32,
                        (g(1) * dpr) as f32,
                        (g(2) * dpr) as f32,
                        (g(3) * dpr) as f32,
                    );
                    if std::env::var("NV_PROFILE").is_ok() {
                        eprintln!(
                            "layout: surface {}x{} | stage {:.0}x{:.0} @{dpr}",
                            surface.0, surface.1, stage.2, stage.3
                        );
                    }
                    Some(Layout { surface, stage })
                });
                if let Some(l) = measured {
                    if l.surface.0 > 0 && l.surface.1 > 0 {
                        *shared.layout.lock().unwrap() = Some(l);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    });
}
