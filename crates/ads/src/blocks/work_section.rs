//! Work section — the work as a grid of project cards.

use dioxus::prelude::*;

/// One project card.
#[derive(Clone, PartialEq)]
pub struct WorkCard {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub tags: Vec<String>,
    pub href: Option<String>,
}

impl WorkCard {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: impl Into<String>, description: impl Into<String>) -> Self {
        Self { name: name.into(), kind: kind.into(), description: description.into(), tags: Vec::new(), href: None }
    }
    #[must_use]
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    #[must_use]
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
}

/// The work as a grid of project cards: kind, name, description, and tags.
/// The tags are labels, not filters, so they read as plain chips. The live
/// link opens a new tab, and its name says so.
#[component]
pub fn WorkSection(
    #[props(into)] items: Vec<WorkCard>,
) -> Element {
    rsx! {
        section { class: "ads-work-section",
            div { class: "ads-work-section__grid",
                for p in &items {
                    article { key: "{p.name}", class: "card ads-work-section__card",
                        span { class: "ads-work-section__kind", "{p.kind}" }
                        div { class: "ads-work-section__title-row",
                            h3 { class: "ads-work-section__name", "{p.name}" }
                            if let Some(h) = p.href.as_deref().filter(|h| !h.is_empty()) {
                                a {
                                    class: "ads-work-section__link",
                                    href: "{h}",
                                    target: "_blank",
                                    rel: "noopener",
                                    "aria-label": "Live site: {p.name} (opens in a new tab)",
                                    "Live ↗"
                                }
                            }
                        }
                        p { class: "ads-work-section__desc", "{p.description}" }
                        div { class: "ads-work-section__tags",
                            for t in &p.tags { span { key: "{t}", class: "chip", "{t}" } }
                        }
                    }
                }
            }
        }
    }
}
