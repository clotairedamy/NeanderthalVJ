//! Theme lab — live token controls (theme, density, radius, depth).

use dioxus::prelude::*;

/// The colour theme.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// How tightly the page is set.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

impl Density {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable",
            Self::Compact => "compact",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Comfortable => "Comfortable",
            Self::Compact => "Compact",
        }
    }
}

/// The token overrides for a radius, depth and density, as CSS declarations
/// to set on the root element (or any ancestor of what should retheme).
///
/// The shadow pair is the app's Soft.raised as a function of depth: an 8px
/// offset and a 28px blur at depth 1. Two darks rather than one — a tight
/// contact shadow that reads as thickness, and a wide fall that seats the
/// shape on the page.
#[must_use]
pub fn theme_lab_tokens(radius: f32, depth: f32, density: Density) -> String {
    let o = 8.0 * depth;
    let b = 28.0 * depth;
    let r = |n: f32| format!("{n:.2}");
    let raised = format!(
        "{}px {}px {}px var(--shade), {}px {}px {}px var(--shade-soft), {}px {}px {}px var(--highlight)",
        r(o * 0.45), r(o * 0.6), r(b * 0.3), r(o), r(o), r(b), r(-o * 0.65), r(-o * 0.65), r(b * 0.75)
    );
    let raised_sm = format!(
        "{}px {}px {}px var(--shade), {}px {}px {}px var(--shade-soft), {}px {}px {}px var(--highlight)",
        r(o * 0.22), r(o * 0.3), r(b * 0.15), r(o * 0.5), r(o * 0.5), r(b * 0.5), r(-o * 0.32), r(-o * 0.32), r(b * 0.38)
    );
    let raised_lg = format!(
        "{}px {}px {}px var(--shade), {}px {}px {}px var(--shade-soft), {}px {}px {}px var(--highlight)",
        r(o * 0.72), r(o * 0.96), r(b * 0.48), r(o * 1.6), r(o * 1.6), r(b * 1.6), r(-o * 1.04), r(-o * 1.04), r(b * 1.2)
    );
    let inset = format!(
        "inset {}px {}px {}px var(--shade), inset {}px {}px {}px var(--highlight)",
        r(o * 0.4), r(o * 0.4), r(b * 0.29), r(-o * 0.4), r(-o * 0.4), r(b * 0.29)
    );
    let inset_sm = format!(
        "inset {}px {}px {}px var(--shade), inset {}px {}px {}px var(--highlight)",
        r(o * 0.25), r(o * 0.25), r(b * 0.18), r(-o * 0.25), r(-o * 0.25), r(b * 0.18)
    );
    let compact = density == Density::Compact;
    format!(
        "--radius:{}px;--radius-lg:{}px;--radius-sm:{}px;--soft-raised:{raised};--soft-raised-sm:{raised_sm};--soft-raised-lg:{raised_lg};--soft-inset:{inset};--soft-inset-sm:{inset_sm};--gutter:{};--row-h:{}",
        radius,
        (radius * 1.3).round(),
        (radius * 0.65).round().max(4.0),
        if compact { "16px" } else { "24px" },
        if compact { "28px" } else { "34px" },
    )
}

/// The theme lab.
///
/// Every token is a CSS custom property, which means the whole site can be
/// rethemed live from four controls. That is the entire argument for building
/// a design system as tokens rather than as a set of finished screens, and
/// it is more convincing to hand someone the sliders than to describe it.
///
/// Depth is the interesting one: it scales the two shadows that make this
/// style work. At zero the page goes flat and you can see exactly how much of
/// the design was doing its job through elevation alone.
///
/// Controlled: the parent keeps the values and applies them — set
/// `data-theme`, and put `theme_lab_tokens(radius, depth, density)` in the
/// root's style. "Reset tokens" calls `onreset`; go back to light, 20px,
/// depth 1, comfortable.
#[component]
pub fn ThemeLab(
    #[props(default)] theme: ThemeMode,
    #[props(default)] density: Density,
    #[props(default = 20.0f32)] radius: f32,
    #[props(default = 1.0f32)] depth: f32,
    #[props(optional)] ontheme: Option<EventHandler<ThemeMode>>,
    #[props(optional)] ondensity: Option<EventHandler<Density>>,
    #[props(optional)] onradius: Option<EventHandler<f32>>,
    #[props(optional)] ondepth: Option<EventHandler<f32>>,
    #[props(optional)] onreset: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div { class: "card ads-theme-lab",
            div { class: "ads-theme-lab__head",
                div {
                    span { class: "eyebrow", "Live tokens" }
                    h3 { class: "h3", "Retheme this page." }
                }
                button {
                    class: "btn btn-quiet",
                    r#type: "button",
                    onclick: move |_| { if let Some(h) = &onreset { h.call(()); } },
                    "Reset tokens"
                }
            }
            div { class: "ads-theme-lab__controls",
                div { class: "ads-theme-lab__control",
                    span { id: "lab-theme", class: "eyebrow ads-theme-lab__label", "Theme" }
                    div { class: "ads-segmented ads-theme-lab__segmented", role: "group", "aria-labelledby": "lab-theme",
                        for t in [ThemeMode::Light, ThemeMode::Dark] {
                            button {
                                key: "{t.as_str()}",
                                r#type: "button",
                                class: if theme == t { "is-on" } else { "" },
                                "aria-pressed": if theme == t { "true" } else { "false" },
                                onclick: move |_| { if let Some(h) = &ontheme { h.call(t); } },
                                "{t.label()}"
                            }
                        }
                    }
                }
                div { class: "ads-theme-lab__control",
                    span { id: "lab-density", class: "eyebrow ads-theme-lab__label", "Density" }
                    div { class: "ads-segmented ads-theme-lab__segmented", role: "group", "aria-labelledby": "lab-density",
                        for d in [Density::Comfortable, Density::Compact] {
                            button {
                                key: "{d.as_str()}",
                                r#type: "button",
                                class: if density == d { "is-on" } else { "" },
                                "aria-pressed": if density == d { "true" } else { "false" },
                                onclick: move |_| { if let Some(h) = &ondensity { h.call(d); } },
                                "{d.label()}"
                            }
                        }
                    }
                }
                div { class: "ads-theme-lab__control",
                    label { class: "eyebrow ads-theme-lab__label", r#for: "lab-radius",
                        "Corner radius "
                        span { class: "num ads-theme-lab__readout", "{radius}px" }
                    }
                    input {
                        id: "lab-radius",
                        class: "ads-range",
                        r#type: "range",
                        min: "4",
                        max: "34",
                        step: "1",
                        value: "{radius}",
                        oninput: move |e| {
                            if let (Some(h), Ok(v)) = (&onradius, e.value().parse::<f32>()) {
                                h.call(v);
                            }
                        },
                    }
                }
                div { class: "ads-theme-lab__control",
                    label { class: "eyebrow ads-theme-lab__label", r#for: "lab-depth",
                        "Shadow depth "
                        span { class: "num ads-theme-lab__readout", "{depth:.2}×" }
                    }
                    input {
                        id: "lab-depth",
                        class: "ads-range",
                        r#type: "range",
                        min: "0",
                        max: "2",
                        step: "0.05",
                        value: "{depth}",
                        oninput: move |e| {
                            if let (Some(h), Ok(v)) = (&ondepth, e.value().parse::<f32>()) {
                                h.call(v);
                            }
                        },
                    }
                }
            }
            p { class: "body ads-theme-lab__foot",
                "Four properties, one stylesheet, every surface on the site. Drag depth to zero and watch how much of this design was elevation doing the work."
            }
        }
    }
}
