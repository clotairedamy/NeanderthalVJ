//! Drawing the Generative Geometry.
//!
//! Two floating-point images take turns: each frame reads the last one
//! through the feedback transform, draws its lines on top, and becomes the
//! one the next frame reads. The result is composited onto the stage inside
//! the head's own render pass, which has a depth attachment -- so the
//! composite declares one too, or the pass would refuse it (the bug that
//! once hid the Hiding Grid).

use crate::generative::{Feedback, GenVertex};

const IMAGE: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Room for the most intricate frame: seven rings and their echoes, the
/// web, the seed and every beat polygon in flight.
const MAX_VERTS: usize = 1 << 16;

struct Image {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Binds this image as the one being read.
    reads: wgpu::BindGroup,
}

pub struct GenerativeRenderer {
    feedback_pipeline: wgpu::RenderPipeline,
    shape_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    vertices: wgpu::Buffer,
    n_vertices: u32,
    images: Vec<Image>,
    size: (u32, u32),
    /// The image holding the newest frame.
    current: usize,
}

impl GenerativeRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("generative"),
            source: wgpu::ShaderSource::Wgsl(include_str!("generative.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("generative"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("generative"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let full = |label, fs, target: wgpu::TextureFormat, depth: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_full",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: fs,
                    targets: &[Some(target.into())],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: depth.then(|| wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let feedback_pipeline = full("generative feedback", "fs_feedback", IMAGE, false);
        let composite_pipeline = full("generative composite", "fs_composite", format, true);

        let shape_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GenVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32],
        };
        let additive = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("generative shapes"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_shape",
                buffers: std::slice::from_ref(&shape_layout),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_shape",
                targets: &[Some(wgpu::ColorTargetState {
                    format: IMAGE,
                    blend: Some(wgpu::BlendState { color: additive, alpha: additive }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("generative"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("generative uniforms"),
            size: std::mem::size_of::<Feedback>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("generative lines"),
            size: (MAX_VERTS * std::mem::size_of::<GenVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            feedback_pipeline,
            shape_pipeline,
            composite_pipeline,
            layout,
            sampler,
            uniforms,
            vertices,
            n_vertices: 0,
            images: Vec::new(),
            size: (0, 0),
            current: 0,
        }
    }

    /// (Re)make the two images at `size`; they start black, so a resize
    /// clears the trails rather than stretching them.
    fn ensure_size(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if size == self.size && !self.images.is_empty() {
            return;
        }
        self.size = size;
        self.images = (0..2)
            .map(|i| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(if i == 0 { "generative image a" } else { "generative image b" }),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: IMAGE,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let reads = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("generative reads"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.uniforms.as_entire_binding(),
                        },
                    ],
                });
                Image { _texture: texture, view, reads }
            })
            .collect();
        self.current = 0;
    }

    /// Draw this frame into the next image: the last frame carried
    /// forward, then `lines` on top. Submitted at once, so it is done
    /// before the stage's pass composites it.
    pub fn advance(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        lines: &[GenVertex],
        feedback: &Feedback,
    ) {
        self.ensure_size(device, size);
        let lines = &lines[..lines.len().min(MAX_VERTS)];
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(feedback));
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(lines));
        self.n_vertices = lines.len() as u32;

        let (read, write) = (self.current, 1 - self.current);
        let mut enc = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("generative") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("generative frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.images[write].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.images[read].reads, &[]);
            pass.set_pipeline(&self.feedback_pipeline);
            pass.draw(0..3, 0..1);
            if self.n_vertices > 0 {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.draw(0..self.n_vertices, 0..1);
            }
        }
        queue.submit([enc.finish()]);
        self.current = write;
    }

    /// Composite the newest frame into the stage's pass.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(image) = self.images.get(self.current) else { return };
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &image.reads, &[]);
        pass.draw(0..3, 0..1);
    }
}
