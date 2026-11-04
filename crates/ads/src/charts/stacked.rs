//! Stacked bar — part of a whole, without reaching for a pie.

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

/// One segment of the whole.
#[derive(Clone, PartialEq)]
pub struct Segment {
    pub name: String,
    pub value: f32,
}

/// Part-to-whole, as a single horizontal bar.
///
/// The form the chart chooser names in place of a pie: angle is the hardest
/// encoding to compare, length is among the easiest. Three segments at most —
/// beyond that it is a pie lying down, and the rule is to show the percentage
/// instead.
///
/// Segments are separated by a surface-coloured gap rather than a stroke, so
/// adjacent fills never read as one block.
#[component]
pub fn AdsStackedBar(
    #[props(into)] segments: Vec<Segment>,
    #[props(into)] label: String,
) -> Element {
    let shown: Vec<Segment> = segments.into_iter().take(3).collect();
    let total: f32 = shown.iter().map(|s| s.value).sum();
    let total = if total <= 0.0 { 1.0 } else { total };

    let parts = shown
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let pct = s.value / total * 100.0;
            (
                i,
                s.name.clone(),
                format!("width:{pct}%"),
                format_number(f64::from(s.value)),
                format!("{pct:.0}%"),
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        figure { class: "ads-stacked", role: "figure", "aria-label": "{label}",
            div { class: "ads-stacked__track", "aria-hidden": "true",
                for (i, name, style, _, pct) in parts.iter().cloned() {
                    span {
                        key: "{i}",
                        class: "ads-stacked__part",
                        "data-part": "{i}",
                        style: "{style}",
                        title: "{name}: {pct}",
                    }
                }
            }
            ul { class: "ads-stacked__legend",
                for (i, name, _, value, pct) in parts.iter().cloned() {
                    li { key: "{i}", "data-part": "{i}",
                        span { class: "ads-dot", "aria-hidden": "true" }
                        span { class: "ads-stacked__name", "{name}" }
                        span { class: "num ads-stacked__value", "{value}" }
                        span { class: "num ads-stacked__pct", "{pct}" }
                    }
                }
            }
        }
    }
}
