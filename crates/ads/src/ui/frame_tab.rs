//! Frame tab — the selected rail item, cut from the frame's own surface.

use dioxus::prelude::*;

/// The frame tab: the selected rail item, cut from the frame's own surface and
/// reaching into the rail with concave fillets, so rail and content read as
/// one piece of material. Used once, for the single selected destination —
/// which is why `active` is one index, never a set. The frame's content is
/// the children.
#[component]
pub fn FrameTab(
    #[props(into)] items: Vec<String>,
    active: usize,
    children: Element,
) -> Element {
    rsx! {
        div { class: "ads-frame-tab",
            div { class: "ads-frame-tab__rail",
                for (i, item) in items.iter().enumerate() {
                    span {
                        key: "{item}",
                        class: if i == active { "ads-frame-tab__row is-on" } else { "ads-frame-tab__row" },
                        "aria-current": if i == active { Some("page") } else { None },
                        if i == active {
                            span { class: "ads-frame-tab__tab", "aria-hidden": "true",
                                i { class: "ads-frame-tab__fillet is-top" }
                                i { class: "ads-frame-tab__fillet is-bottom" }
                            }
                        }
                        span { class: "ads-frame-tab__label", "{item}" }
                    }
                }
            }
            div { class: "ads-frame-tab__frame",
                {children}
            }
        }
    }
}
