# Simple Osc

A single-oscillator mono synth for Windows, as a 64-bit **VST3**, **CLAP** or **standalone app**.

- 25 waveforms: classic shapes, supersaw, noise, phase distortion, wavefolding and 16 sampled
  single-cycle waves
- Resonant filter (low pass, high pass, band pass, notch), ADSR envelope, octave and tuning
- Live oscilloscope
- Themes (Grunge, Cyberpunk, Vintage) and languages (English, Español, Français)

## Build

Needs [Rust](https://rustup.rs) and the Visual Studio C++ Build Tools.

```sh
cargo xtask bundle simple_osc --release
```

Copy `target/bundled/Simple Osc.vst3` into `C:\Program Files\Common Files\VST3` and rescan
plugins in your DAW.

To try it without a DAW, run `cargo run --release`.

## Playing

- **MIDI keyboard:** connected automatically in the standalone app.
- **Computer keyboard:** click the window, then `A`–`L` are the white keys and `W E T Y U O P`
  the black keys.
- **Knobs:** drag or scroll; Shift for fine control; double-click to reset. Wave and filter Type
  turn endlessly: click for next, right-click for previous.
- **Theme and language:** click the labels in the top-right corner.

## License

MIT or Apache-2.0, at your option. The VST3 build uses GPLv3 bindings, so a distributed VST3
binary must follow GPLv3. Third-party credits: [docs/third-party.md](docs/third-party.md).

Developer notes are in [docs/](docs/development.md).
