//! Breadcrumb and pagination.

use dioxus::prelude::*;

/// One crumb: a label, and a link unless it is the page you are on.
#[derive(Default, Clone, PartialEq)]
pub struct Crumb {
    pub label: String,
    pub to: Option<String>,
}

impl Crumb {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: None,
        }
    }
    #[must_use]
    pub fn link(label: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: Some(to.into()),
        }
    }
}

/// Breadcrumb trail: where you are, once a screen is three levels deep or
/// more. Only the last crumb is the current page; a middle crumb without a
/// link is plain text, not "current".
#[component]
pub fn AdsCrumbs(#[props(into)] items: Vec<Crumb>) -> Element {
    let last = items.len().saturating_sub(1);
    rsx! {
        nav { class: "ads-crumbs", "aria-label": "Breadcrumb",
            for (i, c) in items.iter().enumerate() {
                if i > 0 {
                    span { class: "sep", "aria-hidden": "true", "/" }
                }
                if i == last {
                    span { "aria-current": "page", "{c.label}" }
                } else if let Some(to) = &c.to {
                    a { href: "{to}", "{c.label}" }
                } else {
                    span { "{c.label}" }
                }
            }
        }
    }
}

/// One slot in the pager row: a page number, or the gap between runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PagerItem {
    Page(u32),
    Gap,
}

/// The pages to show: all of them up to seven; past that the ends and the
/// neighbours of the current page, with a gap wherever a run breaks.
fn pager_items(page: u32, pages: u32) -> Vec<PagerItem> {
    if pages <= 7 {
        return (1..=pages).map(PagerItem::Page).collect();
    }
    let mut keep: Vec<u32> = [1, pages, page.saturating_sub(1), page, page + 1]
        .into_iter()
        .filter(|n| (1..=pages).contains(n))
        .collect();
    keep.sort_unstable();
    keep.dedup();
    let mut out = Vec::with_capacity(keep.len() * 2);
    for (i, &n) in keep.iter().enumerate() {
        if i > 0 && n - keep[i - 1] > 1 {
            out.push(PagerItem::Gap);
        }
        out.push(PagerItem::Page(n));
    }
    out
}

/// Pagination: which slice of the table you are looking at. Page numbers are
/// figures, so they are monospaced. Past seven pages the row shows the ends
/// and the neighbours of the current page with a gap between — every page
/// stays two clicks away, and "page 400" is still reachable, which infinite
/// scroll would not allow. `onchange` receives the page to go to.
#[component]
pub fn AdsPager(
    #[props(default = 1u32)] page: u32,
    #[props(default = 1u32)] pages: u32,
    #[props(optional)] onchange: Option<EventHandler<u32>>,
) -> Element {
    let go = move |n: u32| {
        if n < 1 || n > pages || n == page {
            return;
        }
        if let Some(h) = &onchange {
            h.call(n);
        }
    };
    rsx! {
        nav { class: "ads-pager", "aria-label": "Pagination",
            button {
                r#type: "button",
                "aria-label": "Previous page",
                disabled: page <= 1,
                onclick: move |_| go(page.saturating_sub(1)),
                "‹"
            }
            for (i, item) in pager_items(page, pages).into_iter().enumerate() {
                match item {
                    PagerItem::Gap => rsx! {
                        span { key: "gap{i}", class: "ads-pager__gap", "aria-hidden": "true", "…" }
                    },
                    PagerItem::Page(n) => rsx! {
                        button {
                            key: "{n}",
                            r#type: "button",
                            class: if n == page { "is-on" } else { "" },
                            "aria-label": "Page {n}",
                            "aria-current": (n == page).then_some("page"),
                            onclick: move |_| go(n),
                            "{n}"
                        }
                    },
                }
            }
            button {
                r#type: "button",
                "aria-label": "Next page",
                disabled: page >= pages,
                onclick: move |_| go(page + 1),
                "›"
            }
        }
    }
}
