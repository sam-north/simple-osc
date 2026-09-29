# Local patches

Vendored from <https://github.com/BillyDM/egui-baseview> at rev
`ec70c3fe6b2f070dcacbc22924431edbe24bd1c0` (MIT, see `LICENSE`), the revision nih-plug uses.
Changes are marked `PATCH(simple-osc)`:

- `src/translate.rs`: letter keys are matched case-insensitively, so holding Shift or having
  Caps Lock on no longer makes them disappear.
- `src/translate.rs`, `src/window.rs`: key events carry `physical_key`, the key's position on the
  keyboard independent of layout and modifiers. Simple Osc plays notes from that.

Drop this folder (and the `[patch]` in the root `Cargo.toml`) once upstream has equivalent fixes.
- `src/window.rs`: the UI only runs when there's input or egui asked for a frame (an animation or
  a scheduled refresh). Upstream runs the whole UI every frame and only skips the GPU draw.
- `Cargo.toml`: `default_fonts` is no longer a default feature, so egui's emoji fonts (~1 MB)
  aren't bundled. Simple Osc loads the two text fonts it uses.
