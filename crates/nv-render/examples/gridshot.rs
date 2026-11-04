//! Render the Hiding Grid offscreen through the same path the app uses.
use nv_audio::AnalysisFrame;
use nv_render::{
    bust::BustModel, BustRenderer, GridKnobs, GridRenderer, GridUniforms, HidingGrid, Uniforms,
};

const W: u32 = 900;
const H: u32 = 900;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "grid.png".into());
    let model = BustModel::build("assets/LeePerrySmith.glb", None, 20_000, 3.6, 7).unwrap();
    pollster::block_on(async move {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default(), None).await.unwrap();
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let bust = BustRenderer::new(&device, format, &model, (W, H));
        let mut gr = GridRenderer::new(&device, format);
        let mut grid = HidingGrid::default();
        let f = AnalysisFrame {
            rms: 0.4,
            bands: [0.6, 0.5, 0.4, 0.3, 0.3, 0.2, 0.2],
            spectrum: std::array::from_fn(|i| (i as f64 / 64.0).min(1.0)),
            ..Default::default()
        };
        for _ in 0..10 {
            grid.update(&f, 1.0 / 60.0, false, GridKnobs::default());
        }
        gr.upload(&queue, &grid, &GridUniforms { fit: [0.92, 0.92], ..Default::default() });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        bust.render_with(&device, &queue, &view, &Uniforms::default(), |pass| gr.draw(pass));
        if let Some(e) = device.pop_error_scope().await {
            println!("VALIDATION ERROR:\n{e}");
            return;
        }
        let bpr = (W * 4).next_multiple_of(256);
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
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
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range();
        let mut px = Vec::new();
        for r in 0..H {
            let s = (r * bpr) as usize;
            px.extend_from_slice(&data[s..s + (W * 4) as usize]);
        }
        let lit = px.chunks(4).filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 60).count();
        println!("grid quads {} lines {} -> lit pixels {lit}", grid.quads.len(), grid.lines.len());
        image::save_buffer(&out, &px, W, H, image::ColorType::Rgba8).unwrap();
    });
}
