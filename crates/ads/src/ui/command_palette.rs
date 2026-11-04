//! Command palette — a filterable ⌘K list of sections, pages and actions.

use dioxus::prelude::*;

use crate::icons::Icon;

/// One palette command.
#[derive(Default, Clone, PartialEq)]
pub struct Command {
    pub id: String,
    pub group: String,
    pub label: String,
    pub icon: String,
    pub hint: Option<String>,
}

impl Command {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        group: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            label: label.into(),
            icon: icon.into(),
            hint: None,
        }
    }
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Forgiving subsequence match: "dsys" finds "Design system".
fn matches(text: &str, q: &str) -> bool {
    let mut i = 0usize;
    let t = text.to_lowercase();
    for ch in q.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        match t[i..].find(ch) {
            Some(j) => i += j + 1,
            None => return false,
        }
    }
    true
}

/// The command palette: ⌘K (or /) from anywhere.
///
/// It is the most "this is an application" gesture a website can offer, and
/// it is also the fastest way around — every section of the page, every page,
/// and the handful of actions worth a shortcut, behind one text field that
/// filters as you type. `open` is controlled; Escape and a click on the scrim
/// call `onclose`. Choosing a command calls `onclose`, then `onselect` with
/// its id. Commands are listed in their groups, in the order given.
#[component]
pub fn CommandPalette(
    #[props(optional)] open: bool,
    #[props(into)] commands: Vec<Command>,
    #[props(optional)] onclose: Option<EventHandler<()>>,
    #[props(optional)] onselect: Option<EventHandler<String>>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut cursor = use_signal(|| 0usize);

    let results: Vec<Command> = {
        let q = query.read();
        commands
            .iter()
            .filter(|c| matches(&format!("{} {}", c.label, c.group), &q))
            .cloned()
            .collect()
    };
    // Groups in order of first appearance, each row keeping its place in the
    // flat result list so the cursor can walk across groups.
    let mut groups: Vec<(String, Vec<(usize, Command)>)> = Vec::new();
    for (index, c) in results.iter().enumerate() {
        match groups.iter_mut().find(|(name, _)| *name == c.group) {
            Some((_, items)) => items.push((index, c.clone())),
            None => groups.push((c.group.clone(), vec![(index, c.clone())])),
        }
    }
    let n = results.len();
    let at = cursor().min(n.saturating_sub(1));
    let active = results.get(at).map(|c| format!("cmd-{}", c.id));
    let count = if n == 1 { "result" } else { "results" };

    let close = move || {
        if let Some(h) = &onclose {
            h.call(());
        }
    };
    let run = move |id: String| {
        close();
        if let Some(h) = &onselect {
            h.call(id);
        }
    };
    let ids: Vec<String> = results.iter().map(|c| c.id.clone()).collect();

    if !open {
        return rsx! {};
    }
    rsx! {
        div {
            class: "ads-command-palette",
            onmousedown: move |_| close(),
            div {
                class: "ads-command-palette__dialog",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Command palette",
                onmousedown: move |e| e.stop_propagation(),
                onkeydown: move |e: KeyboardEvent| match e.key() {
                    Key::ArrowDown => {
                        e.prevent_default();
                        cursor.set((at + 1) % n.max(1));
                    }
                    Key::ArrowUp => {
                        e.prevent_default();
                        cursor.set((at + n.max(1) - 1) % n.max(1));
                    }
                    Key::Enter => {
                        e.prevent_default();
                        if let Some(id) = ids.get(at) {
                            run(id.clone());
                        }
                    }
                    Key::Escape => close(),
                    _ => {}
                },
                div { class: "ads-command-palette__field",
                    Icon { name: "discovery", size: 20 }
                    input {
                        r#type: "text",
                        placeholder: "Search sections, pages and actions…",
                        "aria-label": "Search sections, pages and actions",
                        role: "combobox",
                        "aria-expanded": "true",
                        "aria-controls": "palette-list",
                        "aria-activedescendant": active,
                        autocomplete: "off",
                        spellcheck: "false",
                        value: "{query}",
                        onmounted: move |e: MountedEvent| {
                            let el = e.data();
                            spawn(async move {
                                let _ = el.set_focus(true).await;
                            });
                        },
                        oninput: move |e| {
                            query.set(e.value());
                            cursor.set(0);
                        },
                    }
                    kbd { class: "kbd", "esc" }
                }
                div { id: "palette-list", class: "ads-command-palette__results", role: "listbox", "aria-label": "Results",
                    for (name, items) in groups {
                        div { key: "{name}", role: "group", "aria-label": "{name}",
                            p { class: "eyebrow ads-command-palette__group", "aria-hidden": "true", "{name}" }
                            for (index, c) in items {
                                button {
                                    key: "{c.id}",
                                    id: "cmd-{c.id}",
                                    r#type: "button",
                                    role: "option",
                                    class: "ads-command-palette__row",
                                    "aria-selected": if index == at { "true" } else { "false" },
                                    onmousemove: move |_| cursor.set(index),
                                    onclick: {
                                        let id = c.id.clone();
                                        move |_| run(id.clone())
                                    },
                                    span { class: "ads-command-palette__row-icon", Icon { name: c.icon.clone(), size: 18 } }
                                    span { class: "ads-command-palette__row-label", "{c.label}" }
                                    if let Some(hint) = &c.hint { span { class: "ads-command-palette__row-hint num", "{hint}" } }
                                }
                            }
                        }
                    }
                    if n == 0 { p { class: "body ads-command-palette__empty", "Nothing matches “{query}”." } }
                }
                div { class: "ads-command-palette__foot num",
                    span { kbd { class: "kbd", "↑" } kbd { class: "kbd", "↓" } " move" }
                    span { kbd { class: "kbd", "↵" } " open" }
                    span { class: "ads-command-palette__count", "aria-live": "polite", "{n} {count}" }
                }
            }
        }
    }
}
