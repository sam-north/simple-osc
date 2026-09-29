use nih_plug_egui::egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke};

use super::Canvas;
use crate::ui::theme::{Background, GridStyle};

pub fn draw(c: &Canvas) {
    let full = c.full;
    match &c.theme.background {
        Background::Solid { color } => {
            c.painter.rect_filled(full, 0.0, *color);
        }
        Background::Tolex { .. } => {
            if let Some(texture) = c.texture {
                c.painter
                    .image(texture.id(), full, unit_uv(), Color32::WHITE);
            }
        }
        Background::RustedMetal { tint, .. } => match c.texture {
            Some(texture) => {
                c.painter
                    .image(texture.id(), full, unit_uv(), (*tint).into());
            }
            None => {
                c.painter.rect_filled(full, 0.0, *tint);
            }
        },
        Background::Gradient { top, bottom, grid } => {
            vertical_gradient(c, full, (*top).into(), (*bottom).into());
            if let Some(grid) = grid {
                perspective_grid(c, full, grid);
            }
        }
    }
}

pub fn unit_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
}

pub(super) fn vertical_gradient(c: &Canvas, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    c.painter.add(Shape::mesh(mesh));
}

/// A floor grid receding to a glowing horizon.
fn perspective_grid(c: &Canvas, rect: Rect, grid: &GridStyle) {
    let horizon_y = rect.top() + rect.height() * grid.horizon;
    let vanish = Pos2::new(rect.center().x, horizon_y);
    let line = Stroke::new(1.0_f32, grid.color);
    let depth = rect.bottom() - horizon_y;

    // Lines fanning out from the vanishing point
    let spread = rect.width() * 3.0;
    for i in 0..=grid.columns {
        let t = i as f32 / grid.columns.max(1) as f32 - 0.5;
        let bottom = Pos2::new(vanish.x + t * spread, rect.bottom());
        c.painter.line_segment([vanish, bottom], line);
    }
    // Rows bunch up toward the horizon
    for i in 1..=grid.rows {
        let t = i as f32 / grid.rows.max(1) as f32;
        let y = horizon_y + depth * t * t;
        c.painter.hline(rect.x_range(), y, line);
    }

    // Horizon glow
    let glow: Color32 = grid.horizon_color.into();
    for (offset, alpha) in [(0.0, 1.0), (1.5, 0.35), (4.0, 0.15), (9.0, 0.06)] {
        let stroke = Stroke::new(1.0_f32 + offset, glow.gamma_multiply(alpha));
        c.painter.hline(rect.x_range(), horizon_y, stroke);
    }
}
