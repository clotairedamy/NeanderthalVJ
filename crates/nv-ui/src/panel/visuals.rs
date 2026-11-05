//! Visuals: which visualization runs, how it is coloured, and the panels
//! belonging to the modes that have their own settings.

use ads::ui::{AdsButton, AdsChoice, AdsField, ButtonVariant, ChoiceKind, FieldType, SegOption};
use dioxus::prelude::*;

use super::{Group, Hint, Slider};
use crate::stage::palette_stops;
use crate::state::{
    Mode, Settings, GRID_IMPACTS, MODES, PALETTES, PC_BAND_AXES, PC_EFFECTS, RING_BANDS,
};

fn options(values: &[&str]) -> Vec<SegOption> {
    values.iter().map(|v| SegOption::new(*v, *v)).collect()
}

#[component]
pub fn VisualsPanel(settings: Signal<Settings>) -> Element {
    let mode = settings.read().viz_mode;
    rsx! {
        Visualization { settings }
        Colors { settings }
        // the per-mode panels, shown only for the mode they belong to --
        // the Qt app hid the others rather than greying them out
        match mode {
            Mode::PointCloud | Mode::SynthCloud => rsx! { DepthPointCloud { settings } },
            Mode::HidingGrid => rsx! { HidingGridPanel { settings } },
            Mode::Generative => rsx! { GenerativePanel { settings } },
        }
    }
}

#[component]
fn Visualization(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    rsx! {
        Group { title: "Visualization",
            Hint {
                "Pick what the music draws. Number keys 1 to 4 switch while playing, \
                 and so do notes C to D# on a MIDI controller."
            }
            div { class: "nv-modes", role: "radiogroup", "aria-label": "Visualization",
                for (i, mode) in MODES.into_iter().enumerate() {
                    AdsChoice {
                        key: "{mode}",
                        kind: ChoiceKind::Radio,
                        name: "viz-mode",
                        value: "{i}",
                        checked: s.viz_mode == mode,
                        onchange: move |on| {
                            if on {
                                settings.write().viz_mode = mode;
                            }
                        },
                        "{i + 1}. {mode}"
                    }
                }
            }
            AdsChoice {
                checked: s.auto_switch_sections,
                onchange: move |on| settings.write().auto_switch_sections = on,
                "Auto-switch at song sections"
            }
            Hint { "Moves on to the next visualization by itself while the music plays." }
            if s.auto_switch_sections {
                AdsField {
                    label: "Switch every",
                    field_type: FieldType::Select,
                    value: s.auto_switch_bars.to_string(),
                    options: vec![
                        SegOption::new("4 bars", "4"),
                        SegOption::new("8 bars", "8"),
                        SegOption::new("16 bars", "16"),
                        SegOption::new("32 bars", "32"),
                    ],
                    hint: "A phrase is usually 8 or 16 bars; switching on its boundary lands with the music.",
                    oninput: move |e: FormEvent| {
                        if let Ok(v) = e.value().parse() {
                            settings.write().auto_switch_bars = v;
                        }
                    },
                }
                AdsChoice {
                    checked: s.auto_switch_drops,
                    onchange: move |on| settings.write().auto_switch_drops = on,
                    "Switch on drops too"
                }
                Hint { "When the energy jumps well above the last few seconds, as at a drop, switch at once." }
            }
        }
    }
}

/// A palette's colours as a strip, drawn as data: SVG fills, not chrome.
#[component]
fn Swatches(colors: Vec<[f32; 3]>) -> Element {
    let n = colors.len().max(1);
    rsx! {
        svg {
            class: "nv-swatches",
            view_box: "0 0 {n} 1",
            "preserveAspectRatio": "none",
            role: "img",
            "aria-label": "The palette's colours",
            for (i, c) in colors.iter().enumerate() {
                rect {
                    key: "{i}",
                    x: "{i}",
                    y: "0",
                    width: "1.02",
                    height: "1",
                    fill: "{nv_render::palette::to_hex(*c)}",
                }
            }
        }
    }
}

#[component]
fn Colors(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    let active = s.active_palette().to_string();
    let stops = palette_stops(&s);
    let mut error = use_context::<Signal<Option<String>>>();
    // from a task, never inside the click: see `engine_link::pick_and_load`
    let pick_picture = move |_| {
        spawn(async move {
            let Some(picked) = rfd::AsyncFileDialog::new()
                .add_filter("Pictures", &["jpg", "jpeg", "png"])
                .set_title("Take the colours from a picture")
                .pick_file()
                .await
            else {
                return;
            };
            match nv_render::palette::from_image(picked.path()) {
                Ok(colors) if !colors.is_empty() => {
                    let mut s = settings.write();
                    s.image_palette = colors;
                    s.set_palette("image");
                }
                Ok(_) => error.set(Some("That picture has no colours to take.".into())),
                Err(e) => error.set(Some(format!("Could not read the picture: {e}"))),
            }
        });
    };
    let label = if s.palette_per_mode {
        format!("Palette for {}", s.viz_mode)
    } else {
        "Palette".to_string()
    };
    rsx! {
        Group { title: "Colors",
            AdsField {
                label,
                field_type: FieldType::Select,
                value: active.clone(),
                options: options(&PALETTES),
                hint: "Colours every mode: the head's tint, the wall's lows, mids and highs, \
                       and the generative rings, low to high.",
                oninput: move |e: FormEvent| settings.write().set_palette(&e.value()),
            }
            Swatches { colors: stops }
            AdsChoice {
                checked: s.palette_per_mode,
                onchange: move |on| settings.write().palette_per_mode = on,
                "A palette per mode"
            }
            Hint { "Each mode keeps its own palette, so switching mode switches colours too." }
            Slider {
                label: "Tint strength",
                display: format!("{:.2}", s.tint_strength),
                value: s.tint_strength * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().tint_strength = v / 100.0,
            }
            Hint { "How much of the palette colours the head and the grid. 0 leaves them black and white." }
            div { class: "nv-row",
                AdsButton { variant: ButtonVariant::Quiet, onclick: pick_picture, "Colours from a picture…" }
            }
            Hint { "Takes the five colours a photo is mostly made of. You can also drop a picture on the window." }
            if active == "custom" {
                div { class: "nv-colors",
                    for (i, name) in ["Low", "Mid", "High"].into_iter().enumerate() {
                        label { class: "nv-color", key: "{name}",
                            span { class: "eyebrow", "{name}" }
                            input {
                                r#type: "color",
                                value: s.custom_palette[i].clone(),
                                oninput: move |e: FormEvent| settings.write().custom_palette[i] = e.value(),
                            }
                        }
                    }
                }
                Hint { "Your own palette: the bass end, the middle and the treble end." }
            }
        }
    }
}

/// The Head effect menu's wording for each effect.
fn effect_label(name: &str) -> &'static str {
    match name {
        "orbits" => "Orbits (nannou comets)",
        "halo" => "Halo (nannou superformula)",
        "vortex" => "Vortex",
        _ => "None",
    }
}

/// What the chosen head effect does, under the menu.
fn effect_hint(name: &str) -> &'static str {
    match name {
        "orbits" => {
            "Three comets circle the head on knotted paths built with nannou, re-shaped \
             every bar. The lows flare them, the mids speed them up, the beat throws them \
             wide and the highs make their tails sparkle. The face is never touched."
        }
        "halo" => {
            "A crown of light behind the head, drawn with nannou's superformula. It takes a \
             new symmetry every bar, grows points with the highs, swells with the bass, \
             turns with the mids, and each beat sends a ring out from it."
        }
        "vortex" => {
            "Three arms of particles spiral up around the head: faster with the mids, \
             wider with the bass, brighter on the beat."
        }
        _ => "The plain head: the rings, the crests of light on the beat and the dust.",
    }
}

#[component]
fn DepthPointCloud(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    let synth_mode = s.viz_mode == Mode::SynthCloud;
    rsx! {
        Group { title: "Depth Point Cloud",
            Hint { "The scanned head as 420,000 particles, lit by the music. Drag the stage to turn it, scroll to zoom." }
            if !synth_mode {
                AdsField {
                    label: "Head effect",
                    field_type: FieldType::Select,
                    value: s.pc_effect.clone(),
                    options: PC_EFFECTS.iter().map(|v| SegOption::new(effect_label(v), *v)).collect::<Vec<_>>(),
                    hint: effect_hint(&s.pc_effect),
                    oninput: move |e: FormEvent| settings.write().pc_effect = e.value(),
                }
                Slider {
                    label: "Effect amount",
                    display: format!("{:.2}", s.pc_effect_amount),
                    value: s.pc_effect_amount * 100.0,
                    min: 0.0,
                    max: 200.0,
                    onchange: move |v: f64| settings.write().pc_effect_amount = v / 100.0,
                }
                Hint { "How strongly the head effect plays. 0 turns it off; 2 is twice the usual." }
            }
            Slider {
                label: "Point size",
                display: format!("{:.1}", s.pc_point_size),
                value: s.pc_point_size * 10.0,
                min: 5.0,
                max: 60.0,
                onchange: move |v: f64| settings.write().pc_point_size = v / 10.0,
            }
            Hint { "How big each particle is drawn. Small reads as fine dust, large as a solid surface." }
            Slider {
                label: "Perspective",
                display: format!("{:.2}", s.pc_perspective),
                value: s.pc_perspective * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().pc_perspective = v / 100.0,
            }
            Hint { "The lens: 0 is a flat telephoto, 1 a wide angle held close. The head stays the same size." }
            Slider {
                label: "Reactivity",
                display: format!("{:.2}", s.pc_reactivity),
                value: s.pc_reactivity * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().pc_reactivity = v / 100.0,
            }
            Hint { "How hard the head moves with the music: its turns, nods and the oscillator's swing." }
            AdsField {
                label: "Band axis",
                field_type: FieldType::Select,
                value: s.pc_band_axis.clone(),
                options: options(&PC_BAND_AXES),
                hint: "Lays the spectrum across the head so each region lights with its own band: \
                       bass to treble from neck to crown, face to back, ear to ear, or centre out.",
                oninput: move |e: FormEvent| settings.write().pc_band_axis = e.value(),
            }
            Slider {
                label: "Band push",
                display: format!("{:.2}", s.pc_band_push),
                value: s.pc_band_push * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().pc_band_push = v / 100.0,
            }
            Hint { "Lifts each region off the surface as its band plays. 0 keeps the face whole and only lights it." }
            Slider {
                label: "Drum glitch",
                display: format!("{:.2}", s.pc_drum_glitch),
                value: s.pc_drum_glitch * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().pc_drum_glitch = v / 100.0,
            }
            Hint { "Each drum hit slices the head into offset slabs that snap back. 0 leaves it whole." }
            if synth_mode {
                Hint { "Synth Cloud always follows the synth: pulse width, LFO rate and how fast the tone morphs." }
            } else {
                AdsChoice {
                    checked: s.pc_synth,
                    onchange: move |on| settings.write().pc_synth = on,
                    "Drive the head from the synth's tone"
                }
                Hint { "Adds Synth Cloud's response, for a synth on the line in." }
            }
            AdsChoice {
                checked: s.pc_brainwaves,
                onchange: move |on| settings.write().pc_brainwaves = on,
                "Brainwave rings"
            }
            Hint { "Rings of light that open out behind the head and light its outline as they pass." }
            AdsField {
                label: "Rings answer",
                field_type: FieldType::Select,
                value: s.pc_ring_band.clone(),
                options: options(&RING_BANDS),
                hint: "Which part of the music starts a ring.",
                oninput: move |e: FormEvent| settings.write().pc_ring_band = e.value(),
            }
            Slider {
                label: "Mid reactivity",
                display: format!("{:.2}", s.pc_mid_gain),
                value: s.pc_mid_gain * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().pc_mid_gain = v / 100.0,
            }
            Hint { "How hard the mids push the cheeks and smear the grain." }
            Slider {
                label: "High reactivity",
                display: format!("{:.2}", s.pc_high_gain),
                value: s.pc_high_gain * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().pc_high_gain = v / 100.0,
            }
            Hint { "How hard the highs flake dust off the crown and glitter the silhouette." }
        }
    }
}

#[component]
fn HidingGridPanel(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    rsx! {
        Group { title: "Hiding Grid",
            Hint {
                "A wall the music hits. Kicks send shock waves across it that echo off \
                 the edges, the mids turn the squares, the highs strike sparks, and \
                 beats swell every square until some slide out of sight."
            }
            Slider {
                label: "Columns",
                display: format!("{}", s.grid_cols),
                value: s.grid_cols as f64,
                min: 8.0,
                max: 96.0,
                onchange: move |v: f64| settings.write().grid_cols = v as usize,
            }
            Hint { "How many squares across. More is finer and busier." }
            Slider {
                label: "Feature size",
                display: format!("{:.1}", s.grid_noise),
                value: s.grid_noise * 10.0,
                min: 10.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().grid_noise = v / 10.0,
            }
            Hint { "The flowing field's grain: low makes broad patches, high fine detail." }
            Slider {
                label: "Warp",
                display: format!("{:.2}", s.grid_warp),
                value: s.grid_warp * 100.0,
                min: 0.0,
                max: 250.0,
                onchange: move |v: f64| settings.write().grid_warp = v / 100.0,
            }
            Hint { "How far the lattice bends off-grid. The bass pushes it further." }
            Slider {
                label: "Flow",
                display: format!("{:.2}", s.grid_flow),
                value: s.grid_flow * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().grid_flow = v / 100.0,
            }
            Hint { "How fast the field drifts. Busier music, and the mids, move it faster." }
            Slider {
                label: "Beat swell",
                display: format!("{:.2}", s.grid_beat),
                value: s.grid_beat * 100.0,
                min: 0.0,
                max: 250.0,
                onchange: move |v: f64| settings.write().grid_beat = v / 100.0,
            }
            Hint { "How hard a beat resizes every square. Enough of it pushes some out of sight." }
            Slider {
                label: "Shock waves",
                display: format!("{:.2}", s.grid_waves),
                value: s.grid_waves * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().grid_waves = v / 100.0,
            }
            Hint { "How hard each kick strikes the wall. The wave bulges the lattice and lights the squares it crosses." }
            Slider {
                label: "Wave speed",
                display: format!("{:.2}", s.grid_wave_speed),
                value: s.grid_wave_speed * 100.0,
                min: 20.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().grid_wave_speed = v / 100.0,
            }
            Hint { "How fast a wave crosses the wall." }
            Slider {
                label: "Echo",
                display: format!("{:.2}", s.grid_echo),
                value: s.grid_echo * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().grid_echo = v / 100.0,
            }
            Hint { "How much of each wave bounces back off the edges, as sound does off a real wall." }
            AdsField {
                label: "Impact",
                field_type: FieldType::Select,
                value: s.grid_impact.clone(),
                options: options(&GRID_IMPACTS),
                hint: "Where the waves strike: dead centre like a drum skin, anywhere, rising from \
                       the floor, or from the left and right walls in turn.",
                oninput: move |e: FormEvent| settings.write().grid_impact = e.value(),
            }
            Slider {
                label: "Twist",
                display: format!("{:.2}", s.grid_twist),
                value: s.grid_twist * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().grid_twist = v / 100.0,
            }
            Hint { "How far the mids turn each square in its cell, each its own way." }
            Slider {
                label: "Sparks",
                display: format!("{:.2}", s.grid_sparkle),
                value: s.grid_sparkle * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().grid_sparkle = v / 100.0,
            }
            Hint { "How many cells the highs make flash. Hi-hats and cymbals set them off." }
            AdsChoice {
                checked: s.grid_analyser,
                onchange: move |on| settings.write().grid_analyser = on,
                "Spectrum wall"
            }
            Hint { "Each column lights up the wall to the level of its frequency, like a giant analyser." }
            Slider {
                label: "Tilt",
                display: format!("{:.2}", s.grid_tilt),
                value: s.grid_tilt * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().grid_tilt = v / 100.0,
            }
            Hint { "Leans the wall back in perspective, so the waves read as real height." }
            Slider {
                label: "Colour",
                display: format!("{:.2}", s.grid_color),
                value: s.grid_color * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().grid_color = v / 100.0,
            }
            Hint { "From the original black and white to the palette's colours for lows, mids and highs." }
        }
    }
}

#[component]
fn GenerativePanel(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    rsx! {
        Group { title: "Generative Geometry",
            Hint {
                "A ring for each band of the spectrum, bass at the centre. Each band \
                 swells and sharpens its own ring, beats throw polygons outward, and \
                 every bar the structure mutates."
            }
            Slider {
                label: "Symmetry",
                display: format!("{}", s.gen_symmetry),
                value: s.gen_symmetry as f64,
                min: 3.0,
                max: 12.0,
                onchange: move |v: f64| settings.write().gen_symmetry = v.round() as u32,
            }
            Hint { "How many times every shape repeats around the centre." }
            Slider {
                label: "Complexity",
                display: format!("{:.2}", s.gen_complexity),
                value: s.gen_complexity * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().gen_complexity = v / 100.0,
            }
            Hint { "From three rings to seven." }
            Slider {
                label: "Trails",
                display: format!("{:.2}", s.gen_trails),
                value: s.gen_trails * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().gen_trails = v / 100.0,
            }
            Hint { "How long each frame lingers, drifting outward, before it fades." }
            Slider {
                label: "Evolve",
                display: format!("{:.2}", s.gen_evolve),
                value: s.gen_evolve * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().gen_evolve = v / 100.0,
            }
            Hint { "How much of the structure changes at each bar, and how wild the new shapes are." }
            Slider {
                label: "Line width",
                display: format!("{:.2}", s.gen_width),
                value: s.gen_width * 100.0,
                min: 20.0,
                max: 400.0,
                onchange: move |v: f64| settings.write().gen_width = v / 100.0,
            }
            Hint { "Thin threads to bold ribbons. Each ring still thickens as its band plays." }
            Slider {
                label: "Spin",
                display: format!("{:.2}", s.gen_spin),
                value: s.gen_spin * 100.0,
                min: 0.0,
                max: 400.0,
                onchange: move |v: f64| settings.write().gen_spin = v / 100.0,
            }
            Hint { "How fast the rings turn. 0 holds them still." }
            Slider {
                label: "Flow",
                display: format!("{:.2}", s.gen_flow),
                value: s.gen_flow * 100.0,
                min: 0.0,
                max: 400.0,
                onchange: move |v: f64| settings.write().gen_flow = v / 100.0,
            }
            Hint { "How fast the trails stream outward and turn. High makes a tunnel." }
            Slider {
                label: "Glow",
                display: format!("{:.2}", s.gen_glow),
                value: s.gen_glow * 100.0,
                min: 30.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().gen_glow = v / 100.0,
            }
            Hint { "How luminous the lines are." }
            Slider {
                label: "Mid reactivity",
                display: format!("{:.2}", s.pc_mid_gain),
                value: s.pc_mid_gain * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().pc_mid_gain = v / 100.0,
            }
            Hint { "How hard the mids spin the rings and draw the web between them." }
            Slider {
                label: "High reactivity",
                display: format!("{:.2}", s.pc_high_gain),
                value: s.pc_high_gain * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().pc_high_gain = v / 100.0,
            }
            Hint { "How hard the highs make the outer rings shimmer." }
        }
    }
}
