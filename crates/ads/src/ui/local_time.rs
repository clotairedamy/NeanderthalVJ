//! Local time — the time in the studio, and whether anyone is at the desk.

use dioxus::prelude::*;

/// The time in the studio, and whether anyone is at the desk. More useful
/// than an address: it tells you when to expect a reply.
///
/// The dot is never the only signal: the state is always written beside it,
/// so the reading survives a desaturated screen. With no `state` given, it
/// reads "At the desk" or "Out of hours".
#[component]
pub fn LocalTime(
    #[props(into, default = "Colorado Springs".to_string())] city: String,
    #[props(into, optional)] time: Option<String>,
    #[props(default = true)] open: bool,
    #[props(into, optional)] state: Option<String>,
) -> Element {
    let time = time
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "··:··".to_string());
    let label = state
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| if open { "At the desk" } else { "Out of hours" }.to_string());
    rsx! {
        p { class: "ads-local-time",
            span { class: if open { "ads-local-time__dot" } else { "ads-local-time__dot is-off" }, "aria-hidden": "true" }
            span { "{city}" }
            time { class: "ads-local-time__time num", "{time}" }
            span { class: "ads-local-time__state", "{label}" }
        }
    }
}
