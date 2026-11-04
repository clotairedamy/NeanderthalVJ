//! The rings behind the head.

use nv_render::rings::{RingField, MAX_RINGS, SEGMENTS};

const DT: f32 = 1.0 / 60.0;

#[test]
fn a_rise_in_the_lows_starts_a_ring() {
    let mut f = RingField::default();
    // a sustained level, climbing: this is a kick arriving
    for i in 0..40 {
        f.update(0.05 + 0.02 * i as f32, DT, 1.0);
    }
    assert!(!f.is_empty(), "a rising low end started no rings");
}

#[test]
fn a_held_note_does_not_stack_rings_every_frame() {
    let mut f = RingField::default();
    // reach a level, then hold it: the envelope stops rising, so no more
    // rings -- otherwise a sustained bass note blurs into a solid disc
    for _ in 0..120 {
        f.update(0.6, DT, 1.0);
    }
    let after_rise = f.len();
    for _ in 0..240 {
        f.update(0.6, DT, 1.0);
    }
    assert!(f.len() <= after_rise, "a held note kept spawning rings");
    assert!(f.len() <= MAX_RINGS, "more rings than the cap");
}

#[test]
fn rings_expand_and_die() {
    let mut f = RingField::default();
    for i in 0..40 {
        f.update(0.05 + 0.02 * i as f32, DT, 1.0);
    }
    assert!(!f.is_empty());
    // silence for long enough that every ring has outlived its ttl
    for _ in 0..600 {
        f.update(0.0, DT, 1.0);
    }
    assert!(f.is_empty(), "rings never faded out");
}

#[test]
fn turning_them_off_spawns_nothing() {
    let mut f = RingField::default();
    for i in 0..200 {
        f.update(0.05 + 0.02 * (i % 40) as f32, DT, 0.0);
    }
    assert!(f.is_empty(), "rings appeared with the control off");
}

#[test]
fn geometry_is_closed_loops_behind_the_subject() {
    let mut f = RingField::default();
    for i in 0..40 {
        f.update(0.05 + 0.02 * i as f32, DT, 1.0);
    }
    let v = f.geometry([0.0, 0.0, 0.1], 1.9, 1.0);
    assert_eq!(v.len(), f.len() * (SEGMENTS + 1), "not one closed loop per ring");
    // the loop closes: last point matches the first
    let a = v[0].pos;
    let b = v[SEGMENTS].pos;
    for k in 0..3 {
        assert!((a[k] - b[k]).abs() < 1e-5, "the ring does not close");
    }
    // and they sit behind the head, not in front of it
    assert!(v.iter().all(|p| p.pos[1] > 1.0), "rings are not behind the subject");
}
