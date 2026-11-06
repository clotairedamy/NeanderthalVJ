//! The NeanderthalVJ interface, built with ADS — the Artelier Design System.
//!
//! The layout is the one the Qt app settled on: a control column on the
//! left, the visualization filling the rest, with the readout above it and
//! the scope and transport below. ADS supplies everything except the stage
//! itself, which is the picture rather than the chrome and in the running
//! app is a native wgpu surface behind the webview.

pub mod engine_link;
pub mod midi;
pub mod panel;
pub mod shell;
pub mod stage;
pub mod state;
pub mod store;
#[cfg(all(target_os = "macos", feature = "syphon"))]
pub mod syphon;

use dioxus::html::HasFileData;
use dioxus::prelude::*;

use engine_link::use_engine;

/// ADS's five stylesheets and this app's own, concatenated at compile time.
/// The `@import` has to lead, so the fonts come first.
const ADS_CSS: &str = concat!(
    "@import url('https://fonts.googleapis.com/css2?",
    "family=Bricolage+Grotesque:opsz,wght@12..96,200..800",
    "&family=Inter+Tight:wght@100..900",
    "&family=Martian+Mono:wght@100..800",
    "&family=JetBrains+Mono:wght@100..800&display=swap');\n",
    include_str!("../assets/css/tokens.css"),
    include_str!("../assets/css/base.css"),
    include_str!("../assets/css/ads.css"),
    include_str!("../assets/css/ads-utility.css"),
    include_str!("../assets/css/components.css"),
    include_str!("../assets/css/nv.css"),
);
use shell::{ControlColumn, Scope, Stage, StatusBar, Transport};
use stage::{use_stage, StageHandle};
use state::{Readout, Settings, Sources, Tab};

/// The window: the ADS stylesheets, this app's own layout rules, then the UI.
#[component]
pub fn App() -> Element {
    // ADS's default theme is light. This one runs in a dark room beside a
    // lit visualization, so it asks for dark explicitly rather than
    // leaving the look to whatever the host decides.
    use_effect(|| {
        let _ = document::eval("document.documentElement.setAttribute('data-theme', 'dark')");
    });

    rsx! {
        // The stylesheets are compiled in, not referenced as assets.
        // `asset!` resolves through the `dx` CLI's pipeline; a binary run
        // straight from cargo has no such pipeline, so every sheet 404s and
        // the window comes up as unstyled serif HTML.
        style { dangerous_inner_html: ADS_CSS }
        Workspace {}
    }
}

/// The interface on its own, so it can be rendered to a string and checked
/// without a window — the Rust stand-in for the kit's `check_markup`.
#[component]
pub fn Workspace() -> Element {
    // the last session, or the defaults on a first launch
    let settings = use_signal(|| store::load_session().unwrap_or_default());
    let sources = use_signal(Sources::default);
    let readout = use_signal(Readout::default);
    let tab = use_signal(Tab::default);
    let preset = use_signal(String::new);
    let error = use_signal(|| None::<String>);
    let busy = use_signal(|| false);

    // the engine is a plain Rust struct this crate owns; `use_engine` keeps
    // the readout fed from it
    // what the render thread draws from, shared with it
    let stage: StageHandle = use_hook(|| std::sync::Arc::new(Default::default()));
    let engine = use_engine(readout, settings, sources, stage.clone());
    // the visuals, on a GPU surface behind this page
    if std::env::var("NV_OVERLAY").map(|v| v != "0").unwrap_or(true) {
        use_stage(stage.clone(), settings);
    }

    // the status bar and the file rows reach these without being handed
    // them through every component in between
    use_context_provider(|| sources);
    use_context_provider(|| readout);
    use_context_provider(|| engine.clone());
    use_context_provider(|| error);
    use_context_provider(|| busy);
    use_context_provider(|| stage.clone());
    use_context_provider(|| settings);

    use_keys(settings, readout, engine.clone());
    use_midi(settings, sources, engine.clone());
    use_session(settings);

    let performance = settings.read().performance;
    let drop_engine = engine.clone();

    rsx! {
        div {
            class: if performance { "nv-app is-performance" } else { "nv-app" },
            // a song dropped on the window loads; a picture becomes the
            // palette
            ondragover: move |e: DragEvent| e.prevent_default(),
            ondrop: move |e: DragEvent| {
                e.prevent_default();
                for file in e.files() {
                    dropped(file.path(), drop_engine.clone(), settings, sources, readout);
                }
            },
            if !performance {
                ControlColumn { settings, sources, tab, preset }
            }
            main { class: "nv-main",
                StatusBar { readout, settings }
                if let Some(message) = error.read().clone() {
                    div { class: "nv-error", role: "alert", "{message}" }
                }
                Stage { settings }
                Scope { readout, settings }
                Transport { readout, settings }
            }
        }
    }
}

/// A file dropped on the window: a track loads, a picture is read for its
/// colours and becomes the palette.
fn dropped(
    path: std::path::PathBuf,
    engine: engine_link::EngineHandle,
    mut settings: Signal<Settings>,
    sources: Signal<Sources>,
    readout: Signal<Readout>,
) {
    // an event handler, not a render: the context is read, not hooked
    let mut error = consume_context::<Signal<Option<String>>>();
    let busy = consume_context::<Signal<bool>>();
    if engine_link::is_audio(&path) {
        let p = path.to_string_lossy().to_string();
        engine_link::load_path(engine, p, sources, readout, busy, error);
        return;
    }
    match nv_render::palette::from_image(&path) {
        Ok(colors) if !colors.is_empty() => {
            let mut s = settings.write();
            s.image_palette = colors;
            s.set_palette("image");
        }
        _ => error.set(Some(format!(
            "{} is neither a track nor a picture this can read",
            path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
        ))),
    }
}

/// The keyboard: number keys pick modes, Space plays and pauses, the
/// arrows skip five seconds, F goes fullscreen, P hides the controls and
/// Escape brings everything back. The page forwards a key only when no
/// field has the keyboard and no modifier is held, so typing a value or
/// using a system shortcut never changes the picture.
fn use_keys(
    mut settings: Signal<Settings>,
    readout: Signal<Readout>,
    engine: engine_link::EngineHandle,
) {
    use_future(move || {
        let engine = engine.clone();
        async move {
            let mut keys = document::eval(
                "document.addEventListener('keydown', (e) => { \
                     if (e.metaKey || e.ctrlKey || e.altKey || e.repeat) return; \
                     const t = e.target; \
                     if (t instanceof HTMLInputElement && t.type !== 'radio' && t.type !== 'checkbox' && t.type !== 'range') return; \
                     if (t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement || t.isContentEditable) return; \
                     const k = e.key.length === 1 ? e.key.toLowerCase() : e.key; \
                     if (/^[0-9]$/.test(k) || [' ', 'f', 'p', 'ArrowLeft', 'ArrowRight', 'Escape'].includes(k)) { \
                         e.preventDefault(); dioxus.send(k); \
                     } \
                 });",
            );
            while let Ok(key) = keys.recv::<String>().await {
                let window = dioxus::desktop::window();
                match key.as_str() {
                    " " => engine.borrow_mut().toggle_play(),
                    "f" => window.set_fullscreen(window.fullscreen().is_none()),
                    "p" => {
                        let on = settings.peek().performance;
                        settings.write().performance = !on;
                    }
                    "Escape" => {
                        settings.write().performance = false;
                        window.set_fullscreen(false);
                    }
                    "ArrowLeft" | "ArrowRight" => {
                        let step = if key == "ArrowLeft" { -5.0 } else { 5.0 };
                        let at = readout.peek().position;
                        engine.borrow_mut().seek((at + step).max(0.0));
                    }
                    digit => {
                        if let Some(mode) = state::mode_for_key(digit) {
                            settings.write().viz_mode = mode;
                        }
                    }
                }
            }
        }
    });
}

/// Every MIDI controller plugged in, applied as it plays.
fn use_midi(
    mut settings: Signal<Settings>,
    mut sources: Signal<Sources>,
    engine: engine_link::EngineHandle,
) {
    use_future(move || {
        let engine = engine.clone();
        async move {
            let midi = midi::open_all();
            sources.write().midi_devices = midi.devices.clone();
            loop {
                while let Ok(event) = midi.events.try_recv() {
                    let mut s = settings.peek().clone();
                    match midi::apply(event, &mut s) {
                        midi::MidiAction::Mode(m) => settings.write().viz_mode = m,
                        midi::MidiAction::PlayPause => engine.borrow_mut().toggle_play(),
                        midi::MidiAction::Performance => {
                            settings.write().performance = !s.performance
                        }
                        midi::MidiAction::Set => settings.set(s),
                        midi::MidiAction::Nothing => {}
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(8)).await;
            }
        }
    });
}

/// Save the settings about once a second while they change, so the next
/// launch starts where this one stopped.
fn use_session(settings: Signal<Settings>) {
    use_future(move || async move {
        let mut saved = settings.peek().clone();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let now = settings.peek().clone();
            if now != saved {
                if let Err(e) = store::save_session(&now) {
                    eprintln!("could not save the session: {e}");
                }
                saved = now;
            }
        }
    });
}
