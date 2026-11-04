//! Gantt chart — an engagement timeline. Phases are stored as fractions of the
//! engagement, so one set of numbers serves every duration.

use dioxus::prelude::*;

/// A phase of the engagement, from/to as fractions (0..1).
#[derive(Clone, PartialEq)]
pub struct GanttPhase {
    pub name: String,
    pub tech: Option<String>,
    pub from: f32,
    pub to: f32,
    pub detail: String,
}

impl GanttPhase {
    #[must_use]
    pub fn new(name: impl Into<String>, from: f32, to: f32) -> Self {
        Self {
            name: name.into(),
            tech: None,
            from,
            to,
            detail: String::new(),
        }
    }
    #[must_use]
    pub fn tech(mut self, tech: impl Into<String>) -> Self {
        self.tech = Some(tech.into());
        self
    }
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }
}

fn default_phases() -> Vec<GanttPhase> {
    vec![
        GanttPhase::new("Discovery & read", 0.0, 0.12)
            .detail("We use the product the way your users do and write down every question the screen implies but does not answer."),
        GanttPhase::new("Information design", 0.08, 0.3)
            .detail("The decisions the interface exists to support, ranked. Mostly argument rather than pixels, and where most of the value is."),
        GanttPhase::new("Design system", 0.22, 0.5)
            .tech("Flutter")
            .detail("Tokens, surfaces and type as a themed library — light, dark and density modes from one source."),
        GanttPhase::new("Interface build", 0.38, 0.84)
            .tech("Flutter")
            .detail("Screens assembled from the system and reviewed in a branch. Design and front-end code happen together, never as a handover."),
        GanttPhase::new("Engine & data", 0.3, 0.72)
            .tech("Rust")
            .detail("The work behind the chart: queries, aggregation and whatever has to stay fast when the dataset stops being a demo."),
        GanttPhase::new("Platform build", 0.58, 0.9)
            .tech("iOS")
            .detail("Native shell, platform conventions and the parts of the interface that should not be a web view."),
        GanttPhase::new("Testing & access", 0.62, 0.96)
            .detail("Keyboard paths, contrast, reduced motion and screen readers — checked against the build rather than promised in a deck."),
        GanttPhase::new("Handover", 0.88, 1.0)
            .detail("Tokens, components and the reasoning behind them, written down so the next person does not have to guess."),
    ]
}

// from-index → to-index for the default phases. Drawn as elbows, the way a real
// Gantt shows that one piece of work cannot begin until another has produced
// something.
const DEFAULT_LINKS: &[(usize, usize)] = &[(0, 1), (1, 2), (2, 3), (1, 4), (4, 5), (3, 6), (6, 7)];

const DURATIONS: &[(u32, &str)] = &[(4, "1 month"), (8, "2 months"), (12, "3 months")];

/// Row height in px; the link elbows do their y arithmetic on it.
const ROW: f32 = 46.0;

fn pct(n: f32) -> String {
    format!("{:.3}%", n * 100.0)
}

fn week_range(p: &GanttPhase, weeks: u32) -> String {
    let w = weeks as f32;
    let first = ((p.from * w).ceil() as u32).max(1);
    let last = ((p.to * w).ceil() as u32).max(first);
    if first == last {
        format!("wk {first}")
    } else {
        format!("wk {first}–{last}")
    }
}

/// The engagement, as a Gantt chart.
///
/// The length control rescales the whole plan, because a four-week job and a
/// twelve-week job are the same shape at different sizes; and the marker moves
/// across the timeline to ask "what is happening in week six?" — which is the
/// only question anyone actually brings to a Gantt chart. Phase extents are
/// fractions of the engagement rather than dates, so one set of numbers serves
/// every duration and the bars are positioned in percentages that need no
/// measuring.
///
/// `weeks` is the starting length (4, 8 or 12). The marker moves with the
/// arrow keys, Home and End; the Vue kit also drags it with the pointer, which
/// needs the track measured, so that part is not ported.
#[component]
pub fn GanttChart(
    #[props(into, optional)] phases: Option<Vec<GanttPhase>>,
    /// Dependencies as (from, to) phase indices. Defaults to the default plan's
    /// links when `phases` is not given, and to none otherwise.
    #[props(into, optional)]
    links: Option<Vec<(usize, usize)>>,
    #[props(default = 8u32)] weeks: u32,
) -> Element {
    let links = links.unwrap_or_else(|| {
        if phases.is_none() {
            DEFAULT_LINKS.to_vec()
        } else {
            Vec::new()
        }
    });
    let phases = phases.unwrap_or_else(default_phases);

    let mut weeks_sig = use_signal(|| weeks.max(1));
    // Where the scrub marker sits, as a fraction of the engagement.
    let mut at = use_signal(|| 0.42f32);
    let mut selected = use_signal(|| None::<usize>);

    let w = weeks_sig();
    let now = at();
    let current_week = ((now * w as f32).ceil() as u32).max(1);
    let active = phases
        .iter()
        .map(|p| now >= p.from && now <= p.to)
        .collect::<Vec<_>>();
    let length_label = DURATIONS
        .iter()
        .find(|(n, _)| *n == w)
        .map_or_else(|| format!("{w} weeks"), |(_, l)| (*l).to_owned());

    // Each elbow is two absolutely-positioned rules rather than an SVG overlay,
    // because the x coordinates are percentages of a flexible track and the y
    // coordinates are multiples of a fixed row height.
    let elbows = links
        .iter()
        .filter_map(|&(a, b)| {
            let (from, to) = (phases.get(a)?, phases.get(b)?);
            let top = a.min(b) as f32 * ROW + ROW / 2.0;
            let height = a.abs_diff(b) as f32 * ROW;
            let arm_top = b as f32 * ROW + ROW / 2.0;
            let x = pct(from.to);
            Some((
                format!("{a}-{b}"),
                format!("left:{x};top:{top}px;height:{height}px"),
                format!(
                    "left:{x};top:{arm_top}px;width:{}",
                    pct((to.from - from.to).max(0.0))
                ),
            ))
        })
        .collect::<Vec<_>>();

    let onkey = move |e: KeyboardEvent| {
        let step = 1.0 / (weeks_sig() as f32 * 2.0);
        let next = match e.key() {
            Key::ArrowRight => (at() + step).min(1.0),
            Key::ArrowLeft => (at() - step).max(0.0),
            Key::Home => 0.0,
            Key::End => 1.0,
            _ => return,
        };
        e.prevent_default();
        at.set(next);
    };

    let picked = selected().and_then(|i| phases.get(i).cloned());
    let mut toggle =
        move |i: usize| selected.set(if selected() == Some(i) { None } else { Some(i) });

    rsx! {
        div { class: "ads-gantt-chart card",
            header { class: "ads-gantt-chart__head",
                div {
                    span { class: "eyebrow", "Typical engagement" }
                    h3 { class: "h3", "{length_label}, start to handover" }
                }
                div { class: "ads-segmented", role: "group", "aria-label": "Engagement length",
                    for (n, l) in DURATIONS.iter().copied() {
                        button {
                            key: "{n}",
                            r#type: "button",
                            class: if w == n { "is-on" } else { "" },
                            "aria-pressed": if w == n { "true" } else { "false" },
                            onclick: move |_| weeks_sig.set(n),
                            "{l}"
                        }
                    }
                }
            }

            div { class: "ads-gantt-chart__scroller",
                div { class: "ads-gantt-chart__chart",
                    // Left column: the phase list
                    div { class: "ads-gantt-chart__names",
                        div { class: "ads-gantt-chart__col-head",
                            span { class: "eyebrow", "Phase" }
                            span { class: "eyebrow", "Weeks" }
                        }
                        for (i, p) in phases.iter().enumerate() {
                            button {
                                key: "{p.name}",
                                r#type: "button",
                                class: crate::class::join(&[
                                    "ads-gantt-chart__name-row",
                                    if active[i] { "is-active" } else { "" },
                                    if selected() == Some(i) { "is-picked" } else { "" },
                                ]),
                                "aria-expanded": if selected() == Some(i) { "true" } else { "false" },
                                onclick: move |_| toggle(i),
                                span { class: "ads-gantt-chart__name",
                                    "{p.name}"
                                    if let Some(t) = &p.tech { span { class: "ads-gantt-chart__tech num", "{t}" } }
                                }
                                span { class: "ads-gantt-chart__weeks num", {week_range(p, w)} }
                            }
                        }
                    }

                    // Right column: the timeline
                    div { class: "ads-gantt-chart__lanes",
                        div { class: "ads-gantt-chart__col-head ads-gantt-chart__weeks-head",
                            for n in 1..=w {
                                span { key: "{n}", class: "ads-gantt-chart__week num", style: "width:{pct(1.0 / w as f32)}", "{n}" }
                            }
                        }

                        div {
                            class: "ads-gantt-chart__track",
                            style: "--cols:{w}",
                            role: "slider",
                            tabindex: "0",
                            "aria-valuemin": "1",
                            "aria-valuemax": "{w}",
                            "aria-valuenow": "{current_week}",
                            "aria-valuetext": "Week {current_week} of {w}",
                            "aria-label": "Scrub the timeline",
                            onkeydown: onkey,
                            // Dependency elbows, drawn under the bars
                            for (key, v, _) in &elbows {
                                span { key: "{key}", class: "ads-gantt-chart__elbow-v", style: "{v}" }
                            }
                            for (key, _, h) in &elbows {
                                span { key: "{key}-arm", class: "ads-gantt-chart__elbow-h", style: "{h}" }
                            }
                            for (i, p) in phases.iter().enumerate() {
                                div { key: "{p.name}", class: "ads-gantt-chart__lane", style: "height:{ROW}px",
                                    span {
                                        class: crate::class::join(&[
                                            "ads-gantt-chart__bar",
                                            if active[i] { "is-active" } else { "" },
                                            if selected() == Some(i) { "is-picked" } else { "" },
                                        ]),
                                        style: "left:{pct(p.from)};width:{pct(p.to - p.from)}",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            toggle(i);
                                        },
                                        span { class: "ads-gantt-chart__bar-label",
                                            {p.tech.clone().unwrap_or_else(|| p.name.clone())}
                                        }
                                    }
                                }
                            }
                            // The scrub marker
                            span { class: "ads-gantt-chart__marker", style: "left:{pct(now)}",
                                span { class: "ads-gantt-chart__flag num", "wk {current_week}" }
                            }
                        }
                    }
                }
            }

            footer { class: "ads-gantt-chart__foot",
                p { class: "body ads-gantt-chart__readout",
                    span { class: "eyebrow", "Week {current_week}" }
                    span { class: "ads-gantt-chart__active-list",
                        for (i, p) in phases.iter().enumerate().filter(|(i, _)| active[*i]) {
                            span { key: "{i}", class: "ads-gantt-chart__active-item", "{p.name}" }
                        }
                        if !active.contains(&true) {
                            span { class: "dim", "Nothing scheduled — drag the marker." }
                        }
                    }
                }
                if let Some(ph) = &picked {
                    p { class: "body ads-gantt-chart__detail",
                        strong { "{ph.name}." }
                        " {ph.detail}"
                    }
                }
            }
        }
    }
}
