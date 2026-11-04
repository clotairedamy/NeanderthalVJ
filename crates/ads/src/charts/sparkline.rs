//! Sparkline — a hand-drawn line with a fading area, named for a screen reader.

use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

use crate::ui::count_up::format_number;

const W: f64 = 240.0;
const H: f64 = 64.0;

/// The smoothed line, the area under it, and the point coordinates.
fn path(values: &[f64], w: f64, h: f64) -> (String, String, Vec<(f64, f64)>) {
    if values.len() < 2 {
        return (String::new(), String::new(), Vec::new());
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = if max - min == 0.0 { 1.0 } else { max - min };
    let step = w / (values.len() - 1) as f64;
    // 4px of padding top and bottom so the stroke is never clipped.
    let coords = values
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64 * step, h - 4.0 - ((v - min) / span) * (h - 8.0)))
        .collect::<Vec<_>>();

    // A Catmull-Rom-ish smoothing: each segment gets control points a third of
    // the way toward its neighbours, which reads as a curve without overshoot.
    let mut line = format!("M{} {:.2}", coords[0].0, coords[0].1);
    for pair in coords.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let cx = (x0 + x1) / 2.0;
        line.push_str(&format!(
            " C{cx:.2} {y0:.2} {cx:.2} {y1:.2} {x1:.2} {y1:.2}"
        ));
    }
    let area = format!("{line} L{w} {h} L0 {h} Z");
    (line, area, coords)
}

/// A sparkline, painted by hand. The arithmetic is a pure function so it can be
/// reasoned about without a chart library's opinions.
///
/// `label` is the chart's name and is required: the SVG's accessible name is
/// built from it plus the first, last, lowest and highest values, which is what
/// a reader glancing at a sparkline takes from it anyway. The arrow keys (and
/// Home, End, Escape) inspect it point by point — a crosshair, the point on
/// the line and the value — and the readout is spoken. `point_labels` names
/// each point in the readout ("2026-03-14"); without it the point is `#n`.
///
/// The Vue kit also inspects under the pointer; that needs the element's
/// measured width, so here inspection is from the keyboard only.
#[component]
pub fn AdsSparkline(
    #[props(into)] points: Vec<f64>,
    #[props(into)] label: String,
    #[props(default = true)] fill: bool,
    #[props(into, optional)] unit: Option<String>,
    #[props(into, optional)] point_labels: Option<Vec<String>>,
) -> Element {
    // An id per instance for the area's fade gradient.
    let fade_id = use_hook(|| {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("ads-sparkline-{}", NEXT.fetch_add(1, Ordering::Relaxed))
    });
    let mut hover = use_signal(|| None::<usize>);

    let unit = unit.unwrap_or_default();
    let fmt = |n: f64| format!("{}{unit}", format_number(n));
    let (line, area, coords) = path(&points, W, H);

    let summary = match (points.first(), points.last()) {
        (Some(first), Some(last)) => {
            let low = points.iter().copied().fold(f64::INFINITY, f64::min);
            let high = points.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            format!(
                "{label}: {} points, from {} to {}, low {}, high {}",
                points.len(),
                fmt(*first),
                fmt(*last),
                fmt(low),
                fmt(high)
            )
        }
        _ => format!("{label}: no data"),
    };

    // (x %, y %, value, point label, alignment class)
    let readout = hover().and_then(|i| {
        let (cx, cy) = *coords.get(i)?;
        let name = point_labels
            .as_ref()
            .and_then(|l| l.get(i))
            .cloned()
            .unwrap_or_else(|| format!("#{}", i + 1));
        // Keep the bubble inside the chart at either end.
        let align = if cx / W < 0.2 {
            "is-start"
        } else if cx / W > 0.8 {
            "is-end"
        } else {
            ""
        };
        Some((cx / W * 100.0, cy / H * 100.0, fmt(points[i]), name, align))
    });
    let spoken = readout
        .as_ref()
        .map(|(_, _, v, n, _)| format!("{v}, {n}"))
        .unwrap_or_default();

    let len = points.len();
    let onkey = move |e: KeyboardEvent| {
        if len == 0 {
            return;
        }
        let last = len - 1;
        let i = hover().unwrap_or(last);
        let next = match e.key() {
            Key::ArrowLeft => Some(i.saturating_sub(1)),
            Key::ArrowRight => Some((i + 1).min(last)),
            Key::Home => Some(0),
            Key::End => Some(last),
            Key::Escape => None,
            _ => return,
        };
        e.prevent_default();
        hover.set(next);
    };

    rsx! {
        div {
            class: "ads-sparkline",
            role: "group",
            tabindex: "0",
            "aria-label": "{label}",
            onkeydown: onkey,
            onblur: move |_| hover.set(None),
            svg {
                view_box: "0 0 240 64",
                preserve_aspect_ratio: "none",
                class: "ads-sparkline__svg",
                role: "img",
                "aria-label": "{summary}",
                defs {
                    linearGradient { id: "{fade_id}", x1: "0", y1: "0", x2: "0", y2: "1",
                        stop { offset: "0%", stop_color: "currentColor", stop_opacity: "0.22" }
                        stop { offset: "100%", stop_color: "currentColor", stop_opacity: "0" }
                    }
                }
                if fill {
                    path { d: "{area}", fill: "url(#{fade_id})" }
                }
                path {
                    d: "{line}",
                    "pathLength": "1",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.8",
                    vector_effect: "non-scaling-stroke",
                    stroke_linecap: "round",
                }
            }
            if let Some((x, y, value, name, align)) = &readout {
                span { class: "ads-sparkline__cross", style: "left:{x}%" }
                span { class: "ads-sparkline__point", style: "left:{x}%;top:{y}%" }
                span {
                    class: crate::class::join(&["ads-sparkline__bubble num", align]),
                    style: "left:{x}%",
                    "aria-hidden": "true",
                    b { "{value}" }
                    " {name}"
                }
            }
            span { class: "sr-only", "aria-live": "polite", "{spoken}" }
        }
    }
}
