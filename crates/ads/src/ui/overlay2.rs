//! Overlay components: hover card, sheet, context menu.

use dioxus::prelude::*;

/// A card that appears beside its trigger on hover.
#[component]
pub fn AdsHoverCard(
    trigger: Element,
    children: Element,
) -> Element {
    rsx! {
        div { class: "hovercard",
            span { class: "trigger", {trigger} }
            div { class: "card-pop", role: "tooltip", {children} }
        }
    }
}

/// A side sheet. `open` is controlled; `onclose` handles the scrim and button.
#[component]
pub fn AdsSheet(
    #[props(default = false)] open: bool,
    #[props(into, default = "right".to_string())] side: String,
    #[props(into, optional)] title: Option<String>,
    #[props(optional)] onclose: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    rsx! {
        if open {
            div { class: "scrim", onmousedown: move |e| { if let Some(h) = &onclose { h.call(e); } },
                div { class: "sheet {side}", role: "dialog",
                    if let Some(t) = &title {
                        header { class: "sheet-head",
                            h3 { class: "h3", "{t}" }
                            button { class: "icon-btn", r#type: "button", "aria-label": "Close", onclick: move |e| { if let Some(h) = &onclose { h.call(e); } }, "×" }
                        }
                    }
                    div { class: "sheet-body", {children} }
                }
            }
        }
    }
}

/// A context menu: a soft list of actions.
#[component]
pub fn AdsContextMenu(
    #[props(into)] items: Vec<String>,
    #[props(optional)] onselect: Option<EventHandler<usize>>,
) -> Element {
    rsx! {
        div { class: "ctx-menu", role: "menu",
            for (i, it) in items.iter().enumerate() {
                button { class: "ctx-item", r#type: "button", role: "menuitem", onclick: { let h = onselect; move |_| { if let Some(h) = &h { h.call(i); } } }, "{it}" }
            }
        }
    }
}
