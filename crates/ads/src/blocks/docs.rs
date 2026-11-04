//! Documentation components: shell, nav, callout, window, screenshot.

use dioxus::prelude::*;

use crate::blocks::sections::NavLink;
use crate::blocks::use_uid;
use crate::class::join;
use crate::icons::Icon;

/// A page in the docs nav.
#[derive(Clone, PartialEq)]
pub struct DocPage {
    pub title: String,
    pub path: String,
}

impl DocPage {
    #[must_use]
    pub fn new(title: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            path: path.into(),
        }
    }
}

/// A section of pages in the docs nav.
#[derive(Clone, PartialEq)]
pub struct DocGroup {
    pub section: String,
    pub pages: Vec<DocPage>,
}

impl DocGroup {
    #[must_use]
    pub fn new(section: impl Into<String>, pages: Vec<DocPage>) -> Self {
        Self {
            section: section.into(),
            pages,
        }
    }
}

/// A documentation layout: a sticky sidebar of sections and a reading column.
/// The sidebar is rail-wide, so docs and the dashboard line up; on a phone it
/// drops below the reading column, behind a hairline. `aside` is the sidebar
/// (usually a `DocNav`); the children are the reading column.
#[component]
pub fn DocShell(#[props(optional)] aside: Option<Element>, children: Element) -> Element {
    rsx! {
        div { class: "ads-doc-shell",
            aside { class: "ads-doc-shell__aside",
                if let Some(a) = aside { {a} }
            }
            div { class: "ads-doc-shell__main", {children} }
        }
    }
}

/// A help-centre sidebar: pages grouped by section. Prop-driven, so it works
/// with any content source or a plain list. The current page is pressed into
/// the rail — the shadow inversion, not a colour — and announced with
/// `aria-current`. There is no router here, so pass `current`, the path on
/// show.
#[component]
pub fn DocNav(
    #[props(into)] groups: Vec<DocGroup>,
    #[props(optional)] home: Option<NavLink>,
    #[props(into, optional)] current: Option<String>,
) -> Element {
    let uid = use_uid("ads-doc-nav");
    let is_current = |path: &str| current.as_deref() == Some(path);
    rsx! {
        nav { class: "ads-doc-nav", "aria-label": "Docs",
            if let Some(h) = &home {
                a {
                    class: join(&["ads-doc-nav__link ads-doc-nav__home", if is_current(&h.to) { "is-active" } else { "" }]),
                    href: "{h.to}",
                    "aria-current": if is_current(&h.to) { Some("page") } else { None },
                    "{h.label}"
                }
            }
            for (gi, g) in groups.iter().enumerate() {
                div { key: "{g.section}", class: "ads-doc-nav__group",
                    span { id: "{uid}-{gi}", class: "eyebrow ads-doc-nav__heading", "{g.section}" }
                    ul { class: "ads-doc-nav__list", role: "list", "aria-labelledby": "{uid}-{gi}",
                        for p in &g.pages {
                            li { key: "{p.path}",
                                a {
                                    class: join(&["ads-doc-nav__link", if is_current(&p.path) { "is-active" } else { "" }]),
                                    href: "{p.path}",
                                    "aria-current": if is_current(&p.path) { Some("page") } else { None },
                                    "{p.title}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The three kinds of callout.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum CalloutType {
    #[default]
    Note,
    Tip,
    Warning,
}

impl CalloutType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Tip => "tip",
            Self::Warning => "warning",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Warning => "Warning",
        }
    }
    const fn icon(self) -> &'static str {
        match self {
            Self::Note => "compass",
            Self::Tip => "spark",
            Self::Warning => "alert",
        }
    }
}

/// A docs callout. Every kind is a glyph and a word — never a coloured edge.
/// In a monochrome system the glyph is what survives a greyscale print, and a
/// stripe of hue would be colour carrying meaning on its own. A warning is the
/// brightest of the three, because here the worse thing is the brighter one.
#[component]
pub fn Callout(#[props(default)] callout_type: CalloutType, children: Element) -> Element {
    rsx! {
        div { class: "ads-callout is-{callout_type.as_str()}", role: "note",
            span { class: "ads-callout__glyph", Icon { name: callout_type.icon(), size: 16 } }
            div { class: "ads-callout__content",
                span { class: "ads-label ads-callout__label", "{callout_type.label()}" }
                div { class: "ads-callout__body", {children} }
            }
        }
    }
}

/// A screenshot in a soft window frame. The traffic dots are pressed into the
/// frame and all three are grey: the window is chrome, and the accent is kept
/// for the one action on the page, not spent on a picture of one. The Vue
/// version fades in on scroll; here the window is simply shown.
#[component]
pub fn AppWindow(
    #[props(into)] src: String,
    #[props(into)] alt: String,
    #[props(default = 1440u32)] width: u32,
    #[props(default = 900u32)] height: u32,
    #[props(default = false)] eager: bool,
) -> Element {
    rsx! {
        figure { class: "ads-app-window is-shown",
            div { class: "ads-app-window__chrome", "aria-hidden": "true",
                span { class: "ads-app-window__dot" }
                span { class: "ads-app-window__dot" }
                span { class: "ads-app-window__dot" }
            }
            div { class: "ads-app-window__pane",
                img {
                    class: "ads-app-window__image",
                    src: "{src}",
                    alt: "{alt}",
                    width: "{width}",
                    height: "{height}",
                    loading: if eager { "eager" } else { "lazy" },
                    "fetchpriority": if eager { "high" } else { "auto" },
                    decoding: "async",
                }
            }
        }
    }
}

/// A screenshot figure: the window frame plus an optional caption. The caption
/// belongs to this outer figure, so it describes the shot rather than the
/// frame.
#[component]
pub fn Screenshot(
    #[props(into)] src: String,
    #[props(into)] alt: String,
    #[props(into, optional)] caption: Option<String>,
) -> Element {
    rsx! {
        figure { class: "ads-screenshot",
            AppWindow { src, alt }
            if let Some(c) = caption.as_deref().filter(|c| !c.is_empty()) {
                figcaption { class: "ads-screenshot__caption", "{c}" }
            }
        }
    }
}
