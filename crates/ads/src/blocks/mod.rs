//! Blocks: the dashboard patterns (KPI row, filter bar, table screen,
//! summary → detail), the marketing sections, the site's own sections and the
//! docs building blocks.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

pub mod docs;
pub mod filter_bar;
pub mod kpi_row;
pub mod screen_tour;
pub mod sections;
pub mod statement;
pub mod story;
pub mod summary_detail;
pub mod table_screen;
pub mod work_list;
pub mod work_section;

pub use docs::{AppWindow, Callout, CalloutType, DocGroup, DocNav, DocPage, DocShell, Screenshot};
pub use filter_bar::FilterBar;
pub use kpi_row::{KpiRow, StatItem};
pub use screen_tour::{ScreenTour, TourScreen};
pub use sections::{
    ContactForm, ContactMessage, ContentSection, CtaBand, Faq, FaqSection, Feature, FeatureGrid,
    FooterColumn, HeroSection, ImageSection, LogoMarquee, NavLink, Plan, PricingSection,
    SectionAlign, SiteFooter, SiteNav,
};
pub use statement::StatementSection;
pub use story::StorySection;
pub use summary_detail::SummaryDetail;
pub use table_screen::TableScreen;
pub use work_list::{WorkItem, WorkList};
pub use work_section::{WorkCard, WorkSection};

/// An id that is stable for the life of the component, for `aria-controls`
/// and `aria-labelledby` — the Dioxus stand-in for Vue's `useId()`.
pub(crate) fn use_uid(prefix: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let prefix = prefix.to_owned();
    use_hook(move || format!("{prefix}-{}", NEXT.fetch_add(1, Ordering::Relaxed)))
}

/// Move focus to a mounted element, if it has been mounted.
pub(crate) fn focus(el: Option<Rc<MountedData>>) {
    if let Some(el) = el {
        spawn(async move {
            let _ = el.set_focus(true).await;
        });
    }
}
