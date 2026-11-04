//! Render the particle head offscreen to a PNG, with no window.
//!
//!     cargo run --release -p nv-render --example shot -- out.png [points]

use nv_render::{
    bust::BustModel,
    renderer::{bust_view_proj, camera_eye},
    BustRenderer, Camera, Uniforms,
};

const W: u32 = 840;
const H: u32 = 1040;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "bust.png".into());
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(180_000);
    let assets = std::env::args().nth(3).unwrap_or_else(|| "assets".into());
    let no_nmap = std::env::args().any(|a| a == "--flat");

    let model = BustModel::build(
        format!("{assets}/LeePerrySmith.glb"),
        if no_nmap {
            None
        } else {
            Some(std::path::Path::new(Box::leak(
                format!("{assets}/Infinite-Level_02_Tangent_SmoothUV.jpg").into_boxed_str(),
            )))
        },
        n,
        3.6,
        7,
    )
    .expect("build model");

    pollster::block_on(async move {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("no device");
        println!("adapter: {}", adapter.get_info().name);

        // linear, not sRGB: the shader writes the same values the GL version
        // wrote to a plain framebuffer, and an sRGB target would gamma-encode
        // them and lift every dark
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let r = BustRenderer::new(&device, format, &model, (W, H));

        let aspect = W as f32 / H as f32;
        let mut u = Uniforms {
            view_proj: bust_view_proj(aspect).to_cols_array_2d(),
            pivot: model.pivot.to_array(),
            vp_w: W as f32,
            vp_h: H as f32,
            // the far side is culled against this, so it must be the camera's
            eye: camera_eye(Camera::default()).extend(1.0).to_array(),
            ..Default::default()
        };
        // the same pose the Python comparison used
        let plain = std::env::args().any(|a| a == "--plain");
        u.yaw = 0.18;
        u.pitch = 0.06;
        // the app no longer drives this: Drum glitch is off by default
        u.glitch = 0.0;
        u.smear = 0.0;
        u.dust = 0.8;
        u.seed = 3.0;
        u.time = 2.0;
        u.low = 0.45;
        u.mid = 0.4;
        u.high = 0.5;
        u.swell = 0.05;
        u.point = std::env::args().nth(4).and_then(|v| v.parse().ok()).unwrap_or(2.0);
        // the pose the app holds the head in
        if std::env::args().any(|a| a == "--app-pose") {
            u.yaw = nv_render::motion::REST_YAW;
            u.pitch = nv_render::motion::REST_PITCH;
            u.dust = 0.1;
        }
        if plain {
            u.yaw = 0.0;
            u.pitch = 0.0;
            u.glitch = 0.0;
            u.smear = 0.0;
            u.dust = 0.0;
            u.low = 0.0;
            u.mid = 0.0;
            u.high = 0.0;
            u.swell = 0.0;
        }

        let t0 = std::time::Instant::now();
        r.render(&device, &queue, &view, &u);

        // read it back
        let bpr = (W * 4).next_multiple_of(256);
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bpr * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &buf,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bpr),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        );
        queue.submit([enc.finish()]);

        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let data = slice.get_mapped_range();

        let mut px = Vec::with_capacity((W * H * 4) as usize);
        for row in 0..H {
            let s = (row * bpr) as usize;
            px.extend_from_slice(&data[s..s + (W * 4) as usize]);
        }
        drop(data);
        buf.unmap();

        let lit = px
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 24)
            .count();
        let mean: f64 = px
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0)
            .sum::<f64>()
            / (W * H) as f64;
        println!(
            "{} particles rendered in {:?} — lit pixels {lit}, mean {mean:.3}",
            model.particles.len(),
            t0.elapsed()
        );
        image::save_buffer(&out, &px, W, H, image::ColorType::Rgba8).expect("write png");
        println!("wrote {out}");
    });
}
