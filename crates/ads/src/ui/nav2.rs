//! Navigation: menu bar and navigation menu.

use dioxus::prelude::*;

use crate::icons::Icon;

/// One entry in a menubar or nav menu.
#[derive(Clone, PartialEq)]
pub struct MenubarItem {
    pub label: String,
    pub to: Option<String>,
    pub children: Vec<MenubarItem>,
}

impl MenubarItem {
    #[must_use]
    pub fn link(label: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: Some(to.into()),
            children: Vec::new(),
        }
    }
    #[must_use]
    pub fn group(label: impl Into<String>, children: Vec<MenubarItem>) -> Self {
        Self {
            label: label.into(),
            to: None,
            children,
        }
    }
}

/// A horizontal menu bar. Items may be plain links or dropdown groups.
#[component]
pub fn AdsMenubar(#[props(into)] items: Vec<MenubarItem>) -> Element {
    rsx! {
        nav { class: "menubar", "aria-label": "Menu",
            for it in &items {
                if it.children.is_empty() {
                    a { class: "mb-item", href: it.to.as_deref().unwrap_or("#"), "{it.label}" }
                } else {
                    div { class: "mb-group",
                        button { class: "mb-item", r#type: "button", "{it.label}", Icon { name: "chevron", size: 13 } }
                        div { class: "mb-menu",
                            for c in &it.children { a { class: "mb-sub", href: c.to.as_deref().unwrap_or("#"), "{c.label}" } }
                        }
                    }
                }
            }
        }
    }
}

/// A navigation menu: links with an optional dropdown.
#[component]
pub fn AdsNavMenu(#[props(into)] links: Vec<MenubarItem>) -> Element {
    rsx! {
        nav { class: "navmenu", "aria-label": "Primary",
            for l in &links {
                div { class: "nm-group",
                    a { class: "nm-item", class: if !l.children.is_empty() { "nm-item has" } else { "nm-item" }, href: l.to.as_deref().unwrap_or("#"),
                        "{l.label}"
                        if !l.children.is_empty() { Icon { name: "chevron", size: 13 } }
                    }
                    if !l.children.is_empty() {
                        div { class: "nm-drop",
                            for c in &l.children { a { class: "nm-sub", href: c.to.as_deref().unwrap_or("#"), "{c.label}" } }
                        }
                    }
                }
            }
        }
    }
}
