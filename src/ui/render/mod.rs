//! Drawing only. Every function reads its colors, fonts and sizes from the active [`Theme`], and
//! gets the text to show already translated, so nothing here knows about parameters, languages or
//! specific looks beyond the drawing styles a theme can pick from.

pub mod background;
pub mod keyboard;
pub mod knob;
pub mod logo;
pub mod panel;
pub mod scope;
pub mod switch;
pub mod text;
pub mod texture;

use nih_plug_egui::egui::{Painter, Rect, TextureHandle};

use super::theme::Theme;

pub struct Canvas<'a> {
    pub painter: &'a Painter,
    pub theme: &'a Theme,
    /// The whole editor, used to line up texture regions with the background.
    pub full: Rect,
    /// The generated background texture, for themes that use one.
    pub texture: Option<&'a TextureHandle>,
    /// Chipped-paint overlay for distressed titles.
    pub flecks: Option<&'a TextureHandle>,
}
