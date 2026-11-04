//! Statement section — a few deliberate lines, then a ruled ledger of facts.

use dioxus::prelude::*;

/// A statement: a few deliberate lines, then a ruled ledger of facts
/// (term, value). The lines are a `RevealHeading` at statement size set as a
/// paragraph — a statement is not a heading in the outline — so its markup is
/// written out here, since the Rust `RevealHeading` renders only `h1`–`h3`.
#[component]
pub fn StatementSection(
    #[props(into)] lines: Vec<String>,
    #[props(into, optional)] facts: Option<Vec<(String, String)>>,
) -> Element {
    let facts = facts.unwrap_or_default();
    rsx! {
        section { class: "ads-statement-section",
            p { class: "ads-reveal-heading is-statement",
                for (i, line) in lines.iter().enumerate() {
                    span { key: "{i}", class: "ads-reveal-heading__line", "{line}" }
                }
            }
            if !facts.is_empty() {
                dl { class: "ads-dl ads-statement-section__facts",
                    for (term, value) in &facts {
                        dt { "{term}" }
                        dd { "{value}" }
                    }
                }
            }
        }
    }
}
