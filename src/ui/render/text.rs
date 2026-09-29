use nih_plug_egui::egui::{
    text::LayoutJob, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, TextFormat,
    Vec2,
};

use super::Canvas;
use crate::ui::theme::{Font, TitleEffect};

/// Text with an optional drop shadow.
pub fn shadowed(
    painter: &Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    shadow: Option<Color32>,
) -> Rect {
    if let Some(shadow) = shadow {
        painter.text(
            pos + Vec2::new(1.0, 1.0),
            anchor,
            text,
            font.clone(),
            shadow,
        );
    }
    painter.text(pos, anchor, text, font, color)
}

/// Letter-spaced text with an optional drop shadow. Returns its rect.
pub fn spaced(
    painter: &Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
    shadow: Option<Color32>,
) -> Rect {
    let job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    let rect = anchor.anchor_size(pos, galley.size());
    if let Some(shadow) = shadow {
        painter.galley_with_override_text_color(
            rect.min + Vec2::new(1.5, 1.5),
            galley.clone(),
            shadow,
        );
    }
    painter.galley(rect.min, galley, color);
    rect
}

/// The big title, with the theme's title effect.
pub fn title(c: &Canvas, pos: Pos2, anchor: Align2, text: &str) {
    let style = &c.theme.title;
    let font = style.font.id();
    let shadow = c.theme.text.shadow.map(Into::into);
    match &style.effect {
        TitleEffect::None => {
            spaced(
                c.painter,
                pos,
                anchor,
                text,
                font,
                style.color.into(),
                style.spacing,
                shadow,
            );
        }
        TitleEffect::Distressed { color } => {
            let rect = spaced(
                c.painter,
                pos,
                anchor,
                text,
                font,
                style.color.into(),
                style.spacing,
                shadow,
            );
            // Chip paint off the letters with a pre-rendered fleck overlay: one quad per frame
            if let Some(flecks) = c.flecks {
                c.painter.image(
                    flecks.id(),
                    rect,
                    super::background::unit_uv(),
                    (*color).into(),
                );
            }
        }
        TitleEffect::Nameplate {
            top,
            bottom,
            edge,
            screw,
        } => {
            // Lay the text out first so the plate can be sized around it
            let job = LayoutJob::single_section(
                text.to_owned(),
                TextFormat {
                    font_id: font,
                    color: style.color.into(),
                    extra_letter_spacing: style.spacing,
                    ..Default::default()
                },
            );
            let galley = c.painter.layout_job(job);
            let text_rect = anchor.anchor_size(pos, galley.size());
            let pad = Vec2::new(style.font.size * 0.9, style.font.size * 0.3);
            let plate = text_rect.expand2(pad).translate(Vec2::new(pad.x, 0.0));
            let p = c.painter;
            p.rect_filled(
                plate.translate(Vec2::new(1.5, 2.0)),
                3.0,
                Color32::from_black_alpha(140),
            );
            super::background::vertical_gradient(c, plate, (*top).into(), (*bottom).into());
            p.hline(
                plate.shrink(1.0).x_range(),
                plate.top() + 1.0,
                Stroke::new(1.0_f32, Color32::from_white_alpha(120)),
            );
            p.rect_stroke(plate, 3.0, Stroke::new(1.0_f32, *edge), StrokeKind::Inside);
            for x in [plate.left() + pad.x * 0.45, plate.right() - pad.x * 0.45] {
                let at = Pos2::new(x, plate.center().y);
                p.circle_filled(at, 2.2, *screw);
                p.circle_stroke(at, 2.2, Stroke::new(0.8_f32, *edge));
            }
            // Engraved look: a light line under dark lettering
            p.galley_with_override_text_color(
                text_rect.min + Vec2::new(pad.x, 1.0),
                galley.clone(),
                Color32::from_white_alpha(90),
            );
            p.galley(
                text_rect.min + Vec2::new(pad.x, 0.0),
                galley,
                style.color.into(),
            );
        }
        TitleEffect::Chromatic {
            left,
            right,
            offset,
        } => {
            let o = Vec2::new(*offset, 0.0);
            spaced(
                c.painter,
                pos - o,
                anchor,
                text,
                font.clone(),
                (*left).into(),
                style.spacing,
                None,
            );
            spaced(
                c.painter,
                pos + o,
                anchor,
                text,
                font.clone(),
                (*right).into(),
                style.spacing,
                None,
            );
            spaced(
                c.painter,
                pos,
                anchor,
                text,
                font,
                style.color.into(),
                style.spacing,
                None,
            );
        }
    }
}

/// Small text that only takes the space it needs, used by the theme/language switchers.
pub fn measure(painter: &Painter, text: &str, font: Font) -> Vec2 {
    painter
        .layout_no_wrap(text.to_owned(), font.id(), Color32::WHITE)
        .size()
}
