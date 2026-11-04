//! Summary → detail — the pattern the whole system is arranged around.

use dioxus::prelude::*;

/// One rung of the ladder (answer → reasoning → evidence).
#[derive(Clone, PartialEq)]
pub struct Rung {
    pub title: String,
    pub body: String,
}

impl Rung {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// Summary → detail: the pattern the whole system is arranged around. A ranked
/// answer at the top, the reasoning one level down, the raw rows two levels
/// down — and the reader never loses their place.
#[component]
pub fn SummaryDetail(#[props(into, optional)] rungs: Option<Vec<Rung>>) -> Element {
    let rungs = rungs.unwrap_or_else(|| {
        vec![
            Rung::new("Answer.", "One ranked list. \"Fix the alt text on the blog template — 823 pages, two changes.\""),
            Rung::new("Reasoning.", "An accordion or drawer: why this ranks first, what the score is made of."),
            Rung::new("Evidence.", "The grid, filtered to the 823 rows, with the filter visible as a tag."),
        ]
    });
    rsx! {
        ol { class: "ads-summary-detail",
            for (i, r) in rungs.iter().enumerate() {
                li { key: "{i}", class: "ads-summary-detail__rung",
                    span { class: "num ads-summary-detail__step", "aria-hidden": "true", "{i + 1}" }
                    p { class: "ads-summary-detail__text",
                        strong { "{r.title}" }
                        " {r.body}"
                    }
                }
            }
        }
    }
}
