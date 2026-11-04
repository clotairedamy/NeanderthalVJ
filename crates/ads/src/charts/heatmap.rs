//! Heatmap — a density grid, quantised into the four luminance grades.

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

/// 0 → the empty well; anything above zero lands on grade 1–4.
fn step(v: f64, top: f64) -> u8 {
    if v <= 0.0 || top <= 0.0 {
        return 0;
    }
    ((v / top) * 4.0).ceil().clamp(1.0, 4.0) as u8
}

/// A density grid. Magnitude is carried by the four luminance grades — the
/// channel the eye reads fastest, and the one dense data keeps when hue would
/// be noise: forty cells in three hues is a pattern, not a reading. Brighter is
/// more, in either theme, because the grade tokens invert with it.
///
/// The grid takes real values and quantises them itself into five steps: an
/// empty well for nothing, then grades one to four. The axes are recessive —
/// small, subdued, only as many labels as are given — and a compact legend
/// says which end is which. The accessible name carries the shape and range,
/// since a reader cannot inspect four hundred cells one by one.
#[component]
pub fn AdsHeatmap(
    #[props(into)] values: Vec<Vec<f64>>,
    #[props(into)] label: String,
    #[props(into, optional)] row_labels: Option<Vec<String>>,
    #[props(into, optional)] col_labels: Option<Vec<String>>,
    #[props(optional)] max: Option<f64>,
    #[props(into, optional)] unit: Option<String>,
) -> Element {
    let unit = unit.unwrap_or_default();
    let row_labels = row_labels.filter(|l| !l.is_empty());
    let col_labels = col_labels.filter(|l| !l.is_empty());
    let fmt = |n: f64| format!("{}{unit}", format_number(n));

    let flat = values.iter().flatten().copied().collect::<Vec<_>>();
    let top = max.unwrap_or_else(|| flat.iter().copied().fold(0.0, f64::max));
    let low = if flat.is_empty() {
        0.0
    } else {
        flat.iter().copied().fold(f64::INFINITY, f64::min)
    };

    // Every row shares one column template so the axis labels and the cells
    // line up; the row-label column is fixed rather than max-content for the
    // same reason.
    let n_cols = values.first().map_or(0, Vec::len);
    let columns = format!(
        "grid-template-columns:{}repeat({n_cols}, minmax(0, 1fr))",
        if row_labels.is_some() { "3.5rem " } else { "" }
    );

    let n_rows = values.len();
    let name = if n_rows == 0 || n_cols == 0 {
        format!("{label}: no data")
    } else {
        format!(
            "{label}: {n_rows} by {n_cols} grid, values {} to {}",
            fmt(low),
            fmt(top)
        )
    };

    let grid = values
        .iter()
        .enumerate()
        .map(|(y, row)| {
            let row_name = row_labels.as_ref().and_then(|l| l.get(y)).cloned();
            let cells = row
                .iter()
                .enumerate()
                .map(|(x, v)| {
                    let r = row_name.clone().unwrap_or_else(|| format!("Row {}", y + 1));
                    let c = col_labels
                        .as_ref()
                        .and_then(|l| l.get(x))
                        .cloned()
                        .unwrap_or_else(|| format!("column {}", x + 1));
                    (
                        format!("ads-heatmap__cell is-step-{}", step(*v, top)),
                        format!("{r}, {c}: {}", fmt(*v)),
                    )
                })
                .collect::<Vec<_>>();
            (row_name.unwrap_or_default(), cells)
        })
        .collect::<Vec<_>>();
    let (low_text, top_text) = (fmt(low), fmt(top));

    rsx! {
        figure { class: "ads-heatmap",
            div { class: "ads-heatmap__plot sweep", role: "img", "aria-label": "{name}",
                if let Some(cols) = &col_labels {
                    div { class: "ads-heatmap__row", style: "{columns}", "aria-hidden": "true",
                        if row_labels.is_some() { span { class: "ads-heatmap__axis" } }
                        for (x, c) in cols.iter().enumerate() {
                            span { key: "{x}", class: "ads-heatmap__axis ads-heatmap__axis-col", "{c}" }
                        }
                    }
                }
                for (y, (row_name, cells)) in grid.iter().enumerate() {
                    div { key: "{y}", class: "ads-heatmap__row", style: "{columns}",
                        if row_labels.is_some() {
                            span { class: "ads-heatmap__axis", "aria-hidden": "true", "{row_name}" }
                        }
                        for (x, (class, title)) in cells.iter().enumerate() {
                            span { key: "{x}", class: "{class}", title: "{title}" }
                        }
                    }
                }
            }
            figcaption { class: "ads-heatmap__legend", "aria-hidden": "true",
                span { class: "num", "{low_text}" }
                for s in 0..5u8 {
                    span { key: "{s}", class: "ads-heatmap__cell ads-heatmap__key is-step-{s}" }
                }
                span { class: "num", "{top_text}" }
            }
        }
    }
}
