//! The depth point cloud, in a window, driven by a track.
//!
//!     cargo run --release -p nv-render --example visualizer -- track.wav
//!
//! Space plays and pauses; Escape quits. With no file it still runs, and
//! the head sits still until there is something to hear.

use std::sync::Arc;

use nv_engine::Engine;
use nv_render::{bust::BustModel, renderer::bust_view_proj, BustMotion, BustRenderer, Uniforms};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    model: BustModel,
    engine: Engine,
    motion: BustMotion,
    uniforms: Uniforms,
    last: std::time::Instant,
    frames: u32,
    fps_at: std::time::Instant,
}

struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: BustRenderer,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("NeanderthalVJ — Depth Point Cloud")
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 1100.0));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("no GPU adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("no device");

        let caps = surface.get_capabilities(&adapter);
        // linear, not sRGB: the shader writes the values the GL original
        // wrote to a plain framebuffer
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let renderer =
            BustRenderer::new(&device, format, &self.model, (config.width, config.height));
        println!("{} — {} particles", adapter.get_info().name, self.model.particles.len());
        self.gpu = Some(Gpu { surface, device, queue, config, renderer });
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::Space) => self.engine.toggle_play(),
                    _ => {}
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gpu {
                    g.config.width = size.width.max(1);
                    g.config.height = size.height.max(1);
                    g.surface.configure(&g.device, &g.config);
                    g.renderer.resize(&g.device, (g.config.width, g.config.height));
                }
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

impl App {
    fn draw(&mut self) {
        let Some(g) = &mut self.gpu else { return };
        let dt = self.last.elapsed().as_secs_f32();
        self.last = std::time::Instant::now();

        // what the music is doing right now
        let (frame, beat) = match self.engine.tick() {
            Some(s) => (s.frame, s.beat),
            None => (nv_audio::AnalysisFrame::default(), false),
        };
        self.motion.update(&frame, dt, beat, nv_render::Knobs::default(), &mut self.uniforms);

        let aspect = g.config.width as f32 / g.config.height as f32;
        self.uniforms.view_proj = bust_view_proj(aspect).to_cols_array_2d();
        self.uniforms.pivot = self.model.pivot.to_array();
        self.uniforms.vp_w = g.config.width as f32;
        self.uniforms.vp_h = g.config.height as f32;

        let Ok(surface) = g.surface.get_current_texture() else { return };
        let view = surface.texture.create_view(&Default::default());
        g.renderer.render(&g.device, &g.queue, &view, &self.uniforms);
        surface.present();

        self.frames += 1;
        if self.fps_at.elapsed().as_secs_f64() >= 2.0 {
            let fps = self.frames as f64 / self.fps_at.elapsed().as_secs_f64();
            println!("{fps:.0} fps");
            self.frames = 0;
            self.fps_at = std::time::Instant::now();
        }
    }
}

fn main() {
    let assets = std::env::var("NV_ASSETS").unwrap_or_else(|_| "assets".into());
    let n: usize = std::env::var("NV_POINTS").ok().and_then(|v| v.parse().ok()).unwrap_or(420_000);

    let model = BustModel::build(
        format!("{assets}/LeePerrySmith.glb"),
        Some(std::path::Path::new(Box::leak(
            format!("{assets}/Infinite-Level_02_Tangent_SmoothUV.jpg").into_boxed_str(),
        ))),
        n,
        3.6,
        7,
    )
    .expect("build the bust");

    let mut engine = Engine::new();
    if let Some(path) = std::env::args().nth(1) {
        match engine.load(&path) {
            Ok(()) => {
                println!(
                    "loaded {path}: {:.1}s, {:.1} BPM, {} beats",
                    engine.duration(),
                    engine.grid.bpm,
                    engine.grid.beats.len()
                );
                engine.set_playing(true);
            }
            Err(e) => eprintln!("could not load {path}: {e}"),
        }
    } else {
        println!("no track given — pass one as an argument to drive the head");
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None,
        gpu: None,
        model,
        engine,
        motion: BustMotion::default(),
        uniforms: Uniforms::default(),
        last: std::time::Instant::now(),
        frames: 0,
        fps_at: std::time::Instant::now(),
    };
    event_loop.run_app(&mut app).unwrap();
}
