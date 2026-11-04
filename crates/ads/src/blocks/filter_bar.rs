//! Filter bar & saved views.

use dioxus::prelude::*;

use crate::icons::Icon;
use crate::ui::{AdsSegmented, AdsTag, SegOption};

/// One row, above the data, holding every control that changes what is shown.
/// Saved views turn a repeated set of filters into a named thing a team can
/// share. Serialise the whole bar into the URL, so a view can be bookmarked.
///
/// Every control is named: the search field by `search_label` (its placeholder
/// is an example, never the label), the segmented control by `segments_label`.
/// Each saved view is a tag, and a tag is removable — that is what a tag means.
/// `query` and `segment` are controlled: pass them back from `onquery` and
/// `onsegment`.
#[component]
pub fn FilterBar(
    #[props(into, default = "Filter by URL…".to_string())] placeholder: String,
    #[props(into, default = "Filter by URL".to_string())] search_label: String,
    #[props(into, optional)] query: Option<String>,
    #[props(optional)] onquery: Option<EventHandler<String>>,
    #[props(into, default)] tags: Vec<String>,
    #[props(into, optional)] segments: Option<Vec<String>>,
    #[props(into, default = "All".to_string())] segment: String,
    #[props(into, default = "Status".to_string())] segments_label: String,
    #[props(into, default = "30d".to_string())] from: String,
    #[props(into, default = "now".to_string())] to: String,
    #[props(optional)] onsegment: Option<EventHandler<String>>,
    #[props(optional)] onremovetag: Option<EventHandler<String>>,
    #[props(optional)] onsave: Option<EventHandler<MouseEvent>>,
) -> Element {
    let segments = segments
        .unwrap_or_else(|| vec!["All".into(), "2xx".into(), "4xx".into()])
        .into_iter()
        .map(|s| SegOption::new(s.clone(), s))
        .collect::<Vec<_>>();
    rsx! {
        div { class: "ads-filter-bar stack gap-3",
            div { class: "ads-filter-bar__controls",
                div { class: "ads-search",
                    Icon { name: "compass", size: 16 }
                    input {
                        class: "ads-field",
                        r#type: "search",
                        placeholder: "{placeholder}",
                        "aria-label": "{search_label}",
                        value: query,
                        oninput: move |e| {
                            if let Some(h) = &onquery {
                                h.call(e.value());
                            }
                        },
                    }
                }
                div { class: "ads-daterange",
                    span { class: "num", "{from}" }
                    span { class: "sep", "aria-hidden": "true", "→" }
                    span { class: "sr-only", " to " }
                    span { class: "num", "{to}" }
                }
                AdsSegmented {
                    options: segments,
                    label: segments_label,
                    selected: segment,
                    onselect: move |v: String| {
                        if let Some(h) = &onsegment {
                            h.call(v);
                        }
                    },
                }
            }
            div { class: "ads-filter-bar__saved",
                span { class: "ads-label", "Saved" }
                for t in tags {
                    AdsTag {
                        key: "{t}",
                        label: t.clone(),
                        onremove: move |_| {
                            if let Some(h) = &onremovetag {
                                h.call(t.clone());
                            }
                        },
                    }
                }
                button {
                    class: "btn btn-quiet",
                    r#type: "button",
                    onclick: move |e| {
                        if let Some(h) = &onsave {
                            h.call(e);
                        }
                    },
                    Icon { name: "plus", size: 15 }
                    "Save this view"
                }
            }
        }
    }
}
