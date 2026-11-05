//! Post-processing: the FX tab, over whatever the stage drew.
//!
//! When any effect is on, the stage is drawn into an off-screen copy of the
//! target instead of the target itself; this then blooms it, and composites
//! it back with the beat's flash, a colour split on kicks, grain on the
//! drums, a vignette and an echo of the frames before. With every effect
//! off none of it runs, and the stage draws straight to the screen.

use bytemuck::{Pod, Zeroable};
use nv_audio::AnalysisFrame;

/// The bloom and blur images: floating point, so the glow can exceed 1.
const GLOW: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// What the FX tab decides. Every amount is 0 for off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostKnobs {
    /// Strength of the glow on the brightest parts.
    pub bloom: f32,
    /// How bright a part must be to glow, 0..1.
    pub threshold: f32,
    /// How much a beat brightens the whole frame.
    pub flash: f32,
    /// Film grain, thrown harder by each drum hit.
    pub grain: f32,
    /// Red and blue pulled apart on each kick.
    pub chroma: f32,
    /// Darkening towards the corners.
    pub vignette: f32,
    /// An echo of the frames before, thrown out by drum hits.
    pub ghost: f32,
}

impl Default for PostKnobs {
    fn default() -> Self {
        Self {
            bloom: 0.0,
            threshold: 0.6,
            flash: 0.0,
            grain: 0.0,
            chroma: 0.0,
            vignette: 0.0,
            ghost: 0.0,
        }
    }
}

impl PostKnobs {
    /// Nothing to do: draw straight to the screen.
    pub fn is_off(&self) -> bool {
        self.bloom <= 0.0
            && self.flash <= 0.0
            && self.grain <= 0.0
            && self.chroma <= 0.0
            && self.vignette <= 0.0
            && self.ghost <= 0.0
    }
}

/// How the music drives the effects: envelopes that jump on a hit and
/// fall away after it.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostEnvelopes {
    pub flash: f32,
    pub kick: f32,
    pub drums: f32,
    pub ghost: f32,
    pub time: f32,
    since_ghost: f32,
}

impl PostEnvelopes {
    pub fn update(&mut self, frame: &AnalysisFrame, beat: bool, dt: f32) {
        let dt = dt.min(0.1);
        self.time += dt;
        self.since_ghost += dt;
        let hit = if beat { (frame.beat_strength as f32).max(0.5) } else { 0.0 };
        self.flash = (self.flash * (-dt * 7.0).exp()).max(hit);
        let kick = frame.attack[0].max(frame.attack[1]) as f32;
        self.kick = (self.kick * (-dt * 9.0).exp()).max(kick);
        let drums = frame.attack[..3].iter().fold(0.0f64, |a, b| a.max(*b)) as f32;
        self.drums = (self.drums * (-dt * 8.0).exp()).max(drums);
        // a ghost per hit, not per frame of the hit
        if (drums > 0.35 || beat) && self.since_ghost > 0.12 {
            self.ghost = 1.0;
            self.since_ghost = 0.0;
        }
        self.ghost *= (-dt * 3.0).exp();
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct PostUniforms {
    rect: [f32; 4],
    bloom: f32,
    threshold: f32,
    flash: f32,
    grain: f32,
    chroma: f32,
    vignette: f32,
    ghost: f32,
    time: f32,
    aspect: f32,
    /// 1 when the stage marked the head in its alpha (0 there, 1 elsewhere),
    /// for the echo to leave alone.
    protect: f32,
    _pad: [f32; 2],
}

impl PostUniforms {
    fn new(k: &PostKnobs, e: &PostEnvelopes, rect: [f32; 4], aspect: f32, protect: bool) -> Self {
        Self {
            rect,
            bloom: k.bloom * (1.0 + 0.5 * e.flash),
            threshold: k.threshold,
            flash: k.flash * 0.35 * e.flash,
            grain: k.grain * (0.03 + 0.2 * e.drums),
            chroma: k.chroma * (0.15 + e.kick),
            vignette: k.vignette,
            // below 1, so the echo of an echo always fades
            ghost: (k.ghost * 0.75 * e.ghost).min(0.92),
            time: e.time,
            aspect,
            protect: if protect { 1.0 } else { 0.0 },
            _pad: [0.0; 2],
        }
    }
}

struct Image {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

fn image(
    device: &wgpu::Device,
    label: &str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
) -> Image {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    Image { _texture: texture, view }
}

/// The images sized to one target, and the bindings that read them.
struct Frames {
    size: (u32, u32),
    scene: Image,
    glow: [Image; 2],
    history: [Image; 2],
    bright: wgpu::BindGroup,
    blur_h: wgpu::BindGroup,
    blur_v: wgpu::BindGroup,
    /// Reading history 0 or 1.
    composite: [wgpu::BindGroup; 2],
}

pub struct PostFx {
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    bright: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    frames: Option<Frames>,
    /// The history image the next frame reads.
    history: usize,
}

impl PostFx {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });
        let tex = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                tex(0),
                tex(1),
                tex(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
            label: Some("post"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |label, fs, targets: &[Option<wgpu::ColorTargetState>]| {
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
                    targets,
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let glow = [Some(GLOW.into())];
        let bright = pipeline("post bright", "fs_bright", &glow);
        let blur_h = pipeline("post blur h", "fs_blur_h", &glow);
        let blur_v = pipeline("post blur v", "fs_blur_v", &glow);
        let composite =
            pipeline("post composite", "fs_composite", &[Some(format.into()), Some(format.into())]);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post uniforms"),
            size: std::mem::size_of::<PostUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            format,
            layout,
            sampler,
            uniforms,
            bright,
            blur_h,
            blur_v,
            composite,
            frames: None,
            history: 0,
        }
    }

    fn bind(&self, device: &wgpu::Device, views: [&wgpu::TextureView; 3]) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry { binding: 4, resource: self.uniforms.as_entire_binding() },
            ],
        })
    }

    /// Make the images for a target of `size`, if they are not already.
    pub fn prepare(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.frames.as_ref().is_none_or(|f| f.size != size) {
            let half = ((size.0 / 2).max(1), (size.1 / 2).max(1));
            let scene = image(device, "post scene", size, self.format);
            let glow = [
                image(device, "post glow a", half, GLOW),
                image(device, "post glow b", half, GLOW),
            ];
            let history = [
                image(device, "post history a", size, self.format),
                image(device, "post history b", size, self.format),
            ];
            let bright = self.bind(device, [&scene.view, &scene.view, &scene.view]);
            let blur_h = self.bind(device, [&glow[0].view, &glow[0].view, &glow[0].view]);
            let blur_v = self.bind(device, [&glow[1].view, &glow[1].view, &glow[1].view]);
            let composite = [
                self.bind(device, [&scene.view, &glow[0].view, &history[0].view]),
                self.bind(device, [&scene.view, &glow[0].view, &history[1].view]),
            ];
            self.frames =
                Some(Frames { size, scene, glow, history, bright, blur_h, blur_v, composite });
            self.history = 0;
        }
    }

    /// The image the stage draws into while effects are on; made by
    /// [`PostFx::prepare`].
    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.frames.as_ref().map(|f| &f.scene.view)
    }

    /// Composite the stage, drawn into [`PostFx::target`], onto `view`.
    /// `stage` is the stage's rectangle in the target, in pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        stage: (f32, f32, f32, f32),
        knobs: &PostKnobs,
        env: &PostEnvelopes,
        protect: bool,
    ) {
        let Some(f) = &self.frames else { return };
        let (tw, th) = (f.size.0 as f32, f.size.1 as f32);
        let rect = [stage.0 / tw, stage.1 / th, stage.2 / tw, stage.3 / th];
        let u = PostUniforms::new(knobs, env, rect, stage.2 / stage.3.max(1.0), protect);
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));

        let mut enc =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("post") });
        let pass = |enc: &mut wgpu::CommandEncoder,
                    target: &wgpu::TextureView,
                    pipeline: &wgpu::RenderPipeline,
                    bind: &wgpu::BindGroup| {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
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
            p.set_pipeline(pipeline);
            p.set_bind_group(0, bind, &[]);
            p.draw(0..3, 0..1);
        };
        if knobs.bloom > 0.0 {
            pass(&mut enc, &f.glow[0].view, &self.bright, &f.bright);
            pass(&mut enc, &f.glow[1].view, &self.blur_h, &f.blur_h);
            pass(&mut enc, &f.glow[0].view, &self.blur_v, &f.blur_v);
        }
        {
            let (read, write) = (self.history, 1 - self.history);
            let clear = wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            };
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post composite"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: clear,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &f.history[write].view,
                        resolve_target: None,
                        ops: clear,
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // the stage only: the effects centre on it, not on the window
            p.set_viewport(stage.0, stage.1, stage.2.max(1.0), stage.3.max(1.0), 0.0, 1.0);
            p.set_pipeline(&self.composite);
            p.set_bind_group(0, &f.composite[read], &[]);
            p.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
        self.history = 1 - self.history;
    }
}
