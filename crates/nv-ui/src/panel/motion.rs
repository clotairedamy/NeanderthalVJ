//! Motion: how the analysis listens, how the visuals move, and the camera.

use ads::ui::{AdsChoice, AdsField, FieldType, SegOption};
use dioxus::prelude::*;

use super::{Group, Hint, Slider};
use crate::state::{Mode, Settings, DIVISIONS};

/// What a beat does in each visualization, for the hints.
fn beat_does(mode: Mode) -> &'static str {
    match mode {
        Mode::PointCloud | Mode::SynthCloud => "the head's nod and the crest of light up it",
        Mode::HidingGrid => "the wall's swell and its shock waves",
        Mode::Generative => "the polygon thrown out from the centre",
    }
}

#[component]
pub fn MotionPanel(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    let mode = s.viz_mode;
    let r = s.reaction().clone();
    let head = matches!(mode, Mode::PointCloud | Mode::SynthCloud);
    rsx! {
        Group { title: "Listening",
            Slider {
                label: "Sensitivity",
                display: format!("{:.2}", s.sensitivity),
                value: s.sensitivity * 100.0,
                min: 10.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().sensitivity = v / 100.0,
            }
            Hint { "How loud the music has to be to fill the visuals. Raise it for quiet tracks or a soft live input." }
            Slider {
                label: "Smoothing",
                display: format!("{:.2}", s.smoothing),
                value: s.smoothing * 100.0,
                min: 0.0,
                max: 98.0,
                onchange: move |v: f64| settings.write().smoothing = v / 100.0,
            }
            Hint { "Higher is steadier; lower snaps to every transient." }
            Slider {
                label: "Lowest frequency",
                display: format!("{:.0} Hz", s.fft_min_hz),
                value: s.fft_min_hz,
                min: 10.0,
                max: 400.0,
                onchange: move |v: f64| settings.write().fft_min_hz = v,
            }
            Hint { "Below this nothing counts. Raise it to ignore rumble and stage hum." }
            Slider {
                label: "Highest frequency",
                display: format!("{:.0} Hz", s.fft_max_hz),
                value: s.fft_max_hz,
                min: 2000.0,
                max: 20000.0,
                step: 100.0,
                onchange: move |v: f64| settings.write().fft_max_hz = v,
            }
            Hint { "Above this nothing counts. Lower it to ignore hiss on a noisy line." }
        }
        Group { title: format!("How {mode} reacts"),
            Hint { "Each visualization keeps its own settings here, so the grid can wait for the peaks while the head follows every note." }
            AdsChoice {
                checked: r.auto_level,
                onchange: move |on| settings.write().reaction_mut().auto_level = on,
                "Auto level"
            }
            Hint { "Follows the song's loudest moments and scales to them, so a quiet master fills the picture as fully as a loud one." }
            Slider {
                label: "Dynamics",
                display: match r.dynamics {
                    d if d < -0.02 => format!("lift {:.0}%", -d * 100.0),
                    d if d > 0.02 => format!("peaks {:.0}%", d * 100.0),
                    _ => "as heard".to_string(),
                },
                value: r.dynamics * 100.0,
                min: -100.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().reaction_mut().dynamics = v / 100.0,
            }
            Hint { "Left lifts the quiet parts, so it is always moving; right keeps only the peaks, so it waits for the hits." }
            Slider {
                label: "Rise",
                display: format!("{:.0} ms", r.rise * 400.0),
                value: r.rise * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().reaction_mut().rise = v / 100.0,
            }
            Hint { "How long it takes to climb to a hit. 0 is instant; longer swells into each one." }
            Slider {
                label: "Fall",
                display: format!("{:.0} ms", r.fall * 1500.0),
                value: r.fall * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().reaction_mut().fall = v / 100.0,
            }
            Hint { "How long it holds on after a hit. 0 drops at once; longer lets each one glow and fade." }
            AdsField {
                label: "Hits on",
                field_type: FieldType::Select,
                value: r.hits.clone(),
                options: DIVISIONS
                    .iter()
                    .map(|v| {
                        let label = match *v {
                            "half" => "Half-time: beats 1 and 3",
                            "bar" => "Once a bar",
                            _ => "Every beat",
                        };
                        SegOption::new(label, *v)
                    })
                    .collect::<Vec<_>>(),
                hint: format!("Which beats drive {}. Half-time and once a bar calm a fast track down.", beat_does(mode)),
                oninput: move |e: FormEvent| settings.write().reaction_mut().hits = e.value(),
            }
        }
        Group { title: "Motion",
            Slider {
                label: "Beat impulse",
                display: format!("{:.2}", s.beat_impulse),
                value: s.beat_impulse * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().beat_impulse = v / 100.0,
            }
            Hint { "How hard each beat kicks: {beat_does(mode)}." }
            Slider {
                label: "Damping",
                display: format!("{:.2}", s.damping),
                value: s.damping * 100.0,
                min: 70.0,
                max: 95.0,
                onchange: move |v: f64| settings.write().damping = v / 100.0,
            }
            Hint { "How slowly the visuals let go of a hit. Low snaps back at once; high lets each hit ring out." }
            Slider {
                label: "A/V offset",
                display: format!("{:+.0} ms", s.av_offset_ms),
                value: s.av_offset_ms,
                min: -200.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().av_offset_ms = v,
            }
            Hint { "Delays the visuals against the sound. Bluetooth speakers run late; move this until the hits line up." }
        }
        if !head {
            Group { title: "Camera",
                Hint { "The camera turns the head only. The grid and the geometry fill the frame face-on." }
                AdsChoice {
                    checked: s.show_level,
                    onchange: move |on| settings.write().show_level = on,
                    "Show the level readout"
                }
                Hint { "The level the visuals are moving with, in the status bar above the stage." }
            }
        } else {
        Group { title: "Camera",
            AdsChoice {
                checked: s.auto_camera,
                onchange: move |on| settings.write().auto_camera = on,
                "Auto camera"
            }
            Hint { "Orbits the head slowly by itself. Dragging still turns it, and it carries on from there." }
            Slider {
                label: "Orbit speed",
                display: format!("{:.2}", s.auto_camera_speed),
                value: s.auto_camera_speed * 100.0,
                min: 5.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().auto_camera_speed = v / 100.0,
            }
            Hint { "How fast it goes round. Exports orbit at the same speed." }
            AdsChoice {
                checked: s.show_level,
                onchange: move |on| settings.write().show_level = on,
                "Show the level readout"
            }
            Hint { "The level the visuals are moving with, in the status bar above the stage." }
        }
        }
    }
}
