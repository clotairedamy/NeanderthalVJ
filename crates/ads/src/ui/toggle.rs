//! Toggle button and toggle group: settings that flip and stay flipped,
//! announced as pressed rather than painted in the accent.

use dioxus::prelude::*;

use crate::icons::Icon;

/// A single toggle button, for a setting that flips and stays that way. On is
/// pressed into the page, not filled with the accent, because the accent is
/// kept for the one thing to click. An icon-only toggle needs a `label`, since
/// a glyph has no name of its own.
#[component]
pub fn AdsToggle(
    #[props(default = false)] checked: bool,
    #[props(into, optional)] icon: Option<String>,
    #[props(default = 16u32)] size: u32,
    #[props(into, optional)] label: Option<String>,
    #[props(optional)] onchange: Option<EventHandler<bool>>,
    children: Element,
) -> Element {
    let class = crate::class::join(&["ads-toggle", if checked { "is-on" } else { "" }]);
    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            "aria-pressed": if checked { "true" } else { "false" },
            "aria-label": label,
            onclick: move |_| {
                if let Some(h) = &onchange {
                    h.call(!checked);
                }
            },
            if let Some(n) = &icon { Icon { name: n.clone(), size } }
            {children}
        }
    }
}

/// One option in a toggle group.
#[derive(Clone, PartialEq, Debug)]
pub struct ToggleOption {
    pub label: String,
    pub value: String,
}

impl ToggleOption {
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// A set of toggle buttons, any number on at once. It is a labelled group so
/// a screen reader says what the buttons are for before it reads them, and
/// each button reports its own pressed state.
#[component]
pub fn AdsToggleGroup(
    #[props(into)] label: String,
    options: Vec<ToggleOption>,
    #[props(default)] selected: Vec<String>,
    #[props(optional)] onchange: Option<EventHandler<Vec<String>>>,
) -> Element {
    rsx! {
        div { class: "ads-toggle-group", role: "group", "aria-label": "{label}",
            for o in options.iter() {
                {
                    let on = selected.contains(&o.value);
                    let v = o.value.clone();
                    let sel = selected.clone();
                    rsx! {
                        button {
                            key: "{o.value}",
                            r#type: "button",
                            class: crate::class::join(&["ads-toggle-group__button", if on { "is-on" } else { "" }]),
                            "aria-pressed": if on { "true" } else { "false" },
                            onclick: move |_| {
                                let mut next = sel.clone();
                                if next.contains(&v) {
                                    next.retain(|x| x != &v);
                                } else {
                                    next.push(v.clone());
                                }
                                if let Some(h) = &onchange {
                                    h.call(next);
                                }
                            },
                            "{o.label}"
                        }
                    }
                }
            }
        }
    }
}
