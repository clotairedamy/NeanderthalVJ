//! Mobile navigation — a tab bar along the bottom, and a sheet behind "More".

use dioxus::prelude::*;

use crate::icons::Icon;
use crate::section::Section;
use crate::ui::brand::AdsThemeToggle;
use crate::ui::dash_shell::AppPage;

/// One slot in the tab bar: a section (with its index) or a page.
#[derive(Clone, PartialEq)]
struct Tab {
    key: String,
    label: String,
    icon: String,
    href: String,
    index: Option<usize>,
}

/// Phone navigation: a tab bar along the bottom, where a thumb can reach it,
/// and a sheet behind "More" for everything that does not fit in four slots.
///
/// On a page with sections the tabs are sections — the ones flagged `tab`, or
/// the first four — and the raised pill slides to whichever one is `active`.
/// A section that is not in the bar lights up More instead, so the pill always
/// says where you are. On a page without sections, the tabs are the pages,
/// matched against `path`.
///
/// The sheet is a modal dialog: focus moves to its search button when it
/// opens, and Escape or a tap on the backdrop closes it. `DashShell` renders
/// this for you; use it alone only outside the shell.
#[component]
pub fn MobileNav(
    #[props(into, default)] sections: Vec<Section>,
    #[props(default = 0usize)] active: usize,
    #[props(optional)] onselect: Option<EventHandler<usize>>,
    #[props(into, default = AppPage::site())] pages: Vec<AppPage>,
    #[props(into, optional)] path: Option<String>,
    #[props(into, default = "studio@artelier.agency".to_string())] email: String,
    #[props(optional)] dark: bool,
    #[props(optional)] ontheme: Option<EventHandler<MouseEvent>>,
    #[props(optional)] onsearch: Option<EventHandler<()>>,
) -> Element {
    let mut sheet = use_signal(|| false);
    let path = path.unwrap_or_default();

    let tabs: Vec<Tab> = if sections.is_empty() {
        pages
            .iter()
            .map(|p| Tab {
                key: p.to.clone(),
                label: p.short.clone(),
                icon: p.icon.clone(),
                href: p.to.clone(),
                index: None,
            })
            .collect()
    } else {
        let flagged: Vec<(usize, &Section)> =
            sections.iter().enumerate().filter(|(_, s)| s.tab).collect();
        let picked = if flagged.is_empty() {
            sections.iter().enumerate().collect()
        } else {
            flagged
        };
        picked
            .into_iter()
            .take(4)
            .map(|(i, s)| Tab {
                key: s.id.clone(),
                label: s.display().to_string(),
                icon: s.icon.clone(),
                href: format!("#{}", s.id),
                index: Some(i),
            })
            .collect()
    };
    let more = tabs.len();
    let pill = if sheet() {
        more
    } else if sections.is_empty() {
        tabs.iter().position(|t| t.href == path).unwrap_or(more)
    } else {
        tabs.iter()
            .position(|t| t.index == Some(active))
            .unwrap_or(more)
    };

    rsx! {
        nav { class: "ads-mobile-nav", "aria-label": "Primary",
            span {
                class: "ads-mobile-nav__pill",
                style: "transform: translateX({pill * 100}%)",
                "aria-hidden": "true",
            }
            for (n, t) in tabs.into_iter().enumerate() {
                a {
                    key: "{t.key}",
                    class: if pill == n { "ads-mobile-nav__tab is-active" } else { "ads-mobile-nav__tab" },
                    href: "{t.href}",
                    "aria-current": if pill == n && t.index.is_some() { Some("location") } else { None },
                    onclick: move |_| {
                        if let (Some(i), Some(h)) = (t.index, &onselect) {
                            h.call(i);
                        }
                    },
                    Icon { name: t.icon.clone(), size: 21 }
                    span { "{t.label}" }
                }
            }
            button {
                class: if pill == more { "ads-mobile-nav__tab is-active" } else { "ads-mobile-nav__tab" },
                r#type: "button",
                "aria-haspopup": "dialog",
                "aria-expanded": if sheet() { "true" } else { "false" },
                onclick: move |_| sheet.toggle(),
                Icon { name: "grid", size: 21 }
                span { "More" }
            }
        }
        if sheet() {
            div {
                class: "ads-mobile-nav__backdrop",
                onclick: move |_| sheet.set(false),
                div {
                    class: "ads-mobile-nav__sheet",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": "Menu",
                    // A tap inside the sheet is not a tap on the backdrop.
                    onclick: move |e| e.stop_propagation(),
                    onkeydown: move |e: KeyboardEvent| {
                        if e.key() == Key::Escape {
                            sheet.set(false);
                        }
                    },
                    div { class: "ads-mobile-nav__grab",
                        span { class: "ads-mobile-nav__handle" }
                    }
                    button {
                        class: "ads-mobile-nav__search",
                        r#type: "button",
                        onmounted: move |e: MountedEvent| {
                            let el = e.data();
                            spawn(async move {
                                let _ = el.set_focus(true).await;
                            });
                        },
                        onclick: move |_| {
                            sheet.set(false);
                            if let Some(h) = &onsearch {
                                h.call(());
                            }
                        },
                        Icon { name: "discovery", size: 18 }
                        "Search or jump to…"
                    }
                    if !sections.is_empty() {
                        p { class: "eyebrow ads-mobile-nav__heading", "On this page" }
                        div { class: "ads-mobile-nav__grid",
                            for (i, s) in sections.iter().enumerate() {
                                a {
                                    key: "{s.id}",
                                    class: if i == active { "ads-mobile-nav__item is-active" } else { "ads-mobile-nav__item" },
                                    href: "#{s.id}",
                                    "aria-current": if i == active { Some("location") } else { None },
                                    onclick: move |_| {
                                        if let Some(h) = &onselect {
                                            h.call(i);
                                        }
                                        sheet.set(false);
                                    },
                                    span { class: "ads-mobile-nav__item-icon", Icon { name: s.icon.clone(), size: 19 } }
                                    span { "{s.label}" }
                                }
                            }
                        }
                    }
                    p { class: "eyebrow ads-mobile-nav__heading", "Pages" }
                    div { class: "ads-mobile-nav__grid",
                        for p in pages.iter() {
                            a {
                                key: "{p.to}",
                                class: if p.to == path { "ads-mobile-nav__item is-active" } else { "ads-mobile-nav__item" },
                                href: "{p.to}",
                                onclick: move |_| sheet.set(false),
                                span { class: "ads-mobile-nav__item-icon", Icon { name: p.icon.clone(), size: 19 } }
                                span { "{p.label}" }
                            }
                        }
                    }
                    div { class: "ads-mobile-nav__foot",
                        div { class: "ads-mobile-nav__theme",
                            AdsThemeToggle {
                                dark,
                                id: "ads-theme-moon-sheet",
                                onclick: move |e| { if let Some(h) = &ontheme { h.call(e); } },
                            }
                            span { class: "body", "Appearance" }
                        }
                        a { class: "num ads-mobile-nav__mail", href: "mailto:{email}", "{email}" }
                    }
                }
            }
        }
    }
}
