//! Everything the stage draws, independent of any window.
//!
//! The live view and the video exporter both render through a `Scene`, so
//! an exported clip is the same picture the window shows -- the same
//! motion, rings, grid, generative geometry and tint -- rather than a
//! second implementation that drifts from the first.

use nv_audio::AnalysisFrame;

use crate::bust::BustModel;
use crate::generative::{GenKnobs, GenVertex, Generative};
use crate::generative_render::GenerativeRenderer;
use crate::grid::{GridKnobs, GridUniforms, HidingGrid};
use crate::grid_render::GridRenderer;
use crate::head_fx::{HeadFx, HeadShape};
use crate::motion::{BustMotion, Knobs};
use crate::post::{PostEnvelopes, PostFx, PostKnobs};
use crate::react::{ReactKnobs, Reactor};
use crate::renderer::{
    camera_eye, lens, view_proj_lens, BustRenderer, Camera, Uniforms, RING_SCALE,
};
use crate::rings::RingField;

/// Where the rings are centred, in world space.
const RING_CENTRE: [f32; 3] = [0.0, 0.0, 0.1];

/// What the panels decide about a frame. Plain data, so it can cross
/// threads.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub knobs: Knobs,
    /// The FX tab.
    pub post: PostKnobs,
    /// The lens, 0..1; 0.35 is the usual 45 degrees.
    pub perspective: f32,
    /// The bands laid along the head: 0 off, 1 vertical, 2 depth,
    /// 3 horizontal, 4 radial; and how far each region lifts.
    pub band_axis: u8,
    pub band_push: f32,
    /// `Some` when the Hiding Grid is the mode on stage.
    pub grid: Option<GridKnobs>,
    /// `Some` when the Generative Geometry is.
    pub generative: Option<GenKnobs>,
    /// How the mode on stage listens: the Motion tab's per-mode shaping.
    pub react: ReactKnobs,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            knobs: Knobs::default(),
            post: PostKnobs::default(),
            perspective: 0.35,
            band_axis: 0,
            band_push: 0.0,
            grid: None,
            generative: None,
            react: ReactKnobs::default(),
        }
    }
}

pub struct Scene {
    renderer: BustRenderer,
    grid_renderer: GridRenderer,
    grid: HidingGrid,
    generative: Generative,
    generative_renderer: GenerativeRenderer,
    lines: Vec<GenVertex>,
    post: PostFx,
    envelopes: PostEnvelopes,
    motion: BustMotion,
    reactor: Reactor,
    head_fx: HeadFx,
    rings: RingField,
    uniforms: Uniforms,
    pivot: [f32; 3],
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        model: &BustModel,
        size: (u32, u32),
    ) -> Self {
        Self {
            renderer: BustRenderer::new(device, format, model, size),
            grid_renderer: GridRenderer::new(device, format),
            grid: HidingGrid::default(),
            generative: Generative::default(),
            generative_renderer: GenerativeRenderer::new(device, format),
            lines: Vec::new(),
            post: PostFx::new(device, format),
            envelopes: PostEnvelopes::default(),
            motion: BustMotion::default(),
            reactor: Reactor::default(),
            head_fx: HeadFx::new(HeadShape::of(model), 11),
            rings: RingField::default(),
            uniforms: Uniforms::default(),
            pivot: model.pivot.to_array(),
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.renderer.resize(device, size);
    }

    /// Draw into this rectangle of the target, in physical pixels. It is
    /// clamped to the target, so a stale rectangle cannot fault a frame.
    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.renderer.set_viewport(x, y, w, h);
    }

    /// Advance by `dt` and render one frame into `view`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        frame: &AnalysisFrame,
        beat: bool,
        dt: f32,
        params: &Params,
        camera: Camera,
    ) {
        // the mode hears the music as its Motion settings shape it
        let (shaped, beat) = self.reactor.shape(frame, beat, dt, &params.react);
        let frame = &shaped;
        let knobs = params.knobs;
        self.motion.update(frame, dt, beat, knobs, &mut self.uniforms);
        // only the head has a head effect
        let effect = if params.grid.is_none() && params.generative.is_none() {
            knobs.effect
        } else {
            crate::head_fx::HeadEffect::None
        };
        self.head_fx.update(
            frame,
            dt,
            beat,
            effect,
            knobs.effect_amount,
            knobs.mid_gain,
            knobs.high_gain,
            &mut self.uniforms,
        );

        // whichever band the panel points the rings at
        let level = match knobs.ring_band {
            1 => frame.bands[2..5].iter().sum::<f64>() as f32 / 3.0 * knobs.mid_gain,
            2 => frame.bands[5..].iter().sum::<f64>() as f32 / 2.0 * knobs.high_gain,
            _ => frame.bands[..2].iter().sum::<f64>() as f32 / 2.0,
        };
        self.rings.set_presence(self.uniforms.meter[2]);
        if self.motion.dropped() && knobs.rings {
            self.rings.burst();
        }
        self.rings.update(level, dt, if knobs.rings { 1.0 } else { 0.0 });
        self.uniforms.rims = self.rings.rims(RING_SCALE);
        self.uniforms.meter[0] = RING_CENTRE[0];
        self.uniforms.meter[1] = RING_CENTRE[2];

        // the camera frames the stage, not the target, through the lens
        // the Perspective control chose
        let aspect = self.renderer.aspect();
        let (camera, fov) = lens(camera, params.perspective);
        self.uniforms.view_proj = view_proj_lens(aspect, camera, fov).to_cols_array_2d();
        self.uniforms.eye = camera_eye(camera).extend(1.0).to_array();
        self.uniforms.band_axis = params.band_axis as f32;
        self.uniforms.band_push = params.band_push;
        self.envelopes.update(frame, beat, dt);

        // with any effect on, the stage draws off screen and the effects
        // composite it onto the target
        let effects = !params.post.is_off();
        if effects {
            self.post.prepare(device, self.renderer.target_size());
        }
        let target = match self.post.scene_view() {
            Some(scene) if effects => scene,
            _ => view,
        };
        self.uniforms.pivot = self.pivot;
        let (vw, vh) = self.renderer.viewport_size();
        self.uniforms.vp_w = vw;
        self.uniforms.vp_h = vh;
        self.renderer.set_rings(queue, &self.rings, RING_CENTRE);

        if let Some(gk) = params.generative {
            // the Generative Geometry replaces the head too: its frame is
            // drawn off screen, then composited into the stage
            self.generative.update(frame, dt, beat, &gk);
            self.generative.build(vw, vh, &gk, &mut self.lines);
            let feedback = self.generative.feedback(dt, vw / vh.max(1.0), &gk);
            let size = (vw.round() as u32, vh.round() as u32);
            self.generative_renderer.advance(device, queue, size, &self.lines, &feedback);
            let g = &self.generative_renderer;
            self.renderer.render_with(device, queue, target, &self.uniforms, |pass| g.draw(pass));
        } else {
            match params.grid {
                // the Hiding Grid replaces the head rather than joining it
                Some(gk) => {
                    // the wall is laid out to fill the frame, whatever its
                    // shape: x spans the frame's aspect, y spans 1
                    let aspect = vw / vh.max(1.0);
                    self.grid.update(frame, dt, beat, GridKnobs { aspect, ..gk });
                    self.grid_renderer.upload(
                        queue,
                        &self.grid,
                        &GridUniforms {
                            fit: [1.0 / aspect, 1.0],
                            tint: self.uniforms.tint,
                            mix: self.uniforms.tint_mix,
                            ..Default::default()
                        },
                    );
                    let g = &self.grid_renderer;
                    self.renderer
                        .render_with(device, queue, target, &self.uniforms, |pass| g.draw(pass));
                }
                None => self.renderer.render(device, queue, target, &self.uniforms),
            }
        }
        if effects {
            let stage = self.renderer.stage_rect();
            self.post.apply(device, queue, view, stage, &params.post, &self.envelopes);
        }
    }
}
