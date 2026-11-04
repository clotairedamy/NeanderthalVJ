//! ADS — the Artelier Design System as Dioxus components.
//!
//! This crate mirrors `app/components/` in Dioxus. It renders the **same** CSS
//! classes (`btn`, `ads-field`, `ads-tabs`, …) and reads the same tokens, so
//! the look is identical to the Vue kit. Drop `assets/css/*` into your app and
//! it styles every component here.

pub mod blocks;
pub mod charts;
pub mod class;
pub mod icons;
pub mod section;
pub mod ui;

pub use class::join as join_class;
pub use icons::Icon;
pub use section::Section;
