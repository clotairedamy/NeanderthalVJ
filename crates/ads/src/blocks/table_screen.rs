//! Table screen — header, tabs, filter bar, grid, pagination.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

use crate::icons::Icon;
use crate::ui::{AdsDataGrid, GridColumn};

/// The arrangement that carries most of the working hours in any data product:
/// header, tabs, filter bar, grid, pagination.
///
/// The tabs filter the one table below them — they never swap in a different
/// screen — so they are a real tablist whose panel is the grid, with the arrow
/// keys moving between tabs. The header's action is the screen's single
/// accent; pass an empty `action_label` when the page already spends it
/// elsewhere. A selected row is forwarded as `onselect`, for the drawer beside
/// it. `tab` and `query` are controlled: pass them back from `ontab` and
/// `onquery`. `filters` sits beside the search field; `footer` holds the pager.
#[component]
pub fn TableScreen(
    #[props(into)] title: String,
    #[props(into, optional)] count: Option<String>,
    #[props(into, default = "Export CSV".to_string())] action_label: String,
    #[props(into, default)] tabs: Vec<String>,
    #[props(into, default)] tab: String,
    #[props(into)] columns: Vec<GridColumn>,
    #[props(into)] rows: Vec<Vec<String>>,
    #[props(into, default = "Filter by URL…".to_string())] placeholder: String,
    #[props(into, default = "Filter by URL".to_string())] search_label: String,
    #[props(into, optional)] query: Option<String>,
    #[props(optional)] onquery: Option<EventHandler<String>>,
    #[props(optional)] ontab: Option<EventHandler<String>>,
    #[props(optional)] onaction: Option<EventHandler<MouseEvent>>,
    #[props(optional)] onselect: Option<EventHandler<(Vec<String>, usize)>>,
    #[props(default = VNode::empty())] filters: Element,
    #[props(default = VNode::empty())] footer: Element,
) -> Element {
    let uid = use_hook(|| {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("ads-table-screen-{}", NEXT.fetch_add(1, Ordering::Relaxed))
    });
    let mut mounted = use_signal(Vec::<Option<Rc<MountedData>>>::new);

    let panel_id = format!("{uid}-panel");
    let tab_ids = (0..tabs.len())
        .map(|i| format!("{uid}-tab-{i}"))
        .collect::<Vec<_>>();
    let selected = tabs.iter().position(|t| *t == tab);
    // With no tab selected, the first keeps the row reachable.
    let stop = selected.unwrap_or(0);
    let has_tabs = !tabs.is_empty();
    let labelled_by = if has_tabs {
        tab_ids.get(stop).cloned()
    } else {
        None
    };

    // Roving focus: only the selected tab is in the tab order; the arrows move
    // between tabs and select as they go, as a tablist is expected to.
    let keyed_tabs = tabs.clone();
    let on_tab_key = move |e: KeyboardEvent| {
        let n = keyed_tabs.len();
        if n == 0 {
            return;
        }
        let next = match e.key() {
            Key::ArrowRight => (stop + 1) % n,
            Key::ArrowLeft => (stop + n - 1) % n,
            Key::Home => 0,
            Key::End => n - 1,
            _ => return,
        };
        e.prevent_default();
        if let Some(h) = &ontab {
            h.call(keyed_tabs[next].clone());
        }
        if let Some(el) = mounted.read().get(next).cloned().flatten() {
            spawn(async move {
                let _ = el.set_focus(true).await;
            });
        }
    };

    rsx! {
        div { class: "card ads-table-screen",
            header { class: "ads-table-screen__head",
                div { class: "ads-table-screen__heading",
                    h4 { class: "ads-table-screen__title ads-display", "{title}" }
                    if let Some(c) = count.as_deref().filter(|c| !c.is_empty()) {
                        span { class: "dim num", "{c}" }
                    }
                }
                if !action_label.is_empty() {
                    button {
                        class: "btn btn-accent",
                        r#type: "button",
                        onclick: move |e| {
                            if let Some(h) = &onaction {
                                h.call(e);
                            }
                        },
                        "{action_label}"
                    }
                }
            }

            if has_tabs {
                div { class: "ads-tabs", role: "tablist", "aria-label": "{title}", onkeydown: on_tab_key,
                    for (i, t) in tabs.iter().enumerate() {
                        button {
                            key: "{t}",
                            id: "{tab_ids[i]}",
                            r#type: "button",
                            role: "tab",
                            "aria-selected": if selected == Some(i) { "true" } else { "false" },
                            "aria-controls": "{panel_id}",
                            tabindex: if i == stop { "0" } else { "-1" },
                            class: if selected == Some(i) { "is-on" } else { "" },
                            onmounted: move |e: MountedEvent| {
                                let mut m = mounted.write();
                                if m.len() <= i {
                                    m.resize(i + 1, None);
                                }
                                m[i] = Some(e.data());
                            },
                            onclick: {
                                let v = t.clone();
                                move |_| {
                                    if let Some(h) = &ontab {
                                        h.call(v.clone());
                                    }
                                }
                            },
                            "{t}"
                        }
                    }
                }
            }

            div {
                id: "{panel_id}",
                class: "ads-table-screen__panel",
                role: if has_tabs { Some("tabpanel") } else { None },
                "aria-labelledby": labelled_by,
                div { class: "ads-table-screen__controls",
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
                    {filters}
                }
                AdsDataGrid {
                    columns,
                    rows,
                    caption: title.clone(),
                    onselect: move |sel: (Vec<String>, usize)| {
                        if let Some(h) = &onselect {
                            h.call(sel);
                        }
                    },
                }
            }

            footer { class: "ads-table-screen__foot", {footer} }
        }
    }
}
