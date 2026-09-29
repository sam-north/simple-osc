# Themes, languages and keymaps

All are TOML files in `assets/`, compiled into the plugin:

```
assets/themes/grunge.toml       default
assets/themes/cyberpunk.toml
assets/themes/vintage.toml
assets/languages/en-US.toml     default, and the fallback for missing text
assets/languages/es.toml
assets/languages/fr.toml
assets/keymaps/qwerty.toml
```

Extra themes and languages load at runtime from `%APPDATA%\Simple Osc\themes\` and
`%APPDATA%\Simple Osc\languages\` the next time the window opens. Copy a built-in file and change
its `id`; a file with a built-in's `id` replaces it. Broken files are skipped with a log message.
The chosen theme and language are saved in `%APPDATA%\Simple Osc\prefs.toml`.

## Themes

A theme sets every color, font, size and spacing, and picks a drawing style per element:

| Element | Styles |
|---|---|
| Background | `rusted_metal`, `tolex`, `gradient` (optional grid), `solid` |
| Panels | `riveted_plate`, `enamel_plate`, `neon_frame` |
| Knobs | `iron`, `skirted`, `hex_bolt`, `chicken_head`, `neon_ring`, `neon_segments` |
| Title effect | `distressed`, `nameplate`, `chromatic`, `none` |

Continuous knobs use `[knobs.continuous]`; Wave, filter Type and other selectors use
`[knobs.selector]`. Recombining styles needs no code; a new style is one function in
`src/ui/render/` plus a variant in `src/ui/theme.rs`.

## Languages

A pack holds every visible string (with `{placeholders}`), the decimal separator and note naming
(`octave_offset`: -1 gives `C4`, -2 gives `Do3`). Units include their leading space, since
spacing differs by language. `cargo test` fails if a built-in pack is missing a key.

Adventure Kid wave names default to English; add `wave.akwf_<name>` keys to translate them.

## Keymaps

`qwerty.toml` maps egui key names to semitones above `base_note`. Keys are matched by physical
position, so the same file works on other layouts.
