//! Field-side components: attachment, code block, native select, input group,
//! input otp, select, combobox.

use dioxus::prelude::*;

use crate::icons::Icon;

/// A file attachment chip.
#[component]
pub fn AdsAttachment(
    #[props(into)] name: String,
    #[props(into, optional)] size: Option<String>,
    #[props(default = true)] removable: bool,
    #[props(optional)] onremove: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        span { class: "attachment",
            span { class: "att-icon", Icon { name: "code", size: 15 } }
            span { class: "att-text",
                span { class: "att-name", "{name}" }
                if let Some(s) = &size { span { class: "att-size num", "{s}" } }
            }
            if removable {
                button { class: "att-remove", r#type: "button", "aria-label": "Remove", onclick: move |e| { if let Some(h) = &onremove { h.call(e); } }, "×" }
            }
        }
    }
}

/// A code block with a copy button.
#[component]
pub fn AdsCodeBlock(
    #[props(into)] code: String,
) -> Element {
    rsx! {
        div { class: "codeblock",
            button { class: "cb-btn", r#type: "button", "Copy" }
            pre { class: "cb-pre", code { "{code}" } }
        }
    }
}

/// A native select, styled as a well.
#[component]
pub fn AdsNativeSelect(
    #[props(into)] options: Vec<(String, String)>,
    #[props(into, optional)] value: Option<String>,
    #[props(into, default = "Choose…".to_string())] placeholder: String,
    #[props(optional)] onchange: Option<EventHandler<FormEvent>>,
) -> Element {
    rsx! {
        select { class: "ads-field select", value: value, onchange: move |e| { if let Some(h) = &onchange { h.call(e); } },
            option { value: "", disabled: "true", "{placeholder}" }
            for (v, l) in &options { option { value: "{v}", "{l}" } }
        }
    }
}

/// An input with a prefix/suffix addon.
#[component]
pub fn AdsInputGroup(
    #[props(into, optional)] prefix: Option<String>,
    #[props(into, optional)] suffix: Option<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] value: Option<String>,
    #[props(optional)] oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    rsx! {
        div { class: "input-group",
            if let Some(p) = &prefix { span { class: "addon", "{p}" } }
            input {
                class: "ig-input",
                r#type: "text",
                value: value,
                placeholder: placeholder,
                oninput: move |e| { if let Some(h) = &oninput { h.call(e); } },
            }
            if let Some(s) = &suffix { span { class: "addon", "{s}" } }
        }
    }
}

/// A one-time-password input.
#[component]
pub fn AdsInputOtp(
    #[props(default = 6u8)] length: u8,
    #[props(into, optional)] value: Option<String>,
    #[props(optional)] onchange: Option<EventHandler<String>>,
) -> Element {
    let value = value.unwrap_or_default();
    let chars: Vec<char> = value.chars().collect();
    rsx! {
        div { class: "otp", role: "group", "aria-label": "One-time code",
            for i in 0..length {
                input {
                    class: "otp-box num",
                    inputmode: "numeric",
                    maxlength: "1",
                    value: chars.get(i as usize).map_or_else(String::new, |c| c.to_string()),
                    "aria-label": "Digit {i + 1}",
                }
            }
        }
    }
}

/// A custom select: a button that opens a soft menu.
#[component]
pub fn AdsSelect(
    #[props(into)] options: Vec<(String, String)>,
    #[props(into, optional)] value: Option<String>,
    #[props(into, default = "Choose…".to_string())] placeholder: String,
    #[props(optional)] onchange: Option<EventHandler<String>>,
) -> Element {
    let mut open = use_signal(|| false);
    let label = options
        .iter()
        .find(|(v, _)| Some(v) == value.as_ref())
        .map_or_else(|| placeholder.clone(), |(_, l)| l.clone());

    rsx! {
        div { class: "select",
            button { class: "select-btn", r#type: "button", "aria-expanded": if *open.read() { "true" } else { "false" }, onclick: move |_| open.toggle(),
                span { "{label}" }
                Icon { name: "chevron", size: 14 }
            }
            if *open.read() {
                div { class: "select-menu",
                    for (v, l) in &options {
                        button {
                            class: "opt",
                            class: if Some(v) == value.as_ref() { "opt on" } else { "opt" },
                            r#type: "button",
                            onclick: { let h = onchange; let val = v.clone(); move |_| { if let Some(h) = &h { h.call(val.clone()); } } },
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}

/// A combobox: a text field that filters a list as you type.
#[component]
pub fn AdsCombobox(
    #[props(into)] options: Vec<(String, String)>,
    #[props(into, default = "Search…".to_string())] placeholder: String,
    #[props(optional)] onchange: Option<EventHandler<String>>,
) -> Element {
    let mut query = use_signal(String::new);
    let q = query.read();
    let qs = q.as_str();
    let filtered = options.iter().filter(|(_, l)| l.to_lowercase().contains(&qs.to_lowercase())).collect::<Vec<_>>();

    rsx! {
        div { class: "combo",
            input { class: "combo-input", r#type: "text", placeholder: "{placeholder}", value: "{qs}", oninput: move |e| query.set(e.value()) }
            if !filtered.is_empty() {
                div { class: "combo-menu",
                    for (v, l) in &filtered {
                        button {
                            class: "opt",
                            r#type: "button",
                            onclick: { let h = onchange; let val = v.clone(); move |_| { if let Some(h) = &h { h.call(val.clone()); } } },
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}
