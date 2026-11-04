//! Empty state, skeleton, loading, banner, tooltip, modal, drawer, popover,
//! and the toast host.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::icons::Icon;

/// Focus a mounted element.
fn focus(el: Rc<MountedData>) {
    spawn(async move {
        let _ = el.set_focus(true).await;
    });
}

/// Which of the two empties this is.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum EmptyKind {
    /// Never had data: the action creates something.
    #[default]
    FirstRun,
    /// A filter matched nothing: the action clears the filter.
    NoResults,
}

impl EmptyKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstRun => "first-run",
            Self::NoResults => "no-results",
        }
    }

    const fn glyph(self) -> &'static str {
        match self {
            Self::FirstRun => "discovery",
            Self::NoResults => "compass",
        }
    }
}

/// The first screen every new user sees, so it gets a component rather than
/// a grey sentence. It teaches what appears here, why it is empty, and carries
/// the one action that fixes it in its children — not in a toolbar nobody has
/// looked at. `kind` keeps the two empties apart: `FirstRun` (never had data:
/// the action creates something) and `NoResults` (a filter matched nothing:
/// the action clears the filter). Different glyph, different copy.
#[component]
pub fn AdsEmpty(
    #[props(into)] title: String,
    #[props(default)] kind: EmptyKind,
    #[props(into, optional)] icon: Option<String>,
    #[props(into, optional)] body: Option<String>,
    children: Element,
) -> Element {
    let glyph = icon.unwrap_or_else(|| kind.glyph().to_string());
    rsx! {
        div { class: "ads-empty",
            span { class: "glyph", Icon { name: glyph, size: 20 } }
            h4 { "{title}" }
            if let Some(b) = body.filter(|b| !b.is_empty()) { p { "{b}" } }
            {children}
        }
    }
}

/// A placeholder that holds the layout at the real content's dimensions, so
/// nothing shifts when the data lands. A spinner over the page throws away a
/// layout the reader had already started reading.
#[component]
pub fn AdsSkeleton(
    #[props(into, default = "100%".to_string())] width: String,
    #[props(into, default = "1em".to_string())] height: String,
    #[props(optional)] pill: bool,
) -> Element {
    rsx! {
        div {
            class: if pill { "ads-skeleton round-pill" } else { "ads-skeleton" },
            style: "width:{width};height:{height}",
            "aria-hidden": "true",
        }
    }
}

/// Inline loading, next to the thing that is loading so the rest of the
/// screen stays usable. A spinner where the shape is unknown; a skeleton where
/// it is known. The text is required: a spinner alone tells a screen reader
/// nothing and a sighted reader only that something, somewhere, is busy.
#[component]
pub fn AdsLoading(#[props(into)] text: String, #[props(default = 18u32)] size: u32) -> Element {
    rsx! {
        span { class: "ads-inline-load", role: "status", "aria-live": "polite",
            span { class: "ads-spinner", style: "width:{size}px;height:{size}px", "aria-hidden": "true" }
            "{text}"
        }
    }
}

/// How serious a banner's condition is. A closed set: the colour follows the
/// tone, never the caller.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum BannerTone {
    #[default]
    Info,
    Warn,
    Bad,
}

impl BannerTone {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Bad => "bad",
        }
    }

    const fn glyph(self) -> &'static str {
        match self {
            Self::Info => "signal",
            Self::Warn | Self::Bad => "alert",
        }
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Info => "",
            Self::Warn => "Warning",
            Self::Bad => "Action needed",
        }
    }
}

/// A banner: a condition that persists, and stays until it is resolved — the
/// place for an error the reader must act on, which a toast would carry away
/// before it was read. The glyph and a short word carry the state, so it
/// survives without its colour; colour is spent only on the glyph, and only
/// for `Warn` and `Bad`. An error is announced as an alert; anything else
/// waits its turn. `actions` sit at the end of the row.
#[component]
pub fn AdsBanner(
    #[props(into)] text: String,
    #[props(default)] tone: BannerTone,
    #[props(into, optional)] icon: Option<String>,
    #[props(optional)] actions: Option<Element>,
) -> Element {
    let glyph = icon.unwrap_or_else(|| tone.glyph().to_string());
    let word = tone.word();
    rsx! {
        div { class: "ads-banner", role: if tone == BannerTone::Bad { "alert" } else { "status" },
            span { class: "glyph ads-banner__glyph is-{tone.as_str()}", Icon { name: glyph, size: 17 } }
            span { class: "ads-banner__body",
                if !word.is_empty() { strong { class: "ads-banner__tone", "{word}" } " " }
                "{text}"
            }
            if let Some(a) = actions { {a} }
        }
    }
}

/// Which side of its trigger a tooltip sits on.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum TooltipPosition {
    #[default]
    Top,
    Bottom,
}

impl TooltipPosition {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

/// A tooltip answers a question only if asked: on hover or focus, never on
/// its own. The trigger is the children; the bubble reads `text`. Escape
/// dismisses it without moving focus. The bubble's id is `id` (generated if
/// not given): put `aria-describedby` with it on the trigger, so the tooltip
/// is heard as well as seen.
#[component]
pub fn AdsTooltip(
    #[props(into)] text: String,
    #[props(default)] position: TooltipPosition,
    #[props(into, optional)] id: Option<String>,
    children: Element,
) -> Element {
    let uid = crate::blocks::use_uid("ads-tooltip");
    let id = id.unwrap_or(uid);
    let mut dismissed = use_signal(|| false);
    rsx! {
        span {
            class: if dismissed() { "ads-tooltip__anchor is-dismissed" } else { "ads-tooltip__anchor" },
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    dismissed.set(true);
                }
            },
            onmouseleave: move |_| dismissed.set(false),
            onfocusout: move |_| dismissed.set(false),
            {children}
            span {
                id: "{id}",
                class: "ads-tooltip ads-tooltip__bubble is-{position.as_str()}",
                role: "tooltip",
                "{text}"
            }
        }
    }
}

/// A modal on a scrim. It blocks everything until answered, so it is kept for
/// the few moments that cannot proceed without a decision — a destructive
/// confirmation, where the verb goes on the button ("Delete project", never
/// "OK") and the button stays default weight, never accent. Row detail is a
/// drawer, not this. The title is required: it names the dialog for assistive
/// tech. Escape, the scrim and the close button all call `onclose`; focus
/// moves into the panel when it opens.
#[component]
pub fn AdsModal(
    #[props(into)] title: String,
    #[props(optional)] open: bool,
    #[props(into, optional)] width: Option<String>,
    #[props(optional)] onclose: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let uid = crate::blocks::use_uid("ads-modal");
    let title_id = format!("{uid}-title");
    let close = move || {
        if let Some(h) = &onclose {
            h.call(());
        }
    };
    rsx! {
        if open {
            div {
                class: "ads-modal__root",
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        close();
                    }
                },
                div { class: "ads-modal__scrim", onclick: move |_| close() }
                div {
                    class: "ads-modal",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-labelledby": "{title_id}",
                    tabindex: "-1",
                    style: width.as_ref().filter(|w| !w.is_empty()).map(|w| format!("max-width:{w}")),
                    onmounted: move |e: MountedEvent| focus(e.data()),
                    onclick: move |e| e.stop_propagation(),
                    header { class: "ads-modal__head",
                        h3 { id: "{title_id}", class: "ads-modal__title", "{title}" }
                        button {
                            r#type: "button",
                            class: "ads-modal__close",
                            "aria-label": "Close",
                            onclick: move |_| close(),
                            Icon { name: "plus", size: 16, class: "ads-modal__close-glyph" }
                        }
                    }
                    {children}
                }
            }
        }
    }
}

/// A right-hand detail panel. In a dashboard this is where a selected row
/// explains itself, so the table never has to leave the screen — which is why
/// row detail opens here and never in a modal: a modal hides the very rows
/// the reader is comparing against.
///
/// The panel is a labelled landmark. Given an `onclose` handler it carries a
/// close button and Escape closes it — a button that does nothing is worse
/// than no button. When it opens, focus moves into it. The children are the
/// main content; `panel` is the detail.
#[component]
pub fn AdsDrawer(
    #[props(into, optional)] title: Option<String>,
    #[props(default = true)] open: bool,
    #[props(optional)] onclose: Option<EventHandler<()>>,
    #[props(optional)] panel: Option<Element>,
    children: Element,
) -> Element {
    let closable = onclose.is_some();
    let label = title.clone().unwrap_or_else(|| "Detail".to_string());
    rsx! {
        div { class: if open { "ads-drawer" } else { "ads-drawer is-closed" },
            div { class: "ads-drawer__main", {children} }
            if open {
                aside {
                    class: "panel ads-drawer__panel",
                    tabindex: "-1",
                    "aria-label": "{label}",
                    onmounted: move |e: MountedEvent| focus(e.data()),
                    onkeydown: move |e: KeyboardEvent| {
                        if e.key() != Key::Escape {
                            return;
                        }
                        if let Some(h) = &onclose {
                            e.stop_propagation();
                            h.call(());
                        }
                    },
                    if title.is_some() || closable {
                        div { class: "ads-drawer__head",
                            if let Some(t) = &title { span { class: "ads-label", "{t}" } }
                            if closable {
                                button {
                                    r#type: "button",
                                    class: "ads-icon-button ads-drawer__close",
                                    "aria-label": "Close detail",
                                    onclick: move |_| { if let Some(h) = &onclose { h.call(()); } },
                                    Icon { name: "plus", size: 14, style: "transform: rotate(45deg)" }
                                }
                            }
                        }
                    }
                    if let Some(p) = panel { {p} }
                }
            }
        }
    }
}

/// A link at the foot of a popover.
#[derive(Default, Clone, PartialEq)]
pub struct PopoverLink {
    pub label: String,
    pub to: Option<String>,
}

impl PopoverLink {
    #[must_use]
    pub fn new(label: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: Some(to.into()),
        }
    }
}

/// A small amount of extra context, anchored to what prompted it. More than a
/// tooltip, less than a drawer — supplementary by definition. Nothing the
/// reader needs to finish the task may live only in here: if it matters, it
/// belongs on the page; if it is a row's detail, it belongs in the drawer.
///
/// Without `trigger_label` it renders as a static card, for layouts that
/// place it themselves. With one, it brings its own button: the button
/// reports `aria-expanded`, and Escape closes the panel and returns focus to
/// the button.
#[component]
pub fn AdsPopover(
    #[props(into, optional)] label: Option<String>,
    #[props(into, optional)] body: Option<String>,
    #[props(optional)] link: Option<PopoverLink>,
    #[props(into, optional)] trigger_label: Option<String>,
    children: Element,
) -> Element {
    let id = crate::blocks::use_uid("ads-popover");
    let mut open = use_signal(|| false);
    let mut trigger = use_signal(|| None::<Rc<MountedData>>);
    let label = label.filter(|l| !l.is_empty());
    let body = body.filter(|b| !b.is_empty());

    let content = rsx! {
        if let Some(l) = &label { span { class: "ads-label", "{l}" } }
        if let Some(b) = &body { span { class: "ads-popover__body", "{b}" } }
        {children}
        if let Some(l) = &link {
            a { class: "ads-link", href: l.to.clone().unwrap_or_else(|| "#".to_string()), "{l.label}" }
        }
    };

    let Some(trigger_label) = trigger_label else {
        return rsx! {
            div { class: "ads-popover", {content} }
        };
    };
    let name = label.clone().unwrap_or_else(|| trigger_label.clone());
    rsx! {
        span {
            class: "ads-popover__anchor",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() != Key::Escape || !open() {
                    return;
                }
                e.stop_propagation();
                open.set(false);
                if let Some(t) = trigger() {
                    focus(t);
                }
            },
            button {
                r#type: "button",
                class: "btn btn-quiet btn-sm",
                "aria-expanded": if open() { "true" } else { "false" },
                "aria-controls": "{id}",
                onmounted: move |e: MountedEvent| trigger.set(Some(e.data())),
                onclick: move |_| open.toggle(),
                "{trigger_label}"
            }
            div {
                id: "{id}",
                class: "ads-popover is-anchored",
                role: "dialog",
                "aria-label": "{name}",
                style: if open() { None } else { Some("display: none;") },
                {content}
            }
        }
    }
}

/// The glyphs a toast may carry. There is no alert: a toast confirms
/// something that already happened, so it never carries an error.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ToastIcon {
    #[default]
    Check,
    Mail,
    Code,
    Arrow,
    Layers,
}

impl ToastIcon {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Mail => "mail",
            Self::Code => "code",
            Self::Arrow => "arrow",
            Self::Layers => "layers",
        }
    }
}

/// One toast: an id to dismiss it by, its words and its glyph.
#[derive(Default, Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub icon: ToastIcon,
}

impl Toast {
    #[must_use]
    pub fn new(id: u64, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
            icon: ToastIcon::default(),
        }
    }
    #[must_use]
    pub const fn icon(mut self, icon: ToastIcon) -> Self {
        self.icon = icon;
        self
    }
}

/// The toast host. Mount once, near the root, and pass it the list of toasts
/// you keep; a click on a toast calls `ondismiss` with its id. A toast
/// confirms something that already happened and leaves on its own, so it
/// never carries an error — `ToastIcon` has no alert glyph to give it. The
/// glyph sits in a small well, pressed into the toast rather than tinted.
#[component]
pub fn AdsToastHost(
    #[props(into)] toasts: Vec<Toast>,
    #[props(optional)] ondismiss: Option<EventHandler<u64>>,
) -> Element {
    rsx! {
        div { class: "ads-toast-host", role: "status", "aria-live": "polite",
            for t in toasts {
                button {
                    key: "{t.id}",
                    r#type: "button",
                    class: "ads-toast ads-toast-host__toast",
                    onclick: move |_| { if let Some(h) = &ondismiss { h.call(t.id); } },
                    span { class: "ads-toast-host__icon", Icon { name: t.icon.as_str(), size: 15 } }
                    "{t.text}"
                }
            }
        }
    }
}
