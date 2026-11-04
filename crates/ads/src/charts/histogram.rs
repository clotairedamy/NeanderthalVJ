//! Histogram — the shape of a distribution, which an average hides.

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

/// A distribution, binned.
///
/// The chart chooser names this form for "distribution" and the kit could not
/// draw it until now. It exists because a median is a single point on a shape:
/// "median 139ms" and "p95 261ms" describe the same data as this does, and
/// neither tells you the tail is bimodal.
///
/// `bins` are counts, left to right across the range `0..max`. Height carries
/// the count and opacity carries it a second time, so the shape survives a
/// greyscale print and a reader who cannot separate the two ends.
#[component]
pub fn AdsHistogram(
    #[props(into)] bins: Vec<f32>,
    /// The upper edge of the last bin, in the unit named by `label`.
    #[props(into)]
    max: f32,
    #[props(into)] label: String,
) -> Element {
    let peak = bins.iter().copied().fold(1.0f32, f32::max);
    let n = bins.len().max(1);
    let columns = bins
        .iter()
        .enumerate()
        .map(|(i, count)| {
            // A floor of 2% so an empty-but-present bin is still a mark: a
            // gap in a distribution is information, and a zero-height bar
            // reads as "no bin here" rather than "nothing fell here".
            let h = (count / peak * 100.0).max(2.0);
            let o = 0.35 + count / peak * 0.65;
            let lo = max / n as f32 * i as f32;
            let hi = max / n as f32 * (i + 1) as f32;
            (
                i,
                format!("height:{h}%;opacity:{o}"),
                format_number(f64::from(*count)),
                format_number(f64::from(lo)),
                format_number(f64::from(hi)),
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        figure { class: "ads-histogram", role: "figure", "aria-label": "{label}",
            div { class: "ads-histogram__plot", "aria-hidden": "true",
                for (i, style, count, lo, hi) in columns.iter().cloned() {
                    span {
                        key: "{i}",
                        class: "ads-histogram__bin grow",
                        style: "{style}",
                        title: "{lo}–{hi}: {count}",
                    }
                }
            }
            figcaption { class: "ads-histogram__scale num",
                span { "0" }
                span { "{format_number(f64::from(max))}" }
            }
            // The table fallback: a canvas is invisible to a screen reader,
            // and analysts want the numbers anyway.
            table { class: "sr-only",
                caption { "{label}" }
                thead { tr { th { "Range" } th { "Count" } } }
                tbody {
                    for (i, _, count, lo, hi) in columns.iter().cloned() {
                        tr { key: "{i}", td { "{lo}–{hi}" } td { "{count}" } }
                    }
                }
            }
        }
    }
}
