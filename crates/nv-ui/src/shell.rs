//! The window: a control column beside the stage, with the status bar above
//! it and the scope and transport below.

use ads::ui::{AdsButton, AdsChoice, AdsField, AdsTabs, ButtonVariant, FieldType, SegOption};
use dioxus::prelude::*;

use crate::engine_link::EngineHandle;
use crate::panel::{
    export::ExportPanel, fx::FxPanel, motion::MotionPanel, source::SourcePanel,
    visuals::VisualsPanel,
};
use crate::stage::StageHandle;
use crate::state::{Readout, Settings, Sources, Tab, SCOPE_BARS};
use crate::store;

/// The readout strip above the stage: tempo, frame rate, the level, the
/// running mode, whether it is listening live, and the MIDI state.
#[component]
pub fn StatusBar(readout: Signal<Readout>, settings: Signal<Settings>) -> Element {
    let r = *readout.read();
    let (mode, show_level) = {
        let s = settings.read();
        (s.viz_mode, s.show_level)
    };
    let sources = use_context::<Signal<Sources>>();
    let (devices, live) = {
        let s = sources.read();
        (s.midi_devices.clone(), s.mic_live)
    };
    let bpm = if r.bpm > 0.0 { format!("{:.0}", r.bpm) } else { "—".into() };
    let fps = if r.fps > 0.0 { format!("{:.0}", r.fps) } else { "—".into() };
    let midi = match devices.len() {
        0 => "No MIDI controller".to_string(),
        1 => format!("MIDI: {}", devices[0]),
        n => format!("MIDI: {n} controllers"),
    };
    rsx! {
        div { class: "nv-status",
            div { class: "nv-status__left num",
                span { "BPM {bpm}" }
                span { "FPS {fps}" }
                if show_level {
                    span { "level {r.velocity:.2}" }
                }
                span { class: "nv-status__mode", "{mode}" }
                if live {
                    span { class: "nv-status__live", "LIVE" }
                }
            }
            span { class: "dim nv-status__right", title: "{devices.join(\", \")}", "{midi}" }
        }
    }
}

/// Where the visualization is drawn.
///
/// ADS has no component for this and should not get one — the kit is for
/// chrome, and this is the picture. In the running app a native wgpu
/// surface sits behind this element; here it is the empty stage.
#[component]
pub fn Stage(settings: Signal<Settings>) -> Element {
    let mode = settings.read().viz_mode;
    let stage = use_context::<StageHandle>();
    // the drag's last position; `None` while the button is up
    let mut dragging = use_signal(|| None::<(f32, f32)>);

    let on_down = {
        move |e: MouseEvent| {
            let p = e.data().client_coordinates();
            dragging.set(Some((p.x as f32, p.y as f32)));
        }
    };
    let on_move = {
        let stage = stage.clone();
        move |e: MouseEvent| {
            let Some((lx, ly)) = *dragging.read() else { return };
            let p = e.data().client_coordinates();
            let (x, y) = (p.x as f32, p.y as f32);
            stage.camera.lock().unwrap().drag(x - lx, y - ly);
            dragging.set(Some((x, y)));
        }
    };
    let on_wheel = {
        let stage = stage.clone();
        move |e: WheelEvent| {
            let dy = e.data().delta().strip_units().y as f32;
            stage.camera.lock().unwrap().zoom(dy);
        }
    };

    rsx! {
        div {
            class: "nv-stage",
            role: "img",
            "aria-label": "Visualization: {mode}. Drag to turn it, scroll to zoom.",
            onmousedown: on_down,
            onmousemove: on_move,
            onmouseup: move |_| dragging.set(None),
            onmouseleave: move |_| dragging.set(None),
            onwheel: on_wheel,
            div { class: "nv-stage__surface" }
        }
    }
}

/// The waveform under the stage, with the level and tempo read off it.
#[component]
pub fn Scope(readout: Signal<Readout>, settings: Signal<Settings>) -> Element {
    let r = *readout.read();
    let mode = settings.read().viz_mode;
    let bpm = if r.bpm > 0.0 { format!("{:.0} BPM", r.bpm) } else { "— BPM".into() };
    rsx! {
        div { class: "nv-scope",
            // the spectrum as it is heard, low to high
            svg {
                class: "nv-scope__trace",
                view_box: "0 0 {SCOPE_BARS} 10",
                "preserveAspectRatio": "none",
                role: "img",
                "aria-label": "Spectrum",
                for (i, v) in r.spectrum.iter().enumerate() {
                    rect {
                        key: "{i}",
                        x: "{i as f32 + 0.12}",
                        y: "{10.0 - v * 10.0}",
                        width: "0.76",
                        height: "{(v * 10.0).max(0.15)}",
                        fill: "currentColor",
                    }
                }
            }
            div { class: "nv-scope__meta num",
                span { class: "eyebrow", "{mode}" }
                span { "{r.level_db:.1} dB" }
                span { class: "dim", "{bpm}" }
            }
        }
    }
}

/// Play, position, speed, and the switches that ride with them.
#[component]
pub fn Transport(readout: Signal<Readout>, settings: Signal<Settings>) -> Element {
    let r = *readout.read();
    let s = settings.read().clone();
    let engine = use_context::<EngineHandle>();
    let position = Readout::clock(r.position);
    let duration = Readout::clock(r.duration);
    let progress = if r.duration > 0.0 { (r.position / r.duration * 100.0) as i64 } else { 0 };
    rsx! {
        div { class: "nv-transport",
            AdsButton {
                variant: ButtonVariant::Accent,
                class: "nv-transport__play",
                title: "Play or pause (Space)",
                onclick: {
                    let engine = engine.clone();
                    move |_| engine.borrow_mut().toggle_play()
                },
                span { "aria-hidden": "true", if r.playing { "❚❚" } else { "▶" } }
                span { class: "nv-sr-only", if r.playing { "Pause" } else { "Play" } }
            }
            span { class: "num nv-transport__clock", "{position} / {duration}" }
            input {
                r#type: "range",
                class: "ads-range nv-transport__seek",
                "aria-label": "Position",
                title: "Drag to move through the track; the arrow keys skip five seconds",
                min: "0",
                max: "100",
                value: "{progress}",
                oninput: {
                    let engine = engine.clone();
                    move |e: FormEvent| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            let d = readout.read().duration;
                            engine.borrow_mut().seek(d * v / 100.0);
                        }
                    }
                },
            }
            span { class: "eyebrow nv-transport__label", "Speed" }
            input {
                r#type: "range",
                class: "ads-range nv-transport__speed",
                "aria-label": "Speed",
                title: "Playback speed, from a quarter to twice as fast",
                min: "25",
                max: "200",
                value: "{(s.speed * 100.0) as i64}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        settings.write().speed = v / 100.0;
                    }
                },
            }
            span { class: "num nv-transport__rate", "{s.speed:.2}x" }
            span { title: "Keep the key when the speed changes, as a DJ's key lock does; off, the pitch moves with the speed like a turntable",
                AdsChoice {
                    checked: s.preserve_pitch,
                    onchange: move |on| settings.write().preserve_pitch = on,
                    "Keep pitch"
                }
            }
            span { title: "Start again from the top at the end of the track",
                AdsChoice {
                    checked: s.looping,
                    onchange: move |on| settings.write().looping = on,
                    "Loop"
                }
            }
            AdsButton {
                variant: ButtonVariant::Quiet,
                title: "Hide the controls for a show (P); Escape brings them back",
                onclick: move |_| {
                    let on = settings.read().performance;
                    settings.write().performance = !on;
                },
                "Performance"
            }
        }
    }
}

/// The settings a look should not carry: the hardware, the transport and
/// the export, which belong to this session rather than to a preset.
pub fn apply_look(current: &Settings, look: &Settings) -> Settings {
    Settings {
        input_device: current.input_device.clone(),
        input_gain_db: current.input_gain_db,
        input_gate_db: current.input_gate_db,
        av_offset_ms: current.av_offset_ms,
        frame_rate: current.frame_rate,
        speed: current.speed,
        preserve_pitch: current.preserve_pitch,
        looping: current.looping,
        performance: current.performance,
        export_fps: current.export_fps,
        export_quality: current.export_quality.clone(),
        export_start: current.export_start,
        export_length: current.export_length,
        // a look without a picture of its own keeps the one already chosen
        image_palette: if look.image_palette.is_empty() {
            current.image_palette.clone()
        } else {
            look.image_palette.clone()
        },
        ..look.clone()
    }
}

/// The presets row above the tabs: the looks that ship with the app, the
/// ones saved here, and saving the current one.
#[component]
pub fn Presets(preset: Signal<String>, settings: Signal<Settings>) -> Element {
    let mut saved = use_signal(store::load_presets);
    let mut name = use_signal(String::new);
    let mut error = use_context::<Signal<Option<String>>>();

    let mut options = vec![SegOption::new("Choose a look…", "")];
    options
        .extend(store::factory().iter().map(|(n, _)| SegOption::new(*n, format!("factory:{n}"))));
    options.extend(
        saved.read().keys().map(|n| SegOption::new(format!("{n} (yours)"), format!("user:{n}"))),
    );
    let selected = preset.read().clone();
    let is_user = selected.starts_with("user:");

    let choose = move |e: FormEvent| {
        let value = e.value();
        let look = if let Some(n) = value.strip_prefix("factory:") {
            store::factory().into_iter().find(|(f, _)| *f == n).map(|(_, s)| s)
        } else if let Some(n) = value.strip_prefix("user:") {
            saved.read().get(n).cloned()
        } else {
            None
        };
        if let Some(look) = look {
            let next = apply_look(&settings.peek(), &look);
            settings.set(next);
        }
        preset.set(value);
    };
    let save = move |_| {
        let n = name.read().trim().to_string();
        if n.is_empty() {
            error.set(Some("Name the preset before saving it.".into()));
            return;
        }
        saved.write().insert(n.clone(), settings.peek().clone());
        match store::save_presets(&saved.read()) {
            Ok(()) => {
                preset.set(format!("user:{n}"));
                name.set(String::new());
                error.set(None);
            }
            Err(e) => error.set(Some(format!("Could not save the preset: {e}"))),
        }
    };
    let delete = move |_| {
        let Some(n) = preset.peek().strip_prefix("user:").map(str::to_string) else { return };
        saved.write().remove(&n);
        if let Err(e) = store::save_presets(&saved.read()) {
            error.set(Some(format!("Could not delete the preset: {e}")));
        }
        preset.set(String::new());
    };
    let reset = move |_| {
        let next = apply_look(&settings.peek(), &Settings::default());
        settings.set(next);
        preset.set(String::new());
    };

    rsx! {
        section { class: "card nv-presets",
            h2 { class: "eyebrow nv-group__title", "Presets" }
            AdsField {
                label: "Look",
                field_type: FieldType::Select,
                value: selected,
                options,
                hint: "A complete look: mode, colours, motion and effects. Your device, \
                       transport and export settings stay as they are.",
                oninput: choose,
            }
            div { class: "nv-presets__row",
                AdsField {
                    label: "Save the current look as",
                    value: "{name}",
                    placeholder: "Preset name",
                    oninput: move |e: FormEvent| name.set(e.value()),
                }
                AdsButton { variant: ButtonVariant::Quiet, onclick: save, "Save" }
                AdsButton {
                    variant: ButtonVariant::Quiet,
                    disabled: !is_user,
                    title: "Delete the selected preset of yours",
                    onclick: delete,
                    "Delete"
                }
            }
            div { class: "nv-presets__reset",
                AdsButton { variant: ButtonVariant::Quiet, onclick: reset, "Reset to defaults" }
            }
        }
    }
}

/// The control column: presets, the tab row, and the panel beneath it.
#[component]
pub fn ControlColumn(
    settings: Signal<Settings>,
    sources: Signal<Sources>,
    tab: Signal<Tab>,
    preset: Signal<String>,
) -> Element {
    let tabs: Vec<SegOption> =
        Tab::ALL.iter().map(|t| SegOption::new(t.to_string(), t.slug())).collect();
    let current = tab.read().slug().to_string();
    rsx! {
        aside { class: "nv-controls",
            Presets { preset, settings }
            AdsTabs {
                tabs,
                label: "Controls",
                selected: current,
                onselect: move |v: String| tab.set(Tab::from_slug(&v)),
            }
            div { class: "nv-controls__body",
                match *tab.read() {
                    Tab::Source => rsx! { SourcePanel { settings, sources } },
                    Tab::Visuals => rsx! { VisualsPanel { settings } },
                    Tab::Motion => rsx! { MotionPanel { settings } },
                    Tab::Fx => rsx! { FxPanel { settings } },
                    Tab::Export => rsx! { ExportPanel { sources } },
                }
            }
        }
    }
}
