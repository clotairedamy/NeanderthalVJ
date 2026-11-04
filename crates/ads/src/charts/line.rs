//! Line chart — a hand-drawn SVG line with a filled area.

use dioxus::prelude::*;

fn line_path(points: &[f32], w: f32, h: f32) -> (String, String) {
    if points.len() < 2 {
        return (String::new(), String::new());
    }
    let min = points.iter().copied().fold(f32::INFINITY, f32::min);
    let max = points.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = if (max - min).abs() < 1e-4 {
        1.0
    } else {
        max - min
    };
    let step = w / (points.len() - 1) as f32;
    let coords = points
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f32 * step, h - 8.0 - ((v - min) / span) * (h - 16.0)))
        .collect::<Vec<_>>();

    let mut line = format!("M{:.1} {:.1}", coords[0].0, coords[0].1);
    for &(x, y) in &coords[1..] {
        line.push_str(&format!(" L{:.1} {:.1}", x, y));
    }
    let area = format!("{line} L{w:.0} {h:.0} L0 {h:.0} Z");
    (line, area)
}

/// A line chart, same hand-drawn aesthetic as the sparkline, with a filled
/// area.
#[component]
pub fn AdsLineChart(
    #[props(into)] points: Vec<f32>,
    #[props(default = 160u32)] height: u32,
) -> Element {
    let (line, area) = line_path(&points, 600.0, 160.0);
    rsx! {
        svg {
            view_box: "0 0 600 160",
            preserve_aspect_ratio: "none",
            class: "line",
            style: "height:{height}px",
            "aria-hidden": "true",
            path { d: area, fill: "currentColor", opacity: "0.12" }
            path {
                d: line,
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                vector_effect: "non-scaling-stroke",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}
