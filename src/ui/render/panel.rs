use nih_plug_egui::egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

use super::Canvas;
use crate::ui::theme::PanelStyle;

/// Draws a panel in the theme's panel style.
pub fn draw(c: &Canvas, rect: Rect) {
    match &c.theme.panel {
        PanelStyle::RivetedPlate {
            tint,
            shade,
            bevel_light,
            bevel_dark,
            rivet,
            rivet_highlight,
        } => {
            let p = c.painter;
            p.rect_filled(
                rect.translate(Vec2::new(2.0, 3.0)),
                2.0,
                Color32::from_black_alpha(140),
            );
            match c.texture {
                // Cut the plate from the same texture as the backdrop so the grain lines up
                Some(texture) => {
                    p.image(texture.id(), rect, uv_of(c.full, rect), (*tint).into());
                }
                None => {
                    p.rect_filled(rect, 0.0, *tint);
                }
            }
            p.rect_filled(rect.shrink(1.0), 0.0, *shade);
            let light = Stroke::new(1.0_f32, *bevel_light);
            let dark = Stroke::new(1.5_f32, *bevel_dark);
            p.hline(rect.x_range(), rect.top() + 0.5, light);
            p.vline(rect.left() + 0.5, rect.y_range(), light);
            p.hline(rect.x_range(), rect.bottom() - 0.5, dark);
            p.vline(rect.right() - 0.5, rect.y_range(), dark);
            p.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0_f32, Color32::from_black_alpha(200)),
                StrokeKind::Outside,
            );

            let inset = 7.0;
            for corner in [
                rect.left_top() + Vec2::new(inset, inset),
                rect.right_top() + Vec2::new(-inset, inset),
                rect.left_bottom() + Vec2::new(inset, -inset),
                rect.right_bottom() + Vec2::new(-inset, -inset),
            ] {
                p.circle_filled(
                    corner + Vec2::new(0.8, 1.0),
                    3.6,
                    Color32::from_black_alpha(150),
                );
                p.circle_filled(corner, 3.2, *rivet);
                p.circle_filled(corner - Vec2::new(0.9, 0.9), 1.3, *rivet_highlight);
                p.circle_stroke(
                    corner,
                    3.2,
                    Stroke::new(0.8_f32, Color32::from_black_alpha(180)),
                );
            }
        }
        PanelStyle::EnamelPlate {
            top,
            bottom,
            edge,
            silkscreen,
            screw,
            screw_slot,
        } => {
            let p = c.painter;
            p.rect_filled(
                rect.translate(Vec2::new(2.0, 3.0)),
                3.0,
                Color32::from_black_alpha(150),
            );
            super::background::vertical_gradient(c, rect, (*top).into(), (*bottom).into());
            p.hline(
                rect.shrink(1.0).x_range(),
                rect.top() + 1.0,
                Stroke::new(1.0_f32, Color32::from_white_alpha(60)),
            );
            p.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, *edge), StrokeKind::Inside);
            // Silkscreened border line, like the outline printed on a front panel
            p.rect_stroke(
                rect.shrink(4.0),
                2.0,
                Stroke::new(1.0_f32, *silkscreen),
                StrokeKind::Inside,
            );

            let inset = 8.0;
            for corner in [
                rect.left_top() + Vec2::new(inset, inset),
                rect.right_top() + Vec2::new(-inset, inset),
                rect.left_bottom() + Vec2::new(inset, -inset),
                rect.right_bottom() + Vec2::new(-inset, -inset),
            ] {
                // Phillips-head screw
                p.circle_filled(
                    corner + Vec2::new(0.6, 0.8),
                    3.4,
                    Color32::from_black_alpha(90),
                );
                p.circle_filled(corner, 3.2, *screw);
                p.circle_stroke(corner, 3.2, Stroke::new(0.8_f32, *screw_slot));
                let slot = Stroke::new(1.0_f32, *screw_slot);
                p.line_segment(
                    [corner - Vec2::new(1.8, 1.8), corner + Vec2::new(1.8, 1.8)],
                    slot,
                );
                p.line_segment(
                    [corner - Vec2::new(1.8, -1.8), corner + Vec2::new(1.8, -1.8)],
                    slot,
                );
            }
        }
        PanelStyle::NeonFrame {
            fill,
            border,
            glow,
            corner_cut,
        } => {
            let outline = cut_corners(rect, *corner_cut);
            c.painter
                .add(Shape::convex_polygon(outline.clone(), *fill, Stroke::NONE));
            for width in [6.0_f32, 3.0] {
                c.painter.add(Shape::closed_line(
                    outline.clone(),
                    Stroke::new(width, *glow),
                ));
            }
            c.painter
                .add(Shape::closed_line(outline, Stroke::new(1.0_f32, *border)));
        }
    }
}

/// Texture coordinates of `rect` within `full`.
pub fn uv_of(full: Rect, rect: Rect) -> Rect {
    let uv = |p: Pos2| {
        Pos2::new(
            (p.x - full.left()) / full.width(),
            (p.y - full.top()) / full.height(),
        )
    };
    Rect::from_min_max(uv(rect.min), uv(rect.max))
}

/// Rect outline with the top-left and bottom-right corners clipped.
fn cut_corners(r: Rect, cut: f32) -> Vec<Pos2> {
    vec![
        Pos2::new(r.left() + cut, r.top()),
        r.right_top(),
        Pos2::new(r.right(), r.bottom() - cut),
        Pos2::new(r.right() - cut, r.bottom()),
        r.left_bottom(),
        Pos2::new(r.left(), r.top() + cut),
    ]
}
