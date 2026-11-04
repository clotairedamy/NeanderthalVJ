//! Badge, status, delta, and tag.

use dioxus::prelude::*;

/// A count or code pill. The accent variant is the one number on screen that
/// matters; the quiet variant sits inside the chrome. A bare "5" means nothing
/// to a screen reader, so `label` says what is being counted ("unread").
#[component]
pub fn AdsBadge(
    #[props(into)] value: String,
    #[props(into, optional)] label: Option<String>,
    #[props(optional)] quiet: bool,
) -> Element {
    let class = crate::class::join(&["ads-badge num", if quiet { "is-quiet" } else { "" }]);
    rsx! {
        span { class: "{class}",
            "{value}"
            if let Some(l) = label.as_deref().filter(|l| !l.is_empty()) {
                span { class: "sr-only", " {l}" }
            }
        }
    }
}

/// A single state outside dense data: the three semantic colours, spoken for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Ok,
    Warn,
    Bad,
}

impl StatusTone {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Bad => "bad",
        }
    }
}

/// Severity in a monochrome system is luminance: the worse it is, the brighter
/// it is. The glyph (dot) carries it too, and the label is required, so the
/// rank survives a greyscale print and a colour-vision deficiency alike.
///
/// `grade` (1–4) is for dense data — a severity column, a heat grid. `tone` is
/// for a single state outside it (a health check, a job), where the three
/// semantic colours are allowed; it wins over `grade` when both are given.
/// There is no colour prop on purpose: state has three colours and they are
/// spoken for.
#[component]
pub fn AdsStatus(
    #[props(into)] label: String,
    #[props(default = 1u8)] grade: u8,
    #[props(optional)] tone: Option<StatusTone>,
) -> Element {
    let modifier = match tone {
        Some(t) => format!("is-{}", t.as_str()),
        None => format!("g{}", grade.clamp(1, 4)),
    };
    let class = crate::class::join(&["ads-status", &modifier]);
    rsx! {
        span { class: "{class}",
            span { class: "ads-dot", "aria-hidden": "true" }
            "{label}"
        }
    }
}

/// A change against the previous period. The direction glyph is mandatory — a
/// delta without one is a number nobody can read at speed — so `direction` is
/// required rather than defaulted, and it is spoken as a word too, because a
/// screen reader does not reliably read a CSS triangle.
#[component]
pub fn AdsDelta(
    #[props(into)] value: String,
    direction: DeltaDirection,
) -> Element {
    let modifier = format!("is-{}", direction.as_str());
    let class = crate::class::join(&["ads-delta num", &modifier]);
    rsx! {
        span { class: "{class}",
            span { class: "sr-only", "{direction.spoken()} " }
            "{value}"
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum DeltaDirection {
    Up,
    Down,
    #[default]
    Flat,
}

impl DeltaDirection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Flat => "flat",
        }
    }

    /// The word a screen reader says in place of the triangle.
    #[must_use]
    pub const fn spoken(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Flat => "unchanged",
        }
    }
}

/// A filter the reader applied and can take off again. Always removable: the
/// dismiss affordance is the whole difference from a status chip, which
/// describes the data and cannot be removed — for that, use `AdsStatus`.
#[component]
pub fn AdsTag(
    #[props(into)] label: String,
    #[props(optional)] onremove: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        span { class: "ads-tag",
            "{label}"
            button {
                r#type: "button",
                "aria-label": "Remove filter {label}",
                onclick: move |e| {
                    if let Some(h) = &onremove {
                        h.call(e);
                    }
                },
                "×"
            }
        }
    }
}
