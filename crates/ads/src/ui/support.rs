//! Small behaviour helpers shared by the interactive elements: stable ids,
//! focus moves, a browser-side timer, calendar arithmetic, and a glyph that can
//! carry its own class (the Vue `<Icon class="…">`).
//!
//! The timer goes through `document::eval` so the crate needs no async runtime
//! of its own; where there is no browser to ask (server rendering), the wait
//! simply returns at once.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

use crate::icons::{glyph as glyph_layers, Layer};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A per-instance id, stable across renders — the Vue `useId()`.
pub(crate) fn use_uid() -> String {
    use_hook(|| format!("ads-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)))
}

/// Focus a mounted element, if it has been mounted.
pub(crate) fn focus(el: Option<Rc<MountedData>>) {
    if let Some(el) = el {
        spawn(async move {
            let _ = el.set_focus(true).await;
        });
    }
}

/// Local state that follows a prop: the Rust spelling of a Vue `defineModel`,
/// which keeps working when the caller binds nothing and yields to the
/// caller's value when it changes.
pub(crate) fn use_synced<T: Clone + PartialEq + 'static>(prop: &T) -> Signal<T> {
    let mut state = use_signal(|| prop.clone());
    use_effect(use_reactive((prop,), move |(p,)| {
        if *state.peek() != p {
            state.set(p);
        }
    }));
    state
}

/// Hold a modal panel while it is on the page: lock the body's scroll, keep Tab
/// inside the panel, focus `first` (or `fallback`), and when the panel leaves
/// the page give the scroll back and return focus to whatever opened it. The
/// browser does the bookkeeping, since a trap has to refuse the Tab in the
/// same tick it is pressed.
pub(crate) fn hold_modal(panel: &str, first: &str, fallback: &str) {
    let js = format!(
        r#"
const panel = document.querySelector({panel:?});
if (!panel) return;
const F = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
const back = document.activeElement;
const prev = document.body.style.overflow;
document.body.style.overflow = 'hidden';
const trap = (e) => {{
  if (e.key !== 'Tab') return;
  const items = Array.from(panel.querySelectorAll(F));
  if (!items.length) return;
  const a = items[0], z = items[items.length - 1];
  if (e.shiftKey && document.activeElement === a) {{ e.preventDefault(); z.focus(); }}
  else if (!e.shiftKey && document.activeElement === z) {{ e.preventDefault(); a.focus(); }}
}};
panel.addEventListener('keydown', trap);
(panel.querySelector({first:?}) ?? panel.querySelector({fallback:?}))?.focus();
const watch = new MutationObserver(() => {{
  if (panel.isConnected) return;
  watch.disconnect();
  document.body.style.overflow = prev;
  back?.focus?.();
}});
watch.observe(document.body, {{ childList: true, subtree: true }});
"#
    );
    let _ = document::eval(&js);
}

/// Where a scroller stands: offset, content size and visible size.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub(crate) struct ScrollBox {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub client_width: f64,
    pub client_height: f64,
}

impl ScrollBox {
    pub fn from_event(e: &ScrollData) -> Self {
        Self {
            left: e.scroll_left(),
            top: e.scroll_top(),
            width: f64::from(e.scroll_width()),
            height: f64::from(e.scroll_height()),
            client_width: f64::from(e.client_width()),
            client_height: f64::from(e.client_height()),
        }
    }

    /// Measure a mounted scroller (the client size is the border box, which
    /// is close enough for an edge fade).
    pub async fn measure(el: &MountedData) -> Option<Self> {
        let off = el.get_scroll_offset().await.ok()?;
        let size = el.get_scroll_size().await.ok()?;
        let rect = el.get_client_rect().await.ok()?;
        Some(Self {
            left: off.x,
            top: off.y,
            width: size.width,
            height: size.height,
            client_width: rect.size.width,
            client_height: rect.size.height,
        })
    }
}

/// Wait `ms` milliseconds in the browser (a `setTimeout`).
pub(crate) async fn sleep(ms: u32) {
    let _ = document::eval(&format!(
        "await new Promise((r) => setTimeout(r, {ms})); return true;"
    ))
    .join::<bool>()
    .await;
}

/// Whether the reader asked for less motion.
pub(crate) async fn reduced_motion() -> bool {
    document::eval("return window.matchMedia('(prefers-reduced-motion: reduce)').matches;")
        .join::<bool>()
        .await
        .unwrap_or(false)
}

/// Today as ISO (YYYY-MM-DD). It starts from the system clock where there is
/// one, then asks the browser for the reader's local date.
pub(crate) fn use_today() -> Signal<String> {
    let mut today = use_signal(today_from_clock);
    use_hook(move || {
        spawn(async move {
            let js = "const d = new Date(); const p = (n) => String(n).padStart(2, '0'); \
                      return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;";
            if let Ok(s) = document::eval(js).join::<String>().await {
                if date::parse(&s).is_some() && *today.peek() != s {
                    today.set(s);
                }
            }
        })
    });
    today
}

#[cfg(not(target_arch = "wasm32"))]
fn today_from_clock() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date::iso(i64::try_from(secs / 86_400).unwrap_or(0))
}

#[cfg(target_arch = "wasm32")]
fn today_from_clock() -> String {
    // No clock without the browser; the eval above fills it in.
    String::new()
}

/// Calendar arithmetic on day numbers (days since 1970-01-01), proleptic
/// Gregorian. Plain integer maths, after Howard Hinnant's civil algorithms.
pub(crate) mod date {
    /// Day number of a civil date. `d` may overflow the month, as JS `Date` does.
    #[must_use]
    pub fn from_civil(y: i64, m: u32, d: i64) -> i64 {
        let m = i64::from(m);
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468 + (d - 1)
    }

    /// Civil date of a day number.
    #[must_use]
    pub fn to_civil(z: i64) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        (
            y,
            u32::try_from(m).unwrap_or(1),
            u32::try_from(d).unwrap_or(1),
        )
    }

    /// Days in month `m` (1–12) of year `y`.
    #[must_use]
    pub fn days_in_month(y: i64, m: u32) -> u32 {
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        u32::try_from(from_civil(ny, nm, 1) - from_civil(y, m, 1)).unwrap_or(30)
    }

    /// Weekday of a day number, Monday = 0.
    #[must_use]
    pub fn weekday(z: i64) -> u32 {
        // 1970-01-01 was a Thursday.
        u32::try_from((z + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// ISO date of a day number.
    #[must_use]
    pub fn iso(z: i64) -> String {
        let (y, m, d) = to_civil(z);
        format!("{y:04}-{m:02}-{d:02}")
    }

    /// Parse a strict, real ISO date (YYYY-MM-DD).
    #[must_use]
    pub fn parse(s: &str) -> Option<i64> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |r: std::ops::Range<usize>| -> Option<u32> {
            s.get(r.clone())
                .filter(|t| t.bytes().all(|c| c.is_ascii_digit()))?
                .parse()
                .ok()
        };
        let (y, m, d) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(i64::from(y), m) {
            return None;
        }
        Some(from_civil(i64::from(y), m, i64::from(d)))
    }

    /// Parse a month (YYYY-MM).
    #[must_use]
    pub fn parse_month(s: &str) -> Option<(i64, u32)> {
        let z = parse(&format!("{}-01", s.get(0..7)?))?;
        let (y, m, _) = to_civil(z);
        (s.len() == 7).then_some((y, m))
    }

    /// Move a day by whole months, overflowing a short month as JS does.
    #[must_use]
    pub fn add_months(z: i64, n: i64) -> i64 {
        let (y, m, d) = to_civil(z);
        let total = y * 12 + i64::from(m) - 1 + n;
        let m2 = u32::try_from(total.rem_euclid(12)).unwrap_or(0) + 1;
        from_civil(total.div_euclid(12), m2, i64::from(d))
    }
}

/// An ADS glyph as an inline SVG that can carry its own class and style, so a
/// part class (`ads-item__icon`) sits on the SVG itself, as it does in Vue.
pub(crate) fn glyph(name: &str, size: u32, class: &str, style: Option<&str>) -> Element {
    let layers: &'static [Layer] = glyph_layers(name)
        .or_else(|| glyph_layers("grid"))
        .unwrap_or(&[]);
    let class = (!class.is_empty()).then(|| class.to_owned());
    let style = style.map(str::to_owned);
    rsx! {
        svg {
            class,
            style,
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
                {layer_node(layer)}
            }
        }
    }
}

fn layer_node(layer: &Layer) -> Element {
    const SOFT: &str = "0.38";
    match *layer {
        Layer::Path(d, soft, fill) => rsx! {
            path {
                d,
                fill: if fill { "currentColor" } else { "none" },
                stroke: if fill { "none" } else { "currentColor" },
                opacity: if soft { Some(SOFT) } else { None },
            }
        },
        Layer::Rect(x, y, w, h, rad, soft) => rsx! {
            rect {
                x: "{x}",
                y: "{y}",
                width: "{w}",
                height: "{h}",
                rx: "{rad}",
                opacity: if soft { Some(SOFT) } else { None },
            }
        },
        Layer::Circle(cx, cy, rad, soft, fill) => rsx! {
            circle {
                cx: "{cx}",
                cy: "{cy}",
                r: "{rad}",
                fill: if fill { "currentColor" } else { "none" },
                stroke: if fill { "none" } else { "currentColor" },
                opacity: if soft { Some(SOFT) } else { None },
            }
        },
    }
}
