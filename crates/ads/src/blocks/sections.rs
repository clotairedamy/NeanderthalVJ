//! Marketing page sections: the nav and footer, the hero and the sections
//! between them, and the close.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::blocks::{focus, use_uid};
use crate::class::join;
use crate::icons::Icon;
use crate::ui::{
    AdsAccordion, AdsButton, AdsField, AdsThemeToggle, BrandMark, ButtonType, ButtonVariant,
    FieldType, Wordmark,
};

/// A link in the nav, the footer or the docs sidebar.
#[derive(Clone, PartialEq)]
pub struct NavLink {
    pub label: String,
    pub to: String,
}

impl NavLink {
    #[must_use]
    pub fn new(label: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: to.into(),
        }
    }
}

/// A sitemap column above the footer row.
#[derive(Clone, PartialEq)]
pub struct FooterColumn {
    pub title: String,
    pub links: Vec<NavLink>,
}

impl FooterColumn {
    #[must_use]
    pub fn new(title: impl Into<String>, links: Vec<NavLink>) -> Self {
        Self {
            title: title.into(),
            links,
        }
    }
}

/// A feature card: a glyph, a title and a line of body.
#[derive(Clone, PartialEq)]
pub struct Feature {
    pub icon: String,
    pub title: String,
    pub body: String,
}

impl Feature {
    #[must_use]
    pub fn new(icon: impl Into<String>, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            title: title.into(),
            body: body.into(),
        }
    }
}

/// A pricing plan. `period` is set as written ("/mo"); `cta` is the button's
/// verb and object.
#[derive(Clone, PartialEq)]
pub struct Plan {
    pub name: String,
    pub price: String,
    pub period: Option<String>,
    pub description: String,
    pub features: Vec<String>,
    pub cta: String,
    pub featured: bool,
}

impl Plan {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        price: impl Into<String>,
        description: impl Into<String>,
        cta: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            price: price.into(),
            period: None,
            description: description.into(),
            features: Vec::new(),
            cta: cta.into(),
            featured: false,
        }
    }
    #[must_use]
    pub fn period(mut self, period: impl Into<String>) -> Self {
        self.period = Some(period.into());
        self
    }
    #[must_use]
    pub fn features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
    #[must_use]
    pub fn featured(mut self) -> Self {
        self.featured = true;
        self
    }
}

/// A question and its answer.
#[derive(Clone, PartialEq)]
pub struct Faq {
    pub title: String,
    pub body: String,
}

impl Faq {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// How a section head sits: centred, or on the left edge.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SectionAlign {
    #[default]
    Center,
    Left,
}

impl SectionAlign {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Left => "left",
        }
    }
}

fn default_nav_links() -> Vec<NavLink> {
    vec![
        NavLink::new("Overview", "/"),
        NavLink::new("Dashboard", "/dashboard"),
        NavLink::new("Contact", "/contact"),
    ]
}

fn default_footer_links() -> Vec<NavLink> {
    vec![
        NavLink::new("Dashboard", "/dashboard"),
        NavLink::new("Contact", "/contact"),
        NavLink::new("Privacy", "/privacy"),
    ]
}

/// Drop an empty string, so `""` and "not given" mean the same, as in the Vue.
fn given(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.is_empty())
}

/// The site navigation: brand left, links in the middle, the theme switch and
/// a call to action right. Collapses to a menu button and a sheet on a phone.
///
/// The bar sits over every screen, so its CTA is default weight unless
/// `cta_accent` asks: the hero or the closing band owns the one accent, and a
/// sticky bar that spent it too would double it on every screen it passes
/// over. The brand link is the mark plus the `Wordmark`; when the name is
/// shown it names the link, and `home_label` names it only when it is the bare
/// mark (`brand_name: ""`).
///
/// Opening the sheet moves focus into it; Escape closes it and hands focus
/// back to the button that opened it, so a keyboard reader never loses their
/// place. There is no router here, so pass `current` (the path on show) to
/// mark the active link, and `dark` / `ontheme` to drive the theme switch.
#[component]
pub fn SiteNav(
    #[props(into, optional)] links: Option<Vec<NavLink>>,
    #[props(into, default = "/contact".to_string())] cta_to: String,
    #[props(into, default = "Start a project".to_string())] cta_label: String,
    #[props(default = false)] cta_accent: bool,
    #[props(into, default = "Home".to_string())] home_label: String,
    #[props(into, default = "Artelier".to_string())] brand_name: String,
    #[props(into, optional)] current: Option<String>,
    #[props(optional)] dark: bool,
    #[props(optional)] ontheme: Option<EventHandler<MouseEvent>>,
) -> Element {
    let links = links.unwrap_or_else(default_nav_links);
    let mut open = use_signal(|| false);
    let mut burger = use_signal(|| None::<Rc<MountedData>>);
    let sheet_id = use_uid("ads-site-nav-sheet");
    let is_open = open();
    let cta_class = if cta_accent { "btn btn-accent" } else { "btn" };
    let is_current = |to: &str| current.as_deref() == Some(to);
    // The sheet's first link takes focus when it opens; with no links, the CTA.
    let cta_first = links.is_empty();

    rsx! {
        header {
            class: "ads-site-nav",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape && open() {
                    open.set(false);
                    focus(burger());
                }
            },
            div { class: "ads-site-nav__bar max",
                a {
                    class: "ads-site-nav__brand",
                    href: "/",
                    "aria-label": if brand_name.is_empty() { Some(home_label.clone()) } else { None },
                    BrandMark { size: 28 }
                    if !brand_name.is_empty() {
                        Wordmark { text: brand_name.clone() }
                    }
                }

                nav { class: "ads-site-nav__links", "aria-label": "Main",
                    for l in &links {
                        a {
                            key: "{l.to}",
                            class: join(&["ads-site-nav__link", if is_current(&l.to) { "is-active" } else { "" }]),
                            href: "{l.to}",
                            "aria-current": if is_current(&l.to) { Some("page") } else { None },
                            "{l.label}"
                        }
                    }
                }

                div { class: "ads-site-nav__actions",
                    AdsThemeToggle {
                        dark,
                        onclick: move |e| { if let Some(h) = &ontheme { h.call(e); } },
                    }
                    if !cta_to.is_empty() {
                        a { class: "{cta_class} btn-sm ads-site-nav__cta", href: "{cta_to}", "{cta_label}" }
                    }
                    button {
                        class: "ads-site-nav__burger",
                        r#type: "button",
                        "aria-expanded": if is_open { "true" } else { "false" },
                        "aria-controls": "{sheet_id}",
                        "aria-label": "Menu",
                        onmounted: move |e| burger.set(Some(e.data())),
                        onclick: move |_| open.set(!is_open),
                        Icon {
                            name: if is_open { "plus" } else { "grid" },
                            size: 18,
                            class: join(&["ads-site-nav__burger-glyph", if is_open { "is-open" } else { "" }]),
                        }
                    }
                }
            }

            if is_open {
                nav { id: "{sheet_id}", class: "ads-site-nav__sheet", "aria-label": "Main",
                    for (i, l) in links.iter().enumerate() {
                        a {
                            key: "{l.to}",
                            class: join(&["ads-site-nav__sheet-link", if is_current(&l.to) { "is-active" } else { "" }]),
                            href: "{l.to}",
                            "aria-current": if is_current(&l.to) { Some("page") } else { None },
                            onmounted: move |e| { if i == 0 { focus(Some(e.data())); } },
                            onclick: move |_| open.set(false),
                            "{l.label}"
                        }
                    }
                    if !cta_to.is_empty() {
                        a {
                            class: "{cta_class} ads-site-nav__sheet-cta",
                            href: "{cta_to}",
                            onmounted: move |e| { if cta_first { focus(Some(e.data())); } },
                            onclick: move |_| open.set(false),
                            "{cta_label}"
                        }
                    }
                }
            }
        }
    }
}

/// An address as people say it: no scheme, no www, no trailing slash.
fn bare_host(href: &str) -> &str {
    let h = href
        .strip_prefix("https://")
        .or_else(|| href.strip_prefix("http://"))
        .unwrap_or(href);
    let h = h.strip_prefix("www.").unwrap_or(h);
    h.strip_suffix('/').unwrap_or(h)
}

/// The footer, shaped like the studio site's own: one quiet row under a
/// hairline — the mark, the domain in mono, the links, the year in mono. A
/// larger site can pass `columns` for a sitemap above that row. Nothing here
/// is raised or coloured; the footer is the last place to spend attention.
/// Like the site, the row shows the bare mark; pass `brand_name` to spell it
/// out. The crate keeps no clock, so the caller passes the `year`.
#[component]
pub fn SiteFooter(
    year: i32,
    #[props(into, optional)] links: Option<Vec<NavLink>>,
    #[props(into, optional)] columns: Option<Vec<FooterColumn>>,
    #[props(into, optional)] description: Option<String>,
    #[props(into, optional)] domain: Option<String>,
    #[props(into, optional)] email: Option<String>,
    #[props(into, optional)] owner: Option<String>,
    /// Makes the owner a link, e.g. to their own site.
    #[props(into, optional)]
    owner_href: Option<String>,
    #[props(into, optional)] brand_name: Option<String>,
) -> Element {
    let links = links.unwrap_or_else(default_footer_links);
    let columns = columns.unwrap_or_default();
    let (description, domain, email, owner, brand_name) = (
        given(description),
        given(domain),
        given(email),
        given(owner),
        given(brand_name),
    );
    let owner_href = given(owner_href);

    rsx! {
        footer { class: "ads-site-footer",
            div { class: "ads-site-footer__inner max",
                if !columns.is_empty() {
                    div { class: "ads-site-footer__sitemap",
                        if let Some(d) = &description { p { class: "ads-site-footer__desc", "{d}" } }
                        for col in &columns {
                            nav { key: "{col.title}", class: "ads-site-footer__col", "aria-label": "{col.title}",
                                span { class: "eyebrow", "{col.title}" }
                                for l in &col.links {
                                    a { class: "ads-site-footer__link", href: "{l.to}", "{l.label}" }
                                }
                            }
                        }
                    }
                }

                div { class: "ads-site-footer__bar",
                    div { class: "ads-site-footer__brand",
                        BrandMark { size: 24 }
                        if let Some(b) = &brand_name { Wordmark { text: b.clone(), size: 15 } }
                        if let Some(d) = &domain { span { class: "num ads-site-footer__domain", "{d}" } }
                    }
                    if !links.is_empty() || email.is_some() {
                        nav { class: "ads-site-footer__links", "aria-label": "Footer",
                            for l in &links {
                                a { class: "ads-site-footer__link", href: "{l.to}", "{l.label}" }
                            }
                            if let Some(e) = &email {
                                a { class: "ads-site-footer__link", href: "mailto:{e}", "{e}" }
                            }
                        }
                    }
                    // © year, the owner, and the owner's own site spelled
                    // out. The gap spaces them, not the markup.
                    span { class: "ads-site-footer__copy",
                        span { class: "num", "© {year}" }
                        if let Some(o) = &owner { span { class: "ads-site-footer__owner", "{o}" } }
                        if let Some(h) = &owner_href {
                            span { class: "ads-site-footer__dot", "aria-hidden": "true", "·" }
                            a { class: "num ads-site-footer__link", href: "{h}", rel: "author", "{bare_host(h)}" }
                        }
                    }
                }
            }
        }
    }
}

/// The hero: a big display headline over a soft visual. The visual is a slot,
/// so the landing page can drop in a chart or a dashboard mock.
///
/// Actions are a slot too, but they follow the studio's hero: one accent link
/// on the action the page exists for, a default one beside it. If the nav or
/// anything else above the fold already spends the accent, this does not.
#[component]
pub fn HeroSection(
    #[props(into)] title: String,
    #[props(into, optional)] eyebrow: Option<String>,
    #[props(into, optional)] lead: Option<String>,
    #[props(default = SectionAlign::Center)] align: SectionAlign,
    #[props(optional)] actions: Option<Element>,
    #[props(optional)] visual: Option<Element>,
) -> Element {
    let inner = join(&[
        "ads-hero-section__inner max",
        if align == SectionAlign::Center {
            "is-center"
        } else {
            ""
        },
    ]);
    let (eyebrow, lead) = (given(eyebrow), given(lead));
    rsx! {
        section { class: "ads-hero-section",
            div { class: "{inner}",
                div { class: "ads-hero-section__copy",
                    if let Some(e) = &eyebrow { p { class: "eyebrow", "{e}" } }
                    h1 { class: "ads-hero-section__title", "{title}" }
                    if let Some(l) = &lead { p { class: "lede ads-hero-section__lead", "{l}" } }
                    if let Some(a) = actions {
                        div { class: "ads-hero-section__actions", {a} }
                    }
                }
                if let Some(v) = visual {
                    div { class: "ads-hero-section__visual", {v} }
                }
            }
        }
    }
}

/// A content section: eyebrow, headline and lede, then any body. The workhorse
/// of a marketing page — a titled block of prose or a titled grid. The head is
/// the studio's section head: label, display headline, one lede, and the body
/// below in cards rather than strips.
#[component]
pub fn ContentSection(
    #[props(into)] title: String,
    #[props(into, optional)] eyebrow: Option<String>,
    #[props(into, optional)] lead: Option<String>,
    #[props(default = SectionAlign::Left)] align: SectionAlign,
    children: Element,
) -> Element {
    let head = join(&[
        "ads-content-section__head",
        if align == SectionAlign::Center {
            "is-center"
        } else {
            ""
        },
    ]);
    let (eyebrow, lead) = (given(eyebrow), given(lead));
    rsx! {
        section { class: "ads-content-section",
            div { class: "ads-content-section__inner max",
                header { class: "{head}",
                    if let Some(e) = &eyebrow { p { class: "eyebrow", "{e}" } }
                    h2 { class: "ads-content-section__title h2", "{title}" }
                    if let Some(l) = &lead { p { class: "lede", "{l}" } }
                }
                div { class: "ads-content-section__body", {children} }
            }
        }
    }
}

/// A split section: copy on one side, an image (or any visual) on the other.
/// `flip` swaps them. The visual is the children and sits in a raised frame —
/// depth, not a border, is what separates it from the page.
#[component]
pub fn ImageSection(
    #[props(into)] title: String,
    #[props(into, optional)] eyebrow: Option<String>,
    #[props(into, optional)] body: Option<String>,
    #[props(default = false)] flip: bool,
    #[props(optional)] actions: Option<Element>,
    children: Element,
) -> Element {
    let inner = join(&[
        "ads-image-section__inner max",
        if flip { "is-flipped" } else { "" },
    ]);
    let (eyebrow, body) = (given(eyebrow), given(body));
    rsx! {
        section { class: "ads-image-section",
            div { class: "{inner}",
                div { class: "ads-image-section__copy",
                    if let Some(e) = &eyebrow { p { class: "eyebrow", "{e}" } }
                    h2 { class: "ads-image-section__title h2", "{title}" }
                    if let Some(b) = &body { p { class: "ads-image-section__body", "{b}" } }
                    if let Some(a) = actions { {a} }
                }
                figure { class: "ads-image-section__frame", {children} }
            }
        }
    }
}

/// A grid of feature cards. Each is a raised card with a glyph in a well, a
/// title and a line of body — the studio's service cards. Hover presses the
/// card in rather than lifting it. `columns` is 2, 3 or 4; anything else is
/// held to that range.
#[component]
pub fn FeatureGrid(
    #[props(into)] features: Vec<Feature>,
    #[props(default = 3u8)] columns: u8,
) -> Element {
    let columns = columns.clamp(2, 4);
    rsx! {
        div { class: "ads-feature-grid", style: "--ads-feature-grid-cols: {columns}",
            for f in &features {
                article { key: "{f.title}", class: "card card-press ads-feature-grid__item",
                    span { class: "ads-feature-grid__glyph", Icon { name: f.icon.clone(), size: 20 } }
                    h3 { class: "ads-feature-grid__title h3", "{f.title}" }
                    p { class: "ads-feature-grid__body", "{f.body}" }
                }
            }
        }
    }
}

/// A row of wordmarks, for "used by" or partner logos. Kept monochrome so they
/// do not compete with the data; the accent is not spent here.
///
/// It wraps rather than scrolls. A logo row is read once, and a strip that
/// moves forever costs every reader attention to show the same six names — so
/// there is nothing here for reduced motion to stop.
#[component]
pub fn LogoMarquee(#[props(into, optional)] names: Option<Vec<String>>) -> Element {
    let names = names.unwrap_or_else(|| {
        [
            "Northwind",
            "Vantage",
            "Halo",
            "Meridian",
            "Atlas",
            "Orbital",
        ]
        .map(String::from)
        .to_vec()
    });
    rsx! {
        ul { class: "ads-logo-marquee", role: "list",
            for (i, n) in names.iter().enumerate() {
                li { key: "{i}", class: "ads-logo-marquee__name", "{n}" }
            }
        }
    }
}

/// Pricing cards. The recommended plan is told apart by depth and value, never
/// by a border or a hue: it stands further off the page, its figure is the
/// brightest, and its button carries the one accent. Only the first plan
/// marked `featured` gets that treatment — two recommended plans recommend
/// nothing, and two accent buttons side by side cancel out.
#[component]
pub fn PricingSection(
    #[props(into)] plans: Vec<Plan>,
    #[props(into, default = "Recommended".to_string())] featured_label: String,
    #[props(optional)] onselect: Option<EventHandler<Plan>>,
) -> Element {
    let featured = plans.iter().position(|p| p.featured);
    rsx! {
        div { class: "ads-pricing-section",
            for (i, p) in plans.iter().enumerate() {
                article {
                    key: "{p.name}",
                    class: join(&["card ads-pricing-section__plan", if featured == Some(i) { "is-featured" } else { "" }]),
                    if featured == Some(i) { span { class: "ads-pricing-section__flag", "{featured_label}" } }
                    h3 { class: "ads-pricing-section__name", "{p.name}" }
                    p { class: "ads-pricing-section__price",
                        span { class: "ads-pricing-section__amount num", "{p.price}" }
                        if let Some(per) = p.period.as_deref().filter(|s| !s.is_empty()) {
                            span { class: "ads-pricing-section__period", "{per}" }
                        }
                    }
                    p { class: "ads-pricing-section__desc", "{p.description}" }
                    ul { class: "ads-pricing-section__features", role: "list",
                        for f in &p.features {
                            li { key: "{f}", class: "ads-pricing-section__feature",
                                Icon { name: "check", size: 15, class: "ads-pricing-section__tick" }
                                "{f}"
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: if featured == Some(i) { "btn btn-accent" } else { "btn" },
                        onclick: {
                            let plan = p.clone();
                            move |_| { if let Some(h) = &onselect { h.call(plan.clone()); } }
                        },
                        "{p.cta}"
                    }
                }
            }
        }
    }
}

/// Frequently asked questions, on an accordion. Progressive disclosure: show
/// the question, let the reader open the answer.
#[component]
pub fn FaqSection(#[props(into)] faqs: Vec<Faq>) -> Element {
    let items = faqs
        .iter()
        .map(|f| (f.title.clone(), f.body.clone()))
        .collect::<Vec<_>>();
    rsx! {
        div { class: "ads-faq-section max",
            AdsAccordion { items }
        }
    }
}

/// The close, shaped like the studio site's: one raised panel — label,
/// headline, a lede that says what happens next — and one filled action in
/// it. Everything above should have spent its accent carefully so that this
/// button is the brightest thing on the page by the time anyone reaches it.
/// The secondary action stays default weight, and appears only when it has
/// both a label and a destination.
#[component]
pub fn CtaBand(
    #[props(into)] title: String,
    #[props(into, optional)] eyebrow: Option<String>,
    #[props(into, optional)] body: Option<String>,
    #[props(into, default = "Start a project".to_string())] cta_label: String,
    #[props(into, default = "/contact".to_string())] cta_to: String,
    #[props(into, optional)] secondary_label: Option<String>,
    #[props(into, optional)] secondary_to: Option<String>,
) -> Element {
    let (eyebrow, body) = (given(eyebrow), given(body));
    let secondary = given(secondary_label).zip(given(secondary_to));
    rsx! {
        section { class: "ads-cta-band",
            div { class: "ads-cta-band__inner max",
                div { class: "ads-cta-band__panel card",
                    if let Some(e) = &eyebrow { span { class: "eyebrow", "{e}" } }
                    h2 { class: "ads-cta-band__title h2", "{title}" }
                    if let Some(b) = &body { p { class: "lede ads-cta-band__body", "{b}" } }
                    div { class: "ads-cta-band__actions",
                        a { class: "btn btn-accent", href: "{cta_to}",
                            "{cta_label}"
                            Icon { name: "arrow", size: 18 }
                        }
                        if let Some((l, t)) = &secondary {
                            a { class: "btn", href: "{t}", "{l}" }
                        }
                    }
                }
            }
        }
    }
}

/// What the contact form sends.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct ContactMessage {
    pub first_name: String,
    pub last_name: String,
    pub company: String,
    pub email: String,
    pub phone: String,
    pub message: String,
}

/// `^[^\s@]+@[^\s@]+\.[^\s@]+$`, without a regex engine.
fn looks_like_email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    let clean = |p: &str| !p.is_empty() && !p.contains('@') && !p.chars().any(char::is_whitespace);
    clean(local)
        && clean(domain)
        && domain
            .char_indices()
            .any(|(i, c)| c == '.' && i > 0 && i + 1 < domain.len())
}

/// The contact form. The ADS version of a classic "contact sales" form,
/// composed from `AdsField`, a checkbox and `AdsButton`.
///
/// Every field has a visible label above it — a placeholder disappears exactly
/// when it is needed. The browser's own bubbles are switched off so each error
/// can say what to do rather than that something is wrong, sit under the field
/// it belongs to, and take focus. A failed send is not a toast: the reader
/// must act on it, so it would be a banner. A good send calls `onsubmit` and
/// clears the form; posting it anywhere is the caller's job.
#[component]
pub fn ContactForm(
    #[props(into, default = "/privacy".to_string())] privacy_to: String,
    #[props(optional)] onsubmit: Option<EventHandler<ContactMessage>>,
) -> Element {
    let uid = use_uid("ads-contact-form");
    let mut first_name = use_signal(String::new);
    let mut last_name = use_signal(String::new);
    let mut company = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut phone = use_signal(String::new);
    let mut message = use_signal(String::new);
    let mut agree = use_signal(|| false);
    let mut tried = use_signal(|| false);

    // Each field's error, in the order focus should visit them. Nothing shows
    // until the first send.
    let errors = move || -> [(&'static str, Option<&'static str>); 5] {
        let shown = tried();
        let need = |bad: bool, msg: &'static str| (shown && bad).then_some(msg);
        let mail = email.read().trim().to_owned();
        [
            (
                "firstName",
                need(
                    first_name.read().trim().is_empty(),
                    "Enter your first name.",
                ),
            ),
            (
                "lastName",
                need(last_name.read().trim().is_empty(), "Enter your last name."),
            ),
            (
                "email",
                if mail.is_empty() {
                    need(true, "Enter your email address, so we can reply.")
                } else {
                    need(
                        !looks_like_email(&mail),
                        "Enter an email address like name@company.com.",
                    )
                },
            ),
            (
                "message",
                need(
                    message.read().trim().is_empty(),
                    "Tell us what you need — a sentence or two is enough.",
                ),
            ),
            (
                "agree",
                need(
                    !agree(),
                    "Tick the box to agree to the privacy policy, so we can keep your message.",
                ),
            ),
        ]
    };
    let err = errors();
    let error_of = |i: usize| err[i].1.map(String::from);
    let field_id = |key: &str| format!("{uid}-{key}");

    let submit = {
        let uid = uid.clone();
        move |e: FormEvent| {
            e.prevent_default();
            tried.set(true);
            if let Some((key, _)) = errors().into_iter().find(|(_, m)| m.is_some()) {
                let _ = document::eval(&format!("document.getElementById('{uid}-{key}')?.focus()"));
                return;
            }
            if let Some(h) = &onsubmit {
                h.call(ContactMessage {
                    first_name: first_name(),
                    last_name: last_name(),
                    company: company(),
                    email: email(),
                    phone: phone(),
                    message: message(),
                });
            }
            for mut s in [first_name, last_name, company, email, phone, message] {
                s.set(String::new());
            }
            agree.set(false);
            tried.set(false);
        }
    };

    rsx! {
        form { class: "ads-contact-form", novalidate: true, onsubmit: submit,
            div { class: "ads-contact-form__row",
                AdsField {
                    id: field_id("firstName"), label: "First name", name: "first-name", autocomplete: "given-name",
                    value: first_name(), error: error_of(0), required: true,
                    oninput: move |e: FormEvent| first_name.set(e.value()),
                }
                AdsField {
                    id: field_id("lastName"), label: "Last name", name: "last-name", autocomplete: "family-name",
                    value: last_name(), error: error_of(1), required: true,
                    oninput: move |e: FormEvent| last_name.set(e.value()),
                }
            }

            AdsField {
                label: "Company", name: "company", autocomplete: "organization", hint: "Optional.",
                value: company(),
                oninput: move |e: FormEvent| company.set(e.value()),
            }

            AdsField {
                id: field_id("email"), label: "Email", field_type: FieldType::Email, name: "email", autocomplete: "email",
                value: email(), error: error_of(2), required: true,
                oninput: move |e: FormEvent| email.set(e.value()),
            }

            AdsField {
                label: "Phone number", field_type: FieldType::Tel, name: "phone", autocomplete: "tel",
                hint: "Optional. Include the country code, e.g. +44 20 7946 0000.",
                value: phone(),
                oninput: move |e: FormEvent| phone.set(e.value()),
            }

            AdsField {
                id: field_id("message"), label: "Message", field_type: FieldType::Textarea, name: "message", rows: 4u32,
                value: message(), error: error_of(3), required: true,
                oninput: move |e: FormEvent| message.set(e.value()),
            }

            // AdsChoice with a link in its label: the checkbox markup is
            // written out here, because the Rust AdsChoice takes a plain label.
            div { class: "ads-contact-form__consent",
                label { class: "ads-choice",
                    input {
                        id: field_id("agree"),
                        r#type: "checkbox",
                        checked: agree(),
                        onchange: move |e: FormEvent| agree.set(e.checked()),
                    }
                    span { class: "ads-box",
                        svg {
                            view_box: "0 0 24 24", width: "13", height: "13", fill: "none", stroke: "currentColor",
                            stroke_width: "2.6", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                            path { d: "M4 12.5l5 5L20 6.5" }
                        }
                    }
                    span { class: "ads-choice__label",
                        "I agree to the "
                        a { class: "ads-link", href: "{privacy_to}", "privacy policy" }
                        "."
                    }
                }
                if let Some(m) = err[4].1 {
                    p { class: "ads-contact-form__error", role: "alert",
                        Icon { name: "alert", size: 14 }
                        "{m}"
                    }
                }
            }

            AdsButton {
                button_type: ButtonType::Submit,
                variant: ButtonVariant::Accent,
                icon: "arrow",
                class: "ads-contact-form__submit",
                "Send message"
            }
        }
    }
}
