//! Drawing the Hiding Grid.

use wgpu::util::DeviceExt;

use crate::grid::{GridUniforms, GridVertex, HidingGrid, MAX_CELLS};

/// Enough room for the largest wall the column control allows: four
/// vertices a cell, and at most four lattice endpoints a node.
const MAX_QUAD_VERTS: usize = MAX_CELLS * 4;
const MAX_LINE_VERTS: usize = 4 * (3 * MAX_CELLS + 1);

pub struct GridRenderer {
    quad_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    quad_vtx: wgpu::Buffer,
    quad_idx: wgpu::Buffer,
    line_vtx: wgpu::Buffer,
    quad_indices: u32,
    line_verts: u32,
}

impl GridRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid uniforms"),
            size: std::mem::size_of::<GridUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("grid"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let vbl = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GridVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3],
        };
        let make = |topology, label| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_grid",
                    buffers: std::slice::from_ref(&vbl),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_grid",
                    targets: &[Some(format.into())],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { topology, ..Default::default() },
                // The drawing is flat -- no square is ever over another --
                // so the depth test decides nothing. But the grid draws
                // inside the same pass as the head, which has a depth
                // attachment, and WebGPU requires every pipeline in a pass
                // to declare that attachment's format. Declaring none was
                // a validation error, which is why the grid never showed.
                depth_stencil: Some(wgpu::DepthStencilState {
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
        let quad_pipeline = make(wgpu::PrimitiveTopology::TriangleList, "grid squares");
        let line_pipeline = make(wgpu::PrimitiveTopology::LineList, "grid lattice");

        // two triangles per cell, over four vertices of its own; the
        // indices never change, so they are built once
        let mut idx: Vec<u32> = Vec::with_capacity(MAX_CELLS * 6);
        for c in 0..MAX_CELLS as u32 {
            let b = c * 4;
            idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
        let quad_idx = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid indices"),
            contents: bytemuck::cast_slice(&idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        let vbuf = |label, n: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (n * std::mem::size_of::<GridVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        Self {
            quad_pipeline,
            line_pipeline,
            bind_group,
            uniform_buf,
            quad_vtx: vbuf("grid squares", MAX_QUAD_VERTS),
            quad_idx,
            line_vtx: vbuf("grid lattice", MAX_LINE_VERTS),
            quad_indices: 0,
            line_verts: 0,
        }
    }

    /// Hand this frame's field to the GPU.
    pub fn upload(&mut self, queue: &wgpu::Queue, grid: &HidingGrid, u: &GridUniforms) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(u));
        let q = &grid.quads[..grid.quads.len().min(MAX_QUAD_VERTS)];
        if !q.is_empty() {
            queue.write_buffer(&self.quad_vtx, 0, bytemuck::cast_slice(q));
        }
        self.quad_indices = (q.len() / 4 * 6) as u32;
        let l = &grid.lines[..grid.lines.len().min(MAX_LINE_VERTS)];
        if !l.is_empty() {
            queue.write_buffer(&self.line_vtx, 0, bytemuck::cast_slice(l));
        }
        self.line_verts = l.len() as u32;
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.bind_group, &[]);
        if self.quad_indices > 0 {
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_vertex_buffer(0, self.quad_vtx.slice(..));
            pass.set_index_buffer(self.quad_idx.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.quad_indices, 0, 0..1);
        }
        if self.line_verts > 0 {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vtx.slice(..));
            pass.draw(0..self.line_verts, 0..1);
        }
    }
}
