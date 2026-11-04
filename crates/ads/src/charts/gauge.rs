//! Gauge — a dial for a changing rate, with its threshold notched in.

use dioxus::prelude::*;

const SWEEP: f64 = 260.0; // degrees of arc the scale covers
const START: f64 = 140.0; // where it begins, measured clockwise from 3 o'clock
const R: f64 = 54.0;
const CX: f64 = 64.0;
const CY: f64 = 64.0;

fn clamp01(n: f64) -> f64 {
    n.clamp(0.0, 1.0)
}

// Rounded to two decimals, which is finer than a device pixel at any size this
// gauge is drawn at — and the same figures the Vue kit's server render writes.
fn polar(angle_deg: f64, radius: f64) -> (f64, f64) {
    let rad = angle_deg.to_radians();
    (
        ((CX + radius * rad.cos()) * 100.0).round() / 100.0,
        ((CY + radius * rad.sin()) * 100.0).round() / 100.0,
    )
}

fn arc(from: f64, to: f64, radius: f64) -> String {
    let (x1, y1) = polar(from, radius);
    let (x2, y2) = polar(to, radius);
    let large = i32::from((to - from).abs() > 180.0);
    format!("M{x1} {y1} A{radius} {radius} 0 {large} 1 {x2} {y2}")
}

/// A tick on the scale.
struct Tick {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    major: bool,
}

/// A dial. A dial shows a changing rate far better than a number does: the eye
/// reads the position of the needle without reading the digits. It is a meter
/// bent into a circle, so it obeys the meter's rule: the threshold mark is
/// required, because without it the reader cannot tell whether the arc is good
/// news.
///
/// The threshold is a notch cut through the arc in the dial's own surface, the
/// way the meter's mark is cut in the canvas, with a full-strength tick outside
/// it so it still reads where the arc is unfilled. The Vue kit eases the needle
/// between values; here the needle is set, which is also what the Vue kit's
/// first render does.
#[component]
pub fn AdsGauge(
    value: u32,
    #[props(default = 100u32)] max: u32,
    threshold: u32,
    #[props(into)] label: String,
    #[props(into, optional)] unit: Option<String>,
) -> Element {
    let unit = unit.unwrap_or_default();
    let max_f = if max == 0 { 1.0 } else { f64::from(max) };
    let fraction = clamp01(f64::from(value) / max_f);

    let track = arc(START, START + SWEEP, R);
    let progress = arc(START, START + SWEEP * fraction.max(0.001), R);

    // The scale: fourteen ticks across the sweep, every third one major.
    let ticks = (0..14)
        .map(|i| {
            let angle = START + (SWEEP / 13.0) * f64::from(i);
            let (x1, y1) = polar(angle, R + 7.0);
            let (x2, y2) = polar(angle, R + if i % 3 == 0 { 12.0 } else { 10.0 });
            Tick {
                x1,
                y1,
                x2,
                y2,
                major: i % 3 == 0,
            }
        })
        .collect::<Vec<_>>();

    let notch_angle = START + SWEEP * clamp01(f64::from(threshold) / max_f);
    let (cx1, cy1) = polar(notch_angle, R - 6.0);
    let (cx2, cy2) = polar(notch_angle, R + 6.0);
    let (tx1, ty1) = polar(notch_angle, R + 6.0);
    let (tx2, ty2) = polar(notch_angle, R + 14.0);

    let (nx, ny) = polar(START + SWEEP * fraction, R - 13.0);
    let name = format!("{label}: {value}{unit}, threshold {threshold}{unit}");

    rsx! {
        div { class: "ads-gauge",
            svg { view_box: "0 0 128 128", class: "ads-gauge__svg", role: "img", "aria-label": "{name}",
                g { class: "ads-gauge__ticks",
                    for (i, t) in ticks.iter().enumerate() {
                        line {
                            key: "{i}",
                            x1: "{t.x1}",
                            y1: "{t.y1}",
                            x2: "{t.x2}",
                            y2: "{t.y2}",
                            opacity: if t.major { "0.55" } else { "0.22" },
                        }
                    }
                }
                path { d: "{track}", class: "ads-gauge__track" }
                path { d: "{progress}", class: "ads-gauge__progress" }
                line { x1: "{cx1}", y1: "{cy1}", x2: "{cx2}", y2: "{cy2}", class: "ads-gauge__notch" }
                line { x1: "{tx1}", y1: "{ty1}", x2: "{tx2}", y2: "{ty2}", class: "ads-gauge__threshold" }
                line { x1: "{CX}", y1: "{CY}", x2: "{nx}", y2: "{ny}", class: "ads-gauge__needle" }
                // The cap is drawn over the needle so the reading is never
                // crossed by it.
                circle { cx: "{CX}", cy: "{CY}", r: "31", class: "ads-gauge__cap" }
            }
            div { class: "ads-gauge__read", "aria-hidden": "true",
                span { class: "num ads-gauge__value",
                    "{value}"
                    span { class: "ads-gauge__unit", "{unit}" }
                }
                span { class: "eyebrow", "{label}" }
            }
        }
    }
}
