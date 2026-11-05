//! The wgpu side: device, pipelines, and one frame of the particle head.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::bust::{BustModel, ShellVertex};
use crate::rings::{RingField, RingVertex, SEGMENTS};

/// Everything the shaders read per frame. Laid out to match `Uniforms` in
/// `bust.wgsl`; `bytemuck` and the 16-byte rules do the rest.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub pivot: [f32; 3],
    pub yaw: f32,
    pub shake: [f32; 3],
    pub pitch: f32,
    pub tint: [f32; 3],
    pub tint_mix: f32,
    pub glitch: f32,
    pub smear: f32,
    pub dust: f32,
    pub seed: f32,
    pub bands: f32,
    pub time: f32,
    pub bright: f32,
    pub point: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub swell: f32,
    pub explode: f32,
    pub fade: f32,
    pub dissipate: f32,
    pub diss_t: f32,
    pub vp_w: f32,
    pub vp_h: f32,
    /// Which axis the bands are laid along: 0 off, 1 vertical, 2 depth,
    /// 3 horizontal, 4 radial.
    pub band_axis: f32,
    /// How far each region lifts off the surface with its band.
    pub band_push: f32,
    /// Up to three ripples: `(radius, strength, width, unused)`.
    pub waves: [[f32; 4]; 3],
    /// The camera's position (`w` unused), so the far side can be culled.
    pub eye: [f32; 4],
    /// `(rings' centre x, rings' centre z, the song's intensity, the
    /// flash of a drop)`. Intensity is 1 at full strength.
    pub meter: [f32; 4],
    /// Synth Cloud's sheet of light: `(zone, width, strength, shimmer
    /// phase)`. Strength 0 when the oscillator is not driving the head.
    pub sheet: [f32; 4],
    /// The three newest rings, as they cross the head: `(radius, fade,
    /// width, how much they carry)`.
    pub rims: [[f32; 4]; 3],
    /// The Depth Point Cloud's head effect (see `head_fx`): `(which, amount,
    /// the beat, time)`, which 0 for none.
    pub fx: [f32; 4],
    /// `(lows, mids, highs, how alive the room is)`, smoothed.
    pub fx2: [f32; 4],
    /// The effects' paths: the comets' ratios and clocks (0..3) and phases
    /// (3..6), the head's centre and the comets' lift (6), its radii and
    /// the tail's reach (7), the halo (8), the vortex (9), the supernova's
    /// shells as (radius, strength) pairs (10, 11) and the seven bands for
    /// the corona (12, 13).
    pub paths: [[f32; 4]; 14],
}

/// How far the rings are scaled out from their own units into the world.
pub const RING_SCALE: f32 = 2.4;

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            pivot: [0.0; 3],
            yaw: 0.0,
            shake: [0.0; 3],
            pitch: 0.0,
            tint: [1.0; 3],
            tint_mix: 0.0,
            glitch: 0.0,
            smear: 0.0,
            dust: 0.0,
            seed: 0.0,
            bands: 9.0,
            time: 0.0,
            bright: 1.0,
            point: 2.0,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
            swell: 0.0,
            explode: 0.0,
            fade: 1.0,
            dissipate: 0.0,
            diss_t: 0.0,
            vp_w: 1.0,
            vp_h: 1.0,
            band_axis: 0.0,
            band_push: 0.0,
            waves: [[0.0; 4]; 3],
            eye: [0.0; 4],
            meter: [0.0, 0.1, 1.0, 0.0],
            sheet: [0.0; 4],
            rims: [[0.0; 4]; 3],
            fx: [0.0; 4],
            fx2: [0.0; 4],
            paths: [[0.0; 4]; 14],
        }
    }
}

/// The camera the bust is framed by: low and close, looking up, as
/// `BUST_CAMERA` sets it in the Python.
///
/// vispy applies its `fov` across the **width**, where `Mat4::perspective_rh`
/// takes a vertical angle. On this app's tall window that is a 1.4x
/// difference in how big the head lands, so the horizontal angle is
/// converted rather than used directly.
pub const BUST_FOV_X: f32 = 45.0;
pub const BUST_DISTANCE: f32 = 5.6;

/// Where the camera is looking from. Dragging the stage turns it; the
/// scroll wheel pulls it in and out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        // low and close, looking up: the subject stands over you
        Self { azimuth: 0.0, elevation: -9.0_f32.to_radians(), distance: BUST_DISTANCE }
    }
}

impl Camera {
    /// Turn by a drag, in pixels.
    pub fn drag(&mut self, dx: f32, dy: f32) {
        self.azimuth -= dx * 0.006;
        // stop short of the poles, where the up vector degenerates
        const LIMIT: f32 = 1.45;
        self.elevation = (self.elevation + dy * 0.006).clamp(-LIMIT, LIMIT);
    }

    /// Pull in or push out by a wheel notch.
    pub fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance * (1.0 + amount * 0.0015)).clamp(1.8, 24.0);
    }
}

pub fn bust_view_proj(aspect: f32) -> Mat4 {
    view_proj_from(aspect, Camera::default())
}

/// What the camera looks at.
const CAMERA_CENTRE: Vec3 = Vec3::new(0.0, 0.0, 0.45);

/// Where the camera sits, in world space.
pub fn camera_eye(cam: Camera) -> Vec3 {
    let (elevation, azimuth) = (cam.elevation, cam.azimuth);
    // Z-up, and the model faces -Y, which is the camera's side
    let dir = Vec3::new(
        elevation.cos() * azimuth.sin(),
        -elevation.cos() * azimuth.cos(),
        elevation.sin(),
    );
    CAMERA_CENTRE + dir * cam.distance
}

pub fn view_proj_from(aspect: f32, cam: Camera) -> Mat4 {
    view_proj_lens(aspect, cam, BUST_FOV_X)
}

/// The lens the Perspective control asks for, 0..1, and the camera moved
/// so the head stays the same size through it: 0.35 is the usual 45
/// degrees, lower flattens it towards a telephoto, higher widens it and
/// brings the camera close enough to exaggerate the nose and the ears.
pub fn lens(cam: Camera, perspective: f32) -> (Camera, f32) {
    let p = perspective.clamp(0.0, 1.0);
    let fov = if p < 0.35 {
        12.0 + (BUST_FOV_X - 12.0) * p / 0.35
    } else {
        BUST_FOV_X + (85.0 - BUST_FOV_X) * (p - 0.35) / 0.65
    };
    let keep = (BUST_FOV_X.to_radians() * 0.5).tan() / (fov.to_radians() * 0.5).tan();
    (Camera { distance: cam.distance * keep, ..cam }, fov)
}

/// `view_proj_from` through a lens `fov_x` degrees wide.
pub fn view_proj_lens(aspect: f32, cam: Camera, fov_x: f32) -> Mat4 {
    let view = Mat4::look_at_rh(camera_eye(cam), CAMERA_CENTRE, Vec3::Z);
    // On a portrait frame (a Reel) the horizontal angle would show the
    // whole width and leave the head small in a tall empty frame. The
    // Python rendered landscape and centre-cropped to 9:16 instead, so a
    // narrow frame keeps the square frame's vertical angle and lets the
    // sides fall away -- the same "cover" crop, rendered directly.
    let half_x = (fov_x.to_radians() * 0.5).tan();
    let fov_y = 2.0 * (half_x / aspect.max(1.0)).atan();
    let proj = Mat4::perspective_rh(fov_y, aspect, 0.05, 120.0);
    proj * view
}

/// One particle after the compute pass: where it lands and how it looks.
/// Only the GPU reads it; the size is what `Sprite` in `bust.wgsl` takes.
const SPRITE_BYTES: u64 = 32;

/// Threads per workgroup in `cs_particles`.
const WORKGROUP: u32 = 256;

pub struct BustRenderer {
    /// Moves, lights and sizes every particle, once per particle.
    sprite_pipeline: wgpu::ComputePipeline,
    sprite_out: wgpu::BindGroup,
    particle_pipeline: wgpu::RenderPipeline,
    sprite_in: wgpu::BindGroup,
    shell_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    shell_vtx: wgpu::Buffer,
    shell_idx: wgpu::Buffer,
    depth: wgpu::TextureView,
    n_particles: u32,
    n_indices: u32,
    ring_pipeline: wgpu::RenderPipeline,
    ring_buf: wgpu::Buffer,
    ring_verts: u32,
    /// The rings' vertices, kept between frames for the allocation.
    ring_scratch: Vec<RingVertex>,
    /// Where on the surface to draw, in physical pixels.
    ///
    /// The surface covers the whole window, but the picture belongs in the
    /// stage -- the hole the page leaves beside the controls. Without this
    /// the head centres on the window and so sits left of where it looks
    /// like it should.
    viewport: (f32, f32, f32, f32),
    /// The render target's size, which the viewport is clamped to.
    target: (u32, u32),
    pub ghosts: usize,
}

/// Room for the rings' vertices: six rings of a closed loop each.
const RING_CAPACITY: usize = crate::rings::MAX_RINGS * (SEGMENTS + 1);

impl BustRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        model: &BustModel,
        size: (u32, u32),
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bust"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bust.wgsl").into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bust uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let particle_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: bytemuck::cast_slice(&model.particles),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let shell_vtx = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shell vertices"),
            contents: bytemuck::cast_slice(&model.shell),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let shell_idx = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shell indices"),
            contents: bytemuck::cast_slice(&model.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bust"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX
                        | wgpu::ShaderStages::FRAGMENT
                        | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bust"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: particle_buf.as_entire_binding() },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bust"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        // The per-particle work -- the deform, the dissolve, the light --
        // used to run in the vertex shader, which expands each particle
        // into six vertices and so did all of it six times over. It now
        // runs once per particle in a compute pass that writes a sprite,
        // and the vertex shader only expands that sprite into its quad.
        let sprite_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprites"),
            size: (model.particles.len() as u64).max(1) * SPRITE_BYTES,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let sprite_group = |binding: u32, stage: wgpu::ShaderStages, read_only: bool| {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprites"),
                entries: &[storage_entry(binding, stage, read_only)],
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sprites"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding,
                    resource: sprite_buf.as_entire_binding(),
                }],
            });
            (layout, group)
        };
        let (out_layout, sprite_out) = sprite_group(0, wgpu::ShaderStages::COMPUTE, false);
        let (in_layout, sprite_in) = sprite_group(1, wgpu::ShaderStages::VERTEX, true);

        let sprite_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bust sprites"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bust sprites"),
                bind_group_layouts: &[&layout, &out_layout],
                push_constant_ranges: &[],
            })),
            module: &shader,
            entry_point: "cs_particles",
            compilation_options: Default::default(),
            cache: None,
        });
        let draw_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bust particles"),
            bind_group_layouts: &[&layout, &in_layout],
            push_constant_ranges: &[],
        });

        let depth_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        });
        // The shell is the very surface the particles were sampled from, so
        // the two are coincident in depth. Drawn first, it would win the
        // `Less` test everywhere and throw the face away -- which is what
        // the Python means by the shell sitting "a hair behind" the
        // particles. Push it back rather than relying on depth precision.
        let shell_depth = Some(wgpu::DepthStencilState {
            bias: wgpu::DepthBiasState { constant: 4, slope_scale: 1.0, clamp: 0.0 },
            ..depth_state.clone().unwrap()
        });

        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bust particles"),
            layout: Some(&draw_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_particles",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_particles",
                targets: &[Some(format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth_state.clone(),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let shell_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bust shell"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_shell",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ShellVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_shell",
                targets: &[Some(format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None, // the scan is not watertight
                ..Default::default()
            },
            depth_stencil: shell_depth,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let ring_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rings"),
            size: (RING_CAPACITY * std::mem::size_of::<RingVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ring_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rings"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_rings",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RingVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_rings",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // additive: they are light in the air, not matter
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        // the rings are air, not the head: they leave the
                        // alpha the head marks itself in as it was
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                ..Default::default()
            },
            // they sit behind the head and never hide it
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
        });

        Self {
            sprite_pipeline,
            sprite_out,
            particle_pipeline,
            sprite_in,
            shell_pipeline,
            bind_group,
            uniform_buf,
            shell_vtx,
            shell_idx,
            depth: make_depth(device, size),
            n_particles: model.particles.len() as u32,
            n_indices: model.indices.len() as u32,
            ring_pipeline,
            ring_buf,
            ring_verts: 0,
            ring_scratch: Vec::with_capacity(RING_CAPACITY),
            viewport: (0.0, 0.0, size.0 as f32, size.1 as f32),
            target: size,
            ghosts: 0,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.depth = make_depth(device, size);
        self.target = size;
        self.viewport = (0.0, 0.0, size.0 as f32, size.1 as f32);
    }

    /// The viewport, clamped to the render target.
    ///
    /// The stage's rectangle comes from the page and the target's size
    /// from the window, and the two change at different moments: going
    /// fullscreen, the page reported a 3344-pixel stage while the surface
    /// was still 2960 wide, and an out-of-bounds viewport is a validation
    /// error that took the whole app down. Never trust them to agree.
    /// The stage's rectangle in the target, in pixels, clamped to it.
    pub fn stage_rect(&self) -> (f32, f32, f32, f32) {
        self.clamped_viewport()
    }

    pub fn target_size(&self) -> (u32, u32) {
        self.target
    }

    fn clamped_viewport(&self) -> (f32, f32, f32, f32) {
        let (tw, th) = (self.target.0 as f32, self.target.1 as f32);
        let (x, y, w, h) = self.viewport;
        let x = x.clamp(0.0, (tw - 1.0).max(0.0));
        let y = y.clamp(0.0, (th - 1.0).max(0.0));
        let w = w.min(tw - x);
        let h = h.min(th - y);
        if w >= 1.0 && h >= 1.0 {
            (x, y, w, h)
        } else {
            (0.0, 0.0, tw.max(1.0), th.max(1.0))
        }
    }

    /// Draw into this rectangle of the surface instead of all of it.
    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if w > 1.0 && h > 1.0 {
            self.viewport = (x, y, w, h);
        }
    }

    /// The aspect the camera should use: the stage's, not the window's.
    pub fn aspect(&self) -> f32 {
        let (_, _, w, h) = self.clamped_viewport();
        w / h.max(1.0)
    }

    pub fn viewport_size(&self) -> (f32, f32) {
        let (_, _, w, h) = self.clamped_viewport();
        (w, h)
    }

    /// Upload the rings for this frame.
    pub fn set_rings(&mut self, queue: &wgpu::Queue, field: &RingField, centre: [f32; 3]) {
        // around the head, barely behind it: the Python pushed them far
        // back as a texture in the air, but read as a halo they want to
        // sit in the same space the head does
        field.geometry_into(&mut self.ring_scratch, centre, 0.35, RING_SCALE);
        let verts = &self.ring_scratch[..self.ring_scratch.len().min(RING_CAPACITY)];
        if !verts.is_empty() {
            queue.write_buffer(&self.ring_buf, 0, bytemuck::cast_slice(verts));
        }
        self.ring_verts = verts.len() as u32;
    }

    /// Render the head.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        uniforms: &Uniforms,
    ) {
        self.encode::<fn(&mut wgpu::RenderPass<'_>)>(device, queue, view, uniforms, None);
    }

    /// Let `other` draw into the head's pass **instead of** the head: the
    /// modes replace each other rather than sharing the stage. The head's
    /// compute pass is skipped too, so a mode that hides it costs nothing
    /// for it.
    pub fn render_with(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        uniforms: &Uniforms,
        other: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) {
        self.encode(device, queue, view, uniforms, Some(other));
    }

    fn encode<F: FnOnce(&mut wgpu::RenderPass<'_>)>(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        uniforms: &Uniforms,
        other: Option<F>,
    ) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniforms));
        let mut enc =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("bust") });
        if other.is_none() && self.n_particles > 0 {
            let mut cpass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bust sprites"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.sprite_pipeline);
            cpass.set_bind_group(0, &self.bind_group, &[]);
            cpass.set_bind_group(1, &self.sprite_out, &[]);
            cpass.dispatch_workgroups(self.n_particles.div_ceil(WORKGROUP), 1, 1);
        }
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bust"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (vx, vy, vw, vh) = self.clamped_viewport();
            pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);

            // another mode may own this frame instead
            if let Some(other) = other {
                other(&mut pass);
            } else {
                pass.set_bind_group(0, &self.bind_group, &[]);

                // The rings first, before the shell. Drawn after it they
                // painted over the face, which is what made them look like
                // they were running up and down it.
                if self.ring_verts > 0 {
                    pass.set_pipeline(&self.ring_pipeline);
                    pass.set_vertex_buffer(0, self.ring_buf.slice(..));
                    // one closed loop per ring
                    let per = (SEGMENTS + 1) as u32;
                    for r in 0..(self.ring_verts / per) {
                        pass.draw(r * per..(r + 1) * per, 0..1);
                    }
                }

                // the shell next: it writes depth so the far side of the head
                // and the mouth's interior cannot shine through the gaps
                pass.set_pipeline(&self.shell_pipeline);
                pass.set_vertex_buffer(0, self.shell_vtx.slice(..));
                pass.set_index_buffer(self.shell_idx.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.n_indices, 0, 0..1);

                // then the particles, six vertices each
                pass.set_pipeline(&self.particle_pipeline);
                pass.set_bind_group(1, &self.sprite_in, &[]);
                pass.draw(0..self.n_particles * 6, 0..1);
            }
        }
        queue.submit([enc.finish()]);
    }
}

fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn make_depth(device: &wgpu::Device, (w, h): (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
