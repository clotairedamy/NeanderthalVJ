//! Horizontal bar chart — value maps to luminance as well as length.

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

/// A bar chart where value maps to luminance as well as height. With no hue to
/// spend, brightness is the second channel — the tallest bar is also the
/// brightest, so the ranking survives being glanced at.
///
/// Every bar carries its category and its figure as real text, so the rows are
/// their own table fallback; `label` names the whole figure ("Sessions by day,
/// thousands") and is where the unit lives, not in every row.
#[component]
pub fn AdsBars(
    #[props(into)] data: Vec<(String, f32)>,
    #[props(into)] label: String,
) -> Element {
    let max = data.iter().map(|(_, v)| *v).fold(1.0f32, f32::max);
    let rows = data
        .iter()
        .enumerate()
        .map(|(i, (name, value))| {
            let w = value / max * 100.0;
            let o = 0.35 + value / max * 0.65;
            let style = format!("width:{w}%;opacity:{o};--d:{}ms", i * 90);
            (name.clone(), format_number(f64::from(*value)), style)
        })
        .collect::<Vec<_>>();
    rsx! {
        div { class: "ads-bars", role: "figure", "aria-label": "{label}",
            for (name, value, style) in rows {
                div { key: "{name}", class: "ads-bars__row",
                    span { class: "ads-bars__label", "{name}" }
                    span { class: "ads-bars__track", "aria-hidden": "true",
                        span { class: "ads-bars__bar grow", style: "{style}" }
                    }
                    span { class: "num ads-bars__value", "{value}" }
                }
            }
        }
    }
}
