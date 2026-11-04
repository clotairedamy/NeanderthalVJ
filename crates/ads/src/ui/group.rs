//! Segmented control, tabs, and menu.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::icons::Icon;

/// An option for a segmented control or tab row.
#[derive(Default, Clone, PartialEq)]
pub struct SegOption {
    pub label: String,
    pub value: String,
}

impl SegOption {
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Focus a mounted element, if it has been mounted.
fn focus(el: Option<Rc<MountedData>>) {
    if let Some(el) = el {
        spawn(async move {
            let _ = el.set_focus(true).await;
        });
    }
}

/// A segmented control: two to five mutually exclusive views, all visible at
/// once, in a single pressed track. More than five, or labels that wrap, is a
/// select. The group needs a name because the options alone ("7d", "30d") do
/// not say what they choose between.
#[component]
pub fn AdsSegmented(
    #[props(into)] options: Vec<SegOption>,
    #[props(into)] label: String,
    #[props(into, optional)] selected: Option<String>,
    #[props(optional)] onselect: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div { class: "ads-segmented", role: "group", "aria-label": "{label}",
            for o in &options {
                button {
                    r#type: "button",
                    class: if selected.as_deref() == Some(o.value.as_str()) { "is-on" } else { "" },
                    "aria-pressed": if selected.as_deref() == Some(o.value.as_str()) { "true" } else { "false" },
                    onclick: {
                        let v = o.value.clone();
                        move |_| { if let Some(h) = &onselect { h.call(v.clone()); } }
                    },
                    "{o.label}"
                }
            }
        }
    }
}

/// Tabs: filtered views of one dataset, where the columns stay put — not
/// navigation between screens, which is the rail's job. A hairline with the
/// active tab underlined by the accent, the accent's only appearance in a nav
/// context. One tab stop for the row; arrow keys move between tabs, as the
/// tablist pattern expects. `controls` is the id of the view the tabs filter.
#[component]
pub fn AdsTabs(
    #[props(into)] tabs: Vec<SegOption>,
    #[props(into)] label: String,
    #[props(into, optional)] controls: Option<String>,
    #[props(into, optional)] selected: Option<String>,
    #[props(optional)] onselect: Option<EventHandler<String>>,
) -> Element {
    let mut mounted = use_signal(Vec::<Option<Rc<MountedData>>>::new);
    // With nothing selected yet, the first tab keeps the row reachable.
    let stop = tabs
        .iter()
        .position(|t| selected.as_deref() == Some(t.value.as_str()))
        .unwrap_or(0);
    let count = tabs.len();
    let values: Vec<String> = tabs.iter().map(|t| t.value.clone()).collect();

    rsx! {
        div {
            class: "ads-tabs",
            role: "tablist",
            "aria-label": "{label}",
            onkeydown: move |e: KeyboardEvent| {
                if count == 0 {
                    return;
                }
                let last = count - 1;
                let next = match e.key() {
                    Key::ArrowRight => if stop == last { 0 } else { stop + 1 },
                    Key::ArrowLeft => if stop == 0 { last } else { stop - 1 },
                    Key::Home => 0,
                    Key::End => last,
                    _ => return,
                };
                e.prevent_default();
                if let Some(h) = &onselect {
                    h.call(values[next].clone());
                }
                focus(mounted.read().get(next).cloned().flatten());
            },
            for (i, t) in tabs.iter().enumerate() {
                button {
                    key: "{t.value}",
                    r#type: "button",
                    role: "tab",
                    class: if selected.as_deref() == Some(t.value.as_str()) { "is-on" } else { "" },
                    "aria-selected": if selected.as_deref() == Some(t.value.as_str()) { "true" } else { "false" },
                    "aria-controls": controls.clone(),
                    tabindex: if i == stop { "0" } else { "-1" },
                    onmounted: move |e: MountedEvent| {
                        let mut m = mounted.write();
                        if m.len() <= i {
                            m.resize(i + 1, None);
                        }
                        m[i] = Some(e.data());
                    },
                    onclick: {
                        let v = t.value.clone();
                        move |_| { if let Some(h) = &onselect { h.call(v.clone()); } }
                    },
                    "{t.label}"
                }
            }
        }
    }
}

/// A menu item. A divider is a rule between groups of items.
#[derive(Default, Clone, PartialEq)]
pub struct MenuItem {
    pub label: String,
    pub icon: Option<String>,
    pub shortcut: Option<String>,
    pub destructive: bool,
    pub divider: bool,
}

impl MenuItem {
    #[must_use]
    pub fn item(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            destructive: false,
            divider: false,
        }
    }
    #[must_use]
    pub fn with_icon(label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            icon: Some(icon.into()),
            ..Self::item(label)
        }
    }
    #[must_use]
    pub fn divider() -> Self {
        Self {
            divider: true,
            ..Self::item(String::new())
        }
    }
    /// Show a keyboard shortcut beside the item.
    #[must_use]
    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }
    /// Mark the item destructive; the menu sets it last, below a rule.
    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

/// A menu: everything a reader can do to one record, in one place.
/// Destructive items always go last, below a rule — the menu sorts them there
/// itself, so a "Delete" can never land first under the pointer. Shortcuts
/// sit beside their item in the data face, because a menu is how a reader
/// learns them. `onaction` receives the chosen item's label.
#[component]
pub fn AdsMenu(
    #[props(into)] items: Vec<MenuItem>,
    #[props(into)] label: String,
    #[props(optional)] onaction: Option<EventHandler<String>>,
) -> Element {
    // Ordinary items keep their order and their rules; destructive ones are
    // lifted out and set after a single rule of their own.
    let mut safe: Vec<MenuItem> = items.iter().filter(|i| !i.destructive).cloned().collect();
    while safe.last().is_some_and(|i| i.divider) {
        safe.pop();
    }
    let danger: Vec<MenuItem> = items
        .iter()
        .filter(|i| i.destructive && !i.divider)
        .cloned()
        .collect();
    let rule = !safe.is_empty() && !danger.is_empty();

    // Each button's place in reading order, rules skipped, for arrow-key focus.
    let mut n = 0usize;
    let safe_rows: Vec<(MenuItem, Option<usize>)> = safe
        .into_iter()
        .map(|item| {
            let idx = (!item.divider).then(|| {
                n += 1;
                n - 1
            });
            (item, idx)
        })
        .collect();
    let first_danger = n;
    let count = n + danger.len();

    let mounted = use_signal(Vec::<Option<Rc<MountedData>>>::new);
    let at = use_signal(|| None::<usize>);

    rsx! {
        div {
            class: "ads-menu",
            role: "menu",
            "aria-label": "{label}",
            // Arrow keys walk the items, as they do in every native menu.
            onkeydown: move |e: KeyboardEvent| {
                if count == 0 {
                    return;
                }
                let last = count - 1;
                let cur = at();
                let next = match e.key() {
                    Key::ArrowDown => cur.map_or(0, |c| if c == last { 0 } else { c + 1 }),
                    Key::ArrowUp => cur.map_or(last, |c| if c == 0 { last } else { c - 1 }),
                    Key::Home => 0,
                    Key::End => last,
                    _ => return,
                };
                e.prevent_default();
                focus(mounted.read().get(next).cloned().flatten());
            },
            for (item, idx) in safe_rows.iter() {
                if let Some(i) = idx {
                    {menu_button(item, *i, mounted, at, onaction)}
                } else {
                    hr {}
                }
            }
            if rule { hr {} }
            for (j, item) in danger.iter().enumerate() {
                {menu_button(item, first_danger + j, mounted, at, onaction)}
            }
        }
    }
}

fn menu_button(
    item: &MenuItem,
    i: usize,
    mut mounted: Signal<Vec<Option<Rc<MountedData>>>>,
    mut at: Signal<Option<usize>>,
    onaction: Option<EventHandler<String>>,
) -> Element {
    let l = item.label.clone();
    rsx! {
        button {
            r#type: "button",
            role: "menuitem",
            onmounted: move |e: MountedEvent| {
                let mut m = mounted.write();
                if m.len() <= i {
                    m.resize(i + 1, None);
                }
                m[i] = Some(e.data());
            },
            onfocus: move |_| at.set(Some(i)),
            onclick: move |_| { if let Some(h) = &onaction { h.call(l.clone()); } },
            if let Some(ic) = &item.icon { Icon { name: ic.clone(), size: 15 } }
            "{item.label}"
            if let Some(s) = &item.shortcut { span { class: "shortcut", "{s}" } }
        }
    }
}
