# Contributing to NeanderthalVJ

Thanks for wanting to help. Bug reports, Windows test reports and new
visualizations are all welcome.

## Reporting a bug

Open an issue with your platform (macOS or Windows, and its version), your
GPU, what you did, and what happened. If it is about a particular song,
say what format and sample rate it is. If the app printed anything to the
terminal, include it.

## Making a change

1. Install the tools and run it as the README's *Install and run* says:
   `cargo run --release -p nv-ui`.
2. Keep the tests passing: `cargo test --workspace`. Add a test for the
   behaviour you change. Most of the code can be tested without a window,
   a sound card or a GPU; see the `tests` folder of each crate.
3. Format with `cargo fmt-mine`, not `cargo fmt --all` (that would also
   rewrite `crates/ads`, which keeps its own style).
4. Keep `cargo clippy --workspace --all-targets` free of warnings.
5. If you touch anything platform-specific, check it still compiles for
   Windows:

       cargo check --target x86_64-pc-windows-msvc --workspace --all-targets

## Adding a visualization

A visualization is a mode in `nv-ui/src/state.rs`, its knobs in
`nv_render::Params`, and a branch in `nv_render::Scene::render`, so the
window and the video exporter draw it the same way. `generative.rs` and
`generative_render.rs` are a complete example: the state and geometry in
one file, testable without a GPU, and the drawing in the other.

If it draws inside the head's render pass, its pipelines must declare the
pass's `Depth32Float` depth format, and it should get a check in
`nv-render/tests/gpu.rs`. Without it WebGPU rejects every draw and the mode
silently never appears.

## Style

Comments say why, not what: the reason a value is what it is, the bug a
line prevents, the trade-off that was made. Names are plain words. Match
the code around what you change.

## Licence

By contributing, you agree that your contributions are licensed under the
project's terms: MIT OR Apache-2.0, at the user's option.
