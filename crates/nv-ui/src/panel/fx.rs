//! FX: the effects over every mode, and how often the visuals are drawn.

use ads::ui::{AdsField, FieldType, SegOption};
use dioxus::prelude::*;

use super::{Group, Hint, Slider};
use crate::state::Settings;

#[component]
pub fn FxPanel(settings: Signal<Settings>) -> Element {
    let s = settings.read().clone();
    rsx! {
        Group { title: "Effects",
            Hint { "Over every mode, and in exports. With every effect at 0 the visuals skip this stage entirely." }
            Slider {
                label: "Bloom",
                display: format!("{:.2}", s.fx_bloom),
                value: s.fx_bloom * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().fx_bloom = v / 100.0,
            }
            Hint { "A soft glow around the brightest parts, lit further by each beat." }
            Slider {
                label: "Bloom threshold",
                display: format!("{:.2}", s.fx_threshold),
                value: s.fx_threshold * 100.0,
                min: 10.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().fx_threshold = v / 100.0,
            }
            Hint { "How bright a part has to be to glow. Lower makes everything bloom." }
            Slider {
                label: "Beat flash",
                display: format!("{:.2}", s.fx_flash),
                value: s.fx_flash * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().fx_flash = v / 100.0,
            }
            Hint { "Brightens the whole frame on every beat, harder on strong ones." }
            Slider {
                label: "Grain on drums",
                display: format!("{:.2}", s.fx_grain),
                value: s.fx_grain * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().fx_grain = v / 100.0,
            }
            Hint { "Film grain over the frame, thrown harder by each drum hit." }
            Slider {
                label: "Colour split",
                display: format!("{:.2}", s.fx_chroma),
                value: s.fx_chroma * 100.0,
                min: 0.0,
                max: 200.0,
                onchange: move |v: f64| settings.write().fx_chroma = v / 100.0,
            }
            Hint { "Pulls red and blue apart towards the edges on every kick, like a lens taking the hit." }
            Slider {
                label: "Ghost echo",
                display: format!("{:.2}", s.fx_ghost),
                value: s.fx_ghost * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().fx_ghost = v / 100.0,
            }
            Hint { "Each drum hit throws a fading, swelling echo of the frames before it." }
            Slider {
                label: "Vignette",
                display: format!("{:.2}", s.fx_vignette),
                value: s.fx_vignette * 100.0,
                min: 0.0,
                max: 100.0,
                onchange: move |v: f64| settings.write().fx_vignette = v / 100.0,
            }
            Hint { "Darkens the corners to hold the eye in the middle." }
        }
        Group { title: "Frame rate",
            AdsField {
                label: "Draw the visuals at",
                field_type: FieldType::Select,
                value: s.frame_rate.to_string(),
                options: vec![
                    SegOption::new("60 fps — recommended", "60"),
                    SegOption::new("120 fps — smoother on ProMotion", "120"),
                ],
                hint: "120 is smoother, but drawing that often under the interface makes \
                       the controls a little slower to answer.",
                oninput: move |e: FormEvent| {
                    if let Ok(v) = e.value().parse::<u32>() {
                        settings.write().frame_rate = v;
                    }
                },
            }
        }
    }
}
