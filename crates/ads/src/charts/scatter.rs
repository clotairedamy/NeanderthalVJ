//! Scatter — relationship between two measures.

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

/// One plotted series.
#[derive(Clone, PartialEq)]
pub struct ScatterSeries {
    pub name: String,
    /// Points in data space; both axes are scaled from the data.
    pub points: Vec<(f32, f32)>,
}

/// A relationship between two measures.
///
/// Capped at three series, and not by taste: scatter is an all-pairs form —
/// any two marks can land side by side — and only the first three slots of
/// the palette clear the all-pairs colour-separation test. A fourth series
/// folds into "Other" or becomes small multiples.
///
/// Marks are rings rather than discs so overlapping points stay countable.
#[component]
pub fn AdsScatter(
    #[props(into)] series: Vec<ScatterSeries>,
    #[props(into)] label: String,
    #[props(into)] x_label: String,
    #[props(into)] y_label: String,
) -> Element {
    let shown: Vec<ScatterSeries> = series.into_iter().take(3).collect();

    let all = shown.iter().flat_map(|s| s.points.iter().copied());
    let (mut x_lo, mut x_hi, mut y_lo, mut y_hi) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for (x, y) in all {
        x_lo = x_lo.min(x);
        x_hi = x_hi.max(x);
        y_lo = y_lo.min(y);
        y_hi = y_hi.max(y);
    }
    // A degenerate axis (every point identical) would divide by zero and put
    // every mark in one corner; widen it instead so the points stay visible.
    if (x_hi - x_lo).abs() < f32::EPSILON {
        x_lo -= 1.0;
        x_hi += 1.0;
    }
    if (y_hi - y_lo).abs() < f32::EPSILON {
        y_lo -= 1.0;
        y_hi += 1.0;
    }

    let plotted = shown
        .iter()
        .enumerate()
        .map(|(si, s)| {
            let marks = s
                .points
                .iter()
                .enumerate()
                .map(|(pi, (x, y))| {
                    let cx = (x - x_lo) / (x_hi - x_lo) * 100.0;
                    // SVG y grows downward; the data does not.
                    let cy = 100.0 - (y - y_lo) / (y_hi - y_lo) * 100.0;
                    (
                        pi,
                        cx,
                        cy,
                        format_number(f64::from(*x)),
                        format_number(f64::from(*y)),
                    )
                })
                .collect::<Vec<_>>();
            (si, s.name.clone(), marks)
        })
        .collect::<Vec<_>>();

    rsx! {
        figure { class: "ads-scatter", role: "figure", "aria-label": "{label}",
            svg {
                class: "ads-scatter__plot",
                view_box: "0 0 100 100",
                preserve_aspect_ratio: "none",
                "aria-hidden": "true",
                for (si, _, marks) in plotted.iter().cloned() {
                    g { key: "s{si}", class: "ads-scatter__series", "data-series": "{si}",
                        for (pi, cx, cy, xv, yv) in marks {
                            circle {
                                key: "p{pi}",
                                cx: "{cx}",
                                cy: "{cy}",
                                r: "1.8",
                                "vector-effect": "non-scaling-stroke",
                                title { "{xv}, {yv}" }
                            }
                        }
                    }
                }
            }
            figcaption { class: "ads-scatter__axes num",
                span { "{x_label}" }
                span { "{y_label}" }
            }
            // Identity is never colour alone: the legend names each series.
            ul { class: "ads-scatter__legend",
                for (si, name, _) in plotted.iter().cloned() {
                    li { key: "l{si}", "data-series": "{si}",
                        span { class: "ads-dot", "aria-hidden": "true" }
                        "{name}"
                    }
                }
            }
        }
    }
}
