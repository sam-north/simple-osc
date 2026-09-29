# Simple Osc

A single-oscillator synth instrument written in Rust with [nih-plug](https://github.com/robbert-vdh/nih-plug).
Builds as a 64-bit **VST3**, a **CLAP** plugin and a **standalone app**.

- 25 waveforms on an endless Wave knob, grouped by category and sorted A to Z:
  - **Basic**: sine, triangle, saw, square, pulse, supersaw, noise, phase distortion, fold
  - **Adventure Kid**: 16 sampled single-cycle waves picked for mono playing (sax, cello, oboe,
    double bass, lead guitar, voice, chip, ...)
  - All band-limited, so high notes stay clean
- Shape knob: the current waveform's extra control (pulse width, supersaw detune, noise color,
  distortion amount, fold amount)
- Pitch: plays MIDI notes, with an Octave switch (−2 to +2), Coarse (±24 semitones) and Fine
  (±100 cents)
- Filter: low pass, high pass, band pass or notch, with cutoff and resonance
- Envelope: attack, decay, sustain, release
- Swappable themes (Grunge, Cyberpunk, Vintage) and languages (English, Español, Français), switchable
  live from the top-right corner of the window
- Live oscilloscope locked to the start of each cycle so the waveform holds still
- Play from a MIDI keyboard, the computer keyboard, or by clicking the on-screen keys

## Run it standalone

```sh
cargo run --release
```

The first build takes a few minutes; later ones are quick. Close the window to stop it.

- **MIDI keyboard:** the first MIDI input found is connected automatically. To pick another one,
  run `cargo run --release -- --midi-input "<name>"` (pass an empty name to list them).
- **Computer keyboard:** click inside the window first, then `A S D F G H J K L` are the white
  keys and `W E T Y U O P` the black keys, from middle C up. Use the Octave switch to play higher
  or lower. Caps Lock must be off.
- `cargo run --release -- --no-midi` skips connecting a MIDI keyboard.
- `cargo run --release -- --help` lists all audio and MIDI options.

Debug builds (`cargo run`) abort if the audio thread ever allocates memory, which catches
real-time mistakes early.

## Controls

| Control | How |
|---|---|
| Turn a knob | drag up/down, or scroll wheel |
| Fine adjust | hold Shift while dragging |
| Reset to default | double-click |
| Wave (endless) | drag up or right, scroll up, or click for the next waveform; the opposite or right-click for the previous one. Wraps around at both ends. |
| Filter Type (endless) | same as Wave: turns forever, wrapping through Low Pass, High Pass, Band Pass, Notch |
| Octave switch (above Coarse) | click or drag to a position, or scroll |

## Themes and languages

The two small labels in the top-right corner switch the theme and the language: click for the
next one, right-click for the previous one. The choice is remembered for every instance of the
plugin.

Themes, language packs and the computer-keyboard layout are plain TOML files in `assets/`:

```
assets/themes/grunge.toml       default theme
assets/themes/cyberpunk.toml
assets/themes/vintage.toml
assets/languages/en-US.toml     default language, and the fallback for missing text
assets/languages/es.toml
assets/languages/fr.toml
assets/keymaps/qwerty.toml
```

These are compiled into the plugin. To add your own **without rebuilding**, drop a file into
the user folder, `%APPDATA%\Simple Osc\themes\` or `%APPDATA%\Simple Osc\languages\`; it loads
the next time the window opens. Copy a built-in file as a starting point and give it a new `id`.
A file with the same `id` as a built-in one replaces it. A broken file is skipped, and its error
goes to the log.

- A **theme** sets every color, font, size and spacing, and picks a drawing style for each
  element: background (`rusted_metal`, `tolex`, `gradient` with an optional grid, `solid`), panels
  (`riveted_plate`, `enamel_plate`, `neon_frame`), knobs (`iron`, `skirted`, `hex_bolt`,
  `chicken_head`, `neon_ring`, `neon_segments`) and title effect (`distressed`, `nameplate`,
  `chromatic`, `none`). A new look that recombines these needs
  no code; a new drawing style is one function in `src/ui/render/`.
- A **language pack** holds every visible string, plus the decimal separator and how notes are
  named (`C4` or `Do3`). Missing keys fall back to English. `cargo test` fails if a built-in
  pack is missing a key, so translations can't silently fall behind.

DAWs show the plugin's parameter names for automation. Those stay in English, because hosts
read them once when the plugin loads.

## Logo

The logo is "SS": an S for Simple, then the same S on its side, which is a sine wave for Osc. It's
defined once as geometry in `src/logo.rs`. The GUI draws it in each theme's colors (`[logo]` in
the theme file), and `build.rs` renders the standalone app's icon from it at build time, so there
are no image files.

The window has a fixed size: plugin hosts are told it can't be resized, and the standalone
window has no resize border or maximize button.

## Code layout

```
src/dsp.rs         oscillator, filter, envelope, note handling (audio thread)
src/params.rs      host-facing parameters
src/lib.rs         plugin glue: MIDI and processing
src/shared.rs      lock-free data from the audio thread to the GUI and back
src/ui/
  controls.rs      which controls exist, their roles, how values are formatted
  readout.rs       live values for display (scope trace, sounding note)
  input.rs         computer keyboard and on-screen keys to notes
  widgets.rs       mouse interaction (drag, scroll, click)
  layout.rs        rectangles, sized from the theme
  theme.rs         theme file format
  i18n.rs          language packs and number formatting
  assets.rs        loading built-in and user files, saved preferences
  render/          drawing, one file per element
```

The audio code never touches the UI, and UI code outside `render/` never picks a color, size or
string.

## Build the plugin

```sh
cargo xtask bundle simple_osc --release
```

This creates `target/bundled/Simple Osc.vst3` and `target/bundled/Simple Osc.clap`. To use it in
FL Studio, copy the `Simple Osc.vst3` folder into `C:\Program Files\Common Files\VST3`, then
rescan plugins in FL (Options → Manage plugins → Find installed plugins).

## Performance

Release builds are tuned for speed in the project's own settings (nothing system-wide):

- `Cargo.toml`: whole-program optimization (`lto = "fat"`, `codegen-units = 1`), no unwinding code.
- `.cargo/config.toml`: targets x86-64-v3 (AVX2 + FMA), so builds need a CPU from about 2015 or
  later. Delete those lines to build for any 64-bit x86 CPU.
- Audio: silence is a fast path that skips all DSP; pitch and filter coefficients are only
  recomputed when they change.
- GUI: only redraws continuously while a note is sounding, otherwise ~10 times a second. Only
  the two text fonts are bundled.

## Tests

```sh
cargo test --lib
```

## License

The source code is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.

The VST3 build links nih-plug's VST3 bindings ([vst3-sys](https://github.com/RustAudio/vst3-sys)),
which are GPLv3, so a distributed VST3 binary must follow GPLv3 terms. The CLAP build has no such
requirement.

### Third-party code and assets

| What | Where | License |
|---|---|---|
| Adventure Kid Waveforms, by Kristoffer Ekstrand | `assets/waves/akwf/` | [CC0 1.0](assets/waves/akwf/LICENSE.md) (public domain) |
| egui-baseview, patched | `vendor/egui-baseview/` | [MIT](vendor/egui-baseview/LICENSE) |
| nih-plug's egui adapter, patched | `vendor/nih_plug_egui/` | [ISC](vendor/nih_plug_egui/LICENSE) |
| [nih-plug](https://github.com/robbert-vdh/nih-plug) | Cargo dependency | ISC |
| Ubuntu Light font | bundled via `epaint_default_fonts` | Ubuntu Font Licence 1.0 |
| Hack font | bundled via `epaint_default_fonts` | MIT / Bitstream Vera |

The vendored folders list their changes in `PATCHES.md`.
