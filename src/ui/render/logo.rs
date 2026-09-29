use nih_plug_egui::egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};

use super::Canvas;
use crate::logo;

/// Draws the logo with its left edge centered on `left_center`, and returns the space it used.
pub fn draw(c: &Canvas, left_center: Pos2) -> Rect {
    let s = &c.theme.logo;
    let h = s.height;
    let origin = left_center - Vec2::new(0.0, h / 2.0);
    let to_screen = |points: &[(f32, f32)]| -> Vec<Pos2> {
        points
            .iter()
            .map(|&(x, y)| origin + Vec2::new(x, y) * h)
            .collect()
    };
    let strokes = [
        (to_screen(&logo::letter_s()), s.letter),
        (to_screen(&logo::sideways_s()), s.wave),
    ];
    let width = logo::STROKE * h;
    if let Some(glow) = s.glow {
        let glow: Color32 = glow.into();
        for (points, _) in &strokes {
            c.painter
                .add(Shape::line(points.clone(), Stroke::new(width * 2.6, glow)));
        }
    }
    for (points, color) in strokes {
        c.painter
            .add(Shape::line(points, Stroke::new(width, color)));
    }
    Rect::from_min_size(origin, Vec2::new(logo::WIDTH * h, h))
}
