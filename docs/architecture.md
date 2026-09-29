# Architecture

## Layout

```
src/dsp.rs            filter, envelope, note stack (audio thread)
src/waves/            waveform catalog and oscillator
  basic.rs            math-based waveforms (PolyBLEP / PolyBLAMP band-limiting)
  akwf.rs             Adventure Kid waves (files in assets/waves/akwf/)
  wavetable.rs        band-limited mip-mapped tables for sampled waves
src/params.rs         host-facing parameters
src/lib.rs            plugin glue: MIDI, processing, idle fast path
src/shared.rs         lock-free audio <-> GUI data (scope buffer, sounding note, GUI notes)
src/logo.rs           logo geometry, shared by the GUI and build.rs (app icon)
src/main.rs           standalone entry point (MIDI auto-connect, --no-midi)
src/standalone_window.rs  fixed window size and icon for the standalone (Windows)
src/ui/
  controls.rs         which controls exist, their roles and value formats (no visuals, no text)
  readout.rs          live values for display
  input.rs            computer keyboard and on-screen keys to notes
  widgets.rs          mouse interaction
  layout.rs           rectangles, sized from the theme
  theme.rs, i18n.rs   theme and language file formats
  assets.rs           built-in and user files, saved preferences
  fonts.rs            the two bundled fonts
  render/             drawing, one file per element
vendor/               patched copies of egui-baseview and nih_plug_egui (see their PATCHES.md)
```

## Rules

- The audio thread never allocates, locks or does I/O. `shared.rs` is atomics only.
- UI code outside `render/` never picks a color, size or string. Colors and sizes come from the
  theme, text from the language pack.
- `waves::catalog()` order is what hosts save. **Only append** to it, including in `akwf.rs`.
  The GUI's browse order (category, then A-Z by translated name) is separate.
- Host-facing parameter names stay in English; hosts read them once at load.
- The editor is a fixed 900 x 540. Hosts are told it can't resize; the standalone window's resize
  border is removed at startup.

## Non-obvious decisions

- **WASAPI buffer size:** the standalone passes `--period-size 2048`. nih-plug treats it as a
  maximum and panics if Windows (shared mode) delivers more, e.g. 1056 samples.
- **Keyboard focus:** on Windows the editor is a child window that doesn't take focus on click.
  `input.rs` reports a UI event on every click so egui-baseview's `windows_keyboard_workaround`
  focuses it.
- **Keys use physical positions** (patched egui-baseview), so Shift, Caps Lock and non-QWERTY
  layouts don't break note input or leave notes stuck.
- **Repaints:** both vendored crates are patched so the GUI only runs on input or when asked.
  `ui/mod.rs` requests continuous repaints only while a voice is active, otherwise every 100 ms.
- **Scope** locks to the oscillator's phase wrap (stored per sample), not zero crossings, so
  every waveform holds still.
- **Logo:** "SS", an S then the same S on its side (a sine wave). Defined once in `logo.rs`.
