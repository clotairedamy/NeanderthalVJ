//! The Hiding Grid: a wall the music hits.
//!
//! It began as two reference drawings that speak the same language, a
//! grid plus a coherent noise field: **hiding squares**, where each cell
//! holds a square the field scales and slides so a square leaving its box
//! is progressively hidden, and a **deformed lattice**, where a smoother
//! band of the same field pushes the nodes off-grid. One field does both,
//! so the squares ride the mesh instead of floating over it.
//!
//! On top of that, each part of the music does its own thing to the wall,
//! built on nannou's vectors, maths and seeded randomness:
//!
//! - **Lows** strike it. Each kick sends a shock wave across the wall from
//!   its impact point; the lattice bulges toward the viewer as the front
//!   passes, the squares under it pop and light, and the wave reflects off
//!   the edges as fainter echoes, as sound does off a real wall.
//! - **Mids** turn the squares in their cells and make the field flow.
//! - **Highs** strike sparks: single cells flash, more of them the brighter
//!   the top end.
//! - **The spectrum**, optionally, stands up the wall like a giant
//!   analyser: each column lights to the height of its frequency.
//! - **Beats** swell every square, so the grid breathes holes.

use nannou_core::glam::{vec2, vec3, Vec2, Vec3};
use nannou_core::rand::rand::rngs::StdRng;
use nannou_core::rand::rand::{RngExt, SeedableRng};
use nv_audio::AnalysisFrame;

use crate::noise::{perlin3, Permutation};

/// What the grid's shader reads.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub fit: [f32; 2],
    pub _pad0: [f32; 2],
    pub tint: [f32; 3],
    pub mix: f32,
}

impl Default for GridUniforms {
    fn default() -> Self {
        Self { fit: [1.0, 1.0], _pad0: [0.0; 2], tint: [1.0; 3], mix: 0.0 }
    }
}

/// Measured from `hiding-squares.svg`: the raw file uses `t = (90/7)(1-s)`
/// on a 50-unit cell, which in a cell normalized to [-1, 1] is this.
const CENTRE_K: f32 = 18.0 / 35.0;
/// The file's observed range of `s`.
const S_HI: f32 = 1.0;
const S_LO: f32 = -1.75;
/// The warp field is the same noise an octave and a half down: measured at
/// ~1.9 periods across the mesh against ~7 for the squares.
const WARP_SCALE: f32 = 0.27;
/// Peak node displacement, in cells.
const WARP_CELLS: f32 = 3.5;
/// Shock waves in flight at once.
const MAX_WAVES: usize = 10;

pub const MAX_COLS: usize = 96;
/// The most cells the wall is built from, past the frame's edges included.
pub const MAX_CELLS: usize = MAX_COLS * MAX_COLS * 2;

/// Clip a cell's square to the cell, in normalized [-1, 1] coordinates.
///
/// Returns `(lo, hi)` along one axis; the square is offset equally in x
/// and y so both axes clip identically. `lo >= hi` means it has slid fully
/// out of its cell and is hidden.
pub fn hiding_square(s: f32) -> (f32, f32) {
    let centre = CENTRE_K * (s - s * s);
    let half = s.abs();
    ((centre - half).max(-1.0), (centre + half).min(1.0))
}

/// One vertex of the grid: a position and its colour.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridVertex {
    pub pos: [f32; 2],
    pub color: [f32; 3],
    pub _pad: f32,
}

/// Where the shock waves strike the wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Impact {
    /// Dead centre, every time: it rings like a drum skin.
    #[default]
    Centre,
    /// Anywhere on the wall.
    Scattered,
    /// Rising from the floor.
    Floor,
    /// From the left and right walls in turn.
    Sides,
}

/// What the Visuals panel controls about the grid.
#[derive(Debug, Clone, Copy)]
pub struct GridKnobs {
    pub cols: usize,
    /// Feature size of the field.
    pub noise: f32,
    /// How far the lattice warps.
    pub warp: f32,
    /// How fast the field flows.
    pub flow: f32,
    /// How hard a beat swells the squares.
    pub beat: f32,
    pub beat_impulse: f32,
    pub sensitivity: f32,
    /// How hard the lows' shock waves hit; 0 for none.
    pub waves: f32,
    /// How fast they cross the wall.
    pub wave_speed: f32,
    /// How much of each wave comes back off the edges.
    pub echo: f32,
    pub impact: Impact,
    /// How far the mids turn the squares.
    pub twist: f32,
    /// How many sparks the highs strike.
    pub sparkle: f32,
    /// Light each column to the height of its frequency.
    pub analyser: bool,
    /// How far the wall leans back, 0 face-on to 1.
    pub tilt: f32,
    /// 0 for black and white, 1 for the palette's colours.
    pub color: f32,
    /// The lows', mids' and highs' colours, from the palette.
    pub colors: [[f32; 3]; 3],
    /// The frame's width over its height: the wall is laid out to fill
    /// it, whatever its shape.
    pub aspect: f32,
}

impl Default for GridKnobs {
    fn default() -> Self {
        Self {
            cols: 40,
            noise: 7.0,
            warp: 1.0,
            flow: 1.0,
            beat: 1.0,
            beat_impulse: 1.0,
            sensitivity: 1.0,
            waves: 1.0,
            wave_speed: 1.0,
            echo: 0.5,
            impact: Impact::Centre,
            twist: 0.6,
            sparkle: 1.0,
            analyser: false,
            tilt: 0.3,
            color: 0.8,
            colors: [[1.0, 0.35, 0.2], [0.35, 0.6, 1.0], [0.95, 0.95, 1.0]],
            aspect: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Wave {
    origin: Vec2,
    radius: f32,
    amp: f32,
}

pub struct HidingGrid {
    perm: Permutation,
    rng: StdRng,
    cols: usize,
    /// Cells across and down, past the frame's edges included.
    dims: (usize, usize),
    t: f32,
    /// Beats resize the squares; this is the multiplier on `s`.
    swell: f32,
    swell_v: f32,
    warp_amt: f32,
    waves: Vec<Wave>,
    since_wave: f32,
    side: bool,
    mids: f32,
    highs: f32,
    /// Two triangles per cell, over four vertices of its own.
    pub quads: Vec<GridVertex>,
    /// The lattice, as line segments.
    pub lines: Vec<GridVertex>,
}

impl Default for HidingGrid {
    fn default() -> Self {
        Self {
            perm: Permutation::new(11),
            rng: StdRng::seed_from_u64(11),
            cols: 0,
            dims: (0, 0),
            t: 0.0,
            swell: 1.0,
            swell_v: 0.0,
            warp_amt: 1.0,
            waves: Vec::with_capacity(MAX_WAVES),
            since_wave: 1.0,
            side: false,
            mids: 0.0,
            highs: 0.0,
            quads: Vec::new(),
            lines: Vec::new(),
        }
    }
}

/// Mix `a` towards `b` by `t`.
fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn scale3(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// A cheap per-cell random, stable for a given cell and moment.
fn hash(a: f32, b: f32) -> f32 {
    ((a * 12.9898 + b * 78.233).sin() * 43_758.547).fract().abs()
}

impl HidingGrid {
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// The cells the wall is built from, across and down, including the
    /// ones past the frame's edges that keep it covered as it moves.
    pub fn dims(&self) -> (usize, usize) {
        self.dims
    }

    /// How many shock waves are crossing the wall.
    pub fn waves_in_flight(&self) -> usize {
        self.waves.len()
    }

    /// The wall's height at `p` from every wave and its echoes, and how
    /// close `p` is to a wave front, for the light.
    fn wave_at(&self, p: Vec2, echo: f32, aspect: f32) -> (f32, f32) {
        let width = 0.16;
        let mut height = 0.0;
        let mut crest = 0.0;
        for w in &self.waves {
            // the wave itself, then its reflections off the frame's four
            // edges: a mirror image of the source behind each one
            let o = w.origin;
            let a = aspect;
            let sources = [
                (o, 1.0),
                (vec2(2.0 * a - o.x, o.y), echo),
                (vec2(-2.0 * a - o.x, o.y), echo),
                (vec2(o.x, 2.0 - o.y), echo),
                (vec2(o.x, -2.0 - o.y), echo),
            ];
            for (src, gain) in sources {
                if gain <= 0.0 {
                    continue;
                }
                let x = (p.distance(src) - w.radius) / width;
                if x.abs() > 4.0 {
                    continue;
                }
                let env = (-x * x).exp() * w.amp * gain;
                // a crest with a trough behind it, as a pressure wave has
                height += env * (x * 2.2).cos();
                crest += env;
            }
        }
        (height, crest)
    }

    /// Rebuild the wall for this frame.
    pub fn update(&mut self, frame: &AnalysisFrame, dt: f32, beat: bool, k: GridKnobs) {
        let dt = dt.min(0.05);
        let cols = k.cols.clamp(4, MAX_COLS);
        self.cols = cols;
        // The frame is x in [-aspect, aspect], y in [-1, 1], and `cols`
        // squares span its width. The wall reaches past every edge, so the
        // warp, the waves and the lean never show its border.
        let aspect = k.aspect.clamp(0.2, 5.0);
        let mut cell = 2.0 * aspect / cols as f32;

        let sens = k.sensitivity;
        let band = |lo: usize, hi: usize| {
            (frame.bands[lo..hi].iter().sum::<f64>() as f32 / (hi - lo) as f32 * sens)
                .clamp(0.0, 1.5)
        };
        let bass = band(0, 2);
        let mids = band(2, 5);
        let highs = band(5, 7);
        self.mids += (mids - self.mids) * (dt * 10.0).min(1.0);
        self.highs += (highs - self.highs) * (dt * 14.0).min(1.0);

        // a beat swells every square; a kick pulls them back in, so the
        // grid breathes rather than only growing
        if beat {
            self.swell_v += frame.beat_strength as f32 * k.beat * k.beat_impulse * 1.6;
        }
        self.swell_v -= frame.punch as f32 * k.beat * 0.8 * dt * 16.0;
        // a spring back to 1: accelerate toward the target, with friction
        self.swell_v += (1.0 - self.swell) * dt * 16.0;
        self.swell_v *= 0.88;
        self.swell = (self.swell + self.swell_v * dt).clamp(0.15, 2.4);

        // the lows strike the wall: a wave per kick, not per frame of it
        self.since_wave += dt;
        let on_beat = if beat { frame.beat_strength as f32 * 0.8 } else { 0.0 };
        let kick = (frame.attack[0].max(frame.attack[1]) as f32 * sens).max(on_beat);
        if k.waves > 0.0 && kick > 0.3 && self.since_wave > 0.11 {
            self.since_wave = 0.0;
            let origin = match k.impact {
                Impact::Centre => Vec2::ZERO,
                Impact::Scattered => vec2(
                    self.rng.random_range(-0.8..0.8) * aspect,
                    self.rng.random_range(-0.8..0.8),
                ),
                Impact::Floor => vec2(self.rng.random_range(-0.7..0.7) * aspect, -1.05),
                Impact::Sides => {
                    self.side = !self.side;
                    let x = aspect + 0.05;
                    vec2(if self.side { -x } else { x }, self.rng.random_range(-0.6..0.6))
                }
            };
            if self.waves.len() == MAX_WAVES {
                self.waves.remove(0);
            }
            self.waves.push(Wave { origin, radius: 0.0, amp: kick.min(1.2) * k.waves });
        }
        let speed = 1.25 * k.wave_speed.max(0.05);
        for w in &mut self.waves {
            w.radius += dt * speed;
            w.amp *= (-dt * 1.1).exp();
        }
        self.waves.retain(|w| w.amp > 0.01 && w.radius < 4.5);

        // the field flows faster when the track is busier, and the mids
        // push it on
        self.t += dt * (k.flow * (0.35 + 1.4 * frame.rms as f32 + 0.8 * self.mids));

        // the low-frequency band of the field warps the lattice
        let want = 0.55 + 1.1 * bass;
        self.warp_amt += (want - self.warp_amt) * (dt * 6.0).min(1.0);
        let wa = self.warp_amt.clamp(0.0, 2.5) * k.warp;
        let ns = k.noise;

        // The wall leans back, top away from the viewer, seen in
        // perspective: a wave's height then reads as depth. Leaning back
        // shrinks the far edge, so the wall reaches further the more it
        // leans -- far enough that its receding top still clears the frame.
        let lean = k.tilt.clamp(0.0, 1.0);
        let (sin_a, cos_a) = lean.sin_cos();
        let project = |p: Vec3| -> [f32; 2] {
            let y = p.y * cos_a + p.z * sin_a;
            let z = -p.y * sin_a + p.z * cos_a;
            let persp = 3.2 / (3.2 - z).max(0.4);
            [p.x * persp, y * persp]
        };
        let recede = |y: f32| 3.2 / (3.2 + y * sin_a);
        let mut reach = 1.06f32;
        while reach < 6.0 && reach * cos_a * recede(reach) < 1.06 {
            reach += 0.05;
        }
        // and the warp can pull the edge in by as much as it displaces
        let margin = 2.0 * cell * WARP_CELLS * wa.max(0.3);
        let half_x = aspect * 1.06 / recede(reach) + margin;
        let half_y = reach + margin;
        let (mut nx, mut ny) = (
            ((2.0 * half_x / cell).ceil() as usize).max(4),
            ((2.0 * half_y / cell).ceil() as usize).max(4),
        );
        if nx * ny > MAX_CELLS {
            // the finest columns on the widest frame: coarser cells rather
            // than an uncovered edge
            let k = ((nx * ny) as f32 / MAX_CELLS as f32).sqrt();
            cell *= k;
            nx = ((2.0 * half_x / cell).ceil() as usize).max(4);
            ny = ((2.0 * half_y / cell).ceil() as usize).max(4);
            while nx * ny > MAX_CELLS {
                ny -= 1;
            }
        }
        self.dims = (nx, ny);
        let amp = cell * WARP_CELLS * wa;
        let (x0, y0) = (-(nx as f32) * cell * 0.5, -(ny as f32) * cell * 0.5);
        let n = nx + 1;
        let rows = ny + 1;
        // the field is laid out in the frame's own units, so a feature is
        // the same size whatever the frame's shape
        let unit = aspect.max(1.0);

        // warped and struck node positions, and the light of the waves
        let mut nodes = vec![Vec3::ZERO; n * rows];
        let mut crests = vec![0.0f32; n * rows];
        for row in 0..rows {
            for col in 0..n {
                let gx = x0 + cell * col as f32;
                let gy = y0 + cell * row as f32;
                let (wx, wy) = (gx / unit * ns * WARP_SCALE, gy / unit * ns * WARP_SCALE);
                let dx = perlin3(wx, wy, self.t, &self.perm);
                let dy = perlin3(wx + 31.7, wy + 11.3, self.t, &self.perm);
                let flat = vec2(gx, gy);
                let (height, crest) = self.wave_at(flat, k.echo, aspect);
                // the wave pushes the lattice out from the centre as well
                // as toward the viewer, so it reads even face-on
                let push = flat.normalize_or_zero() * height * 0.05;
                nodes[row * n + col] =
                    vec3(gx + dx * amp + push.x, gy + dy * amp + push.y, height * 0.32);
                crests[row * n + col] = crest;
            }
        }

        let grey = |v: f32| [v, v, v];
        let [low_c, mid_c, high_c] = k.colors;
        let colour = k.color.clamp(0.0, 1.0);
        let spark_clock = (self.t * 9.0).floor();

        // the squares, clipped to their own warped cell
        self.quads.clear();
        self.quads.reserve(nx * ny * 4);
        for row in 0..ny {
            for col in 0..nx {
                let gx = x0 + cell * col as f32;
                let gy = y0 + cell * row as f32;
                let (cx, cy) = ((gx + cell * 0.5) / unit * ns, (gy + cell * 0.5) / unit * ns);
                let fine = perlin3(cx, cy, self.t * 1.7 + 5.0, &self.perm);
                let mut s = S_HI - (fine * 0.5 + 0.5) * (S_HI - S_LO);

                // the spectrum lies across the columns: each grows and
                // lights with its own frequency
                let across = ((gx + cell * 0.5 + aspect) / (2.0 * aspect)).clamp(0.0, 0.999);
                let bin = (across * 64.0) as usize;
                let c = (frame.spectrum[bin] as f32).clamp(0.0, 1.0);
                let crest = (crests[row * n + col] + crests[(row + 1) * n + col + 1]) * 0.5;
                // a passing wave front pops the squares under it
                s *= self.swell * (0.72 + 0.55 * c) * (1.0 + 0.6 * crest.min(1.5));

                let (lo, hi) = hiding_square(s);
                if lo >= hi {
                    // hidden: a zero-area quad keeps the buffer a constant
                    // size while the field animates
                    self.quads.extend_from_slice(&[GridVertex::default(); 4]);
                    continue;
                }

                // the analyser: a column lights to its frequency's height
                let up = ((gy + cell * 0.5 + 1.0) * 0.5).clamp(0.0, 1.0);
                let reach = c * 1.1;
                let lit =
                    if k.analyser && up < reach { 1.0 - 0.5 * up / (reach + 1e-4) } else { 0.0 };
                // sparks: a few cells flash with the highs
                let chance = self.highs * self.highs * 0.06 * k.sparkle;
                let spark =
                    if hash(col as f32 + spark_clock * 0.37, row as f32 + spark_clock) < chance {
                        1.0
                    } else {
                        0.0
                    };

                let shade = 0.5 + 0.45 * c;
                let base = mix3(grey(shade), scale3(mid_c, shade * 1.2), colour * (0.4 + 0.6 * c));
                let struck = (crest * 1.2).min(1.0) * (0.45 + 0.55 * colour);
                let mut rgb = mix3(base, scale3(low_c, 1.6), struck);
                rgb = mix3(rgb, scale3(mix3(low_c, high_c, up), 1.5), lit * 0.8);
                rgb = mix3(rgb, scale3(mix3([1.0; 3], high_c, colour), 2.2), spark);

                // the mids turn each square about its centre, each its own
                // way, so the wall shimmers between order and disorder
                let angle = k.twist * self.mids * fine * 2.6;
                let (sa, ca) = angle.sin_cos();
                let mid = (lo + hi) * 0.5;
                let turn = |u: f32, v: f32| {
                    let (du, dv) = (u - mid, v - mid);
                    (mid + du * ca - dv * sa, mid + du * sa + dv * ca)
                };

                // the square rides its cell's four warped corners
                let at = |u: f32, v: f32| -> Vec3 {
                    let a = nodes[row * n + col];
                    let b = nodes[row * n + col + 1];
                    let d = nodes[(row + 1) * n + col];
                    let e = nodes[(row + 1) * n + col + 1];
                    let (fu, fv) = ((u + 1.0) * 0.5, (v + 1.0) * 0.5);
                    a.lerp(b, fu).lerp(d.lerp(e, fu), fv)
                };
                for (u, v) in [(lo, lo), (hi, lo), (hi, hi), (lo, hi)] {
                    let (u, v) = turn(u, v);
                    self.quads.push(GridVertex { pos: project(at(u, v)), color: rgb, _pad: 0.0 });
                }
            }
        }

        // the lattice itself, lit where the waves cross it
        self.lines.clear();
        self.lines.reserve(4 * n * rows);
        let line = |i: usize| -> GridVertex {
            let g = 0.2 + 0.25 * bass;
            let struck = (crests[i] * 0.8).min(1.0) * (0.3 + 0.7 * colour);
            GridVertex {
                pos: project(nodes[i]),
                color: mix3(grey(g), scale3(low_c, 1.4), struck),
                _pad: 0.0,
            }
        };
        for row in 0..rows {
            for col in 0..n - 1 {
                self.lines.push(line(row * n + col));
                self.lines.push(line(row * n + col + 1));
            }
        }
        for row in 0..rows - 1 {
            for col in 0..n {
                self.lines.push(line(row * n + col));
                self.lines.push(line((row + 1) * n + col));
            }
        }
    }
}
