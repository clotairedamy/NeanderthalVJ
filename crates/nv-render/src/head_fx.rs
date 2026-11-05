//! The Depth Point Cloud's head effects: what sets it apart from Synth
//! Cloud, which is the same head played by an oscillator.
//!
//! **The face is never touched.** Every effect happens around the head, and
//! is built from particles nobody sees: the ones on the far side of the
//! head, which the shell hides from the camera anyway. Those are lifted
//! off and flown about; the face keeps every one of its particles where
//! it is, lit as it was. Anything an effect would carry in front of the
//! face fades out before it covers it.
//!
//! - **Orbits** -- three comets fly knotted paths around the head, built on
//!   nannou's maths and randomness. Each is a 3D Lissajous figure whose
//!   frequency ratios are re-rolled every bar, so the paths keep changing
//!   shape, and each drags a curved tail along its own path. The lows
//!   flare them, the mids set their speed, the beat throws them wide and
//!   the highs make them sparkle.
//! - **Halo** -- three superformula rings, the shape family the Generative
//!   Geometry is built from, each in its own tilted plane, circling the
//!   head and precessing like a gyroscope. They take a new symmetry every
//!   bar, sharpen into points with the highs, swell with the bass, turn
//!   with the mids, and each beat sends a ring out from them.
//! - **Vortex** -- three arms of particles spiral up around the head,
//!   faster with the mids, wider with the bass, flashing on the beat.
//! - **Corona** -- solar flares erupt from the head's outline, behind it.
//!   Each flare's length is one band of the spectrum, highs at the crown
//!   and bass down the sides, so the corona is an analyser in fire; the
//!   kick makes every flare leap.
//! - **Supernova** -- every beat blasts a shell of light out from the head
//!   and past the edges of the frame, and a drop sets off a huge one.
//!
//! The paths are worked out here once a frame and handed to the particle
//! shader as uniforms (`Uniforms::fx`, `fx2`, `paths`); the shader places
//! each borrowed particle on them.

use std::f32::consts::TAU;

use nannou_core::glam::{vec3, Vec3};
use nannou_core::math::map_range;
use nannou_core::rand::rand::rngs::StdRng;
use nannou_core::rand::rand::{RngExt, SeedableRng};
use nv_audio::AnalysisFrame;

use crate::bust::BustModel;
use crate::renderer::Uniforms;

/// What the Depth Point Cloud does around its head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeadEffect {
    /// The plain head: rings, crests of light and dust only.
    #[default]
    None,
    Orbits,
    Halo,
    Vortex,
    Corona,
    Supernova,
}

impl HeadEffect {
    pub const ALL: [HeadEffect; 6] = [
        HeadEffect::None,
        HeadEffect::Orbits,
        HeadEffect::Halo,
        HeadEffect::Vortex,
        HeadEffect::Corona,
        HeadEffect::Supernova,
    ];

    /// The name settings and presets store it by.
    pub fn name(self) -> &'static str {
        match self {
            HeadEffect::None => "none",
            HeadEffect::Orbits => "orbits",
            HeadEffect::Halo => "halo",
            HeadEffect::Vortex => "vortex",
            HeadEffect::Corona => "corona",
            HeadEffect::Supernova => "supernova",
        }
    }

    /// The effect stored under `name`; anything unknown is no effect.
    pub fn from_name(name: &str) -> Self {
        Self::ALL.into_iter().find(|e| e.name() == name).unwrap_or_default()
    }

    /// What the shader switches on. 0 is no effect.
    fn id(self) -> f32 {
        Self::ALL.iter().position(|e| *e == self).unwrap_or(0) as f32
    }
}

/// How many comets fly at once. The shader reads exactly this many.
pub const COMETS: usize = 3;
/// How far back along its path a comet's tail reaches, in radians of its
/// own clock.
const TAIL: f32 = 0.9;
/// How far out the comets fly, in head radii: wide enough that they circle
/// the head rather than skim it.
const LIFT: f32 = 1.45;
/// Supernova shells in flight at once. The shader reads exactly this many.
pub const SHELLS: usize = 4;
/// How far a shell travels before it is gone, in head radii past the
/// surface: well past the edges of the frame.
const SHELL_REACH: f32 = 6.0;

/// The halo's symmetries, one picked each bar.
const SYMMETRIES: [f32; 6] = [3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

/// One comet: a 3D Lissajous figure around the head.
#[derive(Debug, Clone, Copy)]
struct Comet {
    /// Frequency ratios on x, y and z, eased toward `target` so a re-roll
    /// bends the path rather than teleporting the comet.
    ratio: Vec3,
    target: Vec3,
    phase: Vec3,
    /// Its own clock, which the mids speed up.
    angle: f32,
    /// A little faster or slower than the others, so they never line up.
    pace: f32,
}

/// The head's size and where its centre sits, measured from the model, so
/// the effects circle it whatever scan is loaded.
#[derive(Debug, Clone, Copy)]
pub struct HeadShape {
    pub centre: Vec3,
    /// Half-extents on x, y and z.
    pub radii: Vec3,
}

impl HeadShape {
    /// Measured from the particles above the neck.
    pub fn of(model: &BustModel) -> Self {
        let head: Vec<Vec3> =
            model.particles.iter().filter(|p| p.zone > 0.45).map(|p| Vec3::from(p.pos)).collect();
        if head.is_empty() {
            return Self::default();
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        for p in &head {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        Self { centre: (lo + hi) * 0.5, radii: ((hi - lo) * 0.5).max(Vec3::splat(0.1)) }
    }
}

impl Default for HeadShape {
    /// The scan at the app's usual 3.6 units from shoulders to crown.
    fn default() -> Self {
        Self { centre: vec3(0.0, 0.0, 0.85), radii: vec3(0.62, 0.72, 0.95) }
    }
}

/// `x += (target - x) * min(1, dt * rate)`.
fn approach(x: &mut f32, target: f32, dt: f32, rate: f32) {
    *x += (target - *x) * (dt * rate).min(1.0);
}

/// A comet's position on its path at `angle`, as the shader computes it.
pub fn comet_at(shape: &HeadShape, ratio: Vec3, phase: Vec3, angle: f32, lift: f32) -> Vec3 {
    let dir = vec3(
        (ratio.x * angle + phase.x).sin(),
        (ratio.y * angle + phase.y).sin(),
        (ratio.z * angle + phase.z).sin(),
    );
    // a Lissajous point can pass near the centre; the ellipsoid needs a
    // direction, so lean it upward rather than divide by zero
    let dir = (dir + vec3(0.0, 0.0, 1e-3)).normalize();
    shape.centre + dir * shape.radii * lift
}

pub struct HeadFx {
    shape: HeadShape,
    rng: StdRng,
    comets: [Comet; COMETS],
    t: f32,
    last_bar: f32,
    /// 0 in silence, 1 with sound in the room.
    alive: f32,
    lows: f32,
    mids: f32,
    highs: f32,
    /// The beat, falling away fast.
    kick: f32,
    /// The halo's symmetry before and after the last change, and how far
    /// it has blended into the new one; and how far it has turned.
    sym_from: f32,
    sym_to: f32,
    sym_blend: f32,
    spin: f32,
    /// The vortex: how far round it has turned, and how far it has risen.
    swirl: f32,
    rise: f32,
    /// Every band, smoothed: quick to flare, slower to die down.
    bands: [f32; 7],
    /// Supernova shells: `(radius past the surface, strength)`.
    shells: Vec<(f32, f32)>,
    /// A drop came in since the last update.
    drop: bool,
}

impl Default for HeadFx {
    fn default() -> Self {
        Self::new(HeadShape::default(), 11)
    }
}

impl HeadFx {
    pub fn new(shape: HeadShape, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let comets = std::array::from_fn(|i| {
            let ratio = Self::roll(&mut rng);
            Comet {
                ratio,
                target: ratio,
                phase: vec3(
                    rng.random_range(0.0..TAU),
                    rng.random_range(0.0..TAU),
                    rng.random_range(0.0..TAU),
                ),
                angle: i as f32 * TAU / COMETS as f32,
                pace: rng.random_range(0.8..1.25),
            }
        });
        Self {
            shape,
            rng,
            comets,
            t: 0.0,
            last_bar: 0.0,
            alive: 0.0,
            lows: 0.0,
            mids: 0.0,
            highs: 0.0,
            kick: 0.0,
            sym_from: 5.0,
            sym_to: 5.0,
            sym_blend: 1.0,
            spin: 0.0,
            swirl: 0.0,
            rise: 0.0,
            bands: [0.0; 7],
            shells: Vec::with_capacity(SHELLS),
            drop: false,
        }
    }

    /// New frequency ratios for a path: small whole numbers, a touch
    /// detuned, which is what makes a Lissajous figure a knot rather than
    /// a scribble -- and lets it drift slowly round instead of repeating.
    fn roll(rng: &mut StdRng) -> Vec3 {
        let mut pick = || rng.random_range(1..=4) as f32 + rng.random_range(-0.04..0.04);
        vec3(pick(), pick(), pick())
    }

    fn lift(&self) -> f32 {
        LIFT + 0.3 * self.kick
    }

    /// Where each comet is now, and where the end of its tail is.
    pub fn comets(&self) -> ([Vec3; COMETS], [Vec3; COMETS]) {
        let at = |i: usize, back: f32| {
            let c = &self.comets[i];
            comet_at(&self.shape, c.ratio, c.phase, c.angle - back, self.lift())
        };
        (std::array::from_fn(|i| at(i, 0.0)), std::array::from_fn(|i| at(i, TAIL)))
    }

    /// The beat, as it falls away: for the camera's punch.
    pub fn kick(&self) -> f32 {
        self.kick
    }

    /// The music came back in after a breakdown: the next update sets off
    /// the big supernova.
    pub fn drop_in(&mut self) {
        self.drop = true;
    }

    /// The supernova shells in flight: `(radius, strength)`.
    pub fn shells(&self) -> &[(f32, f32)] {
        &self.shells
    }

    /// The halo's symmetry right now.
    pub fn symmetry(&self) -> f32 {
        self.sym_from + (self.sym_to - self.sym_from) * self.sym_blend
    }

    /// Advance by `dt` and write the effect into `u`.
    ///
    /// `beat` is a grid beat crossed since the last call.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        frame: &AnalysisFrame,
        dt: f32,
        beat: bool,
        effect: HeadEffect,
        amount: f32,
        mid_gain: f32,
        high_gain: f32,
        u: &mut Uniforms,
    ) {
        let dt = dt.clamp(0.0, 0.05);
        self.t += dt;

        let lows = frame.bands[..2].iter().sum::<f64>() as f32 / 2.0;
        let mids = (frame.bands[2..5].iter().sum::<f64>() as f32 / 3.0 * mid_gain).min(1.5);
        let highs = (frame.bands[5..].iter().sum::<f64>() as f32 / 2.0 * high_gain).min(1.5);
        approach(&mut self.lows, lows, dt, 10.0);
        approach(&mut self.mids, mids, dt, 6.0);
        approach(&mut self.highs, highs, dt, 12.0);
        let heard = (frame.rms as f32 * 4.0).clamp(0.0, 1.0);
        let rate = if heard > self.alive { 3.0 } else { 0.5 };
        approach(&mut self.alive, heard, dt, rate);

        let bar = frame.bar_phase as f32;
        let new_bar = bar + 0.5 < self.last_bar;
        self.last_bar = bar;

        for (b, level) in self.bands.iter_mut().zip(frame.bands) {
            let target = (level as f32).min(1.5);
            let rate = if target > *b { 20.0 } else { 4.0 };
            approach(b, target, dt, rate);
        }

        if beat && frame.beat_strength > 0.2 {
            let hit = (frame.beat_strength as f32).max(0.6);
            self.kick = self.kick.max(hit);
            self.shell(0.55 + 0.6 * hit);
        }
        if std::mem::take(&mut self.drop) {
            // the drop: the biggest blast, and the flares leap with it
            self.shell(2.2);
            self.kick = 1.6;
        }
        self.kick *= (-dt * 5.0).exp();
        for sh in &mut self.shells {
            sh.0 += dt * (1.8 + 1.4 * sh.1);
            sh.1 *= (-dt * 1.1).exp();
        }
        self.shells.retain(|sh| sh.1 > 0.02 && sh.0 < SHELL_REACH);

        // every bar the comets' paths and the halo's symmetry change
        if new_bar {
            for c in &mut self.comets {
                c.target = Self::roll(&mut self.rng);
            }
            self.sym_from = self.symmetry();
            self.sym_to = SYMMETRIES[self.rng.random_range(0..SYMMETRIES.len())];
            self.sym_blend = 0.0;
        }
        self.sym_blend = (self.sym_blend + dt * 1.5).min(1.0);

        let live = 0.4 + 0.6 * self.alive;
        let speed = map_range(self.mids.min(1.5), 0.0, 1.5, 0.35, 1.9) * live;
        for c in &mut self.comets {
            c.ratio += (c.target - c.ratio) * (dt * 0.8).min(1.0);
            c.angle = (c.angle + dt * speed * c.pace) % (TAU * 100.0);
        }
        self.spin = (self.spin + dt * (0.08 + 0.7 * self.mids) * live) % TAU;
        self.swirl = (self.swirl + dt * (0.4 + 2.2 * self.mids) * live) % (TAU * 100.0);
        self.rise = (self.rise + dt * (0.04 + 0.12 * self.lows) * live) % 1.0;

        self.write(effect, amount, u);
    }

    /// Set off a supernova shell, the oldest giving way.
    fn shell(&mut self, strength: f32) {
        if self.shells.len() == SHELLS {
            self.shells.remove(0);
        }
        self.shells.push((0.0, strength));
    }

    fn write(&self, effect: HeadEffect, amount: f32, u: &mut Uniforms) {
        u.fx = [0.0; 4];
        u.fx2 = [0.0; 4];
        u.paths = [[0.0; 4]; 14];
        if effect == HeadEffect::None || amount <= 0.0 {
            return;
        }
        let amount = amount.min(2.0);
        u.fx = [effect.id(), amount, self.kick, self.t];
        u.fx2 = [self.lows, self.mids, self.highs.min(1.0), 0.4 + 0.6 * self.alive];
        for (i, c) in self.comets.iter().enumerate() {
            u.paths[i] = c.ratio.extend(c.angle).to_array();
            u.paths[3 + i] = c.phase.extend(0.0).to_array();
        }
        u.paths[6] = self.shape.centre.extend(self.lift()).to_array();
        u.paths[7] = self.shape.radii.extend(TAIL).to_array();
        u.paths[8] = [self.sym_from, self.sym_to, self.sym_blend, self.spin];
        u.paths[9] = [self.swirl, self.rise, 0.0, 0.0];
        for (i, sh) in self.shells.iter().enumerate() {
            u.paths[10 + i / 2][(i % 2) * 2] = sh.0;
            u.paths[10 + i / 2][(i % 2) * 2 + 1] = sh.1;
        }
        let b = self.bands;
        u.paths[12] = [b[0], b[1], b[2], b[3]];
        u.paths[13] = [b[4], b[5], b[6], 0.0];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(level: f64) -> AnalysisFrame {
        AnalysisFrame { bands: [level; 7], rms: level, beat_strength: 1.0, ..Default::default() }
    }

    fn run(fx: &mut HeadFx, f: &AnalysisFrame, effect: HeadEffect, frames: usize) -> Uniforms {
        let mut u = Uniforms::default();
        for i in 0..frames {
            fx.update(f, 1.0 / 60.0, i % 30 == 0, effect, 1.0, 1.0, 1.0, &mut u);
        }
        u
    }

    #[test]
    fn names_round_trip_and_unknown_is_none() {
        for e in HeadEffect::ALL {
            assert_eq!(HeadEffect::from_name(e.name()), e);
        }
        // the effects that broke the face are gone, and fall back to none
        assert_eq!(HeadEffect::from_name("shatter"), HeadEffect::None);
    }

    #[test]
    fn the_comets_circle_the_head_well_clear_of_it() {
        let shape = HeadShape::default();
        let mut fx = HeadFx::new(shape, 3);
        let mut u = Uniforms::default();
        for i in 0..2000 {
            fx.update(
                &frame(0.8),
                1.0 / 60.0,
                i % 30 == 0,
                HeadEffect::Orbits,
                1.0,
                1.0,
                1.0,
                &mut u,
            );
            let (heads, ends) = fx.comets();
            for p in heads.into_iter().chain(ends) {
                // on the ellipsoid itself this is 1
                let q = (p - shape.centre) / shape.radii;
                assert!(q.length() >= LIFT * 0.99, "a comet came too close: {q}");
            }
        }
    }

    #[test]
    fn the_comets_move_and_drag_a_tail() {
        let mut fx = HeadFx::default();
        let start = fx.comets().0;
        let u = run(&mut fx, &frame(0.6), HeadEffect::Orbits, 60);
        let (heads, ends) = fx.comets();
        for i in 0..COMETS {
            assert!(heads[i].distance(start[i]) > 0.05, "comet {i} did not move");
            assert!(heads[i].distance(ends[i]) > 0.05, "comet {i} has no tail");
        }
        assert_eq!(u.fx[0], 1.0, "the shader was not told to draw the comets");
    }

    #[test]
    fn the_mids_drive_the_comets_faster() {
        let travel = |level: f64| {
            let mut fx = HeadFx::default();
            let mut f = frame(level);
            f.rms = 0.6;
            run(&mut fx, &f, HeadEffect::Orbits, 120);
            fx.comets[0].angle
        };
        assert!(travel(1.0) > travel(0.0) * 1.5);
    }

    #[test]
    fn the_halo_changes_symmetry_on_the_bar() {
        let mut fx = HeadFx::default();
        let mut u = Uniforms::default();
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..2400 {
            let mut f = frame(0.5);
            // a bar every 120 frames
            f.bar_phase = (i % 120) as f64 / 120.0;
            fx.update(&f, 1.0 / 60.0, false, HeadEffect::Halo, 1.0, 1.0, 1.0, &mut u);
            seen.insert((fx.symmetry() * 10.0).round() as i32);
        }
        assert!(seen.len() > 3, "the halo kept one shape: {seen:?}");
        assert_eq!(u.fx[0], 2.0);
    }

    #[test]
    fn no_effect_and_no_amount_leave_the_head_alone() {
        let mut fx = HeadFx::default();
        let mut u = Uniforms::default();
        fx.update(&frame(1.0), 1.0 / 60.0, true, HeadEffect::None, 1.0, 1.0, 1.0, &mut u);
        assert_eq!(u.fx, [0.0; 4]);
        fx.update(&frame(1.0), 1.0 / 60.0, true, HeadEffect::Vortex, 0.0, 1.0, 1.0, &mut u);
        assert_eq!(u.fx, [0.0; 4]);
        assert_eq!(u.paths, [[0.0; 4]; 14]);
    }

    #[test]
    fn each_effect_tells_the_shader_which_it_is() {
        let ids: Vec<f32> = HeadEffect::ALL
            .into_iter()
            .skip(1)
            .map(|e| run(&mut HeadFx::default(), &frame(0.5), e, 1).fx[0])
            .collect();
        assert_eq!(ids, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn a_beat_sets_off_a_shell_that_flies_out_and_fades() {
        let mut fx = HeadFx::default();
        let mut u = Uniforms::default();
        fx.update(&frame(0.6), 1.0 / 60.0, true, HeadEffect::Supernova, 1.0, 1.0, 1.0, &mut u);
        assert_eq!(fx.shells().len(), 1);
        assert!(u.paths[10][1] > 0.5, "the shader was given no shell");
        for _ in 0..60 {
            fx.update(&frame(0.6), 1.0 / 60.0, false, HeadEffect::Supernova, 1.0, 1.0, 1.0, &mut u);
        }
        let (r, s) = fx.shells()[0];
        assert!(r > 1.5, "the shell did not fly out: {r}");
        assert!(s < 0.6, "the shell did not fade: {s}");
        for _ in 0..400 {
            fx.update(&frame(0.6), 1.0 / 60.0, false, HeadEffect::Supernova, 1.0, 1.0, 1.0, &mut u);
        }
        assert!(fx.shells().is_empty(), "a shell never left");
    }

    #[test]
    fn a_drop_sets_off_the_biggest_blast() {
        let mut fx = HeadFx::default();
        let mut u = Uniforms::default();
        fx.update(&frame(0.6), 1.0 / 60.0, true, HeadEffect::Supernova, 1.0, 1.0, 1.0, &mut u);
        let beat = fx.shells()[0].1;
        fx.drop_in();
        fx.update(&frame(0.6), 1.0 / 60.0, false, HeadEffect::Supernova, 1.0, 1.0, 1.0, &mut u);
        let drop = fx.shells().last().unwrap().1;
        assert!(drop > beat * 1.5, "the drop was no bigger than a beat");
        assert!(fx.kick() > 1.0);
    }

    #[test]
    fn the_corona_flares_with_each_band() {
        let mut fx = HeadFx::default();
        let mut f = frame(0.0);
        f.bands[6] = 1.0;
        let u = run(&mut fx, &f, HeadEffect::Corona, 30);
        assert!(u.paths[13][2] > 0.8, "the highs did not reach the corona");
        assert!(u.paths[12][0] < 0.05, "a silent band flared");
    }
}
