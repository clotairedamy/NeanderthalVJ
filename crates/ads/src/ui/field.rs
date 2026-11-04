//! Text field, textarea and select, and the date range.

use dioxus::prelude::*;

use super::group::SegOption;
use crate::icons::Icon;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    #[default]
    Text,
    Email,
    Password,
    Tel,
    Number,
    Search,
    Date,
    Textarea,
    Select,
}

impl FieldType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Password => "password",
            Self::Tel => "tel",
            Self::Number => "number",
            Self::Search => "search",
            Self::Date => "date",
            Self::Textarea => "textarea",
            Self::Select => "select",
        }
    }
}

/// A labelled field: input, textarea or select, with hint and error.
/// Inputs are wells — the card shape pressed into the page. The label is
/// required and always visible: a placeholder disappears exactly when it is
/// needed, and fails hardest for screen readers. The error replaces the hint
/// rather than sitting beside it, and carries a glyph so it is not read by
/// colour alone. Figures — numbers, dates, phone numbers — are set in the
/// data face. A select's `options` are label/value pairs.
#[component]
pub fn AdsField(
    #[props(into)] label: String,
    #[props(into, optional)] value: Option<String>,
    #[props(default)] field_type: FieldType,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] name: Option<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] hint: Option<String>,
    #[props(into, optional)] error: Option<String>,
    #[props(into, optional)] options: Option<Vec<SegOption>>,
    #[props(default = 4u32)] rows: u32,
    #[props(into, optional)] autocomplete: Option<String>,
    #[props(optional)] required: bool,
    #[props(optional)] disabled: bool,
    #[props(optional)] oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    let uid = crate::blocks::use_uid("ads-field");
    let field_id = id.filter(|i| !i.is_empty()).unwrap_or(uid);
    let msg_id = format!("{field_id}-msg");
    let error = error.filter(|e| !e.is_empty());
    let hint = hint.filter(|h| !h.is_empty());
    let described_by = (error.is_some() || hint.is_some()).then(|| msg_id.clone());
    let invalid = error.is_some().then_some("true");
    let numeric = matches!(
        field_type,
        FieldType::Number | FieldType::Date | FieldType::Tel
    );
    let is_invalid = if error.is_some() { "is-invalid" } else { "" };

    let control = match field_type {
        FieldType::Textarea => rsx! {
            textarea {
                id: "{field_id}",
                name: name,
                class: crate::class::join(&["ads-field", is_invalid]),
                value: value,
                placeholder: placeholder,
                rows: "{rows}",
                required: required,
                disabled: disabled,
                "aria-invalid": invalid,
                "aria-describedby": described_by,
                oninput: move |e| { if let Some(h) = &oninput { h.call(e); } },
            }
        },
        FieldType::Select => {
            let current = value.unwrap_or_default();
            rsx! {
                select {
                    id: "{field_id}",
                    name: name,
                    class: crate::class::join(&["ads-field", is_invalid]),
                    required: required,
                    disabled: disabled,
                    "aria-invalid": invalid,
                    "aria-describedby": described_by,
                    onchange: move |e| { if let Some(h) = &oninput { h.call(e); } },
                    if let Some(p) = placeholder.filter(|p| !p.is_empty()) {
                        option { value: "", disabled: true, selected: current.is_empty(), "{p}" }
                    }
                    for o in options.unwrap_or_default() {
                        option { key: "{o.value}", value: "{o.value}", selected: o.value == current, "{o.label}" }
                    }
                }
            }
        }
        _ => rsx! {
            input {
                id: "{field_id}",
                name: name,
                r#type: field_type.as_str(),
                class: crate::class::join(&[
                    "ads-field ads-field__input",
                    is_invalid,
                    if numeric { "is-numeric" } else { "" },
                    if field_type == FieldType::Number { "is-number" } else { "" },
                ]),
                value: value,
                placeholder: placeholder,
                required: required,
                disabled: disabled,
                autocomplete: autocomplete,
                "aria-invalid": invalid,
                "aria-describedby": described_by,
                oninput: move |e| { if let Some(h) = &oninput { h.call(e); } },
            }
        },
    };

    rsx! {
        div { class: "ads-field-group",
            label { r#for: "{field_id}", class: "ads-label", "{label}" }
            {control}
            if let Some(err) = &error {
                p { id: "{msg_id}", class: "ads-hint ads-field__error",
                    Icon { name: "alert", size: 13, class: "ads-field__error-glyph" }
                    "{err}"
                }
            } else if let Some(h) = &hint {
                p { id: "{msg_id}", class: "ads-hint ads-field__hint", "{h}" }
            }
        }
    }
}

fn default_presets() -> Vec<String> {
    ["7d", "30d", "90d", "Custom"]
        .into_iter()
        .map(String::from)
        .collect()
}

/// The most-used control on any dashboard. ISO dates in the data face so
/// ranges sort and compare visually, presets beside them because most readers
/// want "last 30 days", not a calendar. The whole control is one named group,
/// so a screen reader hears what the dates and presets belong to.
#[component]
pub fn AdsDaterange(
    #[props(into, default = "2026-09-25".to_string())] from: String,
    #[props(into, default = "2026-10-25".to_string())] to: String,
    #[props(into, optional)] presets: Option<Vec<String>>,
    #[props(into, default = "30d".to_string())] selected: String,
    #[props(into, default = "Date range".to_string())] label: String,
    #[props(optional)] onselect: Option<EventHandler<String>>,
) -> Element {
    let presets = presets.unwrap_or_else(default_presets);
    rsx! {
        div { class: "stack gap-2", role: "group", "aria-label": "{label}",
            div { class: "ads-daterange",
                Icon { name: "compass", size: 15 }
                time { class: "num", datetime: "{from}", "{from}" }
                span { class: "sep", "aria-hidden": "true", "→" }
                span { class: "sr-only", "to" }
                time { class: "num", datetime: "{to}", "{to}" }
            }
            if !presets.is_empty() {
                div { class: "ads-segmented", role: "group", "aria-label": "Presets",
                    for p in presets.iter().cloned() {
                        button {
                            key: "{p}",
                            r#type: "button",
                            class: if p == selected { "num is-on" } else { "num" },
                            "aria-pressed": if p == selected { "true" } else { "false" },
                            onclick: {
                                let p = p.clone();
                                move |_| { if let Some(h) = &onselect { h.call(p.clone()); } }
                            },
                            "{p}"
                        }
                    }
                }
            }
        }
    }
}
