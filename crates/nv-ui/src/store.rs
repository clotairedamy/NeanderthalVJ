//! What is kept between launches: the last session's settings and the
//! presets, as JSON in the platform's place for application data.
//!
//! `NV_CONFIG_DIR` puts them somewhere else, which is how the tests keep
//! away from a real user's files.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::state::{Mode, Settings};

/// Where this app keeps its files.
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("NV_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    let home = || std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    if cfg!(target_os = "macos") {
        home().join("Library/Application Support/NeanderthalVJ")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(home).join("NeanderthalVJ")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
            .join("neanderthalvj")
    }
}

fn session_file() -> PathBuf {
    config_dir().join("session.json")
}

fn presets_file() -> PathBuf {
    config_dir().join("presets.json")
}

fn write(path: &PathBuf, json: String) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // written beside and renamed over, so a crash mid-write cannot leave a
    // half file that loses everything on the next launch
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The settings the app was last closed with, if there are any.
pub fn load_session() -> Option<Settings> {
    let text = std::fs::read_to_string(session_file()).ok()?;
    let mut s: Settings = serde_json::from_str(&text).ok()?;
    // a session never starts in Performance, with the controls hidden
    s.performance = false;
    Some(s)
}

pub fn save_session(s: &Settings) -> Result<(), String> {
    write(&session_file(), serde_json::to_string_pretty(s).map_err(|e| e.to_string())?)
}

/// The presets the user saved, by name.
pub fn load_presets() -> BTreeMap<String, Settings> {
    std::fs::read_to_string(presets_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_presets(presets: &BTreeMap<String, Settings>) -> Result<(), String> {
    write(&presets_file(), serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?)
}

/// The looks that ship with the app.
pub fn factory() -> Vec<(&'static str, Settings)> {
    let d = Settings::default;
    vec![
        ("Synth Point Cloud", d()),
        (
            "Studio Portrait",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "mono".into(),
                pc_perspective: 0.2,
                auto_camera: true,
                auto_camera_speed: 0.3,
                pc_effect: "none".into(),
                fx_bloom: 0.2,
                fx_vignette: 0.6,
                fx_flash: 0.3,
                ..d()
            },
        ),
        (
            "Band Lights",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "turbo".into(),
                pc_band_axis: "vertical".into(),
                pc_effect: "vortex".into(),
                tint_strength: 0.8,
                fx_bloom: 0.6,
                fx_threshold: 0.45,
                ..d()
            },
        ),
        (
            "Solar Flare",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "plasma".into(),
                pc_effect: "corona".into(),
                pc_effect_amount: 1.3,
                pc_camera_punch: 0.8,
                tint_strength: 0.85,
                fx_bloom: 0.6,
                fx_threshold: 0.55,
                fx_vignette: 0.5,
                ..d()
            },
        ),
        (
            "Supernova Drop",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "neon".into(),
                pc_effect: "supernova".into(),
                pc_effect_amount: 1.4,
                pc_camera_punch: 1.0,
                tint_strength: 0.7,
                fx_bloom: 0.5,
                fx_flash: 1.1,
                fx_chroma: 0.6,
                ..d()
            },
        ),
        (
            "Comet Head",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "plasma".into(),
                pc_effect: "orbits".into(),
                pc_effect_amount: 1.4,
                tint_strength: 0.7,
                fx_bloom: 0.7,
                fx_threshold: 0.5,
                fx_ghost: 0.35,
                ..d()
            },
        ),
        (
            "Saint Halo",
            Settings {
                viz_mode: Mode::PointCloud,
                palette_per_mode: false,
                palette: "mono".into(),
                pc_effect: "halo".into(),
                fx_flash: 1.2,
                fx_chroma: 0.6,
                fx_vignette: 0.6,
                ..d()
            },
        ),
        (
            "Shock Wall",
            Settings {
                viz_mode: Mode::HidingGrid,
                palette_per_mode: false,
                palette: "plasma".into(),
                grid_impact: "scattered".into(),
                grid_waves: 1.4,
                grid_echo: 0.7,
                grid_tilt: 0.55,
                fx_bloom: 0.6,
                fx_chroma: 0.5,
                ..d()
            },
        ),
        (
            "Club Analyser",
            Settings {
                viz_mode: Mode::HidingGrid,
                palette_per_mode: false,
                palette: "neon".into(),
                grid_analyser: true,
                grid_sparkle: 1.6,
                grid_tilt: 0.15,
                fx_flash: 1.2,
                fx_grain: 0.4,
                fx_chroma: 0.8,
                ..d()
            },
        ),
        (
            "Neon Mandala",
            Settings {
                viz_mode: Mode::Generative,
                palette_per_mode: false,
                palette: "neon".into(),
                gen_symmetry: 8,
                gen_trails: 0.75,
                gen_glow: 1.2,
                fx_bloom: 0.6,
                ..d()
            },
        ),
        (
            "Bass Temple",
            Settings {
                viz_mode: Mode::Generative,
                palette_per_mode: false,
                palette: "turbo".into(),
                gen_symmetry: 4,
                gen_complexity: 1.0,
                gen_flow: 1.8,
                gen_width: 1.4,
                fx_ghost: 0.5,
                fx_bloom: 0.4,
                ..d()
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_survives_a_restart_and_an_older_file_still_loads() {
        let dir = std::env::temp_dir().join(format!("nvj-store-{}", std::process::id()));
        std::env::set_var("NV_CONFIG_DIR", &dir);

        let mine = Settings {
            gen_symmetry: 9,
            palette: "turbo".into(),
            performance: true,
            ..Settings::default()
        };
        save_session(&mine).unwrap();
        let back = load_session().unwrap();
        assert_eq!(back.gen_symmetry, 9);
        assert_eq!(back.palette, "turbo");
        assert!(!back.performance, "a session must not start with the controls hidden");

        // a file from before a setting existed takes that setting's default
        std::fs::write(dir.join("session.json"), r#"{"gen_symmetry": 5}"#).unwrap();
        let old = load_session().unwrap();
        assert_eq!(old.gen_symmetry, 5);
        assert_eq!(old.grid_waves, Settings::default().grid_waves);

        let mut presets = BTreeMap::new();
        presets.insert("Mine".to_string(), mine);
        save_presets(&presets).unwrap();
        assert_eq!(load_presets()["Mine"].gen_symmetry, 9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_factory_presets_use_palettes_and_effects_that_exist() {
        for (name, look) in factory() {
            assert!(
                crate::state::PALETTES.contains(&look.palette.as_str()),
                "{name}: {}",
                look.palette
            );
            assert!(
                crate::state::PC_EFFECTS.contains(&look.pc_effect.as_str()),
                "{name}: {}",
                look.pc_effect
            );
        }
    }

    #[test]
    fn the_factory_presets_have_distinct_names() {
        let names: std::collections::BTreeSet<_> = factory().iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), factory().len());
    }
}
