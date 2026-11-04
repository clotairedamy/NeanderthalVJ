//! Work list — the work as a list.

use dioxus::prelude::*;

use crate::icons::Icon;

/// One entry in the work list. Without an `href` it is plain text.
#[derive(Clone, PartialEq)]
pub struct WorkItem {
    pub name: String,
    pub kind: String,
    pub words: String,
    pub href: Option<String>,
}

impl WorkItem {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: impl Into<String>, words: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            words: words.into(),
            href: None,
        }
    }
    #[must_use]
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
}

/// The work as a list: a mark, what it is in a line, and where it lives. A row
/// without an address is plain text — never a link to nowhere — and only a
/// row that goes somewhere says "Read the case study".
///
/// The mark is a `ProjectLogo` hidden from screen readers (the name beside it
/// already says it); its markup is written out here so it can carry
/// `aria-hidden`.
#[component]
pub fn WorkList(#[props(into)] items: Vec<WorkItem>) -> Element {
    rsx! {
        ul { class: "ads-work-list",
            for p in &items {
                li { key: "{p.name}", class: "ads-work-list__row",
                    {row(p)}
                }
            }
        }
    }
}

fn row(p: &WorkItem) -> Element {
    let initial = p.name.chars().next().map(String::from).unwrap_or_default();
    let inner = rsx! {
        span { class: "ads-project-logo", style: "--s:48px", "aria-hidden": "true",
            span { class: "ads-project-logo__word", "{initial}" }
        }
        span { class: "ads-work-list__text",
            span { class: "ads-work-list__kind", "{p.kind}" }
            span { class: "ads-work-list__name", "{p.name}" }
            span { class: "ads-work-list__words", "{p.words}" }
        }
    };
    match p.href.as_deref().filter(|h| !h.is_empty()) {
        Some(href) => rsx! {
            a { class: "ads-work-list__link", href: "{href}",
                {inner}
                span { class: "ads-work-list__go",
                    "Read the case study "
                    Icon { name: "arrow", size: 15 }
                }
            }
        },
        None => rsx! {
            div { class: "ads-work-list__link is-static", {inner} }
        },
    }
}
