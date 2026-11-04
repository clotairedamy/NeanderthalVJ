//! Dashboard shell — a rail on the left, the page on a raised frame, an app
//! bar along the top.

use dioxus::prelude::*;

use crate::icons::Icon;
use crate::section::Section;
use crate::ui::brand::{AdsThemeToggle, BrandMark};
use crate::ui::mobile_nav::MobileNav;

/// A page of the site, as the rail's foot, the breadcrumb and the phone tab
/// bar list it. `short` is the tab-bar label; `rail` puts the page in the
/// rail's foot, beside the collapse button.
#[derive(Default, Clone, PartialEq)]
pub struct AppPage {
    pub to: String,
    pub label: String,
    pub short: String,
    pub icon: String,
    pub rail: bool,
}

impl AppPage {
    #[must_use]
    pub fn new(
        to: impl Into<String>,
        label: impl Into<String>,
        short: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        Self {
            to: to.into(),
            label: label.into(),
            short: short.into(),
            icon: icon.into(),
            rail: false,
        }
    }

    /// List the page in the rail's foot.
    #[must_use]
    pub const fn rail(mut self) -> Self {
        self.rail = true;
        self
    }

    /// The studio's own pages, as the site lists them.
    #[must_use]
    pub fn site() -> Vec<Self> {
        vec![
            Self::new("/", "Home", "Home", "dashboard"),
            Self::new(
                "/dashboard-example",
                "Dashboard example",
                "Example",
                "chart",
            )
            .rail(),
            Self::new("/design-system", "Design system", "System", "system").rail(),
            Self::new("/components", "Components", "Kit", "layers").rail(),
            Self::new("/contact", "Start a project", "Contact", "mail"),
        ]
    }
}

/// The page a path sits under: the longest page path that is the path itself
/// or one of its parents, so /components/rust/inputs still reads as
/// Components.
#[must_use]
pub fn page_for<'a>(pages: &'a [AppPage], path: &str) -> Option<&'a AppPage> {
    pages
        .iter()
        .filter(|p| path == p.to || (p.to != "/" && path.starts_with(&format!("{}/", p.to))))
        .max_by_key(|p| p.to.len())
}

/// The window chrome: a rail on the left, the page on a raised frame beside
/// it. Measurements are the app's own — a 232px rail collapsing to 88, 48px
/// tiles at a 15px radius, 62px rows.
///
/// The piece that makes it read as this app rather than a generic sidebar is
/// the selected row. It is not a pill drawn on the rail; it is a tab cut from
/// the frame's own surface, reaching left into the rail with concave fillets
/// where it flows back into the frame, so the seam between rail and content
/// disappears behind whatever is selected.
///
/// `sections` is the index of the page you are on, and `active` is the one
/// being read — the caller keeps it, so the rail never disagrees with a
/// finer-grained index on the same page. `pages` feeds the rail's foot (the
/// ones marked `rail`) and, on a page without sections, the breadcrumb;
/// `path` is the current route. The app bar holds the three things you might
/// want from anywhere: search (`onsearch`, also ⌘K), appearance (`dark` and
/// `ontheme`), and the one action. The phone tab bar comes with it.
#[component]
pub fn DashShell(
    #[props(into, default)] sections: Vec<Section>,
    #[props(default = 0usize)] active: usize,
    #[props(optional)] onselect: Option<EventHandler<usize>>,
    #[props(into, default = AppPage::site())] pages: Vec<AppPage>,
    #[props(into, optional)] path: Option<String>,
    #[props(into, default = "Artelier".to_string())] brand: String,
    #[props(into, default = "studio@artelier.agency".to_string())] email: String,
    #[props(optional)] dark: bool,
    #[props(optional)] ontheme: Option<EventHandler<MouseEvent>>,
    #[props(optional)] onsearch: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let mut collapsed = use_signal(|| false);
    let path = path.unwrap_or_default();
    let page = page_for(&pages, &path).cloned();
    let page_to = page.as_ref().map(|p| p.to.clone());
    let where_ = if sections.is_empty() {
        page.map(|p| p.label).unwrap_or_default()
    } else {
        sections
            .get(active)
            .map(|s| s.label.clone())
            .unwrap_or_default()
    };
    let rail_pages: Vec<AppPage> = pages.iter().filter(|p| p.rail).cloned().collect();
    let search = move || {
        if let Some(h) = &onsearch {
            h.call(());
        }
    };

    rsx! {
        div {
            class: if collapsed() { "ads-dash-shell is-collapsed" } else { "ads-dash-shell" },
            // ⌘K opens the palette, the convention people bring from other tools.
            onkeydown: move |e: KeyboardEvent| {
                let m = e.modifiers();
                if (m.meta() || m.ctrl()) && e.key() == Key::Character("k".into()) {
                    e.prevent_default();
                    search();
                }
            },
            aside { class: "ads-dash-shell__rail",
                a { class: "ads-dash-shell__brand", href: "/", "aria-label": "{brand}",
                    BrandMark { class: "ads-dash-shell__mark", size: if collapsed() { 40 } else { 78 } }
                    span { class: "ads-dash-shell__wordmark num", "{brand}" }
                }
                if !sections.is_empty() {
                    hr { class: "ads-dash-shell__rule" }
                    nav { class: "ads-dash-shell__nav", "aria-label": "Sections",
                        // The tab: a piece of the frame reaching into the rail. One
                        // element that slides, so moving between sections is a single
                        // continuous gesture.
                        span {
                            class: "ads-dash-shell__tab",
                            style: "transform: translateY({active * 62}px)",
                            "aria-hidden": "true",
                            span { class: "ads-dash-shell__fillet ads-dash-shell__fillet-top" }
                            span { class: "ads-dash-shell__fillet ads-dash-shell__fillet-bottom" }
                        }
                        for (i, s) in sections.iter().enumerate() {
                            a {
                                key: "{s.id}",
                                class: if i == active { "ads-dash-shell__item is-active" } else { "ads-dash-shell__item" },
                                href: "#{s.id}",
                                "aria-current": if i == active { Some("true") } else { None },
                                onclick: move |_| { if let Some(h) = &onselect { h.call(i); } },
                                span { class: "ads-dash-shell__tile", Icon { name: s.icon.clone(), size: 20 } }
                                span { class: "ads-dash-shell__label", "{s.label}" }
                            }
                        }
                    }
                }
                div { class: "ads-dash-shell__foot",
                    for p in rail_pages.iter() {
                        a {
                            key: "{p.to}",
                            class: if page_to.as_deref() == Some(p.to.as_str()) { "ads-dash-shell__item is-active" } else { "ads-dash-shell__item" },
                            href: "{p.to}",
                            "aria-current": if page_to.as_deref() == Some(p.to.as_str()) { Some("page") } else { None },
                            span { class: "ads-dash-shell__tile", Icon { name: p.icon.clone(), size: 20 } }
                            span { class: "ads-dash-shell__label", "{p.label}" }
                        }
                    }
                    button {
                        class: "ads-dash-shell__item",
                        r#type: "button",
                        "aria-expanded": if collapsed() { "false" } else { "true" },
                        "aria-label": if collapsed() { "Expand the sidebar" } else { "Collapse the sidebar" },
                        onclick: move |_| collapsed.toggle(),
                        span { class: "ads-dash-shell__tile ads-dash-shell__collapse-tile", Icon { name: "chevrons", size: 20 } }
                        span { class: "ads-dash-shell__label", "Collapse" }
                    }
                }
            }
            main { class: "ads-dash-shell__panel",
                header { class: "ads-dash-shell__appbar",
                    a { class: "ads-dash-shell__appbar-brand", href: "/", "aria-label": "{brand}",
                        BrandMark { size: 30 }
                    }
                    nav { class: "ads-dash-shell__crumbs", "aria-label": "Breadcrumb",
                        a { class: "ads-dash-shell__crumb-root", href: "/", "{brand}" }
                        span { class: "ads-dash-shell__crumb-sep", "aria-hidden": "true", "/" }
                        span { class: "ads-dash-shell__crumb-window",
                            span { class: "ads-dash-shell__crumb-here", "aria-current": "location", "{where_}" }
                        }
                    }
                    div { class: "ads-dash-shell__tools",
                        button {
                            class: "ads-dash-shell__search",
                            r#type: "button",
                            "aria-label": "Search or jump to",
                            onclick: move |_| search(),
                            Icon { name: "discovery", size: 18 }
                            span { class: "ads-dash-shell__search-label", "Search or jump to…" }
                            kbd { class: "kbd", "⌘K" }
                        }
                        AdsThemeToggle {
                            dark,
                            onclick: move |e| { if let Some(h) = &ontheme { h.call(e); } },
                        }
                        a { class: "btn btn-accent ads-dash-shell__cta", href: "/contact", "Start a project" }
                    }
                }
                {children}
            }
            MobileNav {
                sections: sections.clone(),
                active,
                onselect: move |i| { if let Some(h) = &onselect { h.call(i); } },
                pages: pages.clone(),
                path: path.clone(),
                email: email.clone(),
                dark,
                ontheme: move |e| { if let Some(h) = &ontheme { h.call(e); } },
                onsearch: move |()| search(),
            }
        }
    }
}
