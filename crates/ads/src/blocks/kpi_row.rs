//! KPI row — the top of nearly every dashboard.

use dioxus::prelude::*;

use crate::ui::{AdsStat, DeltaDirection};

/// One stat tile: the figure and its comparison.
#[derive(Clone, PartialEq)]
pub struct StatItem {
    pub value: String,
    pub label: String,
    pub unit: Option<String>,
    pub note: Option<String>,
    pub delta: Option<String>,
    pub direction: DeltaDirection,
    pub previous: Option<String>,
    pub target: Option<String>,
}

impl StatItem {
    #[must_use]
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            unit: None,
            note: None,
            delta: None,
            direction: DeltaDirection::Flat,
            previous: None,
            target: None,
        }
    }
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
    #[must_use]
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
    /// The change, already formatted ("12.4%"), and which way it went.
    #[must_use]
    pub fn delta(mut self, delta: impl Into<String>, direction: DeltaDirection) -> Self {
        self.delta = Some(delta.into());
        self.direction = direction;
        self
    }
    #[must_use]
    pub fn previous(mut self, previous: impl Into<String>) -> Self {
        self.previous = Some(previous.into());
        self
    }
    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }
}

/// The KPI row — the top of nearly every dashboard. Four figures at most,
/// ranked left to right by how often they are the reason someone opened the
/// screen. Each tile carries its comparison (a delta, the previous period or a
/// target) because a figure with nothing to compare it to is trivia.
/// `columns` is 2, 3 or 4.
#[component]
pub fn KpiRow(
    #[props(into)] stats: Vec<StatItem>,
    #[props(default = 4u8)] columns: u8,
) -> Element {
    let class = format!("ads-kpi-row is-cols-{}", columns.clamp(2, 4));
    rsx! {
        div { class: "{class}",
            for s in &stats {
                AdsStat {
                    key: "{s.label}",
                    value: s.value.clone(),
                    label: s.label.clone(),
                    unit: s.unit.clone(),
                    note: s.note.clone(),
                    delta: s.delta.clone(),
                    direction: s.direction,
                    previous: s.previous.clone(),
                    target: s.target.clone(),
                }
            }
        }
    }
}
