//! Small atomics: aspect ratio, bubble, item, marker, direction, label, kbd,
//! spinner, typography.

use dioxus::prelude::*;

use super::support::glyph;

/// A box that holds its proportions while its width changes, so an image or a
/// chart reserves its space before it loads and nothing below it jumps.
#[component]
pub fn AdsAspectRatio(
    #[props(into, default = "16 / 9".to_string())] ratio: String,
    children: Element,
) -> Element {
    rsx! { div { class: "ads-aspect-ratio", style: "aspect-ratio:{ratio}", {children} } }
}

/// A chat bubble. Theirs is a well; `mine` stands proud of the page instead,
/// so depth and alignment tell the two voices apart. Not the accent: a thread
/// would spend it on every other line, and the accent is for the one thing to
/// click.
#[component]
pub fn AdsBubble(#[props(default = false)] mine: bool, children: Element) -> Element {
    let class = crate::class::join(&["ads-bubble", if mine { "is-mine" } else { "" }]);
    rsx! { div { class: "{class}", {children} } }
}

/// A list item: an optional glyph, then the body. It only answers the pointer
/// when it goes somewhere. Give it `to` and it becomes a real link; without
/// one it is plain text, because a row that lights up and does nothing is a
/// promise the interface breaks.
#[component]
pub fn AdsItem(
    #[props(into, optional)] icon: Option<String>,
    #[props(default = 18u32)] icon_size: u32,
    #[props(into, optional)] to: Option<String>,
    children: Element,
) -> Element {
    let mark = icon
        .as_deref()
        .map(|n| glyph(n, icon_size, "ads-item__icon", None));
    rsx! {
        if let Some(href) = to {
            a { class: "ads-item is-link", href: "{href}",
                {mark}
                span { class: "ads-item__body", {children} }
            }
        } else {
            div { class: "ads-item",
                {mark}
                span { class: "ads-item__body", {children} }
            }
        }
    }
}

/// The state a marker reports, outside dense data.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerTone {
    #[default]
    Neutral,
    Ok,
    Warn,
    Bad,
}

impl MarkerTone {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Bad => "bad",
        }
    }
}

/// A status marker: a glyph and a label, never a coloured dot on its own.
/// Status carried by colour alone fails a greyscale print and one reader in
/// twelve. The colour comes from a closed set, so a caller cannot paint the
/// chrome: `tone` for a state, or `grade` 1–4 for severity inside dense data,
/// where luminance does the work instead. Only the glyph takes the colour; the
/// label stays at full contrast, and each tone has its own shape.
#[component]
pub fn AdsMarker(
    #[props(into)] label: String,
    #[props(default)] tone: MarkerTone,
    #[props(optional)] grade: Option<u8>,
    children: Element,
) -> Element {
    let modifier = match grade {
        Some(g) => format!("is-grade-{}", g.clamp(1, 4)),
        None => format!("is-{}", tone.as_str()),
    };
    let class = crate::class::join(&["ads-marker", &modifier]);
    let mark = match (grade, tone) {
        (Some(_), _) | (None, MarkerTone::Neutral) => None,
        (None, MarkerTone::Ok) => Some(glyph("check", 13, "", None)),
        (None, MarkerTone::Warn) => Some(glyph("alert", 13, "", None)),
        (None, MarkerTone::Bad) => Some(glyph("plus", 13, "", Some("rotate: 45deg"))),
    };
    rsx! {
        span { class: "{class}",
            span { class: "ads-marker__glyph", "aria-hidden": "true",
                if let Some(m) = mark { {m} } else { span { class: "ads-marker__dot" } }
            }
            span { class: "ads-marker__label", "{label}" }
            {children}
        }
    }
}

/// Which way a direction arrow points.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    #[default]
    Right,
}

impl ArrowDirection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// A directional hint: an arrow turned toward `to`. Decorative by default,
/// since it usually sits beside words that already say where; give it a
/// `label` when the arrow alone carries the meaning.
#[component]
pub fn AdsDirection(
    #[props(default)] to: ArrowDirection,
    #[props(into, optional)] label: Option<String>,
) -> Element {
    let class = format!("ads-direction is-{}", to.as_str());
    let label = label.filter(|l| !l.is_empty());
    let named = label.is_some();
    rsx! {
        span {
            class: "{class}",
            role: if named { Some("img") } else { None },
            "aria-label": label,
            "aria-hidden": if named { None } else { Some("true") },
            {glyph("arrow", 15, "", None)}
        }
    }
}

/// A form label, above its field, in the label face. It is the global
/// `.ads-label`, so it cannot drift from the field labels elsewhere. A field
/// always has one: placeholder text is gone exactly when it is needed.
#[component]
pub fn AdsLabel(#[props(into, optional)] r#for: Option<String>, children: Element) -> Element {
    rsx! { label { class: "ads-label", r#for, {children} } }
}

/// A keyboard shortcut: one key, or a chord. Nested `<kbd>`s are how HTML
/// spells "press these together", and the keys sit in the data face.
#[component]
pub fn AdsKbd(keys: Vec<String>) -> Element {
    rsx! {
        kbd { class: "ads-kbd",
            for k in keys.iter() { kbd { class: "kbd", "{k}" } }
        }
    }
}

/// A bare spinner, for an inline wait of unknown shape, next to the thing that
/// is loading and never over the page. Where the shape is known, use a
/// skeleton. It is a status, so a reader hears the label rather than nothing.
#[component]
pub fn AdsSpinner(
    #[props(default = 18u32)] size: u32,
    #[props(into, default = "Loading".to_string())] label: String,
) -> Element {
    rsx! {
        span {
            class: "ads-spinner",
            style: "width:{size}px;height:{size}px",
            role: "status",
            "aria-label": "{label}",
        }
    }
}

/// A type role. Each has one face: display for headlines, UI for reading, the
/// label face for eyebrows, the data face for figures.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TypeRole {
    Display,
    Title,
    Heading,
    #[default]
    Body,
    Lede,
    Eyebrow,
    Caption,
    Mono,
}

impl TypeRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Title => "title",
            Self::Heading => "heading",
            Self::Body => "body",
            Self::Lede => "lede",
            Self::Eyebrow => "eyebrow",
            Self::Caption => "caption",
            Self::Mono => "mono",
        }
    }
}

/// The element a run of typography renders as (the Vue `as`).
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TypeTag {
    #[default]
    P,
    Span,
    Div,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

/// Text in one of the system's type roles, so a page never invents a size.
/// The role sets the look and `tag` the element, so a display line can still
/// be the page's h1 without borrowing its size from the heading level.
#[component]
pub fn AdsTypography(
    #[props(default)] role: TypeRole,
    #[props(default)] tag: TypeTag,
    children: Element,
) -> Element {
    let class = format!("ads-typography is-{}", role.as_str());
    match tag {
        TypeTag::P => rsx! { p { class: "{class}", {children} } },
        TypeTag::Span => rsx! { span { class: "{class}", {children} } },
        TypeTag::Div => rsx! { div { class: "{class}", {children} } },
        TypeTag::H1 => rsx! { h1 { class: "{class}", {children} } },
        TypeTag::H2 => rsx! { h2 { class: "{class}", {children} } },
        TypeTag::H3 => rsx! { h3 { class: "{class}", {children} } },
        TypeTag::H4 => rsx! { h4 { class: "{class}", {children} } },
        TypeTag::H5 => rsx! { h5 { class: "{class}", {children} } },
        TypeTag::H6 => rsx! { h6 { class: "{class}", {children} } },
    }
}
