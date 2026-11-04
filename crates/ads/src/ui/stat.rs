//! Stat tile, meter, progress, structured list, and data grid.

use std::cmp::Ordering;

use dioxus::prelude::*;

use crate::ui::{AdsDelta, AdsStatus, DeltaDirection};

/// A KPI tile. The number is monospaced and large, the label is small and
/// quiet — and the figure always travels with a comparison, because a number
/// on its own is a label with extra steps. Give it a `delta` (read with
/// `direction`), the `previous` period, a `target`, or at the very least a
/// `note` saying what it is against.
#[component]
pub fn AdsStat(
    #[props(into)] value: String,
    #[props(into)] label: String,
    #[props(into, optional)] unit: Option<String>,
    /// The change, already formatted ("12.4%"). Read with `direction`.
    #[props(into, optional)]
    delta: Option<String>,
    #[props(default)] direction: DeltaDirection,
    /// The same figure last period, shown as "prev …".
    #[props(into, optional)]
    previous: Option<String>,
    /// What the figure is aiming at, shown as "target …".
    #[props(into, optional)]
    target: Option<String>,
    /// A free-form comparison: "+37 this run", "of 100".
    #[props(into, optional)]
    note: Option<String>,
) -> Element {
    let unit = unit.unwrap_or_default();
    let delta = delta.filter(|d| !d.is_empty());
    let note = note.filter(|n| !n.is_empty());
    let has_comparison =
        delta.is_some() || previous.is_some() || target.is_some() || note.is_some();
    rsx! {
        div { class: "ads-stat card-press",
            span { class: "eyebrow", "{label}" }
            span { class: "num ads-stat__value",
                "{value}"
                if !unit.is_empty() { span { class: "ads-stat__unit", "{unit}" } }
            }
            if has_comparison {
                span { class: "ads-stat__compare",
                    if let Some(d) = &delta { AdsDelta { direction, value: d.clone() } }
                    if let Some(p) = &previous { span { class: "num ads-stat__note", "prev {p}{unit}" } }
                    if let Some(t) = &target { span { class: "num ads-stat__note", "target {t}{unit}" } }
                    if let Some(n) = &note { span { class: "num ads-stat__note", "{n}" } }
                }
            }
        }
    }
}

/// A position along `max`, as a CSS percentage clamped to the track.
fn at(n: u32, max: u32) -> String {
    let pct = if max == 0 {
        0.0
    } else {
        (f64::from(n) / f64::from(max) * 100.0).clamp(0.0, 100.0)
    };
    format!("{pct}%")
}

/// A quantity against a threshold. Distinct from progress, which is a task
/// moving toward completion. The threshold is required: it draws the mark the
/// reader needs to know whether the fill is good news, and a meter without one
/// is decoration. The label is required too — it is the meter's name, for the
/// eye and for a screen reader alike.
#[component]
pub fn AdsMeter(
    value: u32,
    #[props(default = 100u32)] max: u32,
    threshold: u32,
    #[props(into)] label: String,
    #[props(into, optional)] unit: Option<String>,
) -> Element {
    let unit = unit.unwrap_or_default();
    let value_text = format!("{value}{unit} of {max}{unit}, threshold {threshold}{unit}");
    let fill = at(value, max);
    let mark = at(threshold, max);
    rsx! {
        div { class: "ads-meter__wrap",
            span { class: "ads-meter__head",
                span { class: "ads-meter__label", "{label}" }
                span { class: "num ads-meter__value", "{value}{unit}" }
            }
            div {
                class: "ads-meter",
                role: "meter",
                "aria-label": "{label}",
                "aria-valuenow": "{value}",
                "aria-valuemin": "0",
                "aria-valuemax": "{max}",
                "aria-valuetext": "{value_text}",
                i { style: "width:{fill}" }
                b { style: "left:{mark}" }
            }
        }
    }
}

/// A task moving toward completion. Distinct from a meter, which is a quantity
/// against a threshold — so there is no threshold here, only an end. The label
/// names the task ("Crawling"); a progress bar with no name is a bar a screen
/// reader can only call "progress".
#[component]
pub fn AdsProgress(
    value: u32,
    #[props(default = 100u32)] max: u32,
    #[props(into)] label: String,
) -> Element {
    let percent = if max == 0 {
        0
    } else {
        (f64::from(value) / f64::from(max) * 100.0)
            .clamp(0.0, 100.0)
            .round() as u32
    };
    rsx! {
        div { class: "ads-progress__wrap",
            span { class: "ads-progress__head",
                span { class: "ads-progress__label", "{label}" }
                span { class: "num ads-progress__value", "{percent}%" }
            }
            div {
                class: "ads-progress",
                role: "progressbar",
                "aria-label": "{label}",
                "aria-valuenow": "{value}",
                "aria-valuemin": "0",
                "aria-valuemax": "{max}",
                i { style: "width:{percent}%" }
            }
        }
    }
}

/// Key-value detail, which is what a selected row almost always needs. A
/// two-column grid rather than a table, so the values can wrap.
#[component]
pub fn AdsDl(#[props(into)] items: Vec<(String, String)>) -> Element {
    rsx! {
        dl { class: "ads-dl",
            for (term, value) in &items {
                dt { "{term}" }
                dd { "{value}" }
            }
        }
    }
}

/// A data-grid column. Rows are aligned to columns by index.
#[derive(Default, Clone, PartialEq)]
pub struct GridColumn {
    pub key: String,
    pub label: String,
    /// Shown in the header after the label: "Latency (ms)".
    pub unit: Option<String>,
    /// Right-aligned, tabular. Inferred (`None`) when every cell in the column
    /// parses as a number.
    pub numeric: Option<bool>,
    /// Monospaced but left-aligned: identifiers, hashes, paths.
    pub mono: bool,
    /// Cells 1–4 rendered as a luminance grade with a dot and a word.
    pub grade: bool,
}

impl GridColumn {
    #[must_use]
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            unit: None,
            numeric: None,
            mono: false,
            grade: false,
        }
    }
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
    /// Force the column numeric (or not), instead of inferring it.
    #[must_use]
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = Some(numeric);
        self
    }
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
    #[must_use]
    pub fn grade(mut self) -> Self {
        self.grade = true;
        self
    }
}

/// A cell as a number, if it reads as one ("1,204" and "12.5" both do).
fn as_number(cell: &str) -> Option<f64> {
    let cleaned: String = cell
        .chars()
        .filter(|c| *c != ',' && !c.is_whitespace())
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn compare_cells(x: Option<&String>, y: Option<&String>) -> Ordering {
    let x = x.map(String::as_str).unwrap_or("");
    let y = y.map(String::as_str).unwrap_or("");
    match (as_number(x), as_number(y)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => x.cmp(y),
    }
}

/// A missing cell reads as a dash, not as nothing.
fn display(cell: Option<&String>) -> String {
    cell.filter(|v| !v.is_empty())
        .map_or_else(|| "—".to_owned(), Clone::clone)
}

// Four luminance steps stand in for severity. There is no red here, so the
// rule is simply: the worse it is, the brighter it is — and the word says it.
const GRADE_LABEL: [&str; 5] = ["", "low", "medium", "high", "critical"];

fn as_grade(cell: Option<&String>) -> u8 {
    cell.and_then(|c| c.trim().parse::<f64>().ok())
        .filter(|n| *n != 0.0 && n.is_finite())
        .map_or(1, |n| n.clamp(1.0, 4.0) as u8)
}

/// The table from the app, on the web.
///
/// Rows are separated by a whisper rather than a rule — never striped, because
/// a stripe is a second signal competing with the cursor and the grades.
/// Numeric columns are right-aligned in tabular figures so they read by
/// magnitude down the page; text stays left. A column's unit lives in its
/// header, once, not in every cell. The header sorts, and a keyboard cursor
/// moves with the arrow keys; Enter (or a click) selects the row, which is the
/// cue to open it in a drawer beside the table — never in a modal over it.
#[component]
pub fn AdsDataGrid(
    #[props(into)] columns: Vec<GridColumn>,
    #[props(into)] rows: Vec<Vec<String>>,
    #[props(into, optional)] caption: Option<String>,
    /// Fires with the row (as displayed, after sorting) and its index.
    #[props(optional)]
    onselect: Option<EventHandler<(Vec<String>, usize)>>,
) -> Element {
    let mut sort_key = use_signal(|| None::<usize>);
    let mut sort_asc = use_signal(|| true);
    let mut cursor = use_signal(|| None::<usize>);

    let is_num: Vec<bool> = columns
        .iter()
        .enumerate()
        .map(|(ci, c)| {
            !c.grade
                && c.numeric.unwrap_or_else(|| {
                    !rows.is_empty()
                        && rows
                            .iter()
                            .all(|r| r.get(ci).is_some_and(|v| as_number(v).is_some()))
                })
        })
        .collect();

    let mut sorted = rows.clone();
    if let Some(k) = sort_key() {
        let asc = sort_asc();
        sorted.sort_by(|a, b| {
            let cmp = compare_cells(a.get(k), b.get(k));
            if asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }
    let len = sorted.len();
    // The rows may have shrunk under the cursor.
    let cur = cursor().filter(|_| len > 0).map(|c| c.min(len - 1));

    // The cursor is announced, since a moving highlight is invisible to a
    // screen reader: the first column names the row.
    let announce = match cur {
        Some(c) if !columns.is_empty() => {
            format!("{}, row {} of {len}", display(sorted[c].first()), c + 1)
        }
        _ => String::new(),
    };
    let group_label = format!(
        "{}. Arrow keys move, Enter opens the row.",
        caption.as_deref().unwrap_or("Data table")
    );

    let keyed = sorted.clone();
    let onkey = move |e: KeyboardEvent| {
        if len == 0 {
            return;
        }
        let last = len - 1;
        let at = cur.map_or(-1, |c| c as i64);
        let to = match e.key() {
            Key::ArrowDown => at + 1,
            Key::ArrowUp => at - 1,
            Key::PageDown => at + 10,
            Key::PageUp => at - 10,
            Key::Home => 0,
            Key::End => last as i64,
            Key::Enter => {
                if let (Some(c), Some(h)) = (cur, &onselect) {
                    h.call((keyed[c].clone(), c));
                }
                if cur.is_some() {
                    e.prevent_default();
                }
                return;
            }
            _ => return,
        };
        e.prevent_default();
        cursor.set(Some(to.clamp(0, last as i64) as usize));
    };

    rsx! {
        div {
            class: "ads-data-grid",
            tabindex: "0",
            role: "group",
            "aria-label": "{group_label}",
            onkeydown: onkey,
            table { class: "ads-data-grid__table",
                if let Some(c) = &caption { caption { class: "sr-only", "{c}" } }
                thead {
                    tr {
                        for (ci, c) in columns.iter().enumerate() {
                            th {
                                key: "{c.key}",
                                scope: "col",
                                class: if is_num[ci] { "ads-data-grid__th is-num" } else { "ads-data-grid__th" },
                                "aria-sort": match (sort_key() == Some(ci), sort_asc()) {
                                    (true, true) => "ascending",
                                    (true, false) => "descending",
                                    _ => "none",
                                },
                                button {
                                    r#type: "button",
                                    class: "ads-data-grid__sort",
                                    onclick: move |_| {
                                        if sort_key() == Some(ci) {
                                            sort_asc.toggle();
                                        } else {
                                            sort_key.set(Some(ci));
                                            sort_asc.set(true);
                                        }
                                    },
                                    "{c.label}"
                                    if let Some(u) = &c.unit { span { class: "ads-data-grid__unit", " ({u})" } }
                                    span {
                                        class: crate::class::join(&[
                                            "ads-data-grid__caret",
                                            if sort_key() == Some(ci) { "is-active" } else { "" },
                                            if sort_key() == Some(ci) && !sort_asc() { "is-desc" } else { "" },
                                        ]),
                                        "aria-hidden": "true",
                                        "↑"
                                    }
                                }
                            }
                        }
                    }
                }
                tbody {
                    for (i, r) in sorted.iter().enumerate() {
                        tr {
                            key: "{i}",
                            class: if cur == Some(i) { "ads-data-grid__row is-cursor" } else { "ads-data-grid__row" },
                            "aria-current": if cur == Some(i) { Some("true") } else { None },
                            onmouseenter: move |_| cursor.set(Some(i)),
                            onclick: {
                                let row = r.clone();
                                move |_| {
                                    if let Some(h) = &onselect {
                                        h.call((row.clone(), i));
                                    }
                                }
                            },
                            for (ci, c) in columns.iter().enumerate() {
                                td {
                                    key: "{c.key}",
                                    class: crate::class::join(&[
                                        "ads-data-grid__td",
                                        if is_num[ci] { "is-num" } else { "" },
                                        if is_num[ci] || c.mono { "num" } else { "" },
                                    ]),
                                    if c.grade {
                                        AdsStatus {
                                            grade: as_grade(r.get(ci)),
                                            label: GRADE_LABEL[as_grade(r.get(ci)) as usize],
                                        }
                                    } else {
                                        {display(r.get(ci))}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            span { class: "sr-only", "aria-live": "polite", "{announce}" }
        }
    }
}
