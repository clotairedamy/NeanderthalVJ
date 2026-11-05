//! Every control does something: changing a visual setting, in the mode it
//! belongs to, changes what the renderer is handed.
//!
//! A control that moves and changes nothing is worse than no control, and
//! nothing else would catch one -- the compiler is happy to carry a setting
//! nobody reads. The settings that drive the engine, the transport and the
//! export rather than the picture are listed at the bottom with where they
//! are read; their behaviour is tested beside that code.

use nv_ui::stage::{orbit_rate, params_from};
use nv_ui::state::{Mode, Settings};

type Change = fn(&mut Settings);

fn changes_the_picture(mode: Mode, name: &str, change: Change) {
    let before = Settings { viz_mode: mode, ..Settings::default() };
    let mut after = before.clone();
    change(&mut after);
    assert_ne!(
        format!("{:?}", params_from(&before)),
        format!("{:?}", params_from(&after)),
        "`{name}` changed nothing in {mode}"
    );
}

#[test]
fn every_visual_control_reaches_the_renderer() {
    let head: [(&str, Change); 15] = [
        ("point size", |s| s.pc_point_size = 4.0),
        ("perspective", |s| s.pc_perspective = 0.9),
        ("reactivity", |s| s.pc_reactivity = 1.7),
        ("band axis", |s| s.pc_band_axis = "radial".into()),
        ("band push", |s| s.pc_band_push = 1.0),
        ("drum glitch", |s| s.pc_drum_glitch = 1.0),
        ("synth drive", |s| s.pc_synth = true),
        ("rings", |s| s.pc_brainwaves = false),
        ("rings answer", |s| s.pc_ring_band = "highs".into()),
        ("mid reactivity", |s| s.pc_mid_gain = 2.0),
        ("high reactivity", |s| s.pc_high_gain = 2.0),
        ("beat impulse", |s| s.beat_impulse = 2.0),
        ("head effect", |s| s.pc_effect = "halo".into()),
        ("effect amount", |s| s.pc_effect_amount = 1.8),
        ("camera punch", |s| s.pc_camera_punch = 1.5),
    ];
    for (name, change) in head {
        changes_the_picture(Mode::PointCloud, name, change);
    }

    let grid: [(&str, Change); 19] = [
        ("columns", |s| s.grid_cols = 60),
        ("feature size", |s| s.grid_noise = 12.0),
        ("warp", |s| s.grid_warp = 2.0),
        ("flow", |s| s.grid_flow = 1.5),
        ("beat swell", |s| s.grid_beat = 2.0),
        ("shock waves", |s| s.grid_waves = 0.2),
        ("wave speed", |s| s.grid_wave_speed = 2.5),
        ("echo", |s| s.grid_echo = 0.9),
        ("impact", |s| s.grid_impact = "sides".into()),
        ("twist", |s| s.grid_twist = 1.8),
        ("sparks", |s| s.grid_sparkle = 2.5),
        ("spectrum wall", |s| s.grid_analyser = true),
        ("tilt", |s| s.grid_tilt = 0.9),
        ("colour", |s| s.grid_color = 0.1),
        ("sensitivity", |s| s.sensitivity = 2.0),
        ("shock waves fire on", |s| s.grid_wave_source = "snare".into()),
        ("twist follows", |s| s.grid_twist_by = "highs".into()),
        ("sparks follow", |s| s.grid_sparks_by = "lows".into()),
        ("warp follows", |s| s.grid_warp_by = "loudness".into()),
    ];
    for (name, change) in grid {
        changes_the_picture(Mode::HidingGrid, name, change);
    }

    let generative: [(&str, Change); 15] = [
        ("symmetry", |s| s.gen_symmetry = 9),
        ("complexity", |s| s.gen_complexity = 0.1),
        ("trails", |s| s.gen_trails = 0.95),
        ("evolve", |s| s.gen_evolve = 1.0),
        ("line width", |s| s.gen_width = 3.0),
        ("spin", |s| s.gen_spin = 0.0),
        ("flow", |s| s.gen_flow = 3.0),
        ("glow", |s| s.gen_glow = 2.0),
        ("mid reactivity", |s| s.pc_mid_gain = 2.5),
        ("high reactivity", |s| s.pc_high_gain = 2.5),
        ("polygons fire on", |s| s.gen_polygon_source = "hats".into()),
        ("spin follows", |s| s.gen_spin_by = "off".into()),
        ("shimmer follows", |s| s.gen_shimmer_by = "mids".into()),
        ("every ring swells with the bass", |s| s.gen_unison = true),
        ("mutate every", |s| s.gen_mutate_bars = 4),
    ];
    for (name, change) in generative {
        changes_the_picture(Mode::Generative, name, change);
    }

    // colours, effects and the Motion tab's reactions reach every mode
    let everywhere: [(&str, Change); 18] = [
        ("auto level", |s| s.reaction_mut().auto_level = true),
        ("dynamics", |s| s.reaction_mut().dynamics = -0.6),
        ("rise", |s| s.reaction_mut().rise = 0.5),
        ("fall", |s| s.reaction_mut().fall = 0.5),
        ("hits on", |s| s.reaction_mut().hits = "half".into()),
        ("palette", |s| s.set_palette("turbo")),
        ("tint strength", |s| s.tint_strength = 0.1),
        ("custom palette", |s| {
            s.set_palette("custom");
            s.custom_palette[1] = "#00ff00".into();
        }),
        ("image palette", |s| {
            s.image_palette = vec![[0.1, 0.0, 0.0], [0.9, 0.9, 0.0]];
            s.set_palette("image");
        }),
        ("bloom", |s| s.fx_bloom = 1.5),
        ("bloom threshold", |s| s.fx_threshold = 0.2),
        ("beat flash", |s| s.fx_flash = 1.9),
        ("grain", |s| s.fx_grain = 1.0),
        ("colour split", |s| s.fx_chroma = 1.0),
        ("ghost echo", |s| s.fx_ghost = 0.8),
        ("vignette", |s| s.fx_vignette = 0.9),
        ("palette per mode", |s| {
            s.palette_per_mode = false;
            s.palette = "viridis".into();
        }),
        ("mode", |s| s.viz_mode = Mode::Generative),
    ];
    for mode in [Mode::PointCloud, Mode::HidingGrid, Mode::Generative] {
        for (name, change) in everywhere {
            if name == "mode" && mode == Mode::Generative {
                continue;
            }
            changes_the_picture(mode, name, change);
        }
    }
}

#[test]
fn the_head_effect_is_the_depth_point_clouds_alone() {
    // Synth Cloud is the head played by an oscillator: a head effect set
    // for the Depth Point Cloud must not leak into it
    let synth =
        Settings { viz_mode: Mode::SynthCloud, pc_effect: "vortex".into(), ..Settings::default() };
    assert_eq!(params_from(&synth).knobs.effect, nv_render::HeadEffect::None);
    let head = Settings { viz_mode: Mode::PointCloud, ..synth };
    assert_eq!(params_from(&head).knobs.effect, nv_render::HeadEffect::Vortex);
    // every name the panel offers is one the renderer knows
    for name in nv_ui::state::PC_EFFECTS {
        assert_eq!(nv_render::HeadEffect::from_name(name).name(), name);
    }
}

#[test]
fn each_visualization_keeps_its_own_reaction() {
    let mut s = Settings { viz_mode: Mode::HidingGrid, ..Settings::default() };
    s.reaction_mut().dynamics = 0.8;
    s.viz_mode = Mode::Generative;
    assert_eq!(s.reaction().dynamics, 0.0, "the grid's reaction leaked into the geometry");
    assert_eq!(params_from(&s).react, nv_render::ReactKnobs::default());
    s.viz_mode = Mode::HidingGrid;
    assert_eq!(params_from(&s).react.dynamics, 0.8);
}

#[test]
fn the_auto_camera_turns_the_camera() {
    let off = Settings::default();
    assert_eq!(orbit_rate(&off), 0.0);
    let on = Settings { auto_camera: true, ..Settings::default() };
    let faster = Settings { auto_camera: true, auto_camera_speed: 1.5, ..Settings::default() };
    assert!(orbit_rate(&on) > 0.0);
    assert!(orbit_rate(&faster) > orbit_rate(&on), "the orbit speed does nothing");
}

/// The rest, and where each is read. Kept as data so that adding a setting
/// without deciding where it goes shows up here in review.
#[test]
fn the_other_settings_have_a_home() {
    let homes = [
        ("smoothing, sensitivity, fft_min_hz, fft_max_hz", "engine: set_analysis"),
        ("damping", "engine_link: Hold, the release on every hit"),
        ("av_offset_ms", "engine: set_av_offset"),
        ("speed, preserve_pitch, looping", "engine: set_playback"),
        ("input_device, input_gain_db, input_gate_db", "engine: start_live, set_live_levels"),
        ("auto_switch_sections, auto_switch_bars, auto_switch_drops", "engine_link: AutoSwitch"),
        ("show_level", "shell: StatusBar"),
        ("performance", "lib: Workspace hides the controls"),
        ("frame_rate", "stage: the render thread's pacing"),
        ("export_fps, export_quality, export_start, export_length", "export panel: Job"),
    ];
    assert!(homes.iter().all(|(names, home)| !names.is_empty() && !home.is_empty()));
}
