//! Palette panel — the five pigments, at the values each theme uses.

use dioxus::prelude::*;

/// The five tokens a pigment can be. The set is closed: a pigment is one of
/// these or it is not a pigment.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum PigmentToken {
    #[default]
    BrandPrimary,
    BrandSecondary,
    Ok,
    Warn,
    Bad,
}

impl PigmentToken {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BrandPrimary => "--brand-primary",
            Self::BrandSecondary => "--brand-secondary",
            Self::Ok => "--ok",
            Self::Warn => "--warn",
            Self::Bad => "--bad",
        }
    }
}

/// One pigment: its name, its role, and the token it paints.
#[derive(Default, Clone, PartialEq)]
pub struct Pigment {
    pub name: String,
    pub role: String,
    pub token: PigmentToken,
}

impl Pigment {
    #[must_use]
    pub fn new(name: impl Into<String>, role: impl Into<String>, token: PigmentToken) -> Self {
        Self {
            name: name.into(),
            role: role.into(),
            token,
        }
    }
}

fn defaults() -> Vec<Pigment> {
    vec![
        Pigment::new("Tyrian", "Primary", PigmentToken::BrandPrimary),
        Pigment::new("Verdigris", "Secondary", PigmentToken::BrandSecondary),
        Pigment::new("Celadon", "Success", PigmentToken::Ok),
        Pigment::new("Orpiment", "Warning", PigmentToken::Warn),
        Pigment::new("Vermilion", "Danger", PigmentToken::Bad),
    ]
}

/// The five pigments — the brand and semantic colours — at the values each
/// theme actually uses. The only hue in the whole system.
///
/// Each chip paints its token rather than a copied hex, so the panel can never
/// disagree with the sheet it documents, and it re-steps with the theme for
/// free.
#[component]
pub fn PalettePanel(
    #[props(into, optional)] pigments: Option<Vec<Pigment>>,
    #[props(default = 56u32)] entries: u32,
    #[props(default = 12u32)] groups: u32,
) -> Element {
    let pigments = pigments.unwrap_or_else(defaults);
    rsx! {
        div { class: "well ads-palette-panel",
            dl { class: "ads-palette-panel__stats",
                div { dt { "Entries" } dd { class: "num", "{entries}" } }
                div { dt { "Groups" } dd { class: "num", "{groups}" } }
                div { dt { "Pigments" } dd { class: "num", "{pigments.len()}" } }
            }
            ul { class: "ads-palette-panel__swatches",
                for p in &pigments {
                    li { key: "{p.name}", class: "ads-palette-panel__swatch",
                        span { class: "ads-palette-panel__chip", style: "background:var({p.token.as_str()})" }
                        span { class: "ads-palette-panel__name", "{p.name}" }
                        span { class: "ads-palette-panel__role", "{p.role}" }
                    }
                }
            }
        }
    }
}
