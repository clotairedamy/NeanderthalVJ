//! Checkbox / radio / switch, range, and number stepper.

use dioxus::prelude::*;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceKind {
    #[default]
    Checkbox,
    Radio,
    Switch,
}

impl ChoiceKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Switch => "switch",
        }
    }
}

/// Checkbox, radio or switch. A switch is for a setting that takes effect
/// immediately; a checkbox is for a value that is submitted — so a switch
/// belongs outside a form, or it should be a checkbox. The label is visible
/// and inside the hit target; pass `label`, or children when it needs a link.
/// `onchange` receives the new checked state.
#[component]
pub fn AdsChoice(
    #[props(into, optional)] label: Option<String>,
    #[props(optional)] checked: bool,
    #[props(default)] kind: ChoiceKind,
    #[props(into, optional)] name: Option<String>,
    #[props(into, optional)] value: Option<String>,
    #[props(optional)] disabled: bool,
    #[props(optional)] onchange: Option<EventHandler<bool>>,
    children: Element,
) -> Element {
    let input_type = if kind == ChoiceKind::Radio {
        "radio"
    } else {
        "checkbox"
    };
    let class = crate::class::join(&["ads-choice", if disabled { "is-disabled" } else { "" }]);
    rsx! {
        label { class: "{class}",
            input {
                r#type: input_type,
                role: if kind == ChoiceKind::Switch { Some("switch") } else { None },
                name: name,
                value: value,
                checked: checked,
                disabled: disabled,
                // A radio only fires `change` when it becomes the selected
                // one, and Dioxus reports its `value` ("7") where a checkbox
                // reports "true" -- so for a radio the event is the answer.
                onchange: move |e: FormEvent| {
                    if let Some(h) = &onchange {
                        h.call(kind == ChoiceKind::Radio || e.checked());
                    }
                },
            }
            if kind == ChoiceKind::Switch {
                span { class: "ads-switch" }
            } else {
                span { class: if kind == ChoiceKind::Radio { "ads-box is-round" } else { "ads-box" },
                    if kind == ChoiceKind::Checkbox {
                        svg {
                            view_box: "0 0 24 24",
                            width: "13",
                            height: "13",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.6",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            "aria-hidden": "true",
                            path { d: "M4 12.5l5 5L20 6.5" }
                        }
                    } else {
                        span { class: "ads-choice__dot" }
                    }
                }
            }
            span { class: "ads-choice__label",
                if let Some(l) = &label { "{l}" } else { {children} }
            }
        }
    }
}

/// A slider, pressed into the page, with an accent thumb. For a value where
/// the approximate position matters more than the exact figure — but the
/// figure is shown anyway, always, in the data face beside the label, because
/// somebody always needs it. The label is required and visible; it carries
/// the unit so the readout holds only the number.
#[component]
pub fn AdsRange(
    #[props(into)] label: String,
    #[props(default = 0i64)] value: i64,
    #[props(default = 0i64)] min: i64,
    #[props(default = 100i64)] max: i64,
    #[props(default = 1i64)] step: i64,
    #[props(optional)] oninput: Option<EventHandler<i64>>,
) -> Element {
    let uid = crate::blocks::use_uid("ads-range");
    rsx! {
        div { class: "ads-field-group",
            div { class: "ads-range__head",
                label { r#for: "{uid}", class: "ads-label", "{label}" }
                output { "for": "{uid}", class: "num ads-range__value", "{value}" }
            }
            input {
                id: "{uid}",
                class: "ads-range",
                r#type: "range",
                value: "{value}",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                oninput: move |e: FormEvent| {
                    if let (Some(h), Ok(v)) = (&oninput, e.value().parse::<i64>()) {
                        h.call(v);
                    }
                },
            }
        }
    }
}

/// A stepper: a figure with up/down buttons, right-aligned in the data face
/// so it lines up with the values it will be compared against. The label is
/// required and visible, and carries the unit — "Max URLs", "Timeout (s)" —
/// so the field holds only the figure. The steppers stop at the bounds.
#[component]
pub fn AdsNumber(
    #[props(into)] label: String,
    #[props(default = 0i64)] value: i64,
    #[props(default = 0i64)] min: i64,
    #[props(default = i64::MAX)] max: i64,
    #[props(default = 1i64)] step: i64,
    #[props(optional)] oninput: Option<EventHandler<i64>>,
) -> Element {
    let uid = crate::blocks::use_uid("ads-number");
    let bump = move |dir: i64| {
        if let Some(h) = &oninput {
            h.call(
                value
                    .saturating_add(dir.saturating_mul(step))
                    .clamp(min, max),
            );
        }
    };
    rsx! {
        div { class: "ads-field-group",
            label { r#for: "{uid}", class: "ads-label", "{label}" }
            div { class: "ads-number",
                input {
                    id: "{uid}",
                    r#type: "number",
                    value: "{value}",
                    min: "{min}",
                    max: if max < i64::MAX { Some(max.to_string()) } else { None },
                    step: "{step}",
                    oninput: move |e: FormEvent| {
                        // An emptied field is mid-edit, not zero.
                        if let (Some(h), Ok(v)) = (&oninput, e.value().parse::<i64>()) {
                            h.call(v);
                        }
                    },
                }
                div { class: "steps",
                    button {
                        r#type: "button",
                        "aria-label": "Increase {label}",
                        disabled: value >= max,
                        onclick: move |_| bump(1),
                        "▲"
                    }
                    button {
                        r#type: "button",
                        "aria-label": "Decrease {label}",
                        disabled: value <= min,
                        onclick: move |_| bump(-1),
                        "▼"
                    }
                }
            }
        }
    }
}
