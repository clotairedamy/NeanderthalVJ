//! Screen tour — a tabbed tour of the app, one screen at a time.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::blocks::{focus, use_uid};
use crate::class::join;

/// One screen in the tour.
#[derive(Clone, PartialEq)]
pub struct TourScreen {
    pub id: String,
    pub label: String,
    pub title: String,
    pub body: String,
}

impl TourScreen {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            title: title.into(),
            body: body.into(),
        }
    }
}

/// A tabbed tour: a pressed-in segmented control picks the screen, and the
/// copy changes with it. The visual is the children.
///
/// It is a real tab set — arrow keys, Home and End move between screens, one
/// tab in the tab order, the panel labelled by its tab — because a row of
/// buttons that swaps content is a tab set whether or not it says so. The
/// active screen is the caller's: `onchange` reports the one picked.
#[component]
pub fn ScreenTour(
    #[props(into)] screens: Vec<TourScreen>,
    #[props(default = 0usize)] active: usize,
    #[props(optional)] onchange: Option<EventHandler<usize>>,
    children: Element,
) -> Element {
    let uid = use_uid("ads-screen-tour");
    let mut tabs = use_signal(Vec::<Option<Rc<MountedData>>>::new);
    let count = screens.len();
    let current = screens.get(active);
    let current_id = current.map(|s| s.id.clone()).unwrap_or_default();

    rsx! {
        div { class: "ads-screen-tour",
            div {
                class: "ads-screen-tour__tabs",
                role: "tablist",
                "aria-label": "Screens",
                onkeydown: move |e: KeyboardEvent| {
                    if count == 0 {
                        return;
                    }
                    let to = match e.key() {
                        Key::ArrowRight => (active + 1) % count,
                        Key::ArrowLeft => (active + count - 1) % count,
                        Key::Home => 0,
                        Key::End => count - 1,
                        _ => return,
                    };
                    e.prevent_default();
                    if let Some(h) = &onchange { h.call(to); }
                    focus(tabs.read().get(to).cloned().flatten());
                },
                for (i, s) in screens.iter().enumerate() {
                    button {
                        key: "{s.id}",
                        id: "{uid}-tab-{s.id}",
                        r#type: "button",
                        role: "tab",
                        class: join(&["ads-screen-tour__tab", if i == active { "is-active" } else { "" }]),
                        "aria-selected": if i == active { "true" } else { "false" },
                        "aria-controls": if i == active { Some(format!("{uid}-panel-{}", s.id)) } else { None },
                        tabindex: if i == active { "0" } else { "-1" },
                        onmounted: move |e| {
                            let mut t = tabs.write();
                            if t.len() <= i { t.resize(i + 1, None); }
                            t[i] = Some(e.data());
                        },
                        onclick: move |_| { if let Some(h) = &onchange { h.call(i); } },
                        "{s.label}"
                    }
                }
            }

            div {
                id: "{uid}-panel-{current_id}",
                class: "ads-screen-tour__stage",
                role: "tabpanel",
                "aria-labelledby": "{uid}-tab-{current_id}",
                div { class: "ads-screen-tour__copy",
                    h3 { class: "h3", if let Some(s) = current { "{s.title}" } }
                    p { class: "body", if let Some(s) = current { "{s.body}" } }
                }
                div { class: "ads-screen-tour__shot", {children} }
            }
        }
    }
}
