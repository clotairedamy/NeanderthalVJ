//! Generative Geometry: procedural shapes that grow, mutate and answer the
//! music, built on nannou's geometry, maths and randomness.
//!
//! The picture is a stack of closed curves, one per band of the spectrum,
//! bass innermost. Each is a superformula -- the one family that spans
//! circles, polygons, stars and flowers with four numbers -- so the shapes
//! can wander between all of those without ever jumping. What is happening
//! in the music right now is always readable from it:
//!
//! - **Each band** swells its own ring and sharpens its points as it
//!   attacks, so a kick, a vocal and a hi-hat light different rings.
//! - **The mids** spin the rings and draw the web that ties them together.
//! - **The highs** make the edges shimmer.
//! - **A beat** throws out a polygon that expands and fades.
//! - **Every bar** the structure mutates: each ring picks a new shape and
//!   eases into it, so the piece keeps evolving over a whole song.
//!
//! Each frame is drawn over a slowly zooming, rotating, fading copy of the
//! last (see `generative_render`), which is what turns lines into the
//! evolving structures behind them.

use std::f32::consts::{PI, TAU};

use bytemuck::{Pod, Zeroable};
use nannou_core::glam::{vec2, Vec2};
use nannou_core::math::map_range;
use nannou_core::rand::rand::rngs::StdRng;
use nannou_core::rand::rand::{RngExt, SeedableRng};
use nv_audio::AnalysisFrame;

use crate::noise::{perlin3, Permutation};

/// One ring per analysis band.
pub const MAX_LAYERS: usize = 7;
/// Points around each ring.
const RES: usize = 384;
/// Beat polygons in flight at once.
const MAX_WAVES: usize = 10;
/// Line widths are given in pixels at 1080 and scaled to the target, so
/// the picture is the same in the window and in a 1080x1920 export.
const REFERENCE: f32 = 1080.0;

/// What the panel decides about the Generative Geometry.
#[derive(Debug, Clone, Copy)]
pub struct GenKnobs {
    /// The rotational symmetry every shape is built on.
    pub symmetry: u32,
    /// 0..1: how many rings, and how intricate.
    pub complexity: f32,
    /// 0..1: how long each frame lingers in the trails.
    pub trails: f32,
    /// 0..1: how often and how far the structure mutates.
    pub evolve: f32,
    /// The palette, low to high, the rings are coloured from.
    pub colors: [[f32; 3]; 5],
    pub mid_gain: f32,
    pub high_gain: f32,
    /// Line thickness, 1 as designed.
    pub width: f32,
    /// How fast the rings turn, 1 as designed; 0 holds them still.
    pub spin: f32,
    /// How fast the trails drift outward, 1 as designed.
    pub flow: f32,
    /// How luminous the lines are, 1 as designed.
    pub glow: f32,
}

impl Default for GenKnobs {
    fn default() -> Self {
        Self {
            symmetry: 6,
            complexity: 0.7,
            trails: 0.6,
            evolve: 0.5,
            colors: [
                [0.05, 0.03, 0.53],
                [0.49, 0.01, 0.66],
                [0.8, 0.28, 0.42],
                [0.97, 0.61, 0.25],
                [0.94, 0.97, 0.13],
            ],
            mid_gain: 1.0,
            high_gain: 1.0,
            width: 1.0,
            spin: 1.0,
            flow: 1.0,
            glow: 1.0,
        }
    }
}

/// A vertex of the line geometry, already in clip space.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GenVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
    /// -1..1 across the line's width, for its soft edge.
    pub edge: f32,
}

/// How the last frame is carried into this one.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Feedback {
    pub zoom: f32,
    pub rotate: f32,
    pub decay: f32,
    pub aspect: f32,
    pub exposure: f32,
    pub hue: f32,
    pub _pad: [f32; 2],
}

/// The superformula's four numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shape {
    m: f32,
    n1: f32,
    n2: f32,
    n3: f32,
}

impl Shape {
    fn radius(&self, theta: f32, sharpen: f32) -> f32 {
        let q = self.m * theta / 4.0;
        let a = q.cos().abs().powf(self.n2);
        let b = q.sin().abs().powf(self.n3);
        // an attack drops n1, which pulls the shape into points
        let n1 = (self.n1 * (1.0 - 0.6 * sharpen)).max(0.15);
        (a + b).max(1e-6).powf(-1.0 / n1)
    }

    fn ease(&mut self, to: &Shape, k: f32) {
        self.m += (to.m - self.m) * k;
        self.n1 += (to.n1 - self.n1) * k;
        self.n2 += (to.n2 - self.n2) * k;
        self.n3 += (to.n3 - self.n3) * k;
    }
}

#[derive(Debug, Clone, Copy)]
struct Layer {
    shape: Shape,
    target: Shape,
    rotation: f32,
    /// Turns per second at rest, signed: neighbours counter-rotate.
    spin: f32,
}

#[derive(Debug, Clone, Copy)]
struct Wave {
    radius: f32,
    life: f32,
    sides: u32,
    rotation: f32,
    strength: f32,
}

/// The state of the piece, advanced once a frame.
pub struct Generative {
    rng: StdRng,
    perm: Permutation,
    t: f32,
    layers: [Layer; MAX_LAYERS],
    waves: Vec<Wave>,
    /// Smoothed band energies, and each band's attack flash.
    energy: [f32; MAX_LAYERS],
    flash: [f32; MAX_LAYERS],
    mids: f32,
    highs: f32,
    bass_kick: f32,
    last_bar: f32,
    beats: u32,
    symmetry: u32,
}

impl Default for Generative {
    fn default() -> Self {
        Self::new(7)
    }
}

impl Generative {
    /// A piece grown from `seed`: the same seed and the same music give
    /// the same picture, which an export relies on.
    pub fn new(seed: u64) -> Self {
        let mut g = Self {
            rng: StdRng::seed_from_u64(seed),
            perm: Permutation::new(seed),
            t: 0.0,
            layers: [Layer {
                shape: Shape { m: 6.0, n1: 4.0, n2: 2.0, n3: 2.0 },
                target: Shape { m: 6.0, n1: 4.0, n2: 2.0, n3: 2.0 },
                rotation: 0.0,
                spin: 0.0,
            }; MAX_LAYERS],
            waves: Vec::with_capacity(MAX_WAVES),
            energy: [0.0; MAX_LAYERS],
            flash: [0.0; MAX_LAYERS],
            mids: 0.0,
            highs: 0.0,
            bass_kick: 0.0,
            last_bar: 0.0,
            beats: 0,
            symmetry: 6,
        };
        for i in 0..MAX_LAYERS {
            let shape = g.random_shape(i, 6, 0.5);
            let l = &mut g.layers[i];
            l.shape = shape;
            l.target = shape;
            l.rotation = i as f32 * PI / 6.0;
            l.spin = if i % 2 == 0 { 0.012 } else { -0.017 } * (1.0 + i as f32 * 0.15);
        }
        g
    }

    /// A shape for ring `i` with `symmetry`-fold symmetry. `wild` widens
    /// the range from soft polygons towards stars and flowers.
    fn random_shape(&mut self, i: usize, symmetry: u32, wild: f32) -> Shape {
        let s = symmetry.max(2) as f32;
        // the outer rings may double the symmetry, so the piece reads as
        // one structure with finer detail at its edge
        let m = if i >= 3 && self.rng.random_bool(0.4) { s * 2.0 } else { s };
        let lo: f32 = map_range(wild, 0.0, 1.0, 1.8, 0.35);
        Shape {
            m,
            n1: self.rng.random_range(lo..6.0),
            n2: self.rng.random_range(0.6..(2.0 + 4.0 * wild)),
            n3: self.rng.random_range(0.6..(2.0 + 4.0 * wild)),
        }
    }

    /// Advance by `dt` seconds of `frame`; `beat` is a beat crossed since
    /// the last call.
    pub fn update(&mut self, frame: &AnalysisFrame, dt: f32, beat: bool, k: &GenKnobs) {
        let dt = dt.min(0.05);
        self.t += dt;

        if k.symmetry != self.symmetry {
            // a new symmetry is a new piece: every ring takes it at once
            self.symmetry = k.symmetry;
            for i in 0..MAX_LAYERS {
                let next = self.random_shape(i, k.symmetry, k.evolve);
                self.layers[i].target = next;
            }
        }

        for b in 0..MAX_LAYERS {
            let gain = match b {
                2..=4 => k.mid_gain,
                5.. => k.high_gain,
                _ => 1.0,
            };
            let e = (frame.bands[b] as f32 * gain).clamp(0.0, 1.5);
            // quick to rise, slower to fall, so a hit is seen in full
            let rate = if e > self.energy[b] { 18.0 } else { 5.0 };
            self.energy[b] += (e - self.energy[b]) * (dt * rate).min(1.0);
            let a = frame.attack[b] as f32 * gain;
            self.flash[b] = (self.flash[b] * (-dt * 7.0).exp()).max(a.clamp(0.0, 1.0));
        }
        self.mids = self.energy[2..5].iter().sum::<f32>() / 3.0;
        self.highs = self.energy[5..].iter().sum::<f32>() / 2.0;
        self.bass_kick = (self.bass_kick * (-dt * 6.0).exp()).max(self.flash[0].max(self.flash[1]));

        // the rings turn, faster with the mids
        for (i, l) in self.layers.iter_mut().enumerate() {
            let push = (1.0 + 6.0 * self.mids + 2.0 * self.flash[i]) * k.spin;
            l.rotation = (l.rotation + TAU * l.spin * push * dt) % TAU;
        }

        // a new bar mutates the structure; without a bar clock (a live
        // input that has not locked on yet) every eighth beat does
        let bar = frame.bar_phase as f32;
        let new_bar = bar < self.last_bar - 0.5;
        self.last_bar = bar;
        if beat {
            self.beats += 1;
        }
        let eighth = beat && self.beats.is_multiple_of(8) && frame.bar_phase == 0.0;
        if (new_bar || eighth) && k.evolve > 0.0 {
            // the more evolve, the more rings change at once
            let n = 1 + (k.evolve * (MAX_LAYERS - 1) as f32).round() as usize;
            for _ in 0..n {
                let i = self.rng.random_range(0..MAX_LAYERS);
                let next = self.random_shape(i, self.symmetry, k.evolve);
                self.layers[i].target = next;
            }
        }
        let ease = (dt * (0.5 + 2.5 * k.evolve)).min(1.0);
        for l in &mut self.layers {
            let to = l.target;
            l.shape.ease(&to, ease);
        }

        // a beat throws a polygon out from the centre
        if beat {
            let strength = (frame.beat_strength as f32).clamp(0.3, 1.0);
            let sides =
                if self.beats.is_multiple_of(4) { self.symmetry } else { 3 + self.beats % 4 };
            let rotation = self.rng.random_range(0.0..TAU);
            if self.waves.len() == MAX_WAVES {
                self.waves.remove(0);
            }
            self.waves.push(Wave {
                radius: 0.08,
                life: 1.0,
                sides: sides.max(3),
                rotation,
                strength,
            });
        }
        for w in &mut self.waves {
            w.radius += dt * (0.45 + 0.7 * w.strength);
            w.life -= dt * 0.65;
            w.rotation += dt * 0.4;
        }
        self.waves.retain(|w| w.life > 0.0 && w.radius < 1.7);
    }

    /// How many rings `complexity` asks for.
    fn layer_count(k: &GenKnobs) -> usize {
        (3.0 + k.complexity.clamp(0.0, 1.0) * (MAX_LAYERS - 3) as f32).round() as usize
    }

    /// Ring `i` as points, in units where 1 is half the shorter side.
    fn ring(&self, i: usize, n: usize, out: &mut Vec<Vec2>) {
        let l = &self.layers[i];
        let e = self.energy[i];
        let f = self.flash[i];
        let mut rs = [0.0f32; RES];
        let mut max = 0.0f32;
        for (j, r) in rs.iter_mut().enumerate() {
            let theta = j as f32 / RES as f32 * TAU;
            *r = l.shape.radius(theta, f);
            max = max.max(*r);
        }
        // normalised, so every shape fits its ring whatever its numbers
        let base: f32 = map_range(i as f32, 0.0, (n.max(2) - 1) as f32, 0.16, 0.86);
        let scale = base * (1.0 + 0.32 * e + 0.12 * f) / max.max(1e-6);
        let shimmer = 0.045 * self.highs * (0.4 + i as f32 / n as f32);
        out.clear();
        for (j, r) in rs.iter().enumerate() {
            let theta = j as f32 / RES as f32 * TAU;
            let jitter = shimmer
                * perlin3(
                    theta.cos() * 3.0 + i as f32,
                    theta.sin() * 3.0,
                    self.t * 2.3,
                    &self.perm,
                );
            let p = Vec2::from_angle(theta + l.rotation) * (r * scale + jitter);
            out.push(p);
        }
    }

    /// The colour of ring `i` of `n`, from the palette, brightened by its
    /// band.
    fn color(&self, i: usize, n: usize, k: &GenKnobs) -> [f32; 4] {
        let t = i as f32 / (n.max(2) - 1) as f32 * 4.0;
        let a = (t.floor() as usize).min(3);
        let f = t - a as f32;
        let (c0, c1) = (k.colors[a], k.colors[a + 1]);
        let lift = 0.25 + 0.9 * self.energy[i] + 1.0 * self.flash[i];
        [
            (c0[0] + (c1[0] - c0[0]) * f) * lift,
            (c0[1] + (c1[1] - c0[1]) * f) * lift,
            (c0[2] + (c1[2] - c0[2]) * f) * lift,
            1.0,
        ]
    }

    /// The line geometry for a `w` x `h` pixel target.
    pub fn build(&self, w: f32, h: f32, k: &GenKnobs, out: &mut Vec<GenVertex>) {
        out.clear();
        let short = w.min(h).max(1.0);
        let fit = vec2(short / w.max(1.0), short / h.max(1.0));
        // pixels at the reference size, in units
        let px = 2.0 / REFERENCE * k.width.max(0.05);
        let n = Self::layer_count(k);
        let mut pts = Vec::with_capacity(RES);
        let mut rings: Vec<Vec<Vec2>> = Vec::with_capacity(n);

        for i in 0..n {
            self.ring(i, n, &mut pts);
            let color = self.color(i, n, k);
            let width = (1.4 + 4.5 * self.energy[i] + 3.0 * self.flash[i]) * px;
            ribbon(&pts, true, width, color, fit, out);
            // a faint echo inside each ring gives it depth
            let echo: Vec<Vec2> = pts.iter().map(|p| *p * 0.94).collect();
            let dim = [color[0] * 0.25, color[1] * 0.25, color[2] * 0.25, 1.0];
            ribbon(&echo, true, width * 0.6, dim, fit, out);
            rings.push(pts.clone());
        }

        // the web: each ring tied to the next at its symmetry's spokes,
        // offset half a step so the ties zigzag into a lattice
        let s = self.symmetry.max(2) as usize;
        let web = (0.12 + 0.9 * self.mids).min(1.0);
        for i in 0..n.saturating_sub(1) {
            let c = self.color(i + 1, n, k);
            let color = [c[0] * web, c[1] * web, c[2] * web, 1.0];
            for spoke in 0..s * 2 {
                let a = spoke * RES / (s * 2);
                let b = (a + RES / (s * 4)) % RES;
                let (p, q) = if spoke % 2 == 0 {
                    (rings[i][a], rings[i + 1][b])
                } else {
                    (rings[i + 1][a], rings[i][b])
                };
                ribbon(&[p, q], false, (0.9 + 1.5 * self.mids) * px, color, fit, out);
            }
        }

        // the seed at the centre, breathing with the bass
        let bass = (self.energy[0] + self.energy[1]) * 0.5;
        let seed_r = 0.035 + 0.06 * bass + 0.05 * self.bass_kick;
        let seed: Vec<Vec2> = (0..=s * 2)
            .map(|j| {
                let r = if j % 2 == 0 { seed_r } else { seed_r * 0.45 };
                Vec2::from_angle(j as f32 * PI / s as f32 + self.layers[0].rotation) * r
            })
            .collect();
        let warm = k.colors[4];
        let glow = 0.6 + 2.0 * self.bass_kick;
        ribbon(
            &seed,
            true,
            (2.0 + 4.0 * self.bass_kick) * px,
            [warm[0] * glow, warm[1] * glow, warm[2] * glow, 1.0],
            fit,
            out,
        );

        // the beat polygons
        for wv in &self.waves {
            let pts: Vec<Vec2> = (0..wv.sides)
                .map(|j| {
                    Vec2::from_angle(wv.rotation + j as f32 * TAU / wv.sides as f32) * wv.radius
                })
                .collect();
            let fade = wv.life.powf(1.6) * (0.5 + wv.strength);
            let c = k.colors[3];
            ribbon(
                &pts,
                true,
                (1.0 + 3.0 * wv.life) * px,
                [c[0] * fade, c[1] * fade, c[2] * fade, 1.0],
                fit,
                out,
            );
        }
    }

    /// How the trails carry this frame into the next.
    pub fn feedback(&self, dt: f32, aspect: f32, k: &GenKnobs) -> Feedback {
        // per-frame values at 60 fps, scaled to this frame's length
        let frames = (dt * 60.0).clamp(0.0, 3.0);
        let decay: f32 = map_range(k.trails.clamp(0.0, 1.0), 0.0, 1.0, 0.78, 0.945);
        let zoom = 1.0 + (0.0006 + 0.006 * self.bass_kick + 0.0015 * self.mids) * k.flow;
        let turn = (0.0006 + 0.003 * self.mids) * k.flow;
        Feedback {
            zoom: zoom.powf(frames),
            rotate: turn * frames * if self.beats % 16 < 8 { 1.0 } else { -1.0 },
            decay: decay.powf(frames),
            aspect,
            exposure: 1.15 * k.glow,
            hue: 0.0025 * frames * (1.0 + 2.0 * self.highs),
            _pad: [0.0; 2],
        }
    }
}

/// A soft-edged line through `pts`, `width` units wide, appended to `out`
/// as triangles in clip space.
fn ribbon(
    pts: &[Vec2],
    closed: bool,
    width: f32,
    color: [f32; 4],
    fit: Vec2,
    out: &mut Vec<GenVertex>,
) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let half = width * 0.5;
    let at = |j: usize| pts[j % n];
    let normal = |j: usize| {
        let (prev, next) = if closed {
            (at(j + n - 1), at(j + 1))
        } else {
            (pts[j.saturating_sub(1)], pts[(j + 1).min(n - 1)])
        };
        let d = (next - prev).normalize_or_zero();
        vec2(-d.y, d.x)
    };
    let clip = |p: Vec2, e: f32| GenVertex { pos: (p * fit).to_array(), color, edge: e };
    let segments = if closed { n } else { n - 1 };
    for j in 0..segments {
        let (a, b) = (at(j), at(j + 1));
        let (na, nb) = (normal(j) * half, normal((j + 1) % n) * half);
        let (a0, a1, b0, b1) =
            (clip(a - na, -1.0), clip(a + na, 1.0), clip(b - nb, -1.0), clip(b + nb, 1.0));
        out.extend_from_slice(&[a0, a1, b1, a0, b1, b0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(level: f64) -> AnalysisFrame {
        AnalysisFrame { bands: [level; 7], rms: level, ..Default::default() }
    }

    #[test]
    fn a_seed_gives_the_same_piece() {
        let k = GenKnobs::default();
        let (mut a, mut b) = (Generative::new(3), Generative::new(3));
        let (mut va, mut vb) = (Vec::new(), Vec::new());
        for i in 0..120 {
            let f = frame(if i % 10 < 5 { 0.8 } else { 0.1 });
            a.update(&f, 1.0 / 60.0, i % 30 == 0, &k);
            b.update(&f, 1.0 / 60.0, i % 30 == 0, &k);
        }
        a.build(1080.0, 1920.0, &k, &mut va);
        b.build(1080.0, 1920.0, &k, &mut vb);
        assert_eq!(bytemuck::cast_slice::<_, u8>(&va), bytemuck::cast_slice::<_, u8>(&vb));
    }

    #[test]
    fn a_louder_band_swells_its_ring() {
        let k = GenKnobs::default();
        let mut quiet = Generative::new(1);
        let mut loud = Generative::new(1);
        let mut f = frame(0.0);
        for _ in 0..30 {
            quiet.update(&f, 1.0 / 60.0, false, &k);
        }
        f.bands[0] = 1.0;
        for _ in 0..30 {
            loud.update(&f, 1.0 / 60.0, false, &k);
        }
        let n = Generative::layer_count(&k);
        let extent = |g: &Generative| {
            let mut pts = Vec::new();
            g.ring(0, n, &mut pts);
            pts.iter().map(|p| p.length()).fold(0.0f32, f32::max)
        };
        assert!(extent(&loud) > extent(&quiet) * 1.2, "bass did not swell the inner ring");
    }

    #[test]
    fn everything_stays_inside_the_frame() {
        let k = GenKnobs { complexity: 1.0, ..GenKnobs::default() };
        let mut g = Generative::new(9);
        let mut out = Vec::new();
        for i in 0..600 {
            g.update(&frame(1.0), 1.0 / 60.0, i % 15 == 0, &k);
        }
        g.build(1920.0, 1080.0, &k, &mut out);
        assert!(!out.is_empty());
        for v in &out {
            assert!(v.pos.iter().all(|c| c.is_finite() && c.abs() < 1.6), "{:?}", v.pos);
            assert!(v.color.iter().all(|c| c.is_finite()));
        }
    }

    #[test]
    fn beats_throw_polygons_that_fade() {
        let k = GenKnobs::default();
        let mut g = Generative::new(2);
        let f = AnalysisFrame { beat_strength: 1.0, ..frame(0.3) };
        g.update(&f, 1.0 / 60.0, true, &k);
        assert_eq!(g.waves.len(), 1);
        for _ in 0..240 {
            g.update(&f, 1.0 / 60.0, false, &k);
        }
        assert!(g.waves.is_empty(), "the polygon never faded");
    }

    #[test]
    fn a_new_bar_mutates_the_structure() {
        let k = GenKnobs { evolve: 1.0, ..GenKnobs::default() };
        let mut g = Generative::new(4);
        let before: Vec<Shape> = g.layers.iter().map(|l| l.target).collect();
        let mut f = frame(0.5);
        f.bar_phase = 0.95;
        g.update(&f, 1.0 / 60.0, false, &k);
        f.bar_phase = 0.02;
        g.update(&f, 1.0 / 60.0, false, &k);
        let after: Vec<Shape> = g.layers.iter().map(|l| l.target).collect();
        assert_ne!(before, after);
    }
}
