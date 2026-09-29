use nih_plug_egui::egui::{Align2, Color32, Pos2, Rect, Shape, Stroke, StrokeKind};

use super::{panel, Canvas};
use crate::ui::theme::BezelShape;

/// `trace` holds evenly spaced samples in -1..1 across the screen, if anything has played.
pub fn draw(c: &Canvas, rect: Rect, trace: Option<&[f32]>, info: &str, note: Option<&str>) {
    let s = &c.theme.scope;
    let p = c.painter;
    if s.bezel == BezelShape::Panel {
        panel::draw(c, rect);
    }
    let glass = rect.shrink(s.bezel_inset);
    p.rect_filled(glass, 3.0, s.glass);

    let plot = glass.shrink(8.0);
    for i in 1..s.grid_columns {
        let x = plot.left() + plot.width() * i as f32 / s.grid_columns as f32;
        p.vline(x, plot.y_range(), Stroke::new(1.0_f32, s.grid));
    }
    for y in [
        plot.top() + plot.height() * 0.25,
        plot.bottom() - plot.height() * 0.25,
    ] {
        p.hline(plot.x_range(), y, Stroke::new(1.0_f32, s.grid));
    }
    p.hline(
        plot.x_range(),
        plot.center().y,
        Stroke::new(1.0_f32, s.grid_center),
    );

    let half_height = plot.height() * 0.5 * 0.9;
    let points: Vec<Pos2> = match trace {
        Some(samples) if samples.len() > 1 => {
            let last = (samples.len() - 1) as f32;
            samples
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    Pos2::new(
                        plot.left() + plot.width() * i as f32 / last,
                        plot.center().y - v.clamp(-1.1, 1.1) * half_height,
                    )
                })
                .collect()
        }
        _ => vec![plot.left_center(), plot.right_center()],
    };

    // Layered strokes fake a phosphor glow
    let trace_color: Color32 = s.trace.into();
    if s.glow > 0.0 {
        p.add(Shape::line(
            points.clone(),
            Stroke::new(
                s.trace_width * 5.0,
                trace_color.gamma_multiply(0.07 * s.glow),
            ),
        ));
        p.add(Shape::line(
            points.clone(),
            Stroke::new(
                s.trace_width * 2.2,
                trace_color.gamma_multiply(0.22 * s.glow),
            ),
        ));
    }
    p.add(Shape::line(points, Stroke::new(s.trace_width, trace_color)));

    if let Some(scanline) = s.scanlines {
        let mut y = glass.top() + 1.0;
        while y < glass.bottom() {
            p.hline(glass.x_range(), y, Stroke::new(1.0_f32, scanline));
            y += 3.0;
        }
    }
    p.rect_stroke(
        glass,
        3.0,
        Stroke::new(3.0_f32, s.inner_shadow),
        StrokeKind::Inside,
    );
    p.rect_stroke(
        glass,
        3.0,
        Stroke::new(1.0_f32, s.glass_edge),
        StrokeKind::Outside,
    );

    let font = s.font.id();
    let line_y = glass.top() + font.size + 1.0;
    p.text(
        Pos2::new(glass.left() + 10.0, line_y),
        Align2::LEFT_CENTER,
        info,
        font.clone(),
        s.info.into(),
    );
    if let Some(note) = note {
        p.text(
            Pos2::new(glass.right() - 10.0, line_y),
            Align2::RIGHT_CENTER,
            note,
            font,
            s.note.into(),
        );
    }
}
