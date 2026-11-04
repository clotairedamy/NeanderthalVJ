//! Wordmark — the name, set small.

use dioxus::prelude::*;

/// The wordmark, set small: the studio's name in the display face, in the
/// ink. It stands on its own where the name has to be read; where it sits
/// beside `BrandMark`, the mark is the logo and this only spells it out.
#[component]
pub fn Wordmark(
    #[props(into, default = "Artelier".to_string())] text: String,
    #[props(default = 18u32)] size: u32,
) -> Element {
    rsx! {
        span { class: "ads-wordmark", style: "font-size:{size}px", "{text}" }
    }
}
