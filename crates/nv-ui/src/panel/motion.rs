//! Motion: how the analysis listens, how the visuals move, and the camera.

use ads::ui::AdsChoice;
use dioxus::prelude::*;

use super::{Group, Hint, Slider};
use crate::state::Settings;

#[component]
pub fn MotionPanel(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
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
        Group { title: "Motion",
            Slider {
                label: "Beat impulse",
                display: format!("{:.2}", s.beat_impulse),
                value: s.beat_impulse * 100.0,
                min: 0.0,
                max: 300.0,
                onchange: move |v: f64| settings.write().beat_impulse = v / 100.0,
            }
            Hint { "How hard each beat kicks: the head's nod and ripple, the grid's swell." }
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
