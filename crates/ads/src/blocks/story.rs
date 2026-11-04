//! Story section — a narrative with the rest one click away.

use dioxus::prelude::*;

use crate::blocks::use_uid;
use crate::ui::{RevealHeading, RevealSize};

/// A narrative: a heading, the story, and the rest one click away. The button
/// says what it will do and is tied to the passage it opens with
/// `aria-controls`; the passage stays in the page, hidden, as `v-show` keeps
/// it.
#[component]
pub fn StorySection(
    #[props(into, optional)] heading: Option<Vec<String>>,
    #[props(into, optional)] body: Option<Vec<String>>,
    #[props(into, optional)] more: Option<Vec<String>>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let more_id = use_uid("ads-story-section-more");
    let heading = heading.unwrap_or_else(|| vec!["Less, but".to_string(), "done well.".to_string()]);
    let body = body.unwrap_or_default();
    let more = more.unwrap_or_default();
    let open = expanded();

    rsx! {
        section { class: "ads-story-section",
            RevealHeading { lines: heading, size: RevealSize::Title }
            div { class: "ads-story-section__body",
                for (i, p) in body.iter().enumerate() { p { key: "{i}", class: "body", "{p}" } }
            }
            if !more.is_empty() {
                button {
                    class: "btn",
                    r#type: "button",
                    "aria-expanded": if open { "true" } else { "false" },
                    "aria-controls": "{more_id}",
                    onclick: move |_| expanded.set(!open),
                    if open { "Read less" } else { "Read the rest" }
                }
            }
            div {
                id: "{more_id}",
                class: "ads-story-section__more",
                style: if open { None } else { Some("display: none;") },
                for (i, p) in more.iter().enumerate() { p { key: "{i}", class: "body", "{p}" } }
            }
        }
    }
}
