//! With the `syphon` feature on a Mac: compile Syphon's Metal shaders into
//! `default.metallib` beside the program.
//!
//! Syphon is linked in statically, so when it looks for its shaders in "its
//! bundle" it finds the program's directory -- and without them it cannot
//! share a single frame. Nothing happens in any other build.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let syphon = std::env::var_os("CARGO_FEATURE_SYPHON").is_some();
    let mac = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");
    if !(syphon && mac) {
        return;
    }
    println!("cargo:rerun-if-changed=syphon/SyphonMetalShaders.metal");
    println!("cargo:rerun-if-changed=syphon/SyphonServerMetalTypes.h");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let air = out.join("SyphonMetalShaders.air");
    let lib = out.join("default.metallib");
    let run = |args: &[&std::ffi::OsStr]| {
        let status = std::process::Command::new("xcrun").args(args).status();
        if !matches!(status, Ok(s) if s.success()) {
            panic!(
                "compiling Syphon's Metal shaders failed. The `syphon` feature needs full Xcode \
                 with its Metal Toolchain: xcodebuild -downloadComponent MetalToolchain"
            );
        }
    };
    run(&[
        "-sdk".as_ref(),
        "macosx".as_ref(),
        "metal".as_ref(),
        "-c".as_ref(),
        "syphon/SyphonMetalShaders.metal".as_ref(),
        "-o".as_ref(),
        air.as_os_str(),
    ]);
    run(&[
        "-sdk".as_ref(),
        "macosx".as_ref(),
        "metallib".as_ref(),
        air.as_os_str(),
        "-o".as_ref(),
        lib.as_os_str(),
    ]);

    // beside the program: OUT_DIR is target/<profile>/build/<crate>/out
    if let Some(profile_dir) = out.ancestors().nth(3) {
        for dir in [profile_dir.to_path_buf(), profile_dir.join("examples")] {
            let _ = std::fs::create_dir_all(&dir);
            if let Err(e) = std::fs::copy(&lib, dir.join("default.metallib")) {
                println!(
                    "cargo:warning=could not place default.metallib in {}: {e}",
                    dir.display()
                );
            }
        }
    }
}
