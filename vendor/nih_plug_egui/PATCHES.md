# Local patches

Vendored from <https://github.com/robbert-vdh/nih-plug> (`nih_plug_egui/`) at rev
`de421011f41a6d10fc8c7a6084e4f4dee0143683` (ISC, see `LICENSE`). Changes are marked
`PATCH(simple-osc)`:

- `src/editor.rs`: redraws are only forced for the first second after the window opens.
  Upstream forces one every frame forever, so the GUI can never idle.
- `src/editor.rs`: the OpenGL window doesn't allocate depth or stencil buffers, which egui never
  uses.
- `Cargo.toml`: `default_fonts` is no longer a default feature; depends on nih-plug by git rev
  instead of a relative path.
