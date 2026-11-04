//! Dark concentric rings behind the head, ported from
//! `visualizer/viz/brainwaves.py`.
//!
//! Rings are born on a transient, expand outward and fade -- wavy rather
//! than perfectly round, so they read like the rings of an EEG plot rather
//! than sonar pings. They are drawn in a dark grey just above the
//! background, so they thicken the air behind the subject instead of
//! competing with it.
//!
//! The Python rides them on the treble. Here they ride the **lows**, which
//! is what the music's weight is actually doing and reads as the head
//! breathing rather than twitching.

use std::f32::consts::TAU;

/// Points around one ring.
pub const SEGMENTS: usize = 96;
pub const MAX_RINGS: usize = 6;

#[derive(Debug, Clone, Copy)]
struct Ring {
    radius: f32,
    speed: f32,
    life: f32,
    ttl: f32,
    amp: f32,
    lobes: f32,
    phase: f32,
    wobble: f32,
    /// How hard it was struck, which decides whether it carries grains.
    strength: f32,
}

/// One vertex of a ring, as the line pipeline reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RingVertex {
    pub pos: [f32; 3],
    pub alpha: f32,
}

/// Spawning, growth and fading.
pub struct RingField {
    rings: Vec<Ring>,
    t: f32,
    /// Smoothed band energy, for the spawn decision.
    env: f32,
    since: f32,
    rng: u32,
    /// The song's intensity: rings slow and fade through a breakdown.
    presence: f32,
}

impl Default for RingField {
    fn default() -> Self {
        Self { rings: Vec::new(), t: 0.0, env: 0.0, since: 10.0, rng: 12, presence: 1.0 }
    }
}

impl RingField {
    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    /// Advance the field; a step up in the lows starts a new ring.
    pub fn update(&mut self, level: f32, dt: f32, gain: f32) {
        self.t += dt;
        self.since += dt;
        let prev = self.env;
        self.env += (level - self.env) * (dt * 8.0).min(1.0);
        let rise = self.env - prev;
        // a step up, not merely a loud passage: otherwise a sustained bass
        // note spawns a ring every frame and they stack into a blur
        if self.env > 0.12 && rise > 0.004 && self.since > 0.18 && gain > 0.0 {
            self.spawn(self.env);
        }
        let pace = 0.6 + 0.4 * self.presence;
        for r in &mut self.rings {
            r.radius += dt * r.speed * pace;
            r.life -= dt / r.ttl;
        }
        self.rings.retain(|r| r.life > 0.0);
    }

    /// How present the rings are, 0.55..1, from the song's intensity.
    pub fn set_presence(&mut self, presence: f32) {
        self.presence = presence.clamp(0.0, 1.0);
    }

    /// The drop: three rings at once, struck hard, a beat apart in radius.
    pub fn burst(&mut self) {
        for k in 0..3 {
            self.spawn(1.0);
            self.rings[0].radius = 0.15 - 0.06 * k as f32;
            self.rings[0].speed *= 1.0 + 0.15 * k as f32;
        }
    }

    /// The three newest rings as the head sees them, in world units:
    /// `(radius, fade, width, how much they carry)`. Only rings struck
    /// hard carry grains.
    pub fn rims(&self, scale: f32) -> [[f32; 4]; 3] {
        let mut out = [[0.0; 4]; 3];
        for (o, r) in out.iter_mut().zip(self.rings.iter().filter(|r| r.radius > 0.0)) {
            let fade = r.life.clamp(0.0, 1.0).powf(1.5) * self.presence;
            let carry = ((r.strength - 0.3) / 0.7).clamp(0.0, 1.0);
            *o = [r.radius * scale, fade, 0.10, carry];
        }
        out
    }

    fn spawn(&mut self, strength: f32) {
        let ring = Ring {
            strength,
            radius: 0.15,
            speed: 0.55 + 1.1 * strength,
            life: 1.0,
            ttl: 2.2 + 1.4 * self.rand(),
            // gently wavy, not lobed: they should read as circles
            amp: 0.015 + 0.045 * strength,
            lobes: 4.0 + (self.rand() * 6.0).floor(),
            phase: self.rand() * TAU,
            wobble: 0.7 + 1.1 * self.rand(),
        };
        self.rings.insert(0, ring);
        self.rings.truncate(MAX_RINGS);
        self.since = 0.0;
    }

    pub fn len(&self) -> usize {
        self.rings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    /// Ring vertices in the plane facing the camera (world X / Z), pushed
    /// `depth` behind the subject. One closed loop per ring.
    pub fn geometry(&self, centre: [f32; 3], depth: f32, scale: f32) -> Vec<RingVertex> {
        let mut out = Vec::with_capacity(self.rings.len() * (SEGMENTS + 1));
        self.geometry_into(&mut out, centre, depth, scale);
        out
    }

    /// [`geometry`](Self::geometry) into a buffer the caller keeps, so the
    /// render loop does not allocate a frame's worth of rings each frame.
    pub fn geometry_into(
        &self,
        out: &mut Vec<RingVertex>,
        centre: [f32; 3],
        depth: f32,
        scale: f32,
    ) {
        out.clear();
        for r in &self.rings {
            let fade = r.life.clamp(0.0, 1.0).powf(1.5);
            for i in 0..=SEGMENTS {
                let a = TAU * (i % SEGMENTS) as f32 / SEGMENTS as f32;
                let wob = r.amp * (r.lobes * a + r.phase + self.t * r.wobble).sin();
                let rad = r.radius * (1.0 + wob) * scale;
                out.push(RingVertex {
                    pos: [centre[0] + rad * a.cos(), centre[1] + depth, centre[2] + rad * a.sin()],
                    alpha: 0.5 * fade * self.presence,
                });
            }
        }
    }
}
