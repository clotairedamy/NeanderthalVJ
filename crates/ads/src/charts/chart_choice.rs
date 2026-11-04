//! The chart chooser — the decision before the component.

use dioxus::prelude::*;

/// One row of the chooser: the data's job, the form, and what decides it.
pub type ChartChoiceRow = (String, String, String);

const ROWS: &[(&str, &str, &str)] = &[
    (
        "Change over time",
        "Line, or a sparkline beside the figure",
        "Minimal grid; label the first and last value, never every point",
    ),
    (
        "Comparing categories",
        "Horizontal bars, ordered by value",
        "Never a pie — angle is the hardest encoding to compare",
    ),
    (
        "One headline number",
        "Stat tile with a delta",
        "A figure with no comparison is trivia; pair it with a target or a prior period",
    ),
    (
        "Part of a whole",
        "Stacked bar, or just the percentage",
        "Two or three parts at most, or it becomes a pie lying down",
    ),
    (
        "Distribution",
        "Histogram or box plot",
        "An average hides the shape; show the spread where the spread is the point",
    ),
    (
        "Density over two axes",
        "Heatmap",
        "Luminance steps for magnitude — the channel the eye reads fastest",
    ),
    (
        "Progress to a target",
        "Meter with a threshold mark",
        "The mark is the component; without it the fill means nothing",
    ),
    (
        "Relationship",
        "Scatter",
        "Three series maximum — any two marks can sit side by side",
    ),
];

/// The decision before the component: pick the chart form from the job, not
/// from a gallery. Most bad charts are the wrong shape, not the wrong colour.
///
/// The table is styled through its own classes rather than bare `table`/`td`
/// selectors, so in the one global sheet it styles this table and no other.
#[component]
pub fn ChartChoice(#[props(into, optional)] rows: Option<Vec<ChartChoiceRow>>) -> Element {
    let rows = rows.unwrap_or_else(|| {
        ROWS.iter()
            .map(|(a, b, c)| ((*a).to_owned(), (*b).to_owned(), (*c).to_owned()))
            .collect()
    });
    rsx! {
        div { class: "ads-chart-choice",
            table { class: "ads-chart-choice__table",
                caption { class: "sr-only", "Choosing a chart form from the data's job" }
                thead {
                    tr {
                        th { scope: "col", class: "ads-chart-choice__th", "The data's job" }
                        th { scope: "col", class: "ads-chart-choice__th", "Form" }
                        th { scope: "col", class: "ads-chart-choice__th", "What decides it" }
                    }
                }
                tbody {
                    for (job, form, decides) in &rows {
                        tr { key: "{job}",
                            th { scope: "row", class: "ads-chart-choice__td ads-chart-choice__job", "{job}" }
                            td { class: "ads-chart-choice__td", "{form}" }
                            td { class: "ads-chart-choice__td ads-chart-choice__decides", "{decides}" }
                        }
                    }
                }
            }
        }
    }
}
