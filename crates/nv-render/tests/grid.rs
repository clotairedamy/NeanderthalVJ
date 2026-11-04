//! The Hiding Grid's field, which is pure arithmetic and so can be checked
//! without a GPU.

use nv_audio::AnalysisFrame;
use nv_render::grid::{hiding_square, GridKnobs, HidingGrid, Impact, MAX_COLS};
use nv_render::noise::{perlin3, Permutation};

#[test]
fn the_noise_field_stays_in_range_and_is_smooth() {
    let perm = Permutation::new(11);
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for i in 0..20_000 {
        let t = i as f32 * 0.013;
        let v = perlin3(t, t * 0.7 + 3.0, t * 0.3, &perm);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    assert!(lo >= -1.0 && hi <= 1.0, "field left [-1, 1]: {lo}..{hi}");
    assert!(hi > 0.5 && lo < -0.5, "field barely moves: {lo}..{hi}");

    // neighbouring samples are close: a creased field would jump
    let mut worst: f32 = 0.0;
    let mut prev = perlin3(0.0, 0.0, 0.0, &perm);
    for i in 1..5_000 {
        let x = i as f32 * 0.002;
        let v = perlin3(x, 0.0, 0.0, &perm);
        worst = worst.max((v - prev).abs());
        prev = v;
    }
    assert!(worst < 0.05, "the field has a crease in it: step {worst}");
}

#[test]
fn a_square_slides_out_of_its_cell_and_disappears() {
    // s = 1 is a full cell
    let (lo, hi) = hiding_square(1.0);
    assert!(hi - lo > 1.0, "a full square is too small: {lo}..{hi}");
    // Within the drawing's own range of s the square is clipped but never
    // vanishes; it takes the beat swell driving s further to push one out
    // of sight, which is how the grid breathes holes.
    let (lo, hi) = hiding_square(-1.74);
    assert!(lo < hi, "a square inside the nominal range should still show");
    let (lo, hi) = hiding_square(-2.2);
    assert!(lo >= hi, "a swelled square should slide out of sight: {lo}..{hi}");
    // and it is clipped to the cell on the way
    for i in 0..60 {
        let s = 1.0 - i as f32 * 0.046;
        let (lo, hi) = hiding_square(s);
        assert!(lo >= -1.0 - 1e-6 && hi <= 1.0 + 1e-6, "escaped its cell at s={s}");
    }
}

fn frame(rms: f64, beat: bool) -> AnalysisFrame {
    AnalysisFrame {
        rms,
        bands: [0.6, 0.5, 0.4, 0.3, 0.3, 0.2, 0.2],
        spectrum: std::array::from_fn(|i| (i as f64 / 64.0).min(1.0)),
        beat,
        beat_strength: if beat { 0.8 } else { 0.0 },
        ..Default::default()
    }
}

#[test]
fn the_grid_builds_the_geometry_it_promises() {
    let mut g = HidingGrid::default();
    let k = GridKnobs::default();
    g.update(&frame(0.4, false), 1.0 / 60.0, false, k);

    assert_eq!(g.cols(), k.cols);
    let (nx, ny) = g.dims();
    assert!(nx >= k.cols && ny >= k.cols, "the wall should reach past the frame: {nx}x{ny}");
    assert_eq!(g.quads.len(), nx * ny * 4, "four vertices per cell");
    let (n, rows) = (nx + 1, ny + 1);
    let edges = rows * (n - 1) + (rows - 1) * n;
    assert_eq!(g.lines.len(), edges * 2, "two endpoints per lattice edge");
    assert!(g.quads.iter().all(|v| v.pos.iter().all(|c| c.is_finite())));
    assert!(g.lines.iter().all(|v| v.pos.iter().all(|c| c.is_finite())));
}

#[test]
fn beats_swell_the_squares() {
    let mut g = HidingGrid::default();
    let k = GridKnobs::default();
    let area = |g: &HidingGrid| -> f32 {
        g.quads
            .chunks(4)
            .map(|q| ((q[1].pos[0] - q[0].pos[0]) * (q[3].pos[1] - q[0].pos[1])).abs())
            .sum()
    };
    for _ in 0..30 {
        g.update(&frame(0.4, false), 1.0 / 60.0, false, k);
    }
    let quiet = area(&g);
    g.update(&frame(0.4, true), 1.0 / 60.0, true, k);
    for _ in 0..4 {
        g.update(&frame(0.4, false), 1.0 / 60.0, false, k);
    }
    assert!(area(&g) > quiet, "a beat did not swell the squares");
}

#[test]
fn the_column_count_is_a_live_control() {
    let mut g = HidingGrid::default();
    for cols in [4usize, 24, 40, MAX_COLS, 500] {
        let k = GridKnobs { cols, ..Default::default() };
        g.update(&frame(0.3, false), 1.0 / 60.0, false, k);
        let want = cols.clamp(4, MAX_COLS);
        assert_eq!(g.cols(), want, "asked for {cols} columns");
        let (nx, ny) = g.dims();
        assert_eq!(g.quads.len(), nx * ny * 4);
    }
}

fn brightness(g: &HidingGrid) -> f32 {
    g.quads.iter().map(|v| v.color.iter().sum::<f32>()).sum::<f32>()
}

#[test]
fn a_kick_sends_a_wave_across_the_wall_and_it_dies_away() {
    let mut g = HidingGrid::default();
    let k = GridKnobs { impact: Impact::Scattered, ..Default::default() };
    let mut kick = frame(0.5, false);
    kick.attack[0] = 0.9;
    g.update(&kick, 1.0 / 60.0, false, k);
    assert_eq!(g.waves_in_flight(), 1, "a kick did not strike the wall");
    // the same kick held over the next frames is one wave, not a train
    g.update(&kick, 1.0 / 60.0, false, k);
    assert_eq!(g.waves_in_flight(), 1);
    for _ in 0..600 {
        g.update(&frame(0.2, false), 1.0 / 60.0, false, k);
    }
    assert_eq!(g.waves_in_flight(), 0, "the wave never faded");
    // and with waves off, kicks leave the wall alone
    let mut still = HidingGrid::default();
    still.update(&kick, 1.0 / 60.0, false, GridKnobs { waves: 0.0, ..Default::default() });
    assert_eq!(still.waves_in_flight(), 0);
}

#[test]
fn a_wave_front_lights_the_wall_as_it_passes() {
    let k = GridKnobs::default();
    let mut struck = HidingGrid::default();
    let mut calm = HidingGrid::default();
    let mut kick = frame(0.4, false);
    kick.attack[0] = 1.0;
    struck.update(&kick, 1.0 / 60.0, false, k);
    calm.update(&frame(0.4, false), 1.0 / 60.0, false, k);
    // let the front travel out across the squares
    for _ in 0..20 {
        struck.update(&frame(0.4, false), 1.0 / 60.0, false, k);
        calm.update(&frame(0.4, false), 1.0 / 60.0, false, k);
    }
    assert!(brightness(&struck) > brightness(&calm) * 1.02, "the wave front left no light");
}

#[test]
fn bright_highs_strike_sparks() {
    let k = GridKnobs::default();
    let mut dull = HidingGrid::default();
    let mut bright = HidingGrid::default();
    let mut highs = frame(0.4, false);
    highs.bands[5] = 1.0;
    highs.bands[6] = 1.0;
    let mut quiet = frame(0.4, false);
    quiet.bands[5] = 0.0;
    quiet.bands[6] = 0.0;
    for _ in 0..30 {
        dull.update(&quiet, 1.0 / 60.0, false, k);
        bright.update(&highs, 1.0 / 60.0, false, k);
    }
    let hot = |g: &HidingGrid| g.quads.iter().filter(|v| v.color.iter().any(|c| *c > 1.6)).count();
    assert!(hot(&bright) > hot(&dull), "the highs struck no sparks");
}

#[test]
fn the_analyser_stands_the_spectrum_up_the_wall() {
    let plain = GridKnobs { color: 0.0, ..Default::default() };
    let analyser = GridKnobs { analyser: true, ..plain };
    let (mut a, mut b) = (HidingGrid::default(), HidingGrid::default());
    a.update(&frame(0.4, false), 1.0 / 60.0, false, plain);
    b.update(&frame(0.4, false), 1.0 / 60.0, false, analyser);
    assert!(brightness(&b) > brightness(&a), "the analyser lit nothing");
}

#[test]
fn the_wall_covers_the_whole_frame_whatever_its_shape_and_lean() {
    for aspect in [9.0 / 16.0, 1.0, 16.0 / 9.0] {
        for tilt in [0.0f32, 0.5, 1.0] {
            let mut g = HidingGrid::default();
            let k = GridKnobs { aspect, tilt, warp: 2.5, ..Default::default() };
            let mut f = frame(0.9, false);
            f.bands = [1.5; 7];
            for _ in 0..20 {
                g.update(&f, 1.0 / 60.0, false, k);
            }
            let (nx, ny) = g.dims();
            let (n, rows) = (nx + 1, ny + 1);
            // the horizontal segments come first, bottom row to top, then
            // the vertical ones, row by row
            let horizontal = |row: usize| &g.lines[row * (n - 1) * 2..(row + 1) * (n - 1) * 2];
            let vertical_start = rows * (n - 1) * 2;
            let column = |col: usize| {
                (0..rows - 1).flat_map(move |r| {
                    let i = vertical_start + (r * n + col) * 2;
                    [i, i + 1]
                })
            };
            let what = format!("aspect {aspect:.2}, tilt {tilt}");
            assert!(horizontal(0).iter().all(|v| v.pos[1] <= -1.0), "bottom edge shows ({what})");
            assert!(
                horizontal(rows - 1).iter().all(|v| v.pos[1] >= 1.0),
                "top edge shows ({what})"
            );
            assert!(
                column(0).all(|i| g.lines[i].pos[0] / aspect <= -1.0),
                "left edge shows ({what})"
            );
            assert!(
                column(n - 1).all(|i| g.lines[i].pos[0] / aspect >= 1.0),
                "right edge shows ({what})"
            );
        }
    }
}
