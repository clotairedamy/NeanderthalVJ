//! Music to head movement, ported from `BustMotion` in `visualizer/viz/bust.py`.
//!
//! Beats throw the head into slices: each picks a fresh random band pattern
//! and the twist decays back, so the face snaps together between hits. The
//! first beat of the bar hits hardest and smears; bass keeps a hairline
//! twist on heavy passages. Highs flake dust off the crown and the
//! silhouette.
//!
//! Two things this gets wrong if written from memory rather than ported,
//! and both make the head look broken: the shake is driven **only** by the
//! noise-dissolve, never by beats, and the glitch decays exponentially
//! toward a floor set by the bass rather than being re-hit every frame.
//!
//! **The pose is a deliberate departure from the Python.** There the head
//! looked around through ±26° and an LFO swung it another ±49°; on a
//! synth line it whipped from side to side, and the movement drowned out
//! what it was answering. Here the head holds its three-quarter pose with
//! only a slow few degrees of drift, nods a couple of degrees on the beat,
//! and every turn goes through a critically damped spring, so nothing
//! snaps. What the music is doing is shown around the head instead -- the
//! rings lighting its outline as they pass, the scene dimming through a
//! breakdown and bursting on the drop, and for Synth Cloud a sheet of
//! light swept by the LFO -- which leaves the face whole.

use std::f32::consts::TAU;

use nv_audio::AnalysisFrame;

use crate::renderer::Uniforms;

/// The pose the head holds: turned three-quarters, chin raised.
pub const REST_YAW: f32 = -0.42;
pub const REST_PITCH: f32 = 0.20;

/// How quickly the pose follows its target, in rad/s. Critically damped,
/// so it settles in about `4 / POSE_OMEGA` seconds without overshooting.
const POSE_OMEGA: f32 = 11.0;

/// What the Visuals panel controls about the head.
#[derive(Debug, Clone, Copy)]
pub struct Knobs {
    /// How hard the head answers the music (`pc_reactivity`).
    pub reactivity: f32,
    /// How far a drum hit slices it (`pc_drum_glitch`).
    pub drum_glitch: f32,
    /// Sprite size (`pc_point_size`).
    pub point_size: f32,
    /// Scale of the beat's kick (`beat_impulse`).
    pub beat_impulse: f32,
    /// Drive the head from the synth's tone (`pc_synth`).
    pub synth: bool,
    /// Rings behind the head (`pc_brainwaves`).
    pub rings: bool,
    /// How hard the mids drive the visuals.
    pub mid_gain: f32,
    /// How hard the highs drive them.
    pub high_gain: f32,
    /// Tint colour and how much of it to mix into the grey.
    pub tint: [f32; 3],
    pub tint_mix: f32,
    /// Which band the rings answer: 0 lows, 1 mids, 2 highs.
    pub ring_band: u8,
    /// The Depth Point Cloud's head effect, and how strongly it plays
    /// (`pc_effect`, `pc_effect_amount`). None for Synth Cloud.
    pub effect: crate::head_fx::HeadEffect,
    pub effect_amount: f32,
    /// How far the camera lunges in on each beat (`pc_camera_punch`).
    pub punch: f32,
    /// How hard a drum hit flashes the head into lit facets
    /// (`pc_facets`), apart from the Drum glitch's slices. 0 for none.
    pub facets: f32,
    /// The Depth Point Cloud's look. False for Synth Cloud, which renders
    /// as it always did: its own light, streaks, rings and shell.
    pub depth_cloud: bool,
}

impl Default for Knobs {
    fn default() -> Self {
        Self {
            reactivity: 1.0,
            // Off, matching the app's own default: the slab twist cuts the
            // face apart, and anything built from defaults -- the exporter
            // included -- would otherwise quietly bring it back. The parity
            // test asks for the Python's 1.0 explicitly.
            drum_glitch: 0.0,
            point_size: 2.2,
            beat_impulse: 1.0,
            synth: false,
            rings: true,
            mid_gain: 1.0,
            high_gain: 1.0,
            tint: [1.0, 1.0, 1.0],
            tint_mix: 0.0,
            ring_band: 0,
            effect: crate::head_fx::HeadEffect::None,
            effect_amount: 1.0,
            punch: 0.0,
            facets: 0.0,
            depth_cloud: true,
        }
    }
}

pub struct BustMotion {
    /// The head's own clock: it stops in silence.
    t: f32,
    /// Real seconds, for cooldowns that must not stop with it.
    wall: f32,
    /// 0 in silence, 1 with sound in the room.
    alive: f32,
    glitch: f32,
    smear: f32,
    dust: f32,
    seed: f32,
    bands: f32,
    nod: f32,
    swell: f32,
    /// 0..1 noise-dissolve envelope, and its own clock.
    dissipate: f32,
    diss_t: f32,
    /// How far the noise throws the head open.
    explode: f32,
    shake: [f32; 3],
    shake_v: [f32; 3],
    /// Smoothed synth features.
    pwm: f32,
    wob: f32,
    morph: f32,
    lfo_phase: f32,
    /// Smoothed drum energy, and the cooldown on throwing a ghost.
    drum: f32,
    last_ghost: f32,
    rng: u32,
    spare_normal: Option<f32>,
    /// Ripples in flight: `(radius, strength)`, oldest last.
    waves: Vec<(f32, f32)>,
    /// The pose, and how fast it is moving: `(yaw, pitch)`.
    pose: [f32; 2],
    pose_v: [f32; 2],
    /// The song's shape: loudness over half a second and over eight, the
    /// intensity they imply, how long it has been quiet, and the drop.
    fast: f32,
    slow: f32,
    intensity: f32,
    low_for: f32,
    since_drop: f32,
    drop: bool,
    flash: f32,
    /// Synth Cloud's sheet: its phase, the shimmer's, and the smoothed
    /// resonance that sharpens it.
    sheet_phase: f32,
    shim_phase: f32,
    reso: f32,
    /// The facet flash: thrown by drum hits, falling away fast.
    facet: f32,
}

impl Default for BustMotion {
    fn default() -> Self {
        Self {
            t: 0.0,
            wall: 0.0,
            alive: 0.0,
            glitch: 0.0,
            smear: 0.0,
            dust: 0.0,
            seed: 0.0,
            bands: 9.0,
            nod: 0.0,
            swell: 0.0,
            dissipate: 0.0,
            diss_t: 0.0,
            explode: 0.0,
            shake: [0.0; 3],
            shake_v: [0.0; 3],
            pwm: 0.0,
            wob: 0.0,
            morph: 0.0,
            lfo_phase: 0.0,
            drum: 0.0,
            last_ghost: -99.0,
            rng: 5,
            spare_normal: None,
            waves: Vec::with_capacity(4),
            pose: [REST_YAW, REST_PITCH],
            pose_v: [0.0; 2],
            fast: 0.0,
            slow: 0.0,
            intensity: 1.0,
            low_for: 0.0,
            since_drop: 99.0,
            drop: false,
            flash: 0.0,
            sheet_phase: 0.0,
            shim_phase: 0.0,
            reso: 0.0,
            facet: 0.0,
        }
    }
}

/// One step of a critically damped spring toward `target`.
fn spring(x: &mut f32, v: &mut f32, target: f32, omega: f32, dt: f32) {
    let a = omega * omega * (target - *x) - 2.0 * omega * *v;
    *v += a * dt;
    *x += *v * dt;
}

/// `x += (target - x) * min(1, dt * rate)` — the Python's smoothing step.
fn approach(x: &mut f32, target: f32, dt: f32, rate: f32) {
    *x += (target - *x) * (dt * rate).min(1.0);
}

impl BustMotion {
    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    /// A standard normal, for the shake's damped random walk.
    fn normal(&mut self) -> f32 {
        if let Some(z) = self.spare_normal.take() {
            return z;
        }
        let (u1, u2) = (self.rand().max(1e-7), self.rand());
        let r = (-2.0 * u1.ln()).sqrt();
        self.spare_normal = Some(r * (TAU * u2).sin());
        r * (TAU * u2).cos()
    }

    /// Drum hits glitch the head. Read from the low-band transient, so it
    /// works with no stem separation.
    fn drums(&mut self, frame: &AnalysisFrame, dt: f32, beat: bool, k: Knobs) {
        let drums = frame.attack[..3].iter().fold(0.0f64, |a, b| a.max(*b)) as f32;
        let mut hit = (drums - self.drum).max(0.0);
        approach(&mut self.drum, drums, dt, 9.0);
        if beat {
            hit = hit.max(frame.beat_strength as f32 * 0.6);
        }
        // the cooldown runs on real time: the head's own clock stops in
        // silence, which swallowed the first hits of a track
        if hit > 0.12 && self.wall - self.last_ghost > 0.1 {
            self.last_ghost = self.wall;
            self.glitch = (self.glitch + 0.7 * hit * k.beat_impulse * k.drum_glitch).min(1.3);
            self.smear = (self.smear + 0.5 * hit * k.drum_glitch).min(1.0);
            self.facet = (self.facet + 1.4 * hit * k.facets).min(1.5);
            self.seed = self.rand() * 100.0;
        }
    }

    /// An oscillator's own movement: pulse width opens the slices, an LFO
    /// swings the head, a sweep smears the particles.
    fn synth_drive(&mut self, frame: &AnalysisFrame, dt: f32, drama: f32, k: Knobs) {
        approach(&mut self.pwm, frame.pwm as f32, dt, 5.0);
        approach(&mut self.wob, frame.wobble as f32, dt, 3.0);
        approach(&mut self.morph, frame.morph as f32, dt, 5.0);
        let hz = (frame.wobble_hz as f32).clamp(0.05, 6.0);
        self.lfo_phase = (self.lfo_phase + TAU * dt * hz) % TAU;
        // the slices are the Drum glitch control's, like the beat's: off
        // by default, so pulse width shows as smear rather than cuts
        self.glitch = self.glitch.max(0.55 * self.pwm * drama * k.drum_glitch);
        if self.pwm > 0.05 {
            self.bands = (4.0 + 16.0 * self.pwm).clamp(4.0, 20.0);
        }
        self.smear = self.smear.max(0.7 * self.morph.powf(1.5) * drama);
    }

    /// The song's shape. Constant reaction is soon ignored; a change in
    /// intensity is not. A stretch well under the song's running level --
    /// a breakdown, a build -- dims the scene and calms the rings, and the
    /// moment the music comes back in, the drop, is answered with a burst.
    fn dynamics(&mut self, frame: &AnalysisFrame, dt: f32) {
        let rms = frame.rms as f32;
        approach(&mut self.fast, rms, dt, 2.0); // about half a second
        approach(&mut self.slow, rms, dt, 0.12); // about eight
        let ratio = self.fast / (self.slow + 0.05);
        let target = (0.55 + 0.45 * (ratio - 0.4) / 0.45).clamp(0.55, 1.0);
        // falls slowly into a breakdown, comes back at once
        let rate = if target > self.intensity { 6.0 } else { 0.6 };
        approach(&mut self.intensity, target, dt, rate);

        self.since_drop += dt;
        self.drop = false;
        if ratio < 0.7 {
            self.low_for += dt;
        } else if self.low_for > 2.0 && ratio > 1.3 && self.fast > 0.3 && self.since_drop > 6.0 {
            self.drop = true;
            self.since_drop = 0.0;
            self.low_for = 0.0;
            self.flash = 1.0;
        } else {
            // a bump inside a breakdown should not cancel it
            self.low_for = (self.low_for - dt * 0.5).max(0.0);
        }
        self.flash *= (-dt * 2.5).exp();
    }

    /// The song's intensity: 1 at full strength, down to 0.55 in a lull.
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// The facet flash, 0..1.5: how much of the head is cut crystal now.
    pub fn facet(&self) -> f32 {
        self.facet
    }

    /// Whether the music came back in -- a drop -- on the last update.
    pub fn dropped(&self) -> bool {
        self.drop
    }

    /// Synth Cloud's sheet of light. It rides the LFO when there is one --
    /// that is the movement the ear is following -- and sweeps once a bar
    /// otherwise, so it still has a pulse on material with no modulation.
    fn sheet(&mut self, frame: &AnalysisFrame, dt: f32, drama: f32, u: &mut Uniforms) {
        approach(&mut self.reso, frame.resonance as f32, dt, 4.0);
        let hz = frame.wobble_hz as f32;
        let rate = if self.wob > 0.12 && hz > 0.05 {
            hz.min(6.0)
        } else {
            let bpm = if frame.bpm > 0.0 { frame.bpm as f32 } else { 100.0 };
            bpm / 60.0 / 4.0
        };
        self.sheet_phase = (self.sheet_phase + TAU * dt * rate) % TAU;
        self.shim_phase = (self.shim_phase + TAU * dt * (2.0 + 9.0 * self.reso)) % TAU;
        u.sheet = [
            // from the collarbones to the crown and back
            0.62 - 0.36 * self.sheet_phase.cos(),
            0.06 - 0.04 * self.reso,
            (0.45 + 0.75 * self.reso) * (0.35 + 0.65 * self.alive) * drama,
            self.shim_phase,
        ];
    }

    /// Advance by `dt` and write the pose into `u`.
    ///
    /// `beat` is a grid beat crossed since the last call.
    pub fn update(
        &mut self,
        frame: &AnalysisFrame,
        dt: f32,
        beat: bool,
        k: Knobs,
        u: &mut Uniforms,
    ) {
        let dt = dt.min(0.05);
        let drama = k.reactivity;

        // Silence should be still: the slow look around and the dust both
        // fade out with nothing to answer, rising fast so the head is
        // never late and falling slowly so it does not freeze between
        // phrases.
        let heard = (frame.rms as f32 * 4.0).clamp(0.0, 1.0);
        let rate = if heard > self.alive { 3.0 } else { 0.5 };
        approach(&mut self.alive, heard, dt, rate);
        self.wall += dt;
        self.t += dt * self.alive;

        let bass = frame.bands[..2].iter().sum::<f64>() as f32 / 2.0;
        // the mid and high gains are the user's hand on how much of the
        // music's top half reaches the visuals
        let mids = (frame.bands[2..5].iter().sum::<f64>() as f32 / 3.0 * k.mid_gain).min(1.5);
        let treble = (frame.bands[5..].iter().sum::<f64>() as f32 / 2.0 * k.high_gain).min(1.5);
        let rms = frame.rms as f32;

        // beats only: the low-end punch also fires on bass transients
        // between kicks, which kept the head permanently broken
        let mut hit = if beat { frame.beat_strength as f32 } else { 0.0 };
        if hit > 0.2 {
            hit = hit.max(0.6);
            // the one of the bar breaks the head apart with streaks; the
            // other beats twist a few slices
            let bar = frame.bar_phase as f32;
            let big = !(0.08..=0.96).contains(&bar);
            let kk = hit * k.beat_impulse;
            // the slab twist is scaled by the Drum glitch control, which
            // is off by default: a beat answers with the travelling crest
            // below instead of by cutting the face apart
            let slab = k.drum_glitch;
            self.glitch = (self.glitch + if big { 1.05 } else { 0.62 } * kk * slab).min(1.2);
            if big {
                self.smear = (self.smear + 0.8 * kk * slab).min(1.0);
            }
            self.seed = self.rand() * 100.0;
            self.bands = [5.0, 7.0, 9.0, 12.0, 16.0][(self.rand() * 5.0) as usize % 5];
            // the beat's real answer: a crest that travels up through the
            // head from the neck and settles behind itself
            self.waves.insert(0, (0.0, (0.35 + 0.5 * kk).min(1.0)));
            self.waves.truncate(3);
        }

        // Snap back to a whole head before the next beat. This is the part
        // that keeps it coherent: an exponential fall toward a floor the
        // bass sets, not a value re-hit every frame.
        let floor = 0.05 * bass;
        self.glitch = floor + (self.glitch - floor) * (-dt * 4.5).exp();
        self.smear *= (-dt * 5.0).exp();
        self.smear = self.smear.max(0.25 * mids * mids);
        approach(&mut self.dust, 0.02 + 0.23 * self.alive + 1.4 * treble + 0.6 * rms, dt, 3.0);

        // the ripples travel outward and fade; three at a time is plenty
        for w in &mut self.waves {
            w.0 += dt * 2.3;
            w.1 *= (-dt * 1.7).exp();
        }
        self.waves.retain(|w| w.1 > 0.002 && w.0 < 4.0);

        self.drums(frame, dt, beat, k);
        if beat {
            // as hard as the beat is
            self.nod = (0.4 + 0.6 * frame.beat_strength as f32).min(1.0);
        }
        self.nod *= (-dt * 6.0).exp();
        self.facet *= (-dt * 4.0).exp();

        if k.synth {
            self.synth_drive(frame, dt, drama, k);
        } else {
            self.pwm = 0.0;
            self.wob = 0.0;
            self.morph = 0.0;
        }
        let swing = self.wob * drama;

        // Noise dissolves the head, and the slicing steps back as it does:
        // there is no beat in a wash of noise to answer. A mix sits around
        // 0.3 flatness, a synth line 0.35, white noise 0.92 -- so the
        // window starts above anything with notes in it.
        let target = (((frame.flatness as f32) - 0.70) / 0.20).clamp(0.0, 1.0);
        approach(&mut self.dissipate, target, dt, 1.2);
        self.diss_t += dt * (0.22 + 0.30 * self.dissipate);
        let calm = 1.0 - 0.6 * self.dissipate;
        self.glitch *= calm;
        self.smear *= calm;

        // the explosion: noise throws the head open, louder noise further
        let want = 0.75 * self.dissipate * (0.35 + 0.95 * rms.clamp(0.0, 1.0));
        let rate = if want > self.explode { 3.5 } else { 1.4 };
        approach(&mut self.explode, want, dt, rate);
        // and shakes it -- a damped random walk, driven by the noise alone.
        // Shaking on every beat is what made it look like it was being
        // hit rather than played.
        // A third of the Python's: at 0.16 the whole bust jumped half a
        // unit about the stage, which read as the picture breaking.
        let amp = 0.05 * self.explode;
        for i in 0..3 {
            self.shake_v[i] += self.normal() * amp * (dt * 60.0).min(2.0);
            self.shake_v[i] *= 0.72;
            self.shake[i] = (self.shake[i] * 0.55 + self.shake_v[i]).clamp(-0.15, 0.15);
        }

        // The pose: held, with a few degrees of slow drift. The beat nods
        // it two degrees; a modulation sways it a few more in time. Every
        // part goes through the spring, so a beat eases the head down
        // rather than snapping it.
        let yaw_target = REST_YAW
            + 0.06 * (self.t * 0.31).sin()
            + 0.02 * (self.t * 0.73 + 1.3).sin()
            + 0.09 * swing * self.lfo_phase.sin();
        let pitch_target = REST_PITCH + 0.015 * (self.t * 0.23).sin() - 0.035 * self.nod * drama
            + 0.03 * swing * (self.lfo_phase * 0.5 + 0.7).sin();
        self.swell = 0.025 * swing * (self.lfo_phase + 1.2).sin();
        spring(&mut self.pose[0], &mut self.pose_v[0], yaw_target, POSE_OMEGA, dt);
        spring(&mut self.pose[1], &mut self.pose_v[1], pitch_target, POSE_OMEGA, dt);

        self.dynamics(frame, dt);
        // the rings' centre (meter[0..2]) is the scene's to set
        u.meter[2] = 1.0 - (1.0 - self.intensity) * drama.min(1.0);
        u.meter[3] = 0.6 * drama * self.flash;
        if k.synth {
            self.sheet(frame, dt, drama, u);
        } else {
            u.sheet = [0.0; 4];
        }

        u.yaw = self.pose[0];
        u.pitch = self.pose[1];
        u.low = bass;
        u.mid = mids;
        u.high = treble;
        u.glitch = self.glitch;
        u.smear = self.smear;
        u.dust = self.dust;
        u.seed = self.seed;
        u.bands = self.bands;
        u.time = self.t;
        u.swell = self.swell;
        u.explode = self.explode;
        u.dissipate = self.dissipate;
        u.diss_t = self.diss_t;
        u.shake = self.shake;
        u.waves = [[0.0; 4]; 3];
        for (i, w) in self.waves.iter().enumerate().take(3) {
            // a wide crest: a narrow one reads as a scanline, not a swell
            u.waves[i] = [w.0, w.1, 0.45, 0.0];
        }
        u.point = k.point_size;
        u.tint = k.tint;
        u.tint_mix = k.tint_mix;
        u.bright = (0.72 + 0.23 * self.alive) + 0.30 * rms;
    }
}
