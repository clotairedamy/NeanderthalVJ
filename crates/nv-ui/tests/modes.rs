//! Which mode a key picks, and what each mode asks of the renderer.

use nv_ui::stage::params_from;
use nv_ui::state::{mode_for_key, Mode, Settings, MODES};

#[test]
fn number_keys_pick_the_modes_in_list_order() {
    for (i, mode) in MODES.iter().enumerate() {
        assert_eq!(mode_for_key(&(i + 1).to_string()), Some(*mode));
    }
    for not_a_mode in ["", "0", "a", "12", "-1", "F5", &(MODES.len() + 1).to_string()] {
        assert_eq!(mode_for_key(not_a_mode), None, "{not_a_mode:?}");
    }
}

#[test]
fn each_mode_asks_for_its_own_picture() {
    let with = |mode| params_from(&Settings { viz_mode: mode, ..Settings::default() });
    // the head answering loudness and bands
    assert!(!with(Mode::PointCloud).knobs.synth && with(Mode::PointCloud).grid.is_none());
    // the same head, played by the oscillator
    assert!(with(Mode::SynthCloud).knobs.synth && with(Mode::SynthCloud).grid.is_none());
    assert!(with(Mode::HidingGrid).grid.is_some());
    // only the generative mode asks for the generative picture
    let generative = with(Mode::Generative);
    assert!(generative.generative.is_some() && generative.grid.is_none());
    for mode in [Mode::PointCloud, Mode::SynthCloud, Mode::HidingGrid] {
        assert!(with(mode).generative.is_none(), "{mode} drew the generative picture");
    }
}

#[test]
fn the_generative_rings_wear_the_whole_palette() {
    let mut s = Settings { viz_mode: Mode::Generative, ..Settings::default() };
    s.set_palette("turbo");
    let colors = params_from(&s).generative.unwrap().colors;
    assert_eq!(colors[0], nv_render::palette::sample("turbo", 0.0));
    assert_eq!(colors[4], nv_render::palette::sample("turbo", 1.0));
}
