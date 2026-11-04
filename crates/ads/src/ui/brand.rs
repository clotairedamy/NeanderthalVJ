//! The Artelier mark and the light/dark toggle.

use dioxus::prelude::*;

/// The mark: a lowercase a in a high-contrast display cut — hairline entry
/// stroke and bowl top, a heavy bowl and stem, and a hairline tail. One filled
/// path in currentColor, so it follows the theme like the text. A bare `svg`,
/// identical to the site's canonical mark; the name, where it is needed, is
/// `Wordmark` beside it.
///
/// The hairlines are about 1% of the glyph's height, so below roughly 24px
/// they disappear and the mark becomes its heavy strokes alone. That is
/// expected: a favicon will not show them either.
#[component]
pub fn BrandMark(
    #[props(default = 32u32)] size: u32,
    #[props(into, optional)] class: Option<String>,
) -> Element {
    rsx! {
        svg {
            class: class,
            width: "{size}",
            height: "{size}",
            view_box: "38 31 350 350",
            fill: "currentColor",
            "aria-hidden": "true",
            "focusable": "false",
            path { fill_rule: "evenodd", d: MARK }
        }
    }
}

const MARK: &str = "M 169.5 37 C 113.2 41.7 75.3 80.4 75.7 133 C 75.9 156 81.7 174 94.5 190.7 C 97.5 194.7 100 198.2 100 198.5 C 100 198.7 97.6 200.2 94.8 201.6 C 62.7 217.7 46.3 250.3 49.9 290.7 C 54.8 345.1 90.8 374.6 152 374.4 C 179.5 374.4 195 370.1 212.5 357.6 C 234.1 342.1 253.8 307.7 259.3 275.5 C 260.7 267.5 260.7 267.6 261.2 294 C 261.6 317.7 261.9 321.3 263.9 328 C 272.2 355.7 293.5 373.3 320.5 374.7 C 352.3 376.4 373.2 359.7 378 329 C 379.4 319.8 379.3 315 377.6 315 C 376.7 315 376.1 317.2 375.6 322.6 C 373.6 345.1 360.7 364.5 347 365.8 C 332.2 367.2 322.6 360.6 316.9 345 L 314.6 338.5 313.9 237 C 313.4 147.9 313.1 134.6 311.7 128 C 299.6 73.3 259.4 41.9 195 37 C 187.6 36.5 180.8 36.1 180 36.1 C 179.2 36.2 174.5 36.6 169.5 37 M 165 40.7 C 111.8 48.8 78.5 84 78.7 132 C 78.7 155.4 86 175.1 101.3 193.3 L 103.7 196.1 115.3 192.6 C 143.5 184.1 155.5 182.8 212.3 182.3 L 261 181.8 261 146.7 C 261 110.1 260.4 102 257 92 C 245.5 57.9 218.6 39.8 180 40.1 C 173.1 40.2 166.4 40.4 165 40.7 M 179.5 186.1 C 122 193.5 95.3 229.2 100.1 292.4 C 103.9 342.7 126.9 367.3 170 367.2 C 191.3 367.1 205 361.3 220 346.1 C 241.8 324.2 253.9 296.8 258 260.5 C 259.2 249.9 260.9 211 261 194.8 L 261 185 223.8 185.1 C 203.3 185.2 183.4 185.7 179.5 186.1";

/// The theme switch. One glyph that changes shape rather than two icons that
/// swap: the sun's rays draw in, and a second disc slides across the body to
/// carve it into a crescent. Pass `dark` when the dark theme is showing; the
/// name says what pressing it will do, and flipping `data-theme` on the root
/// is the caller's job. Two toggles can be on screen at once (app bar and
/// menu sheet), so give each its own `id` to keep the mask ids unique.
#[component]
pub fn AdsThemeToggle(
    #[props(optional)] dark: bool,
    #[props(into, default = "ads-theme-moon".to_string())] id: String,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let class = crate::class::join(&["ads-theme-toggle", if dark { "is-dark" } else { "" }]);
    rsx! {
        button {
            class: "{class}",
            r#type: "button",
            "aria-label": if dark { "Switch to light theme" } else { "Switch to dark theme" },
            title: if dark { "Light theme" } else { "Dark theme" },
            onclick: move |e| { if let Some(h) = &onclick { h.call(e); } },
            svg { class: "ads-theme-toggle__svg", width: "20", height: "20", view_box: "0 0 24 24", "aria-hidden": "true",
                mask { id: "{id}",
                    rect { width: "24", height: "24", fill: "#fff" }
                    circle { class: "ads-theme-toggle__bite", cx: "24", cy: "2", r: "7", fill: "#000" }
                }
                circle { class: "ads-theme-toggle__body", cx: "12", cy: "12", r: "5", fill: "currentColor", mask: "url(#{id})" }
                g { class: "ads-theme-toggle__rays", stroke: "currentColor", stroke_width: "1.6", stroke_linecap: "round",
                    path { d: "M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" }
                }
            }
        }
    }
}
