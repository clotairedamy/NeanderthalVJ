//! Count-up — a figure, server-rendered with its final value.

use dioxus::prelude::*;

/// A figure as `toLocaleString('en-GB')` writes it: thousands grouped with a
/// comma, a fixed number of decimals.
#[must_use]
pub(crate) fn format_en_gb(n: f64, decimals: usize) -> String {
    let fixed = format!("{:.*}", decimals, n.abs());
    let (int, frac) = fixed
        .split_once('.')
        .map_or((fixed.as_str(), None), |(i, f)| (i, Some(f)));
    let mut grouped = String::with_capacity(int.len() + int.len() / 3);
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let negative = n < 0.0 && fixed.chars().any(|c| c.is_ascii_digit() && c != '0');
    let sign = if negative { "-" } else { "" };
    match frac {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

/// A figure as a bare `toLocaleString('en-GB')` writes it: grouped, with up to
/// three decimals and no trailing zeros.
#[must_use]
pub(crate) fn format_number(n: f64) -> String {
    let s = format_en_gb(n, 3);
    match s.split_once('.') {
        Some((int, frac)) => {
            let frac = frac.trim_end_matches('0');
            if frac.is_empty() {
                int.to_owned()
            } else {
                format!("{int}.{frac}")
            }
        }
        None => s,
    }
}

/// Counts up to `to` the first time it scrolls into view — once, as an
/// arrival, never again. Server-rendered with the final value, so the number is
/// right without JavaScript, and a reader who asks for less motion only ever
/// sees the final value. When `to` changes later the figure is simply set: a
/// value that re-animates on every update is a slot machine, not a reading.
/// Assistive tech is given the final value and never the frames in between.
///
/// The arrival animation itself (an intersection observer driving frames) is a
/// client enhancement of the Vue kit; here the final value is rendered.
#[component]
pub fn CountUp(#[props(into)] to: f64, #[props(default = 0usize)] decimals: usize) -> Element {
    let shown = format_en_gb(to, decimals);
    rsx! {
        span { class: "ads-count-up num",
            span { "aria-hidden": "true", "{shown}" }
            span { class: "sr-only", "{shown}" }
        }
    }
}
