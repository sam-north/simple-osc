# Third-party code and assets

| What | Where | License |
|---|---|---|
| [nih-plug](https://github.com/robbert-vdh/nih-plug) | Cargo dependency | ISC |
| [vst3-sys](https://github.com/RustAudio/vst3-sys) VST3 bindings | via nih-plug | GPLv3 |
| Adventure Kid Waveforms, by Kristoffer Ekstrand | `assets/waves/akwf/` | [CC0 1.0](../assets/waves/akwf/LICENSE.md) |
| egui-baseview, patched | `vendor/egui-baseview/` | [MIT](../vendor/egui-baseview/LICENSE) |
| nih-plug's egui adapter, patched | `vendor/nih_plug_egui/` | [ISC](../vendor/nih_plug_egui/LICENSE) |
| Ubuntu Light font | bundled via `epaint_default_fonts` | Ubuntu Font Licence 1.0 |
| Hack font | bundled via `epaint_default_fonts` | MIT / Bitstream Vera |

Because of vst3-sys, a distributed VST3 binary must follow GPLv3. The CLAP build and the source
code (MIT or Apache-2.0) have no such requirement.
