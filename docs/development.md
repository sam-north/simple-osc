# Development

## Commands

```sh
cargo run --release                      # standalone app
cargo run --release -- --no-midi         # without connecting a MIDI keyboard
cargo run --release -- --midi-input ""   # list MIDI inputs (pass a name to pick one)
cargo run --release -- --help            # all audio and MIDI options
cargo test --lib                         # tests
cargo xtask bundle simple_osc --release  # target/bundled/Simple Osc.vst3 and .clap
```

Debug builds (`cargo run`) abort if the audio thread allocates memory (`assert_process_allocs`),
and log every computer-key press to the console.

If the standalone is already running, `cargo build` can't replace its `.exe`. Close it, or build
elsewhere with `--target-dir target/verify`.

## Docs

- [architecture.md](architecture.md): code layout, rules and non-obvious decisions
- [themes-and-languages.md](themes-and-languages.md): theme, language pack and keymap files
- [performance.md](performance.md): build settings and runtime optimizations
- [third-party.md](third-party.md): bundled code and assets, and their licenses
