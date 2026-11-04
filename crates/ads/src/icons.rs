//! ADS glyphs, ported from `app/utils/glyphs.ts`. A 24px grid, 1.6 stroke,
//! round caps. Soft layers are the chrome (rendered at 0.38 opacity); the
//! thing the glyph is about is drawn at full strength.

use dioxus::prelude::*;

/// A single SVG layer in a glyph.
#[derive(Clone, Copy)]
pub enum Layer {
    Path(&'static str, bool /* soft */, bool /* fill */),
    Rect(f32, f32, f32, f32, f32, bool),
    Circle(f32, f32, f32, bool /* soft */, bool /* fill */),
}

const SOFT: &str = "0.38";

const fn p(d: &'static str) -> Layer {
    Layer::Path(d, false, false)
}
const fn ps(d: &'static str) -> Layer {
    Layer::Path(d, true, false)
}
const fn pf(d: &'static str) -> Layer {
    Layer::Path(d, false, true)
}
const fn r(x: f32, y: f32, w: f32, h: f32, rad: f32) -> Layer {
    Layer::Rect(x, y, w, h, rad, false)
}
const fn rs(x: f32, y: f32, w: f32, h: f32, rad: f32) -> Layer {
    Layer::Rect(x, y, w, h, rad, true)
}
const fn c(cx: f32, cy: f32, rad: f32) -> Layer {
    Layer::Circle(cx, cy, rad, false, false)
}
const fn cs(cx: f32, cy: f32, rad: f32) -> Layer {
    Layer::Circle(cx, cy, rad, true, false)
}
const fn cf(cx: f32, cy: f32, rad: f32) -> Layer {
    Layer::Circle(cx, cy, rad, false, true)
}

const GRID: &[Layer] = &[p("M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z")];
const ALERT: &[Layer] = &[p("M12 4 2.5 20.5h19zM12 10v4M12 17.5v.5")];
const LAYERS: &[Layer] = &[p("M12 3 3 8l9 5 9-5zM3 13l9 5 9-5M3 17.5l9 5 9-5")];
const SLIDERS: &[Layer] = &[p("M4 7h10M18 7h2M4 17h4M12 17h8M16 4.5v5M8 14.5v5")];
const CHART: &[Layer] = &[p("M4 20V9M9.5 20V4M15 20v-7M20.5 20v-4")];
const FLOW: &[Layer] = &[p(
    "M5 6h6a3 3 0 0 1 3 3v6a3 3 0 0 0 3 3h2M3 6h1M19 18h2M16 3l3 3-3 3",
)];
const COMPASS: &[Layer] = &[p(
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM15.5 8.5l-2 5-5 2 2-5z",
)];
const MAIL: &[Layer] = &[p("M3 6h18v12H3zM3 7l9 6 9-6")];
const ARROW: &[Layer] = &[p("M4 12h15M13 6l6 6-6 6")];
const CHEVRONS: &[Layer] = &[p("M13 6l-6 6 6 6M19 6l-6 6 6 6")];
const CHECK: &[Layer] = &[p("M4 12.5l5 5L20 6.5")];
const PLUS: &[Layer] = &[p("M12 5v14M5 12h14")];
const CODE: &[Layer] = &[p("M8.5 7 4 12l4.5 5M15.5 7 20 12l-4.5 5")];
const SPARK: &[Layer] = &[p(
    "M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M18 6l-2.5 2.5M8.5 15.5 6 18",
)];

const DASHBOARD: &[Layer] = &[
    rs(3.0, 4.0, 18.0, 16.0, 3.5),
    ps("M8.5 4v16"),
    p("M11.5 15.5 14 12.5l2.5 1.75 2-4"),
    cf(18.5, 10.25, 1.4),
];
const SIGNAL: &[Layer] = &[
    ps("M3.5 20h17"),
    p("M4 15.5 7.5 12.5l3 2L15 7.5l5 4.5"),
    cf(15.0, 7.5, 1.6),
];
const GROWTH: &[Layer] = &[
    ps("M5.5 19.5V16M9.5 19.5v-6M13.5 19.5V11"),
    p("M17.5 19.5V9"),
    cf(17.5, 5.0, 1.6),
];
const GAUGE: &[Layer] = &[
    ps("M4.5 17a7.5 7.5 0 0 1 15 0"),
    p("M4.5 17a7.5 7.5 0 0 1 11.25-6.5"),
    p("M12 17l3.25-4"),
    cf(12.0, 17.0, 1.6),
];
const DISCOVERY: &[Layer] = &[
    c(10.5, 10.5, 6.5),
    p("M15.5 15.5 20 20"),
    ps("M8 12.5l2.5-4.5 2.5 4.5"),
];
const BLUEPRINT: &[Layer] = &[
    rs(3.0, 4.0, 18.0, 16.0, 3.5),
    ps("M3 9h18"),
    r(6.5, 12.0, 5.0, 4.5, 1.25),
    p("M14.5 12.75h3M14.5 15.75h2"),
];
const SYSTEM: &[Layer] = &[
    r(3.5, 3.5, 7.0, 7.0, 2.0),
    rs(13.5, 3.5, 7.0, 7.0, 2.0),
    rs(3.5, 13.5, 7.0, 7.0, 2.0),
    c(17.0, 17.0, 3.5),
    cf(17.0, 17.0, 1.1),
];
const PROTOTYPE: &[Layer] = &[
    rs(3.0, 4.0, 18.0, 13.0, 3.5),
    p("M12.5 11 19 13.5l-2.75 1.1L15.1 17.4z"),
];
const HANDOFF: &[Layer] = &[
    rs(3.0, 4.0, 18.0, 16.0, 3.5),
    p("M10 9.5 7.5 12l2.5 2.5M14 9.5l2.5 2.5-2.5 2.5"),
];
const TABLE: &[Layer] = &[
    rs(3.0, 4.0, 18.0, 16.0, 3.5),
    ps("M3 9h18"),
    p("M7 12.75h3M13.5 12.75H17M7 16h3M13.5 16h2"),
];
const AUDIT: &[Layer] = &[
    rs(5.0, 4.5, 14.0, 16.0, 3.0),
    p("M9.5 4.5V3.75h5v.75"),
    p("M9 12.75 11.25 15 15.5 10.5"),
];
const ACCESS: &[Layer] = &[
    cs(12.0, 12.0, 9.0),
    cf(12.0, 7.5, 1.4),
    p("M8 10.5h8M12 10.5V14l-2.5 4.25M12 14l2.5 4.25"),
];
const THEME: &[Layer] = &[c(12.0, 12.0, 8.5), pf("M12 3.5a8.5 8.5 0 0 1 0 17z")];
const ENGINE: &[Layer] = &[
    ps("M12 2.75l8 4.6v9.3l-8 4.6-8-4.6v-9.3z"),
    p("M13 6.75 9.5 12.5H14l-3.5 5.75"),
];

/// Return the layers for a glyph name, or `None`.
#[must_use]
pub fn glyph(name: &str) -> Option<&'static [Layer]> {
    Some(match name {
        "grid" => GRID,
        "alert" => ALERT,
        "layers" => LAYERS,
        "sliders" => SLIDERS,
        "chart" => CHART,
        "flow" => FLOW,
        "compass" => COMPASS,
        "mail" => MAIL,
        "arrow" => ARROW,
        "chevrons" => CHEVRONS,
        "check" => CHECK,
        "plus" => PLUS,
        "code" => CODE,
        "spark" => SPARK,
        "dashboard" => DASHBOARD,
        "signal" => SIGNAL,
        "growth" => GROWTH,
        "gauge" => GAUGE,
        "discovery" => DISCOVERY,
        "blueprint" => BLUEPRINT,
        "system" => SYSTEM,
        "prototype" => PROTOTYPE,
        "handoff" => HANDOFF,
        "table" => TABLE,
        "audit" => AUDIT,
        "access" => ACCESS,
        "theme" => THEME,
        "engine" => ENGINE,
        _ => return None,
    })
}

fn render_layer(layer: &Layer) -> Element {
    match layer {
        Layer::Path(d, soft, fill) => rsx! {
            path {
                d: *d,
                fill: if *fill { "currentColor" } else { "none" },
                stroke: if *fill { "none" } else { "currentColor" },
                opacity: if *soft { Some(SOFT) } else { None },
            }
        },
        Layer::Rect(x, y, w, h, rad, soft) => rsx! {
            rect {
                x: "{x}",
                y: "{y}",
                width: "{w}",
                height: "{h}",
                rx: "{rad}",
                opacity: if *soft { Some(SOFT) } else { None },
            }
        },
        Layer::Circle(cx, cy, rad, soft, fill) => rsx! {
            circle {
                cx: "{cx}",
                cy: "{cy}",
                r: "{rad}",
                fill: if *fill { "currentColor" } else { "none" },
                stroke: if *fill { "none" } else { "currentColor" },
                opacity: if *soft { Some(SOFT) } else { None },
            }
        },
    }
}

/// Inline SVG glyph. `name` is one of the ADS glyph names (see `glyph()`).
/// `class` and `style` are for the component placing it, as a Vue parent's
/// class and style fall through onto `<Icon>`.
#[component]
pub fn Icon(
    #[props(into)] name: String,
    #[props(default = 20u32)] size: u32,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] style: Option<String>,
) -> Element {
    let layers = glyph(name.as_str()).unwrap_or(GRID);
    rsx! {
        svg {
            class: class,
            style: style,
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.6",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for layer in layers {
                {render_layer(layer)}
            }
        }
    }
}
