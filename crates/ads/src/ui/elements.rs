//! Avatar, separator, collapsible, and date picker.

use std::rc::Rc;

use dioxus::prelude::*;

use super::support::{date, focus, sleep, use_today, use_uid};
use super::widget2::AdsCalendar;
use crate::icons::Icon;

/// A soft circle with initials. It names the person (`role="img"`) unless it
/// sits beside their name already, when `decorative` hides it so the name is
/// not read twice.
#[component]
pub fn AdsAvatar(
    #[props(into)] name: String,
    #[props(default = 40u32)] size: u32,
    #[props(default = false)] decorative: bool,
) -> Element {
    let initials = name
        .split_whitespace()
        .filter_map(|p| p.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let font = (size * 4 + 5) / 10;
    rsx! {
        span {
            class: "ads-avatar",
            style: "width:{size}px;height:{size}px;font-size:{font}px",
            role: if decorative { None } else { Some("img") },
            "aria-label": if decorative { None } else { Some(name.clone()) },
            "aria-hidden": if decorative { Some("true") } else { None },
            span { "aria-hidden": "true", "{initials}" }
        }
    }
}

/// A hairline between sections. An `<hr>` is a separator already, so it
/// carries no role of its own.
#[component]
pub fn AdsSeparator() -> Element {
    rsx! { hr { class: "ads-separator" } }
}

/// A single collapsible on native details/summary, so it opens from the
/// keyboard and reads as a disclosure with no script. `open` is the state
/// and `ontoggle` reports each flip, the Rust spelling of `v-model:open`.
#[component]
pub fn AdsCollapsible(
    #[props(into)] title: String,
    #[props(default = false)] open: bool,
    #[props(optional)] ontoggle: Option<EventHandler<bool>>,
    children: Element,
) -> Element {
    let mut is_open = use_signal(|| open);
    use_effect(use_reactive((&open,), move |(open,)| {
        if *is_open.peek() != open {
            is_open.set(open);
        }
    }));
    let now = is_open();
    rsx! {
        details { class: "ads-collapsible", open: now,
            summary {
                class: "ads-collapsible__summary",
                // The summary's own click is the one toggle, so the state here
                // and the element's never disagree.
                onclick: move |e| {
                    e.prevent_default();
                    let next = !is_open();
                    is_open.set(next);
                    if let Some(h) = &ontoggle {
                        h.call(next);
                    }
                },
                "{title}"
            }
            div { class: "ads-collapsible__panel", {children} }
        }
    }
}

/// A date preset, as an offset in days from today, so "7 days ago" is always
/// a real date.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct DatePreset {
    pub label: String,
    pub offset: i32,
}

impl DatePreset {
    #[must_use]
    pub fn new(label: impl Into<String>, offset: i32) -> Self {
        Self {
            label: label.into(),
            offset,
        }
    }
}

fn default_presets() -> Vec<DatePreset> {
    vec![
        DatePreset::new("Today", 0),
        DatePreset::new("Yesterday", -1),
        DatePreset::new("7 days ago", -7),
    ]
}

/// One date, typed or picked. The field takes ISO (YYYY-MM-DD) in the data
/// face, so what you type is what you read back, and it says plainly when a
/// date will not parse. The calendar opens from its own labelled button in a
/// popover; Escape closes it and focus goes back to the button. Presets are
/// offsets from today.
#[component]
pub fn AdsDatePicker(
    #[props(into)] label: String,
    #[props(default = default_presets())] presets: Vec<DatePreset>,
    #[props(into, default)] hint: String,
    #[props(into, optional)] id: Option<String>,
    #[props(into, default)] value: String,
    #[props(optional)] onchange: Option<EventHandler<String>>,
) -> Element {
    let uid = use_uid();
    let field_id = id.filter(|s| !s.is_empty()).unwrap_or(uid);
    let today = use_today();
    let mut open = use_signal(|| false);
    // `None` mirrors the bound value; `Some` is what the reader is typing.
    let mut draft = use_signal(|| None::<String>);
    let mut invalid = use_signal(|| false);
    let mut inside = use_signal(|| false);
    let mut toggle_btn = use_signal(|| None::<Rc<MountedData>>);

    use_effect(use_reactive((&value,), move |_| {
        draft.set(None);
        invalid.set(false);
    }));

    let preset_iso = move |offset: i32| -> Option<String> {
        date::parse(&today()).map(|z| date::iso(z + i64::from(offset)))
    };
    let emit = move |v: String| {
        if let Some(h) = &onchange {
            h.call(v);
        }
    };
    let shown = draft().unwrap_or_else(|| value.clone());
    let commit = {
        let shown = shown.clone();
        move || {
            let s = shown.trim().to_owned();
            if s.is_empty() {
                invalid.set(false);
                emit(String::new());
                return;
            }
            let bad = date::parse(&s).is_none();
            invalid.set(bad);
            if !bad {
                emit(s);
            }
        }
    };
    let mut hide = move |refocus: bool| {
        open.set(false);
        if refocus {
            focus(toggle_btn.peek().clone());
        }
    };

    let is_invalid = invalid();
    let err_id = format!("{field_id}-err");
    let hint_id = format!("{field_id}-hint");
    let described: Vec<&str> = [
        is_invalid.then_some(err_id.as_str()),
        (!hint.is_empty()).then_some(hint_id.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect();
    let described = (!described.is_empty()).then(|| described.join(" "));
    let input_class = crate::class::join(&[
        "ads-date-picker__input ads-field num",
        if is_invalid { "is-invalid" } else { "" },
    ]);
    let example = preset_iso(0).unwrap_or_else(|| "2026-10-27".to_owned());
    let is_open = open();
    let mut commit_enter = commit.clone();
    let mut commit_change = commit;

    rsx! {
        div {
            class: "ads-date-picker ads-field-group",
            onkeydown: move |e| {
                if e.key() == Key::Escape && open() {
                    e.prevent_default();
                    e.stop_propagation();
                    hide(true);
                }
            },
            onfocusin: move |_| inside.set(true),
            onfocusout: move |_| {
                inside.set(false);
                spawn(async move {
                    sleep(0).await;
                    if !*inside.peek() && *open.peek() {
                        open.set(false);
                    }
                });
            },
            label { class: "ads-label", r#for: "{field_id}", "{label}" }
            div { class: "ads-date-picker__row",
                input {
                    id: "{field_id}",
                    class: "{input_class}",
                    r#type: "text",
                    inputmode: "numeric",
                    placeholder: "YYYY-MM-DD",
                    autocomplete: "off",
                    value: "{shown}",
                    "aria-invalid": if is_invalid { Some("true") } else { None },
                    "aria-describedby": described,
                    oninput: move |e| draft.set(Some(e.value())),
                    onchange: move |_| commit_change(),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            e.prevent_default();
                            commit_enter();
                        }
                    },
                }
                button {
                    r#type: "button",
                    class: "ads-icon-button",
                    "aria-label": "Choose date from calendar",
                    "aria-haspopup": "dialog",
                    "aria-expanded": if is_open { "true" } else { "false" },
                    onmounted: move |e| toggle_btn.set(Some(e.data())),
                    onclick: move |_| {
                        if open() {
                            hide(true);
                        } else {
                            open.set(true);
                        }
                    },
                    Icon { name: "compass", size: 16 }
                }
                if is_open {
                    div { class: "ads-date-picker__pop", role: "dialog", "aria-label": "{label}: calendar",
                        AdsCalendar {
                            value: value.clone(),
                            label: label.clone(),
                            autofocus: true,
                            onchange: move |v: String| {
                                emit(v);
                                hide(true);
                            },
                        }
                    }
                }
            }
            if is_invalid {
                span { id: "{err_id}", class: "ads-hint", role: "alert",
                    "Use a real date as YYYY-MM-DD, for example {example}."
                }
            }
            if !hint.is_empty() {
                span { id: "{hint_id}", class: "ads-hint", "{hint}" }
            }
            if !presets.is_empty() {
                div { class: "ads-segmented", role: "group", "aria-label": "{label} presets",
                    for p in presets.iter() {
                        {
                            let iso = preset_iso(p.offset);
                            let on = iso.as_deref() == Some(value.as_str()) && !value.is_empty();
                            rsx! {
                                button {
                                    key: "{p.label}",
                                    r#type: "button",
                                    class: if on { "is-on" } else { "" },
                                    "aria-pressed": if on { "true" } else { "false" },
                                    onclick: move |_| {
                                        if let Some(v) = iso.clone() {
                                            emit(v);
                                        }
                                    },
                                    "{p.label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
