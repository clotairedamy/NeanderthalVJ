//! Alert dialog: a confirmation that interrupts, for one irreversible step.

use dioxus::prelude::*;

use super::support::{hold_modal, use_uid};

/// A confirmation that interrupts. It is for one irreversible step, so it says
/// the verb on its button ("Delete project", never "OK") and, when that step
/// is destructive, the button stays in the default style (the one accent on
/// the screen never points at a loss) and focus starts on Cancel. Escape
/// cancels, Tab stays inside, a click on the scrim does nothing (this is a
/// decision, not a notice), and focus goes back to whatever opened it.
///
/// `open` is the caller's state: set it false again in `oncancel` and
/// `onaction`.
#[component]
pub fn AdsAlertDialog(
    #[props(into)] title: String,
    #[props(into)] action_label: String,
    #[props(into, default)] description: String,
    #[props(into, default = "Cancel".to_string())] cancel_label: String,
    #[props(default = true)] destructive: bool,
    #[props(default = false)] open: bool,
    #[props(optional)] onaction: Option<EventHandler<()>>,
    #[props(optional)] oncancel: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let uid = use_uid();
    if !open {
        return rsx! {};
    }
    let title_id = format!("{uid}-title");
    let desc_id = format!("{uid}-desc");
    let has_desc = !description.is_empty();
    let action_class = crate::class::join(&["btn", if destructive { "" } else { "btn-accent" }]);
    let panel = format!("[aria-labelledby=\"{title_id}\"]");
    let first = if destructive {
        ".ads-alert-dialog__actions .btn-quiet"
    } else {
        ".ads-alert-dialog__actions .btn:not(.btn-quiet)"
    };
    let cancel = move || {
        if let Some(h) = &oncancel {
            h.call(());
        }
    };
    rsx! {
        div {
            class: "ads-alert-dialog",
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    e.prevent_default();
                    cancel();
                }
            },
            div { class: "ads-alert-dialog__scrim", "aria-hidden": "true" }
            div {
                class: "ads-alert-dialog__panel ads-modal",
                role: "alertdialog",
                "aria-modal": "true",
                "aria-labelledby": "{title_id}",
                "aria-describedby": if has_desc { Some(desc_id.clone()) } else { None },
                onmounted: move |_| hold_modal(&panel, first, ".btn"),
                h2 { id: "{title_id}", class: "ads-alert-dialog__title", "{title}" }
                if has_desc {
                    p { id: "{desc_id}", class: "ads-alert-dialog__desc", "{description}" }
                }
                {children}
                div { class: "ads-alert-dialog__actions",
                    button { r#type: "button", class: "btn btn-quiet", onclick: move |_| cancel(), "{cancel_label}" }
                    button {
                        r#type: "button",
                        class: "{action_class}",
                        onclick: move |_| {
                            if let Some(h) = &onaction {
                                h.call(());
                            }
                        },
                        "{action_label}"
                    }
                }
            }
        }
    }
}
