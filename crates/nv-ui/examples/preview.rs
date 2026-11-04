//! Renders the interface to a standalone HTML file, for looking at it
//! without launching a window.
//!
//!     cargo run -p nv-ui --example preview -- /tmp/preview.html

use dioxus::prelude::*;
use nv_ui::Workspace;

/// Preview-only: pin a desktop viewport and scale it to whatever is
/// looking at the file, so the two-column layout can be seen at once
/// rather than collapsing into the narrow fallback.
const PREVIEW_CSS: &str = "
/* stands in for the GPU surface behind the page */
html { background: #101014; }
body { margin: 0; width: 1480px; height: 920px; zoom: 0.54; }
/* The narrow fallback keys off the viewport, which `zoom` does not change,
   so the desktop layout is pinned here instead. */
.nv-app { height: 920px; grid-template-columns: minmax(320px, 400px) 1fr; grid-template-rows: none; overflow: hidden; }
";

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "preview.html".into());
    let mut dom = VirtualDom::new(Workspace);
    dom.rebuild_in_place();
    let body = dioxus_ssr::render(&dom);

    let css = ["tokens.css", "base.css", "ads.css", "ads-utility.css", "components.css", "nv.css"]
        .iter()
        .map(|f| {
            std::fs::read_to_string(format!("{}/assets/css/{f}", env!("CARGO_MANIFEST_DIR")))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let page = format!(
        "<!doctype html>\n<html lang=\"en\" data-theme=\"dark\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>NeanderthalVJ</title>\
         <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\
         <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?\
family=Bricolage+Grotesque:opsz,wght@12..96,200..800&family=Inter+Tight:wght@100..900\
&family=Martian+Mono:wght@100..800&family=JetBrains+Mono:wght@100..800&display=swap\">\
         <style>\n{css}\n{PREVIEW_CSS}</style></head><body>{body}</body></html>"
    );
    std::fs::write(&out, page).expect("write preview");
    println!("wrote {out}");
}
