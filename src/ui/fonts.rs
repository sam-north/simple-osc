//! The fonts the GUI uses: Ubuntu Light (proportional) and Hack (monospace). egui's own defaults
//! also bundle two emoji fonts (~1 MB) that nothing here uses, so they're left out.

use nih_plug_egui::egui::{Context, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const PROPORTIONAL: &str = "Ubuntu-Light";
const MONOSPACE: &str = "Hack";

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert(
        PROPORTIONAL.to_owned(),
        Arc::new(FontData::from_static(epaint_default_fonts::UBUNTU_LIGHT)),
    );
    fonts.font_data.insert(
        MONOSPACE.to_owned(),
        Arc::new(FontData::from_static(epaint_default_fonts::HACK_REGULAR)),
    );
    // Each family falls back to the other for any character it lacks
    fonts.families.insert(
        FontFamily::Proportional,
        vec![PROPORTIONAL.to_owned(), MONOSPACE.to_owned()],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![MONOSPACE.to_owned(), PROPORTIONAL.to_owned()],
    );
    ctx.set_fonts(fonts);
}
