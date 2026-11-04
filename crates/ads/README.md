# ADS (Rust) — Dioxus

ADS comes from the Artelier design system and is bundled here so
NeanderthalVJ builds on its own. It is licensed like the rest of the
project, MIT OR Apache-2.0.

The Artelier Design System as **Dioxus 0.7** components. It renders the exact
same CSS classes as the Vue kit and shares the same stylesheets, so the look is
identical — monochrome soft UI, nothing has a border, shape comes from two
shadows.

## What's here

```
ads-rs/
├── assets/css/
│   ├── tokens.css        # the design tokens (shared with the Vue kit)
│   ├── base.css          # reset, .btn .card .well .chip .grid
│   ├── ads.css           # the ads-* component layer
│   ├── ads-utility.css   # the utility layer (paint-*, ink-*, lift/press, …)
│   └── components.css    # the components' scoped styles, de-scoped
└── src/
    ├── lib.rs            # pub mod ui / charts / blocks / icons / class
    ├── class.rs          # class joining (ADS classes, no Tailwind)
    ├── icons.rs          # the 28 glyphs + Icon component
    ├── ui/               # atoms (button, field, tabs, badge, modal, …)
    ├── charts/           # bars, sparkline, gauge, heatmap, chart choice
    └── blocks/           # KPI row, filter bar, table screen, summary → detail
```

## Use it

```toml
# Cargo.toml
[dependencies]
ads = { path = "../ads/ads-rs" }
dioxus = { version = "=0.7.9", features = ["web"] }
```

Include the five stylesheets in `assets/css/` (e.g. via `dioxus` asset or a
`<link>`), then compose:

```rust
use dioxus::prelude::*;
use ads::ui::{AdsButton, ButtonVariant};

#[component]
fn App() -> Element {
    rsx! {
        AdsButton { variant: ButtonVariant::Accent, icon: "plus", "New crawl" }
    }
}
```

## Components

- **ui** — `AdsButton` `AdsIconButton` `AdsSegmented` `AdsTabs` `AdsMenu`
  `AdsField` `AdsDaterange` `AdsChoice` `AdsRange` `AdsNumber` `AdsTag`
  `AdsStat` `AdsDelta` `AdsBadge` `AdsStatus` `AdsMeter` `AdsProgress` `AdsDl`
  `AdsDataGrid` `AdsAccordion` `AdsSteps` `AdsTree` `AdsCrumbs` `AdsPager`
  `AdsEmpty` `AdsSkeleton` `AdsLoading` `AdsBanner` `AdsTooltip` `AdsModal`
  `AdsDrawer` `AdsPopover` `AdsThemeToggle` `BrandMark` `Icon`
- **charts** — `AdsBars` `AdsSparkline` `AdsGauge` `AdsHeatmap` `ChartChoice`
- **blocks** — `KpiRow` `FilterBar` `TableScreen` `SummaryDetail`

## Style

The five CSS files are the source of truth and are byte-identical to the Vue
kit's (plus `components.css`, which consolidates the Vue components' scoped
styles). Change a token in `tokens.css` and both kits move together. Light is
the default; dark applies via `data-theme="dark"`.

## Parity notes

- Stateful controls use the controlled pattern (`value` + `onchange`/
  `oninput`/`onselect` handlers, or Dioxus `Signal`s) rather than Vue's
  `v-model`.
- `AdsDataGrid` renders the table (sortable headers are stubs); the keyboard
  cursor and windowed virtualisation from the Engineering section are not yet
  ported.
