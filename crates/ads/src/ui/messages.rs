//! Messaging: message, message scroller, scroll area.

use dioxus::prelude::*;

use crate::ui::AdsAvatar;

/// A single message in a thread: avatar, name/time, and a bubble.
#[component]
pub fn AdsMessage(
    #[props(into, optional)] name: Option<String>,
    #[props(default = false)] mine: bool,
    #[props(into, optional)] time: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div { class: if mine { "msg mine" } else { "msg" },
            if let Some(n) = &name {
                if !mine { AdsAvatar { name: n.clone(), size: 32 } }
            }
            div { class: "msg-col",
                if name.is_some() || time.is_some() {
                    div { class: "msg-meta",
                        if let Some(n) = &name { span { class: "msg-name", "{n}" } }
                        if let Some(t) = &time { time { class: "msg-time num", "{t}" } }
                    }
                }
                crate::ui::AdsBubble { mine, {children} }
            }
        }
    }
}

/// A scrollable column that stays pinned to the newest message.
#[component]
pub fn AdsMessageScroller(
    children: Element,
) -> Element {
    rsx! { div { class: "scroller", {children} } }
}

/// A scrollable area with a soft, slim scrollbar.
#[component]
pub fn AdsScrollArea(
    #[props(into, optional)] height: Option<String>,
    children: Element,
) -> Element {
    rsx! { div { class: "scroll-area", style: if let Some(h) = &height { format!("height:{h}") } else { String::new() }, {children} } }
}
