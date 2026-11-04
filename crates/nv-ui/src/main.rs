//! NeanderthalVJ.
//!
//! The interface is ADS (see the `nv_ui` library) in a transparent
//! webview; the visualization is drawn by a GPU surface on the window
//! behind it. Everything the page leaves transparent — the stage — is a
//! window onto the visuals.

// A release build on Windows is a windowed app, without a console window
// beside it; a debug build keeps the console for its diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::desktop::tao::window::WindowBuilder;
use dioxus::desktop::{Config, LogicalSize};
use nv_ui::App;

fn main() {
    // The visuals are drawn on a GPU surface behind the page, which needs
    // the window and the webview to be transparent. That has a cost: ADS
    // blurs what is behind its overlays, and over a transparent window
    // there is nothing to blur. `NV_OVERLAY=0` runs the chrome opaque with
    // no visuals, which is the way to tell the two apart.
    let overlay = std::env::var("NV_OVERLAY").map(|v| v != "0").unwrap_or(true);

    let window = WindowBuilder::new()
        .with_title("NeanderthalVJ")
        .with_transparent(overlay)
        .with_inner_size(LogicalSize::new(1480.0, 940.0));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                // fully transparent, so the GPU surface behind the webview
                // shows through wherever the page does not paint
                .with_background_color(if overlay { (0, 0, 0, 0) } else { (19, 19, 21, 255) })
                // the default menu stays: dropping it on macOS takes the
                // application menu with it, and Quit and copy/paste go too
                .with_window(window),
        )
        .launch(App);
}
