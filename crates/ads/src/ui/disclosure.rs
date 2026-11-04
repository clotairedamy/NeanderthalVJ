//! Accordion, steps, and tree view.

use dioxus::prelude::*;

/// Progressive disclosure on `<details>`, so it works before any script
/// arrives and is announced correctly without a single ARIA attribute.
#[component]
pub fn AdsAccordion(#[props(into)] items: Vec<(String, String)>) -> Element {
    rsx! {
        div { class: "ads-accordion",
            for (i, (title, body)) in items.iter().enumerate() {
                details { open: i == 0,
                    summary { "{title}" }
                    div { class: "panel", "{body}" }
                }
            }
        }
    }
}

/// Where you are in a sequence with a known end. Distinct from a progress bar:
/// this counts discrete stages, not percentage.
///
/// The state is never the pip's fill alone: a done step swaps its number for a
/// tick, the current one is marked `aria-current="step"`, and both are spoken.
#[component]
pub fn AdsSteps(#[props(into)] steps: Vec<Step>) -> Element {
    rsx! {
        ol { class: "ads-steps",
            for (i, s) in steps.iter().enumerate() {
                li {
                    key: "{i}",
                    class: s.state.class(),
                    "aria-current": if s.state == StepState::Now { Some("step") } else { None },
                    span { class: "pip num", "aria-hidden": "true",
                        if s.state == StepState::Done { "✓" } else { "{i + 1}" }
                    }
                    "{s.label}"
                    match s.state {
                        StepState::Done => rsx! { span { class: "sr-only", "(done)" } },
                        StepState::Now => rsx! { span { class: "sr-only", "(current)" } },
                        StepState::Todo => rsx! {},
                    }
                }
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    #[default]
    Todo,
    Now,
    Done,
}

impl StepState {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Todo => "",
            Self::Now => "is-now",
            Self::Done => "is-done",
        }
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct Step {
    pub label: String,
    pub state: StepState,
}

impl Step {
    #[must_use]
    pub fn new(label: impl Into<String>, state: StepState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }
}

/// A tree node with optional count and children.
#[derive(Default, Clone, PartialEq)]
pub struct TreeNode {
    pub label: String,
    pub count: Option<u32>,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    #[must_use]
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            count: None,
            children: Vec::new(),
        }
    }
    #[must_use]
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
    #[must_use]
    pub fn with_children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }
}

fn render_nodes(nodes: &[TreeNode]) -> Element {
    rsx! {
        ul { class: "ads-tree",
            for n in nodes {
                li {
                    span {
                        "{n.label}"
                        if let Some(c) = n.count {
                            span { class: "count num", {crate::ui::count_up::format_en_gb(f64::from(c), 0)} }
                        }
                    }
                    if !n.children.is_empty() {
                        {render_nodes(&n.children)}
                    }
                }
            }
        }
    }
}

/// A tree is the right shape for anything with real nesting — site structure, a
/// category hierarchy, a file system. Built on nested lists so the nesting is
/// in the markup, not only in the indentation. Counts are figures, so they are
/// set in tabular mono and right-aligned on the row.
#[component]
pub fn AdsTree(#[props(into)] nodes: Vec<TreeNode>) -> Element {
    render_nodes(&nodes)
}
