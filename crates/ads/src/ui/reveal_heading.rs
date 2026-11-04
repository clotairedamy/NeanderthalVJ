//! Reveal heading — a heading, one block per line.

use dioxus::prelude::*;

/// The three heading scales a reveal heading is set in.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum RevealSize {
    Display,
    #[default]
    Title,
    Statement,
}

impl RevealSize {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Title => "title",
            Self::Statement => "statement",
        }
    }
}

/// The heading level, so the outline stays true wherever the heading is
/// placed. The size is set by `RevealSize`, not by the level.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    H1,
    #[default]
    H2,
    H3,
}

impl HeadingLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
        }
    }
}

/// A heading rendered one block per line, so a deliberate break lands where it
/// was written.
#[component]
pub fn RevealHeading(
    #[props(into)] lines: Vec<String>,
    #[props(default)] tag: HeadingLevel,
    #[props(default)] size: RevealSize,
) -> Element {
    let class = format!("ads-reveal-heading is-{}", size.as_str());
    let body = rsx! {
        for line in &lines { span { class: "ads-reveal-heading__line", "{line}" } }
    };
    match tag {
        HeadingLevel::H1 => rsx! { h1 { class: "{class}", {body} } },
        HeadingLevel::H2 => rsx! { h2 { class: "{class}", {body} } },
        HeadingLevel::H3 => rsx! { h3 { class: "{class}", {body} } },
    }
}
