//! Dash empty — explains why a panel is empty, with the fix and the action.

use dioxus::prelude::*;

/// Why the panel is empty. A closed set, because the causes are.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum DashEmptyStatus {
    /// The panel has never had data.
    #[default]
    Empty,
    /// The filters exclude everything.
    NoResults,
    Error,
    Forbidden,
    Unconfigured,
}

impl DashEmptyStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::NoResults => "no-results",
            Self::Error => "error",
            Self::Forbidden => "forbidden",
            Self::Unconfigured => "unconfigured",
        }
    }

    const fn tone(self) -> &'static str {
        match self {
            Self::Forbidden | Self::Error => "is-warn",
            Self::Unconfigured => "is-info",
            Self::Empty | Self::NoResults => "is-quiet",
        }
    }

    const fn glyph(self) -> &'static str {
        match self {
            Self::Forbidden | Self::Error => "!",
            Self::Unconfigured => "?",
            Self::NoResults => "∅",
            Self::Empty => "—",
        }
    }

    const fn spoken(self) -> &'static str {
        match self {
            Self::Forbidden => "No access",
            Self::Error => "Error",
            Self::Unconfigured => "Not set up",
            Self::NoResults => "No results",
            Self::Empty => "No data yet",
        }
    }

    const fn default_fix(self) -> &'static str {
        match self {
            Self::Forbidden => "Fix the access grant and this panel fills in on its own.",
            Self::Unconfigured => "Add the missing environment variable and redeploy.",
            Self::NoResults => "Clear a filter to widen the search.",
            Self::Empty => "Nothing to fix. The panel fills in once the data exists.",
            Self::Error => "",
        }
    }
}

/// Explains why a panel is empty, in plain words, with the fix when there is
/// one — and the action that applies it, so the empty state is a way forward
/// rather than a dead end.
///
/// The status is a closed set because the causes are: a panel that has never
/// had data (`Empty`) is a different situation from one whose filters exclude
/// everything (`NoResults`), and both differ from a failure. The tone is
/// carried three ways — the glyph, the words, and the mark's colour — so it
/// survives without colour. No stripe, no brand hue: the panel is chrome, and
/// chrome is grey; the only colour is the warning state on the mark.
#[component]
pub fn DashEmpty(
    #[props(into)] title: String,
    #[props(into, optional)] reason: Option<String>,
    #[props(default)] status: DashEmptyStatus,
    #[props(into, optional)] fix: Option<String>,
    #[props(into, optional)] action_label: Option<String>,
    #[props(optional)] onaction: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let class = crate::class::join(&["ads-dash-empty", status.tone()]);
    let reason = reason.filter(|r| !r.is_empty());
    let fix = fix
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| status.default_fix().to_owned());
    let action_label = action_label.filter(|a| !a.is_empty());

    rsx! {
        div { class: "{class}",
            span { class: "ads-dash-empty__mark", "aria-hidden": "true", "{status.glyph()}" }
            div { class: "ads-dash-empty__copy",
                p { class: "ads-dash-empty__title",
                    span { class: "sr-only", "{status.spoken()}: " }
                    "{title}"
                }
                if let Some(r) = &reason { p { class: "ads-dash-empty__reason", "{r}" } }
                if !fix.is_empty() { p { class: "ads-dash-empty__fix", "{fix}" } }
                if let Some(a) = &action_label {
                    button {
                        r#type: "button",
                        class: "btn btn-sm ads-dash-empty__action",
                        onclick: move |e| {
                            if let Some(h) = &onaction {
                                h.call(e);
                            }
                        },
                        "{a}"
                    }
                }
                {children}
            }
        }
    }
}
