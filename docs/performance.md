# Performance

## Build settings (project-only)

- `Cargo.toml` release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`.
- `.cargo/config.toml`: `target-cpu=x86-64-v3` (AVX2 + FMA). Builds need a CPU from about 2015
  or later; delete those lines to build for any 64-bit x86 CPU.
- Debug builds use `opt-level = 1` so the DSP and generated textures stay usable.

## Audio

- Silence (idle envelope, no notes, no events) is a fast path: zero the buffer, advance the
  smoothers, skip all DSP.
- Pitch and filter coefficients are cached and only recomputed when their inputs change.
- Supersaw side voices only run while a supersaw is playing.
- Wavetable levels shrink with their harmonic count (~270 KB for all 16 waves).

## GUI

- Continuous 60 fps only while a voice is active; otherwise a pass every 100 ms or on input.
- The rust and tolex textures are generated once per window open and not kept on the CPU.
- The distressed title is one texture quad, not per-frame shapes.
- Only Ubuntu Light and Hack are bundled; egui's emoji fonts are left out.
- No depth or stencil buffers are allocated for the GL window.
