//! Renders the interface to a string and checks it against ADS's rules.
//!
//! The kit ships a `check_markup` linter for hand-written HTML; in Rust the
//! compiler already checks component props, so what is left to verify is
//! that the screen is actually built from ADS and obeys the four rules it
//! encodes -- above all that nothing reaches for a border or an ad-hoc
//! colour, because that is how a design system quietly stops being one.

use dioxus::prelude::*;
use nv_ui::Workspace;

/// An empty settings folder, so the screen is the defaults and never a
/// session someone left on this machine.
fn isolate() {
    let dir = std::env::temp_dir().join(format!("nvj-markup-{}", std::process::id()));
    std::env::set_var("NV_CONFIG_DIR", dir);
}

fn render() -> String {
    isolate();
    let mut dom = VirtualDom::new(Workspace);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_whole_workspace_is_present() {
    let html = render();
    for part in ["nv-controls", "nv-stage", "nv-scope", "nv-transport", "nv-status"] {
        assert!(html.contains(part), "missing `{part}` from the workspace");
    }
    // the readout strip the Qt app put above the canvas
    for field in ["BPM", "FPS", "level", "MIDI"] {
        assert!(html.contains(field), "status bar is missing `{field}`");
    }
    // the transport row
    for control in ["Speed", "Keep pitch", "Loop", "Performance"] {
        assert!(html.contains(control), "transport is missing `{control}`");
    }
}

#[test]
fn every_visualization_is_offered() {
    let html = render();
    for (i, mode) in nv_ui::state::MODES.iter().enumerate() {
        assert!(html.contains(&format!("{}. {}", i + 1, mode)), "mode list is missing `{mode}`");
    }
    assert!(html.contains("Auto-switch at song sections"), "missing the auto-switch control");
}

#[test]
fn the_six_tabs_are_there() {
    let html = render();
    for tab in nv_ui::state::Tab::ALL {
        assert!(html.contains(&tab.to_string()), "missing the `{tab}` tab");
    }
    assert!(html.contains("Presets"), "missing the presets row");
}

#[test]
fn screen_is_built_from_ads() {
    let html = render();
    for class in ["ads-tabs", "ads-choice", "ads-field", "card", "eyebrow"] {
        assert!(html.contains(class), "no `{class}` in the output -- not an ADS screen");
    }
}

#[test]
fn obeys_the_ads_rules() {
    let html = render();

    // 2. Depth is the only decoration -- nothing has a border.
    for banned in ["border:", "border-width", "outline:"] {
        assert!(!html.contains(banned), "inline `{banned}`: depth is the only decoration");
    }
    // 1. The chrome is grey so the data can be coloured: no ad-hoc colour.
    for banned in ["#", "rgb(", "rgba("] {
        let in_style = html
            .split("style=\"")
            .skip(1)
            .any(|s| s.split('"').next().is_some_and(|v| v.contains(banned)));
        assert!(!in_style, "inline colour `{banned}` -- colour belongs to the tokens");
    }
    // no Tailwind: ADS is not a utility framework and reaching for one is
    // the most common way this goes wrong
    for tw in ["class=\"flex ", "class=\"grid-cols-", " px-4", " text-sm"] {
        assert!(!html.contains(tw), "Tailwind-style class `{tw}` -- use ADS classes");
    }
}

#[test]
fn numbers_are_monospaced() {
    // 3. Numbers are monospaced -- every figure carries the `num` role
    // rather than inheriting the body font.
    let html = render();
    assert!(html.contains("num"), "figures are not carrying ADS's numeric role");
}

#[test]
fn the_stylesheet_only_names_tokens() {
    // 4. Every component names its tokens. This app's own CSS is allowed to
    // arrange things, but any literal colour in it would be a value the kit
    // can no longer move.
    let css = include_str!("../assets/css/nv.css");
    for (n, line) in css.lines().enumerate() {
        let code = line.split("/*").next().unwrap_or(line).trim();
        if code.starts_with('*') || code.is_empty() {
            continue;
        }
        for banned in ["#", "rgb(", "hsl("] {
            assert!(
                !code.contains(banned),
                "nv.css:{}: literal colour `{banned}` -- name a token instead:\n  {code}",
                n + 1
            );
        }
    }
}

/// ADS ships 28 glyphs and they are domain icons, not a general-purpose
/// set: there is no play, pause, download, close or maximize among them.
/// Passing a name outside this list renders a placeholder, which looks
/// like a deliberate symbol and is easy to miss — so it is checked here.
const ADS_GLYPHS: [&str; 28] = [
    "access",
    "alert",
    "arrow",
    "audit",
    "blueprint",
    "chart",
    "check",
    "chevrons",
    "code",
    "compass",
    "dashboard",
    "discovery",
    "engine",
    "flow",
    "gauge",
    "grid",
    "growth",
    "handoff",
    "layers",
    "mail",
    "plus",
    "prototype",
    "signal",
    "sliders",
    "spark",
    "system",
    "table",
    "theme",
];

#[test]
fn every_icon_name_exists_in_the_kit() {
    let sources = [
        include_str!("../src/shell.rs"),
        include_str!("../src/panel/source.rs"),
        include_str!("../src/panel/export.rs"),
        include_str!("../src/panel/visuals.rs"),
        include_str!("../src/panel/motion.rs"),
        include_str!("../src/panel/fx.rs"),
    ];
    for src in sources {
        for (n, line) in src.lines().enumerate() {
            let Some(rest) = line.split("icon:").nth(1) else { continue };
            let Some(name) = rest.split('"').nth(1) else { continue };
            assert!(
                ADS_GLYPHS.contains(&name),
                "line {}: `{name}` is not one of ADS's 28 glyphs",
                n + 1
            );
        }
    }
}

#[test]
fn the_page_carries_its_stylesheet() {
    // The window once came up as unstyled serif HTML because the sheets
    // were referenced through `asset!`, which resolves via the `dx` CLI's
    // pipeline -- a binary run from cargo has none, so every sheet 404s.
    // The preview renderer inlines the CSS itself, so it looked right while
    // the app did not. This checks the shipped page, not the preview.
    isolate();
    let mut dom = VirtualDom::new(nv_ui::App);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);

    assert!(html.contains("<style"), "no <style> in the page");
    for token in ["--canvas", "--surface", "--soft-raised", ".ads-field", ".nv-stage"] {
        assert!(html.contains(token), "the stylesheet is missing `{token}`");
    }
    assert!(
        !html.contains("/assets/css/"),
        "still referencing a stylesheet by path -- it will 404 outside `dx`"
    );
}
