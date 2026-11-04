//! Widgets: calendar, carousel, resizable, questionnaire, trace.

use std::collections::HashMap;
use std::rc::Rc;

use dioxus::html::geometry::PixelsVector2D;
use dioxus::html::ScrollBehavior;
use dioxus::prelude::*;

use super::support::{date, reduced_motion, sleep, use_synced, use_today, use_uid, ScrollBox};
use crate::icons::Icon;

const WEEKDAYS: [(&str, &str); 7] = [
    ("Mo", "Monday"),
    ("Tu", "Tuesday"),
    ("We", "Wednesday"),
    ("Th", "Thursday"),
    ("Fr", "Friday"),
    ("Sa", "Saturday"),
    ("Su", "Sunday"),
];

fn month_of(iso: &str) -> Option<String> {
    date::parse(iso).map(|_| iso[..7].to_owned())
}

/// A month grid for picking one day. Dates are ISO (the month reads
/// "2026-10", the value "2026-10-27") and every figure is in the data face so
/// the columns line up. It is a real grid: arrows move a day or a week,
/// PageUp/PageDown a month (with Shift, a year), Home/End go to the week's
/// ends, and each day is a button that reads out its full date. The chosen
/// day takes the accent; today is pressed in, so the two never depend on
/// colour to be told apart.
///
/// `value`/`onchange` and `month`/`onmonthchange` are the two Vue models;
/// either can be left unbound. `autofocus` puts focus on the roving day when
/// the grid mounts, for a calendar that opens in a popover.
#[component]
pub fn AdsCalendar(
    #[props(into, default = "Calendar".to_string())] label: String,
    #[props(into, default)] value: String,
    #[props(optional)] onchange: Option<EventHandler<String>>,
    #[props(into, optional)] month: Option<String>,
    #[props(optional)] onmonthchange: Option<EventHandler<String>>,
    #[props(default = false)] autofocus: bool,
) -> Element {
    let uid = use_uid();
    let today = use_today();
    let today_iso = today();
    let mut sel = use_synced(&value);
    let mut shown = use_signal(|| {
        month
            .clone()
            .filter(|m| date::parse_month(m).is_some())
            .or_else(|| month_of(&value))
            .or_else(|| month_of(&today.peek()))
            .unwrap_or_default()
    });
    let mut focus_iso = use_signal(|| {
        if date::parse(&value).is_some() {
            value.clone()
        } else {
            String::new()
        }
    });
    let mut refs = use_signal(HashMap::<String, Rc<MountedData>>::new);
    let mut did_autofocus = use_signal(|| false);

    let mut set_month = move |m: String| {
        if *shown.peek() != m {
            shown.set(m.clone());
            if let Some(h) = &onmonthchange {
                h.call(m);
            }
        }
    };

    // The caller's month wins when it changes.
    use_effect(use_reactive((&month,), move |(m,)| {
        if let Some(m) = m.filter(|m| date::parse_month(m).is_some()) {
            if *shown.peek() != m {
                shown.set(m);
            }
        }
    }));
    // A new value brings its month into view.
    use_effect(use_reactive((&value,), move |(v,)| {
        if let Some(m) = month_of(&v) {
            focus_iso.set(v);
            set_month(m);
        }
    }));

    // The month on view; until the reader's today is known it may be empty.
    let shown_now = shown();
    let cur = date::parse_month(&shown_now)
        .or_else(|| date::parse_month(today_iso.get(..7).unwrap_or("")));
    let month_text = cur
        .map(|(y, m)| format!("{y:04}-{m:02}"))
        .unwrap_or_default();

    // The roving day: the focused day if it is in this month, else the same
    // day number clamped to the month, else today, else the 1st.
    let roving = cur.map_or_else(String::new, |(y, m)| {
        let dim = date::days_in_month(y, m);
        let focus_now = focus_iso();
        let day = match date::parse(&focus_now) {
            Some(z) => date::to_civil(z).2,
            None if today_iso.starts_with(&month_text) => {
                date::parse(&today_iso).map_or(1, |z| date::to_civil(z).2)
            }
            None => 1,
        };
        date::iso(date::from_civil(y, m, i64::from(day.min(dim))))
    });

    let weeks: Vec<Vec<Option<String>>> = cur.map_or_else(Vec::new, |(y, m)| {
        let first = date::from_civil(y, m, 1);
        let lead = date::weekday(first) as usize;
        let count = date::days_in_month(y, m);
        let mut cells: Vec<Option<String>> = vec![None; lead];
        cells.extend((0..count).map(|d| Some(date::iso(first + i64::from(d)))));
        while !cells.len().is_multiple_of(7) {
            cells.push(None);
        }
        cells.chunks(7).map(<[_]>::to_vec).collect()
    });

    let shift_month = move |n: i64| {
        if let Some((y, m)) = date::parse_month(&shown.peek()) {
            let z = date::add_months(date::from_civil(y, m, 1), n);
            set_month(date::iso(z)[..7].to_owned());
        }
    };
    let mut prev = shift_month;
    let mut next = shift_month;

    let focus_day = move |iso: String| {
        spawn(async move {
            sleep(0).await;
            let el = refs.peek().get(&iso).cloned();
            if let Some(el) = el {
                let _ = el.set_focus(true).await;
            }
        });
    };

    let roving_k = roving.clone();
    let current_value = sel();
    let month_id = format!("{uid}-month");

    rsx! {
        div { class: "ads-calendar", role: "group", "aria-labelledby": "{month_id}",
            div { class: "ads-calendar__head",
                button {
                    r#type: "button",
                    class: "ads-calendar__nav",
                    "aria-label": "Previous month",
                    onclick: move |_| prev(-1),
                    {super::support::glyph("arrow", 14, "", Some("transform: rotate(180deg)"))}
                }
                p { id: "{month_id}", class: "ads-calendar__month", "aria-live": "polite",
                    span { class: "sr-only", "{label}, " }
                    "{month_text}"
                }
                button {
                    r#type: "button",
                    class: "ads-calendar__nav",
                    "aria-label": "Next month",
                    onclick: move |_| next(1),
                    Icon { name: "arrow", size: 14 }
                }
            }
            table {
                class: "ads-calendar__grid",
                role: "grid",
                "aria-labelledby": "{month_id}",
                onkeydown: move |e| {
                    let Some(z) = date::parse(&roving_k) else { return };
                    let dow = i64::from(date::weekday(z));
                    let jump = if e.modifiers().contains(Modifiers::SHIFT) { 12 } else { 1 };
                    let to = match e.key() {
                        Key::ArrowLeft => z - 1,
                        Key::ArrowRight => z + 1,
                        Key::ArrowUp => z - 7,
                        Key::ArrowDown => z + 7,
                        Key::Home => z - dow,
                        Key::End => z + (6 - dow),
                        Key::PageUp => date::add_months(z, -jump),
                        Key::PageDown => date::add_months(z, jump),
                        _ => return,
                    };
                    e.prevent_default();
                    let iso = date::iso(to);
                    focus_iso.set(iso.clone());
                    set_month(iso[..7].to_owned());
                    focus_day(iso);
                },
                thead {
                    tr {
                        for (short, full) in WEEKDAYS {
                            th { key: "{short}", scope: "col", class: "ads-calendar__weekday",
                                abbr { title: "{full}", "{short}" }
                            }
                        }
                    }
                }
                tbody {
                    for (w, week) in weeks.into_iter().enumerate() {
                        tr { key: "{month_text}-{w}",
                            for (i, day) in week.into_iter().enumerate() {
                                {
                                    let chosen = day.as_deref() == Some(current_value.as_str());
                                    let selected = day.as_ref().map(|_| if chosen { "true" } else { "false" });
                                    rsx! {
                                        td {
                                            key: "{i}",
                                            role: "gridcell",
                                            "aria-selected": selected,
                                            if let Some(d) = day {
                                                {
                                                    let is_today = d == today_iso;
                                                    let is_roving = d == roving;
                                                    let class = crate::class::join(&[
                                                        "ads-calendar__day",
                                                        if chosen { "is-on" } else { "" },
                                                        if is_today { "is-today" } else { "" },
                                                    ]);
                                                    let num = d[8..].trim_start_matches('0').to_owned();
                                                    let (d_mount, d_click, d_focus) = (d.clone(), d.clone(), d.clone());
                                                    rsx! {
                                                        button {
                                                            r#type: "button",
                                                            class: "{class}",
                                                            "data-iso": "{d}",
                                                            tabindex: if is_roving { "0" } else { "-1" },
                                                            "aria-label": "{d}",
                                                            "aria-current": if is_today { Some("date") } else { None },
                                                            "aria-pressed": if chosen { "true" } else { "false" },
                                                            onmounted: move |e| {
                                                                let el = e.data();
                                                                refs.write().insert(d_mount.clone(), el.clone());
                                                                if autofocus && is_roving && !*did_autofocus.peek() {
                                                                    did_autofocus.set(true);
                                                                    spawn(async move {
                                                                        let _ = el.set_focus(true).await;
                                                                    });
                                                                }
                                                            },
                                                            onclick: move |_| {
                                                                sel.set(d_click.clone());
                                                                focus_iso.set(d_click.clone());
                                                                if let Some(h) = &onchange {
                                                                    h.call(d_click.clone());
                                                                }
                                                            },
                                                            onfocus: move |_| {
                                                                if *focus_iso.peek() != d_focus {
                                                                    focus_iso.set(d_focus.clone());
                                                                }
                                                            },
                                                            "{num}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A horizontal rail of cards that scrolls by a page at a time. The arrows
/// are real buttons named for what they scroll, and switch off at the ends;
/// the rail is a focusable group, so the arrow keys scroll it natively. Edge
/// fades show only where there is more to see.
#[component]
pub fn AdsCarousel(#[props(into)] label: String, children: Element) -> Element {
    let mut rail = use_signal(|| None::<Rc<MountedData>>);
    let mut at_start = use_signal(|| true);
    let mut at_end = use_signal(|| true);

    let mut apply = move |b: ScrollBox| {
        let s = b.left <= 1.0;
        let e = b.left + b.client_width >= b.width - 1.0;
        if *at_start.peek() != s {
            at_start.set(s);
        }
        if *at_end.peek() != e {
            at_end.set(e);
        }
    };
    let remeasure = move || {
        let el = rail.peek().clone();
        if let Some(el) = el {
            spawn(async move {
                if let Some(b) = ScrollBox::measure(&el).await {
                    apply(b);
                }
            });
        }
    };
    let scroll = move |dir: f64| {
        let el = rail.peek().clone();
        if let Some(el) = el {
            spawn(async move {
                let Some(b) = ScrollBox::measure(&el).await else {
                    return;
                };
                let behaviour = if reduced_motion().await {
                    ScrollBehavior::Instant
                } else {
                    ScrollBehavior::Smooth
                };
                let _ = el
                    .scroll(
                        PixelsVector2D::new(b.left + dir * b.client_width * 0.8, b.top),
                        behaviour,
                    )
                    .await;
            });
        }
    };

    let start = at_start();
    let end = at_end();
    let rail_class = crate::class::join(&[
        "ads-carousel__rail",
        if start { "" } else { "is-more-start" },
        if end { "" } else { "is-more-end" },
    ]);
    let remeasure_resize = remeasure;
    rsx! {
        section { class: "ads-carousel", "aria-label": "{label}", "aria-roledescription": "carousel",
            button {
                r#type: "button",
                class: "ads-carousel__button is-prev",
                "aria-label": "Scroll {label} back",
                disabled: start,
                onclick: move |_| scroll(-1.0),
                Icon { name: "arrow", size: 15 }
            }
            div {
                class: "{rail_class}",
                tabindex: "0",
                role: "group",
                "aria-label": "{label} items",
                onmounted: move |e| {
                    rail.set(Some(e.data()));
                    remeasure();
                },
                onscroll: move |e| apply(ScrollBox::from_event(&e.data())),
                onresize: move |_| remeasure_resize(),
                {children}
            }
            button {
                r#type: "button",
                class: "ads-carousel__button",
                "aria-label": "Scroll {label} forward",
                disabled: end,
                onclick: move |_| scroll(1.0),
                Icon { name: "arrow", size: 15 }
            }
        }
    }
}

/// A panel whose width the reader sets. The handle is a focusable separator
/// with its value in pixels, so it moves from the keyboard too: arrows by 16
/// (Shift, 64), Home and End to the limits. `width`/`onwidthchange` are the
/// Vue `v-model:width`; unbound, it keeps its own width.
#[component]
pub fn AdsResizable(
    #[props(default = 280u32)] width: u32,
    #[props(default = 160u32)] min: u32,
    #[props(default = 720u32)] max: u32,
    #[props(into, default = "Resize panel".to_string())] label: String,
    #[props(optional)] onwidthchange: Option<EventHandler<u32>>,
    children: Element,
) -> Element {
    let uid = use_uid();
    let handle_id = format!("{uid}-handle");
    let mut w = use_synced(&width);
    let mut dragging = use_signal(|| false);
    let mut root = use_signal(|| None::<Rc<MountedData>>);
    let mut left = use_signal(|| None::<f64>);

    let clamp = move |n: f64| -> u32 {
        // Within u32 after the clamp, so the cast is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = n.clamp(f64::from(min), f64::from(max)).round() as u32;
        v
    };
    let mut set = move |v: u32| {
        if *w.peek() != v {
            w.set(v);
            if let Some(h) = &onwidthchange {
                h.call(v);
            }
        }
    };
    let mut set_drag = set;

    let now = w();
    let class = crate::class::join(&["ads-resizable", if dragging() { "is-dragging" } else { "" }]);
    let hid = handle_id.clone();
    rsx! {
        div {
            class: "{class}",
            style: "width:{now}px",
            onmounted: move |e| root.set(Some(e.data())),
            div { class: "ads-resizable__body", {children} }
            div {
                id: "{handle_id}",
                class: "ads-resizable__handle",
                role: "separator",
                tabindex: "0",
                "aria-orientation": "vertical",
                "aria-label": "{label}",
                "aria-valuenow": "{now}",
                "aria-valuemin": "{min}",
                "aria-valuemax": "{max}",
                "aria-valuetext": "{now} pixels",
                onpointerdown: move |e| {
                    e.prevent_default();
                    dragging.set(true);
                    left.set(None);
                    let pid = e.pointer_id();
                    let _ = document::eval(&format!(
                        "document.getElementById('{hid}')?.setPointerCapture({pid});"
                    ));
                    let el = root.peek().clone();
                    if let Some(el) = el {
                        spawn(async move {
                            if let Ok(r) = el.get_client_rect().await {
                                left.set(Some(r.origin.x));
                            }
                        });
                    }
                },
                onpointermove: move |e| {
                    if !dragging() {
                        return;
                    }
                    if let Some(l) = *left.peek() {
                        set_drag(clamp(e.client_coordinates().x - l));
                    }
                },
                onpointerup: move |_| dragging.set(false),
                onpointercancel: move |_| dragging.set(false),
                onkeydown: move |e| {
                    let step = if e.modifiers().contains(Modifiers::SHIFT) { 64.0 } else { 16.0 };
                    let cur = f64::from(*w.peek());
                    let to = match e.key() {
                        Key::ArrowLeft => cur - step,
                        Key::ArrowRight => cur + step,
                        Key::Home => f64::from(min),
                        Key::End => f64::from(max),
                        _ => return,
                    };
                    e.prevent_default();
                    set(clamp(to));
                },
            }
        }
    }
}

/// One question in a questionnaire.
#[derive(Clone, PartialEq, Debug)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub options: Vec<String>,
    /// Several answers allowed, not one.
    pub multi: bool,
}

impl Question {
    #[must_use]
    pub fn new(id: impl Into<String>, text: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            options,
            multi: false,
        }
    }

    /// Allow several answers.
    #[must_use]
    pub fn multi(mut self) -> Self {
        self.multi = true;
        self
    }
}

/// A short run of questions, one at a time. The chosen option is pressed in
/// with a tick and says so (`aria-pressed`); progress is greyscale, since it
/// is not the thing to click; Next stays off until there is an answer; and
/// each new question takes focus, so a screen reader hears it at once.
#[component]
pub fn AdsQuestionnaire(
    questions: Vec<Question>,
    #[props(into, default = "Send answers".to_string())] submit_label: String,
    #[props(optional)] ondone: Option<EventHandler<HashMap<String, Vec<String>>>>,
) -> Element {
    let mut step = use_signal(|| 0usize);
    let mut answers = use_signal(HashMap::<String, Vec<String>>::new);
    let mut heading = use_signal(|| None::<Rc<MountedData>>);

    let total = questions.len();
    let at = step().min(total.saturating_sub(1));
    let current = questions.get(at).cloned();
    #[allow(clippy::cast_precision_loss)]
    let progress = if total == 0 {
        0.0
    } else {
        (at + 1) as f64 / total as f64 * 100.0
    };
    let picked: Vec<String> = current
        .as_ref()
        .and_then(|q| answers.read().get(&q.id).cloned())
        .unwrap_or_default();
    let answered = !picked.is_empty();
    let last = at + 1 >= total;

    let mut go = move |n: usize| {
        step.set(n);
        spawn(async move {
            sleep(0).await;
            let el = heading.peek().clone();
            if let Some(el) = el {
                let _ = el.set_focus(true).await;
            }
        });
    };
    let mut go_next = go;

    let (text, multi, options, qid) = match &current {
        Some(q) => (q.text.clone(), q.multi, q.options.clone(), q.id.clone()),
        None => (String::new(), false, Vec::new(), String::new()),
    };
    let shown_step = at + 1;
    rsx! {
        div { class: "ads-questionnaire",
            div { class: "ads-questionnaire__progress", "aria-hidden": "true",
                span { class: "ads-questionnaire__bar", style: "width:{progress}%" }
            }
            p { class: "ads-questionnaire__count num",
                span { class: "sr-only", "Question " }
                "{shown_step}"
                span { "aria-hidden": "true", " / " }
                span { class: "sr-only", " of " }
                "{total}"
            }
            h3 {
                class: "ads-questionnaire__question h3",
                tabindex: "-1",
                onmounted: move |e| heading.set(Some(e.data())),
                "{text}"
            }
            div {
                class: "ads-questionnaire__options",
                role: "group",
                "aria-label": if multi { "Choose any that apply" } else { "Choose one" },
                for o in options.into_iter() {
                    {
                        let on = picked.contains(&o);
                        let qid = qid.clone();
                        let opt = o.clone();
                        rsx! {
                            button {
                                key: "{o}",
                                r#type: "button",
                                class: crate::class::join(&["ads-questionnaire__option", if on { "is-on" } else { "" }]),
                                "aria-pressed": if on { "true" } else { "false" },
                                onclick: move |_| {
                                    let mut all = answers.write();
                                    let cur = all.entry(qid.clone()).or_default();
                                    if multi {
                                        if cur.contains(&opt) {
                                            cur.retain(|x| x != &opt);
                                        } else {
                                            cur.push(opt.clone());
                                        }
                                    } else {
                                        *cur = vec![opt.clone()];
                                    }
                                },
                                if on { Icon { name: "check", size: 13 } }
                                "{o}"
                            }
                        }
                    }
                }
            }
            div { class: "ads-questionnaire__foot",
                if at > 0 {
                    button { r#type: "button", class: "btn btn-quiet", onclick: move |_| go(at - 1), "Previous question" }
                }
                button {
                    r#type: "button",
                    class: "btn btn-accent",
                    disabled: !answered,
                    onclick: move |_| {
                        if !answered {
                            return;
                        }
                        if last {
                            if let Some(h) = &ondone {
                                h.call(answers.peek().clone());
                            }
                        } else {
                            go_next(at + 1);
                        }
                    },
                    if last { "{submit_label}" } else { "Next question" }
                }
            }
        }
    }
}

/// Where a trace step stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TraceState {
    Done,
    Now,
    Todo,
}

impl TraceState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Now => "now",
            Self::Todo => "todo",
        }
    }

    const fn spoken(self) -> &'static str {
        match self {
            Self::Done => "Done",
            Self::Now => "In progress",
            Self::Todo => "To do",
        }
    }
}

/// One step in a trace.
#[derive(Clone, PartialEq, Debug)]
pub struct TraceStep {
    pub title: String,
    pub meta: Option<String>,
    /// Left unset, the first step is done and the rest are to do.
    pub state: Option<TraceState>,
}

impl TraceStep {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: None,
            state: None,
        }
    }

    #[must_use]
    pub fn state(mut self, state: TraceState) -> Self {
        self.state = Some(state);
        self
    }

    #[must_use]
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }
}

/// A run of steps on a line: what has happened, what is happening, what is
/// next. The state is in the shape (a tick, a pressed dot, an empty one) and
/// in words for a screen reader, never in colour alone; the current step is
/// `aria-current="step"`.
#[component]
pub fn AdsTrace(steps: Vec<TraceStep>, #[props(into, optional)] label: Option<String>) -> Element {
    let count = steps.len();
    rsx! {
        ol { class: "ads-trace", "aria-label": label,
            for (i, s) in steps.iter().enumerate() {
                {
                    let st = s.state.unwrap_or(if i == 0 { TraceState::Done } else { TraceState::Todo });
                    let class = format!("ads-trace__step is-{}", st.as_str());
                    rsx! {
                        li {
                            key: "{s.title}",
                            class: "{class}",
                            "aria-current": if st == TraceState::Now { Some("step") } else { None },
                            span { class: "ads-trace__dot", "aria-hidden": "true",
                                if st == TraceState::Done { Icon { name: "check", size: 10 } }
                            }
                            div { class: "ads-trace__body",
                                span { class: "ads-trace__title",
                                    "{s.title}"
                                    span { class: "sr-only", ", {st.spoken()}" }
                                }
                                if let Some(m) = &s.meta {
                                    span { class: "ads-trace__meta num", "{m}" }
                                }
                            }
                            if i + 1 < count {
                                span { class: "ads-trace__line", "aria-hidden": "true" }
                            }
                        }
                    }
                }
            }
        }
    }
}
