//! Theme data model. A theme describes every visual choice (colors, fonts, sizes, which drawing
//! style each element uses) and is loaded from a TOML file, so new looks need no code changes
//! unless they introduce a new drawing style. See `assets/themes/` for the built-in themes.

use nih_plug_egui::egui::{Color32, FontFamily, FontId};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub metrics: Metrics,
    pub background: Background,
    pub text: TextStyle,
    pub title: TitleStyle,
    pub logo: LogoStyle,
    pub panel: PanelStyle,
    pub knobs: KnobTheme,
    pub switch: SwitchStyle,
    pub scope: ScopeStyle,
    pub keyboard: KeyboardStyle,
    pub switcher: SwitcherStyle,
}

/// Layout sizes, in logical pixels.
#[derive(Debug, Clone, Deserialize)]
pub struct Metrics {
    pub margin: f32,
    pub header_height: f32,
    pub scope_height: f32,
    pub section_height: f32,
    pub section_gap: f32,
    pub section_padding: f32,
    pub section_title_height: f32,
    pub section_title_indent: f32,
    /// Height of the strip above each knob for attached controls like the octave switch.
    pub attachment_height: f32,
    pub keyboard_label_height: f32,
    /// How many octaves the on-screen keyboard shows (plus the top C).
    pub keyboard_octaves: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum Background {
    Solid {
        color: Color,
    },
    /// Procedurally generated corroded metal. The palette drives the generator.
    RustedMetal {
        tint: Color,
        palette: RustPalette,
    },
    /// Procedurally generated amp-style tolex: a fine pebbled vinyl grain.
    Tolex {
        base: Color,
        grain: Color,
        highlight: Color,
    },
    /// Vertical gradient with an optional perspective grid toward a horizon.
    Gradient {
        top: Color,
        bottom: Color,
        grid: Option<GridStyle>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RustPalette {
    pub iron: Color,
    pub rust: Color,
    pub rust_bright: Color,
    pub stain: Color,
    pub patina_dark: Color,
    pub patina_light: Color,
    pub scratch: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GridStyle {
    pub color: Color,
    pub horizon_color: Color,
    /// Horizon height as a fraction of the window height.
    pub horizon: f32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextStyle {
    pub muted: Color,
    pub shadow: Option<Color>,
    pub label: Font,
    pub value: Font,
    pub section: Font,
    pub section_spacing: f32,
    pub hint: Font,
    pub subtitle: Font,
    pub subtitle_spacing: f32,
    /// Color for text drawn straight on the background (subtitle, keyboard label and hint) when
    /// it needs to differ from `muted`, which is used on panels.
    #[serde(default)]
    pub on_background: Option<Color>,
    /// Case transforms, since some looks want labels in capitals.
    pub uppercase_labels: bool,
    pub uppercase_sections: bool,
    pub uppercase_title: bool,
    pub uppercase_info: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TitleStyle {
    pub font: Font,
    pub color: Color,
    pub spacing: f32,
    pub effect: TitleEffect,
}

/// The "SS" logo in the header: an S, then the same S on its side as a sine wave.
#[derive(Debug, Clone, Deserialize)]
pub struct LogoStyle {
    pub height: f32,
    /// Space between the logo and the title.
    pub gap: f32,
    /// The upright S.
    pub letter: Color,
    /// The sideways S (the sine wave).
    pub wave: Color,
    /// Soft halo behind both strokes, if any.
    #[serde(default)]
    pub glow: Option<Color>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum TitleEffect {
    None,
    /// Paint chipped off the letters.
    Distressed {
        color: Color,
    },
    /// Offset colored copies, like a glitching screen.
    Chromatic {
        left: Color,
        right: Color,
        offset: f32,
    },
    /// Text on a raised metal nameplate, like an amp logo badge.
    Nameplate {
        top: Color,
        bottom: Color,
        edge: Color,
        screw: Color,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum PanelStyle {
    /// A bolted-on metal plate cut from the background texture.
    RivetedPlate {
        tint: Color,
        shade: Color,
        bevel_light: Color,
        bevel_dark: Color,
        rivet: Color,
        rivet_highlight: Color,
    },
    /// A painted enamel face plate with a silkscreened border and screws in the corners.
    EnamelPlate {
        top: Color,
        bottom: Color,
        edge: Color,
        silkscreen: Color,
        screw: Color,
        screw_slot: Color,
    },
    /// Dark translucent panel with a glowing border and one clipped corner.
    NeonFrame {
        fill: Color,
        border: Color,
        glow: Color,
        corner_cut: f32,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnobTheme {
    pub radius: f32,
    /// Total rotation range of a knob, in degrees.
    pub sweep_degrees: f32,
    /// Detents around an endless knob (it turns one detent per step).
    pub endless_detents: u32,
    /// Space between the top of a control slot and the knob.
    pub top_margin: f32,
    /// Distance from the knob edge to the label, and from the label to the value.
    pub label_offset: f32,
    pub value_offset: f32,
    /// Used for continuous controls (cutoff, attack, ...).
    pub continuous: KnobStyle,
    /// Used for controls that pick one of several choices (waveform, filter type).
    pub selector: KnobStyle,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnobShape {
    /// Knurled iron knob with a value arc.
    Iron,
    /// Rotating hex bolt with position markers.
    HexBolt,
    /// Flat disc inside a glowing ring.
    NeonRing,
    /// Ring of discrete segments, one lit per choice.
    NeonSegments,
    /// Moog-style knob: a knurled cap on a metal skirt, with an optional printed scale.
    Skirted,
    /// Amp-style pointer knob.
    ChickenHead,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnobStyle {
    pub shape: KnobShape,
    /// Print a 0-5-10 scale (or - 0 + for centered knobs) around the knob in this font, if the
    /// shape supports it.
    #[serde(default)]
    pub scale: Option<Font>,
    pub body: Color,
    pub rim: Color,
    pub face: Color,
    pub highlight: Color,
    pub pointer: Color,
    pub track: Color,
    pub arc: Color,
    pub arc_active: Color,
    pub marker: Color,
    pub marker_active: Color,
    /// Small blemishes on the knob face, if any.
    pub speckle: Option<Color>,
    pub speckle_alt: Option<Color>,
    pub label: Color,
    pub value: Color,
    pub value_active: Color,
    pub shadow: Option<Color>,
}

/// A small left-to-right position switch, like the octave switch.
#[derive(Debug, Clone, Deserialize)]
pub struct SwitchStyle {
    pub width: f32,
    pub track: Color,
    pub track_edge: Color,
    pub thumb: Color,
    pub thumb_active: Color,
    pub thumb_edge: Color,
    /// Soft halo around the thumb, if any.
    pub glow: Option<Color>,
    pub tick: Color,
    pub tick_active: Color,
    pub font: Font,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BezelShape {
    Panel,
    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScopeStyle {
    pub bezel: BezelShape,
    pub bezel_inset: f32,
    pub glass: Color,
    pub glass_edge: Color,
    pub inner_shadow: Color,
    pub grid: Color,
    pub grid_center: Color,
    pub grid_columns: u32,
    pub trace: Color,
    pub trace_width: f32,
    /// 0 disables the glow.
    pub glow: f32,
    pub scanlines: Option<Color>,
    pub info: Color,
    pub note: Color,
    pub font: Font,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyboardStyle {
    pub panel: bool,
    pub white: Color,
    pub black: Color,
    pub held: Color,
    pub sounding: Color,
    pub outline: Option<Color>,
    pub grime: Option<Color>,
    pub label: Color,
    pub black_label: Color,
    /// Label color on a held or sounding key.
    pub active_label: Color,
    pub octave_label: Color,
    pub font: Font,
    pub corner_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwitcherStyle {
    pub text: Color,
    pub hover: Color,
    pub font: Font,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Font {
    pub family: Family,
    pub size: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Proportional,
    Monospace,
}

impl Font {
    pub fn id(&self) -> FontId {
        let family = match self.family {
            Family::Proportional => FontFamily::Proportional,
            Family::Monospace => FontFamily::Monospace,
        };
        FontId::new(self.size, family)
    }
}

/// A color written as `"#rrggbb"` or `"#rrggbbaa"` in theme files.
#[derive(Debug, Clone, Copy)]
pub struct Color(pub Color32);

impl Color {
    pub fn parse(s: &str) -> Option<Self> {
        let hex = s.strip_prefix('#')?;
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        match hex.len() {
            6 => Some(Self(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))),
            8 => Some(Self(Color32::from_rgba_unmultiplied(
                byte(0)?,
                byte(2)?,
                byte(4)?,
                byte(6)?,
            ))),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Color::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid color '{s}'")))
    }
}

impl From<Color> for Color32 {
    fn from(c: Color) -> Self {
        c.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(
            Color::parse("#ff8000").unwrap().0,
            Color32::from_rgb(255, 128, 0)
        );
        assert_eq!(
            Color::parse("#00000080").unwrap().0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 128)
        );
        assert!(Color::parse("ff8000").is_none());
        assert!(Color::parse("#ff80").is_none());
    }
}
