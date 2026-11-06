//! Export: render the visualization out as an Instagram-ready video.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use ads::ui::{AdsButton, AdsChoice, AdsField, ButtonVariant, FieldType, SegOption};
use dioxus::prelude::*;
use nv_export::{Format, Job, Progress};

use super::{Group, Hint, Slider};
use crate::stage::{params_from, StageHandle};
use crate::state::{Settings, Sources};

/// Where an export stands.
#[derive(Clone, PartialEq)]
enum State {
    Idle,
    Running,
    Done(String),
    Failed(String),
}

#[component]
pub fn ExportPanel(sources: Signal<Sources>) -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let readout = use_context::<Signal<crate::state::Readout>>();
    let stage = use_context::<StageHandle>();
    let mut format = use_signal(|| "reel".to_string());
    let mut state = use_signal(|| State::Idle);
    let mut fraction = use_signal(|| 0.0f32);
    let progress: Signal<Option<Arc<Progress>>> = use_signal(|| None);

    let audio = sources.read().audio.clone();
    let running = *state.read() == State::Running;

    let start = {
        let stage = stage.clone();
        let audio = audio.clone();
        move |_| {
            let Some(audio) = audio.clone() else { return };
            let fmt =
                if format.read().as_str() == "square" { Format::Square } else { Format::Reel };
            let stem = std::path::Path::new(&audio)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "NeanderthalVJ".into());
            let stage = stage.clone();
            // the dialog is asked for from a task, never inside this
            // handler: on macOS a modal dialog runs the event loop inside
            // the call, the interface redraws meanwhile, and redrawing a
            // panel whose handler is still running aborts the app
            spawn(async move {
                let Some(out) = rfd::AsyncFileDialog::new()
                    .set_title("Export video")
                    .set_directory(videos_dir())
                    .set_file_name(format!("{stem} — {}.mp4", fmt.label()))
                    .add_filter("MP4 video", &["mp4"])
                    .save_file()
                    .await
                else {
                    return;
                };

                // the clip renders what is on screen: the panels' settings
                // and the camera as they are now
                let s = settings.read().clone();
                let job = Job {
                    audio: audio.into(),
                    out: out.path().to_path_buf(),
                    format: fmt,
                    fps: s.export_fps,
                    params: params_from(&s),
                    camera: *stage.camera.lock().unwrap(),
                    assets: crate::stage::assets_dir().into(),
                    start: s.export_start,
                    length: s.export_length,
                    bitrate_mbps: if s.export_quality == "high" { 24 } else { 12 },
                    orbit: crate::stage::orbit_rate(&s),
                };
                let p = Arc::new(Progress::default());
                let mut progress = progress;
                progress.set(Some(p.clone()));
                state.set(State::Running);
                fraction.set(0.0);
                let handle = nv_export::spawn(job, p.clone());

                // watch it from the UI thread without ever blocking it
                while !handle.is_finished() {
                    fraction.set(p.fraction());
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                match handle.join() {
                    Ok(Ok(path)) => state.set(State::Done(path.to_string_lossy().to_string())),
                    Ok(Err(e)) if e == "cancelled" => state.set(State::Idle),
                    Ok(Err(e)) => state.set(State::Failed(e)),
                    Err(_) => state.set(State::Failed("the export thread stopped".into())),
                }
                progress.set(None);
            });
        }
    };

    let cancel = move |_| {
        if let Some(p) = progress.read().as_ref() {
            p.cancel.store(true, Ordering::Relaxed);
        }
    };

    let pct = (*fraction.read() * 100.0).round() as u32;
    let current = state.read().clone();
    let s = settings.read().clone();
    let duration = readout.read().duration;
    let clip = if s.export_length > 0.0 {
        format!(
            "{} from {}",
            crate::state::Readout::clock(s.export_length),
            crate::state::Readout::clock(s.export_start)
        )
    } else if s.export_start > 0.0 {
        format!("from {} to the end", crate::state::Readout::clock(s.export_start))
    } else {
        "the whole track".to_string()
    };

    rsx! {
        Group { title: "Export",
            Hint {
                "Render the visualization to a video at the current settings, effects and \
                 camera. Every frame is rendered, whatever the machine's speed, and the \
                 song is muxed in."
            }
            AdsField {
                label: "Format",
                field_type: FieldType::Select,
                value: format.read().clone(),
                options: vec![
                    SegOption::new("Instagram Reel — 9:16, 1080×1920", "reel"),
                    SegOption::new("Instagram Square — 1:1, 1080×1080", "square"),
                ],
                disabled: running,
                hint: "A Reel fills a phone held upright; a square post sits in the feed.",
                oninput: move |e: FormEvent| format.set(e.value()),
            }
            AdsField {
                label: "Frame rate",
                field_type: FieldType::Select,
                value: s.export_fps.to_string(),
                options: vec![SegOption::new("60 fps — smoothest", "60"), SegOption::new("30 fps — half the size", "30")],
                disabled: running,
                hint: "Instagram plays both; 60 keeps fast motion fluid.",
                oninput: move |e: FormEvent| {
                    if let Ok(v) = e.value().parse() {
                        settings.write().export_fps = v;
                    }
                },
            }
            AdsField {
                label: "Quality",
                field_type: FieldType::Select,
                value: s.export_quality.clone(),
                options: vec![SegOption::new("High — 24 Mbit/s", "high"), SegOption::new("Standard — 12 Mbit/s", "standard")],
                disabled: running,
                hint: "Instagram re-encodes every upload; starting high keeps the particle grain through it.",
                oninput: move |e: FormEvent| settings.write().export_quality = e.value(),
            }
            Slider {
                label: "Start",
                display: crate::state::Readout::clock(s.export_start),
                value: s.export_start,
                min: 0.0,
                max: duration.max(1.0),
                onchange: move |v: f64| settings.write().export_start = v,
            }
            Hint { "Where in the track the clip begins." }
            Slider {
                label: "Length",
                display: if s.export_length > 0.0 { crate::state::Readout::clock(s.export_length) } else { "to the end".to_string() },
                value: s.export_length,
                min: 0.0,
                max: 180.0,
                onchange: move |v: f64| settings.write().export_length = v,
            }
            Hint { "How long the clip runs. Reels run up to 3 minutes; 0 renders to the end of the track. Exporting: {clip}." }
            if running {
                div { class: "nv-progress",
                    div { class: "nv-progress__bar", style: "width: {pct}%" }
                }
                Hint { "Rendering… {pct}%" }
                AdsButton { variant: ButtonVariant::Quiet, onclick: cancel, "Cancel" }
            } else {
                AdsButton {
                    variant: ButtonVariant::Accent,
                    disabled: audio.is_none(),
                    onclick: start,
                    "Export video…"
                }
            }
            match current {
                State::Done(path) => rsx! {
                    Hint { "Saved to {path}" }
                    AdsButton {
                        variant: ButtonVariant::Quiet,
                        onclick: move |_| reveal(&path),
                        "{REVEAL}"
                    }
                },
                State::Failed(why) => rsx! { p { class: "nv-error", role: "alert", "{why}" } },
                _ => rsx! {},
            }
            if audio.is_none() {
                Hint { "Load a track first." }
            }
        }
        if cfg!(all(target_os = "macos", feature = "syphon")) {
            Group { title: "Live output",
                AdsChoice {
                    checked: s.syphon,
                    onchange: move |on| settings.write().syphon = on,
                    "Share the visuals through Syphon"
                }
                Hint {
                    "Sends the visualization, without the controls, live to Resolume, VDMX, \
                     MadMapper, OBS or any other Syphon app on this Mac, as a source named \
                     \u{201c}NeanderthalVJ\u{201d}. No copy is made until an app connects."
                }
                if s.syphon {
                    p { class: "nv-hint",
                        if stage.syphon_clients.load(Ordering::Relaxed) {
                            "Connected: another app is taking the frames."
                        } else {
                            "Waiting for an app to connect."
                        }
                    }
                }
            }
        }
    }
}

/// Where videos go by default: Movies on a Mac, Videos elsewhere.
fn videos_dir() -> std::path::PathBuf {
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let videos = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
    std::path::Path::new(&home).join(videos)
}

/// What the system's file browser is called, for the button that opens it.
const REVEAL: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(windows) {
    "Show in Explorer"
} else {
    "Show in folder"
};

/// Open the system's file browser on the saved video, selected.
fn reveal(path: &str) {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    let opened = Command::new("open").arg("-R").arg(path).spawn();
    // Explorer wants the quotes around the path only, after the comma, so
    // the argument is passed raw rather than quoted whole
    #[cfg(windows)]
    let opened = {
        use std::os::windows::process::CommandExt;
        Command::new("explorer").raw_arg(format!("/select,\"{path}\"")).spawn()
    };
    #[cfg(not(any(target_os = "macos", windows)))]
    let opened = Command::new("xdg-open")
        .arg(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(".")))
        .spawn();
    if let Err(e) = opened {
        eprintln!("could not open the file browser: {e}");
    }
}
