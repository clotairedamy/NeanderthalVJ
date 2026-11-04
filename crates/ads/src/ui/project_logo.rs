//! Project logo — a word on a small inset tile.

use dioxus::prelude::*;

/// A client's word on a small inset tile, always in the ink.
#[component]
pub fn ProjectLogo(
    #[props(into, default = "A".to_string())] word: String,
    #[props(default = 76u32)] size: u32,
) -> Element {
    rsx! {
        span { class: "ads-project-logo", style: "--s:{size}px",
            span { class: "ads-project-logo__word", "{word}" }
        }
    }
}
