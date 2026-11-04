//! Marquee — a scrolling band of pills.

use dioxus::prelude::*;

fn defaults() -> Vec<String> {
    [
        "Accessibility",
        "SEO",
        "Performance",
        "Content",
        "Structure",
        "Security",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// A scrolling band of pills — a quiet way to show breadth. The second lane
/// is a copy for the seamless loop, hidden from assistive tech.
///
/// Anything that moves for more than five seconds needs a way to stop it, so
/// the band pauses on hover, on focus, and from its own toggle. Under reduced
/// motion it never starts: the pills wrap into a still row instead (CSS).
#[component]
pub fn Marquee(#[props(into, optional)] items: Option<Vec<String>>) -> Element {
    let items = items.unwrap_or_else(defaults);
    let mut paused = use_signal(|| false);
    let class = crate::class::join(&["ads-marquee", if paused() { "is-paused" } else { "" }]);
    rsx! {
        div { class: "{class}",
            div { class: "ads-marquee__viewport",
                div { class: "ads-marquee__track",
                    for copy in 1..=2u8 {
                        ul {
                            key: "{copy}",
                            class: "ads-marquee__lane",
                            role: "list",
                            "aria-hidden": if copy == 2 { Some("true") } else { None },
                            for item in &items { li { class: "ads-marquee__item", "{item}" } }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "btn btn-quiet btn-sm ads-marquee__toggle",
                "aria-pressed": if paused() { "true" } else { "false" },
                onclick: move |_| paused.toggle(),
                "Pause motion"
            }
        }
    }
}
