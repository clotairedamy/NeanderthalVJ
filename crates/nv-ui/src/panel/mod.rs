//! The six panels of the control column.

pub mod export;
pub mod fx;
pub mod motion;
pub mod source;
pub mod visuals;

use dioxus::prelude::*;

/// A titled group of controls — the Qt `QGroupBox` this port replaces.
///
/// ADS has no "group box": depth is the only decoration, so a group is a
/// `.card` with an eyebrow, not a box with a border.
#[component]
pub fn Group(#[props(into)] title: String, children: Element) -> Element {
    rsx! {
        section { class: "card nv-group",
            h2 { class: "eyebrow nv-group__title", "{title}" }
            div { class: "nv-group__body", {children} }
        }
    }
}

/// A one-line explanation under a control, in the voice the Qt app used.
#[component]
pub fn Hint(children: Element) -> Element {
    rsx! { p { class: "dim nv-hint", {children} } }
}

/// A labelled row holding a value readout.
#[component]
pub fn ValueRow(#[props(into)] label: String, #[props(into)] value: String) -> Element {
    rsx! {
        div { class: "nv-value-row",
            span { class: "nv-value-row__label", "{label}" }
            span { class: "num nv-value-row__value", "{value}" }
        }
    }
}

/// A slider with one label and one value.
///
/// `AdsRange` draws its own label and raw number, which beside a formatted
/// readout gave every control two captions -- "Gain +0.0 dB" above
/// "GAIN 0". This keeps the kit's `.ads-range` track and supplies the head
/// once, formatted, with the label bound to the input for a screen reader.
#[component]
pub fn Slider(
    #[props(into)] label: String,
    #[props(into)] display: String,
    value: f64,
    min: f64,
    max: f64,
    #[props(default = 1.0)] step: f64,
    onchange: EventHandler<f64>,
) -> Element {
    let id = use_hook(|| {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("nv-slider-{}", NEXT.fetch_add(1, Ordering::Relaxed))
    });
    rsx! {
        div { class: "ads-field-group nv-slider",
            div { class: "nv-value-row",
                label { r#for: "{id}", class: "nv-value-row__label", "{label}" }
                span { class: "num nv-value-row__value", "{display}" }
            }
            input {
                id: "{id}",
                class: "ads-range",
                r#type: "range",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                value: "{value}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        onchange.call(v);
                    }
                },
            }
        }
    }
}
