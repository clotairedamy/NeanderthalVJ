//! The ADS button and icon button.

use dioxus::prelude::*;

use crate::icons::Icon;

/// How much weight a button carries. `Accent` is the one action per screen.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Accent,
    Quiet,
}

impl ButtonVariant {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Accent => "accent",
            Self::Quiet => "quiet",
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Md,
    Sm,
}

impl ButtonSize {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Md => "md",
            Self::Sm => "sm",
        }
    }
}

/// The native `type` of a button. `Button` by default, so a button inside a
/// form never submits it by accident.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonType {
    #[default]
    Button,
    Submit,
    Reset,
}

impl ButtonType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

/// The ADS button. `.btn` is the raised soft surface; the accent variant is
/// the one action per screen, so spend it once. A destructive action never
/// gets the accent — the accent means "the thing to do", and deleting is not
/// it — so `destructive` pins the button to the default weight whatever
/// variant is asked for. The confirm that follows it says the verb, not "OK".
#[component]
pub fn AdsButton(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(into, optional)] icon: Option<String>,
    #[props(default = 16u32)] icon_size: u32,
    #[props(optional)] loading: bool,
    #[props(optional)] destructive: bool,
    #[props(default)] button_type: ButtonType,
    #[props(optional)] disabled: bool,
    #[props(into, optional)] class: Option<String>,
    /// A tooltip: what the button does, for a button whose face is terse.
    #[props(into, optional)] title: Option<String>,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let weight = if destructive && variant == ButtonVariant::Accent {
        ButtonVariant::Default
    } else {
        variant
    };
    let merged = crate::class::join(&[
        "btn",
        if weight == ButtonVariant::Accent {
            "btn-accent"
        } else {
            ""
        },
        if weight == ButtonVariant::Quiet {
            "btn-quiet"
        } else {
            ""
        },
        if size == ButtonSize::Sm { "btn-sm" } else { "" },
        class.as_deref().unwrap_or(""),
    ]);
    rsx! {
        button {
            class: "{merged}",
            r#type: button_type.as_str(),
            title: title,
            disabled: disabled || loading,
            "aria-busy": if loading { Some("true") } else { None },
            onclick: move |e| { if let Some(h) = &onclick { h.call(e); } },
            if loading {
                span { class: "ads-spinner ads-button__spinner", "aria-hidden": "true" }
            } else if let Some(n) = &icon {
                Icon { name: n.clone(), size: icon_size }
            }
            {children}
        }
    }
}

/// A button that holds only a glyph. Round or square, accent or soft. The
/// label is required: with no words on the face, the name is all a screen
/// reader has.
#[component]
pub fn AdsIconButton(
    #[props(into)] icon: String,
    #[props(into)] label: String,
    #[props(optional)] round: bool,
    #[props(optional)] accent: bool,
    #[props(default = 18u32)] size: u32,
    #[props(default)] button_type: ButtonType,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let merged = crate::class::join(&[
        "ads-icon-button",
        if round { "is-round" } else { "" },
        if accent { "is-accent" } else { "" },
    ]);
    rsx! {
        button {
            class: "{merged}",
            r#type: button_type.as_str(),
            "aria-label": label,
            onclick: move |e| { if let Some(h) = &onclick { h.call(e); } },
            Icon { name: icon, size }
        }
    }
}
