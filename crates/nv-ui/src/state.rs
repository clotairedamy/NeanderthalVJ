//! The settings the interface edits.
//!
//! One struct behind one signal would re-render every panel on every tick,
//! so the fast-moving readouts ([`Readout`]) are kept apart from the
//! settings, which change only when someone touches a control.
//!
//! Every field here drives something; a setting nothing reads is a control
//! that lies. `tests/settings.rs` checks each panel's controls reach the
//! renderer or the engine.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// The scanned head as a point cloud.
    PointCloud,
    /// The same head, played by a synth's oscillator.
    SynthCloud,
    /// A lattice of squares the music strikes like a wall.
    HidingGrid,
    /// Procedural shapes that grow, mutate and answer the music.
    Generative,
}

/// The visualizations, in the order the mode list shows them; number key
/// N picks the Nth.
pub const MODES: [Mode; 4] =
    [Mode::PointCloud, Mode::SynthCloud, Mode::HidingGrid, Mode::Generative];

impl Mode {
    /// Its place in [`MODES`].
    pub fn index(self) -> usize {
        MODES.iter().position(|m| *m == self).unwrap_or(0)
    }

    /// The next mode in the list, round to the first.
    pub fn next(self) -> Mode {
        MODES[(self.index() + 1) % MODES.len()]
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::PointCloud => "Depth Point Cloud",
            Mode::SynthCloud => "Synth Cloud (Crave)",
            Mode::HidingGrid => "Hiding Grid",
            Mode::Generative => "Generative Geometry",
        })
    }
}

/// The mode a number key picks: 1 is the first.
pub fn mode_for_key(key: &str) -> Option<Mode> {
    let d = key.parse::<usize>().ok().filter(|_| key.len() == 1)?;
    MODES.get(d.checked_sub(1)?).copied()
}

/// The palettes on offer: the named ones, then the one pulled from an
/// image and the one picked by hand.
pub const PALETTES: [&str; 8] =
    ["nsku", "viridis", "plasma", "turbo", "neon", "mono", "image", "custom"];
pub const PC_BAND_AXES: [&str; 5] = ["off", "vertical", "depth", "horizontal", "radial"];
pub const RING_BANDS: [&str; 3] = ["lows", "mids", "highs"];
/// The Depth Point Cloud's head effects, as `nv_render::HeadEffect` names them.
pub const PC_EFFECTS: [&str; 4] = ["none", "orbits", "halo", "vortex"];
pub const GRID_IMPACTS: [&str; 4] = ["centre", "scattered", "floor", "sides"];

/// Which panel the left column is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    Source,
    #[default]
    Visuals,
    Motion,
    Fx,
    Export,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Source, Tab::Visuals, Tab::Motion, Tab::Fx, Tab::Export];

    pub const fn slug(self) -> &'static str {
        match self {
            Tab::Source => "source",
            Tab::Visuals => "visuals",
            Tab::Motion => "motion",
            Tab::Fx => "fx",
            Tab::Export => "export",
        }
    }

    pub fn from_slug(s: &str) -> Self {
        Tab::ALL.into_iter().find(|t| t.slug() == s).unwrap_or_default()
    }
}

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Tab::Source => "Source",
            Tab::Visuals => "Visuals",
            Tab::Motion => "Motion",
            Tab::Fx => "FX",
            Tab::Export => "Export",
        };
        f.write_str(name)
    }
}

/// Everything the panels set. Saved between sessions and in presets; a
/// field missing from an older file takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // analysis and motion
    pub smoothing: f64,
    pub sensitivity: f64,
    /// How slowly the visuals let go of a hit, 0.70..0.95.
    pub damping: f64,
    pub beat_impulse: f64,
    pub fft_min_hz: f64,
    pub fft_max_hz: f64,
    /// The visuals' delay against the sound, in ms.
    pub av_offset_ms: f64,
    /// Orbit the camera slowly around the head.
    pub auto_camera: bool,
    /// How fast it orbits, 0..2.
    pub auto_camera_speed: f64,
    /// Show the level readout in the status bar.
    pub show_level: bool,

    // colour
    pub palette: String,
    /// Each mode keeps its own palette.
    pub palette_per_mode: bool,
    /// The palette of each mode, in [`MODES`] order, when it keeps its own.
    pub mode_palettes: [String; 4],
    /// How much of the palette colours the head and the grid, 0..1.
    pub tint_strength: f64,
    /// The colours pulled from a picture, low to high.
    pub image_palette: Vec<[f32; 3]>,
    /// The hand-picked palette: low, mid and high, as `#rrggbb`.
    pub custom_palette: [String; 3],

    // visualization
    pub viz_mode: Mode,
    /// Move to the next mode on its own as the song goes on.
    pub auto_switch_sections: bool,
    /// Every this many bars.
    pub auto_switch_bars: u32,
    /// And at once when the energy jumps, as at a drop.
    pub auto_switch_drops: bool,

    // live input, always the device's first stereo pair
    pub input_device: String,
    pub input_gain_db: f64,
    pub input_gate_db: f64,

    // the point cloud modes: the particle head
    pub pc_point_size: f64,
    /// The lens, 0..1; 0.35 is the usual 45 degrees.
    pub pc_perspective: f64,
    pub pc_band_axis: String,
    pub pc_band_push: f64,
    pub pc_drum_glitch: f64,
    pub pc_synth: bool,
    pub pc_brainwaves: bool,
    pub pc_reactivity: f64,
    /// How hard the mids drive the visuals.
    pub pc_mid_gain: f64,
    /// How hard the highs drive them.
    pub pc_high_gain: f64,
    /// Which band the rings answer: "lows" | "mids" | "highs".
    pub pc_ring_band: String,
    /// The Depth Point Cloud's head effect (one of `PC_EFFECTS`) and how
    /// strongly it plays, 0..2. Synth Cloud has none.
    pub pc_effect: String,
    pub pc_effect_amount: f64,

    // hiding grid
    /// Columns; the source drawing used 40.
    pub grid_cols: usize,
    /// Noise periods across the canvas.
    pub grid_noise: f64,
    /// Lattice deformation amount.
    pub grid_warp: f64,
    /// How hard beats resize the squares.
    pub grid_beat: f64,
    /// How fast the field drifts.
    pub grid_flow: f64,
    /// How hard the lows' shock waves strike.
    pub grid_waves: f64,
    pub grid_wave_speed: f64,
    /// How much of each wave comes back off the edges.
    pub grid_echo: f64,
    /// Where the waves strike: "centre" | "scattered" | "floor" | "sides".
    pub grid_impact: String,
    /// How far the mids turn the squares.
    pub grid_twist: f64,
    /// How many sparks the highs strike.
    pub grid_sparkle: f64,
    pub grid_analyser: bool,
    /// How far the wall leans back.
    pub grid_tilt: f64,
    /// Black and white to full palette colour.
    pub grid_color: f64,

    // generative geometry
    pub gen_symmetry: u32,
    pub gen_complexity: f64,
    pub gen_trails: f64,
    pub gen_evolve: f64,
    pub gen_width: f64,
    pub gen_spin: f64,
    pub gen_flow: f64,
    pub gen_glow: f64,

    // effects, each 0 for off
    pub fx_bloom: f64,
    pub fx_threshold: f64,
    pub fx_flash: f64,
    pub fx_grain: f64,
    pub fx_chroma: f64,
    pub fx_vignette: f64,
    pub fx_ghost: f64,

    // transport
    pub speed: f64,
    pub preserve_pitch: bool,
    pub looping: bool,
    pub performance: bool,
    /// How often the visuals are drawn: 60, or 120 for ProMotion smoothness
    /// at some cost to the interface's responsiveness.
    pub frame_rate: u32,

    // export
    pub export_fps: u32,
    /// "standard" or "high" bitrate.
    pub export_quality: String,
    /// Where the clip starts, in seconds.
    pub export_start: f64,
    /// How long it runs, in seconds; 0 for to the end.
    pub export_length: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            smoothing: 0.8,
            sensitivity: 1.0,
            damping: 0.85,
            beat_impulse: 1.0,
            fft_min_hz: 20.0,
            fft_max_hz: 16_000.0,
            av_offset_ms: 0.0,
            auto_camera: false,
            auto_camera_speed: 0.5,
            show_level: true,
            palette: "plasma".into(),
            palette_per_mode: true,
            mode_palettes: ["plasma".into(), "plasma".into(), "nsku".into(), "plasma".into()],
            tint_strength: 0.55,
            image_palette: Vec::new(),
            custom_palette: ["#1a1446".into(), "#ff3d7f".into(), "#ffe53b".into()],
            viz_mode: Mode::SynthCloud,
            auto_switch_sections: false,
            auto_switch_bars: 16,
            auto_switch_drops: true,
            input_device: String::new(),
            input_gain_db: 0.0,
            input_gate_db: -55.0,
            pc_point_size: 2.2,
            pc_perspective: 0.35,
            pc_band_axis: "off".into(),
            pc_band_push: 0.0,
            // Off by default: on a drum track it fires every kick and cuts
            // the face into offset slabs. The slider goes up for anyone
            // who wants that.
            pc_drum_glitch: 0.0,
            pc_synth: false,
            pc_brainwaves: true,
            pc_reactivity: 1.0,
            pc_mid_gain: 1.0,
            pc_high_gain: 1.0,
            pc_ring_band: "lows".into(),
            // the head effect is what sets the Depth Point Cloud apart from
            // Synth Cloud, so it starts on, with the nannou comets
            pc_effect: "orbits".into(),
            pc_effect_amount: 1.0,
            grid_cols: 40,
            grid_noise: 7.0,
            grid_warp: 1.0,
            grid_beat: 0.7,
            grid_flow: 0.35,
            grid_waves: 1.0,
            grid_wave_speed: 1.0,
            grid_echo: 0.5,
            grid_impact: "centre".into(),
            grid_twist: 0.6,
            grid_sparkle: 1.0,
            grid_analyser: false,
            grid_tilt: 0.3,
            grid_color: 0.8,
            gen_symmetry: 6,
            gen_complexity: 0.7,
            gen_trails: 0.6,
            gen_evolve: 0.5,
            gen_width: 1.0,
            gen_spin: 1.0,
            gen_flow: 1.0,
            gen_glow: 1.0,
            fx_bloom: 0.35,
            fx_threshold: 0.6,
            fx_flash: 0.6,
            fx_grain: 0.0,
            fx_chroma: 0.0,
            fx_vignette: 0.35,
            fx_ghost: 0.0,
            speed: 1.0,
            preserve_pitch: true,
            looping: false,
            performance: false,
            frame_rate: 60,
            export_fps: 60,
            export_quality: "high".into(),
            export_start: 0.0,
            export_length: 0.0,
        }
    }
}

impl Settings {
    /// The palette the current mode wears.
    pub fn active_palette(&self) -> &str {
        if self.palette_per_mode {
            &self.mode_palettes[self.viz_mode.index()]
        } else {
            &self.palette
        }
    }

    /// Choose a palette for the current mode, or for every mode.
    pub fn set_palette(&mut self, name: &str) {
        if self.palette_per_mode {
            self.mode_palettes[self.viz_mode.index()] = name.to_string();
        } else {
            self.palette = name.to_string();
        }
    }
}

/// How many spectrum bars the scope draws.
pub const SCOPE_BARS: usize = 32;

/// What the status bar and the scope show. Updated every frame, so it is
/// deliberately not part of [`Settings`] -- a control panel has no reason
/// to re-render because the FPS counter moved.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Readout {
    pub bpm: f64,
    pub fps: f64,
    pub velocity: f64,
    pub level_db: f64,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    /// The spectrum, low to high, 0..1, for the scope.
    pub spectrum: [f32; SCOPE_BARS],
}

impl Readout {
    /// `m:ss`, as the transport shows it.
    pub fn clock(seconds: f64) -> String {
        let s = seconds.max(0.0) as u64;
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// The loaded media, which decides what several panels can offer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sources {
    pub audio: Option<String>,
    /// The live input is what the visuals follow.
    pub mic_live: bool,
    pub midi_devices: Vec<String>,
}
