//! Source: what feeds the visuals -- a track, the live input -- and what
//! plays them: MIDI controllers and the keyboard.

use ads::ui::{AdsButton, AdsChoice, AdsField, ButtonVariant, ChoiceKind, FieldType, SegOption};
use dioxus::prelude::*;

use super::{Group, Hint, Slider};
use crate::engine_link::{pick_and_load, EngineHandle};
use crate::state::{Readout, Settings, Sources};

#[component]
pub fn SourcePanel(settings: Signal<Settings>, sources: Signal<Sources>) -> Element {
    rsx! {
        Files { sources }
        LiveInput { settings, sources }
        Midi { sources }
        Keys {}
    }
}

/// One loaded file, or the button to load one.
#[component]
fn FileRow(
    #[props(into)] label: String,
    #[props(into)] loaded: Option<String>,
    onpick: EventHandler<()>,
) -> Element {
    let name = loaded
        .as_deref()
        .map(|p| p.rsplit('/').next().unwrap_or(p).to_string())
        .unwrap_or_else(|| "nothing loaded".into());
    let is_loaded = loaded.is_some();
    rsx! {
        div { class: "nv-file-row",
            div { class: "nv-file-row__text",
                span { class: "eyebrow", "{label}" }
                span { class: if is_loaded { "nv-file-row__name" } else { "dim nv-file-row__name" },
                    "{name}"
                }
            }
            AdsButton {
                variant: ButtonVariant::Quiet,
                onclick: move |_| onpick.call(()),
                "Browse…"
            }
        }
    }
}

#[component]
fn Files(sources: Signal<Sources>) -> Element {
    let s = sources.read().clone();
    let engine = use_context::<EngineHandle>();
    let readout = use_context::<Signal<Readout>>();
    let error = use_context::<Signal<Option<String>>>();

    let busy = use_context::<Signal<bool>>();
    let load_audio = move |()| {
        pick_and_load(engine.clone(), sources, readout, busy, error);
    };

    rsx! {
        Group { title: "Files",
            FileRow { label: "Audio", loaded: s.audio.clone(), onpick: load_audio }
            Hint { "WAV, FLAC, MP3, AAC or ALAC. You can also drop a track on the window." }
            if *use_context::<Signal<bool>>().read() {
                Hint { "Decoding and finding the beats…" }
            }
        }
    }
}

#[component]
fn LiveInput(settings: Signal<Settings>, mut sources: Signal<Sources>) -> Element {
    let s = settings.read().clone();
    let listening = sources.read().mic_live;
    let engine = use_context::<EngineHandle>();
    let mut error = use_context::<Signal<Option<String>>>();
    // the devices, looked up when the panel opens
    let devices = use_hook(nv_engine::input_devices);

    // open (or reopen) the chosen device, or close it
    let listen = use_callback(move |on: bool| {
        let mut e = engine.borrow_mut();
        if !on {
            e.stop_live();
            sources.write().mic_live = false;
            return;
        }
        match e.start_live(&settings.peek().input_device) {
            Ok(_) => {
                sources.write().mic_live = true;
                error.set(None);
            }
            Err(why) => {
                sources.write().mic_live = false;
                error.set(Some(format!("Live input: {why}")));
            }
        }
    });

    let mut options = vec![SegOption::new("System default", "")];
    options.extend(devices.iter().map(|d| SegOption::new(d.as_str(), d.as_str())));

    rsx! {
        Group { title: "Live input",
            Hint {
                "Line in, a microphone, or a loopback device, in stereo. Nothing \
                 is played back, so there is no feedback."
            }
            AdsChoice {
                kind: ChoiceKind::Switch,
                checked: listening,
                onchange: move |on| listen.call(on),
                "Listen to live input"
            }
            AdsField {
                label: "Device",
                field_type: FieldType::Select,
                value: s.input_device.clone(),
                options,
                oninput: move |e: FormEvent| {
                    settings.write().input_device = e.value();
                    if listening {
                        listen.call(true);
                    }
                },
            }
            Slider {
                label: "Gain",
                display: format!("{:+.1} dB", s.input_gain_db),
                value: s.input_gain_db,
                min: -24.0,
                max: 24.0,
                onchange: move |v: f64| settings.write().input_gain_db = v,
            }
            Hint { "Trim for a quiet line signal." }
            Slider {
                label: "Gate",
                display: format!("{:.0} dB", s.input_gate_db),
                value: s.input_gate_db,
                min: -90.0,
                max: -20.0,
                onchange: move |v: f64| settings.write().input_gate_db = v,
            }
            Hint { "Below this the input counts as silence, and the visuals rest." }
        }
    }
}

#[component]
fn Midi(sources: Signal<Sources>) -> Element {
    let devices = sources.read().midi_devices.clone();
    rsx! {
        Group { title: "MIDI",
            if devices.is_empty() {
                Hint { "No controller found. Plug one in and restart the app; it works with nothing to set up." }
            } else {
                Hint { "Listening to {devices.join(\", \")}." }
            }
            dl { class: "nv-mapping",
                for (control, action) in crate::midi::MAPPING {
                    dt { class: "num", key: "{control}", "{control}" }
                    dd { class: "dim", "{action}" }
                }
            }
        }
    }
}

#[component]
fn Keys() -> Element {
    let keys = [
        ("1 – 4", "Pick a visualization"),
        ("Space", "Play or pause"),
        ("← →", "Back or on five seconds"),
        ("F", "Fullscreen"),
        ("P", "Performance: hide the controls"),
        ("Esc", "Bring everything back"),
    ];
    rsx! {
        Group { title: "Keyboard",
            dl { class: "nv-mapping",
                for (key, action) in keys {
                    dt { class: "num", key: "{key}", "{key}" }
                    dd { class: "dim", "{action}" }
                }
            }
            Hint { "Keys are ignored while you type in a field." }
        }
    }
}
