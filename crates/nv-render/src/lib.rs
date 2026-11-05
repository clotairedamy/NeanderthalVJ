//! The visualizations, on the GPU.
pub mod bust;
pub mod generative;
pub mod generative_render;
pub mod grid;
pub mod grid_render;
pub mod head_fx;
pub mod motion;
pub mod noise;
pub mod palette;
pub mod post;
pub mod react;
pub mod renderer;
pub mod rings;
pub mod scene;

pub use bust::BustModel;
pub use generative::{GenKnobs, Generative};
pub use generative_render::GenerativeRenderer;
pub use grid::{GridKnobs, GridUniforms, HidingGrid, Impact};
pub use grid_render::GridRenderer;
pub use head_fx::{HeadEffect, HeadFx};
pub use motion::{BustMotion, Knobs};
pub use post::{PostEnvelopes, PostFx, PostKnobs};
pub use react::{Division, Drive, ReactKnobs, Reactor, Source};
pub use renderer::{bust_view_proj, view_proj_from, BustRenderer, Camera, Uniforms};
pub use rings::{RingField, RingVertex};
pub use scene::{Params, Scene};
