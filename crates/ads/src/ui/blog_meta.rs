//! Blog meta — a post's tag, date and reading time.

use dioxus::prelude::*;

/// A post's metadata: its tag, the date, and the reading time. The date is
/// ISO and the figures are in the data face, like every other number here.
#[component]
pub fn BlogMeta(
    #[props(into)] tag: String,
    #[props(into)] date: String,
    #[props(optional)] reading_time: Option<u32>,
) -> Element {
    rsx! {
        div { class: "ads-blog-meta",
            span { class: "ads-blog-meta__tag", "{tag}" }
            time { class: "num", datetime: "{date}", "{date}" }
            if let Some(rt) = reading_time.filter(|&m| m > 0) {
                span { "· " span { class: "num", "{rt}" } " min read" }
            }
        }
    }
}
