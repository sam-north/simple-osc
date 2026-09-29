use nih_plug_egui::egui::{Align2, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::Canvas;

pub struct SwitchView<'a> {
    pub area: Rect,
    /// x coordinate of each position.
    pub positions: &'a [f32],
    pub labels: &'a [String],
    pub selected: usize,
    pub active: bool,
}

/// Position labels on top, a slotted track below, and a thumb sitting at the selected position.
pub fn draw(c: &Canvas, v: &SwitchView) {
    let s = &c.theme.switch;
    let p = c.painter;
    let (Some(&first), Some(&last)) = (v.positions.first(), v.positions.last()) else {
        return;
    };

    let font = s.font.id();
    let label_y = v.area.top() + font.size * 0.7;
    let shadow = c.theme.text.shadow;
    for (i, (&x, label)) in v.positions.iter().zip(v.labels).enumerate() {
        let color = if i == v.selected {
            s.tick_active
        } else {
            s.tick
        };
        if let Some(shadow) = shadow {
            p.text(
                Pos2::new(x + 1.0, label_y + 1.0),
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                shadow.into(),
            );
        }
        p.text(
            Pos2::new(x, label_y),
            Align2::CENTER_CENTER,
            label,
            font.clone(),
            color.into(),
        );
    }

    let track_h = 6.0;
    let track_y = v.area.bottom() - track_h / 2.0 - 3.0;
    let track = Rect::from_min_max(
        Pos2::new(first - 7.0, track_y - track_h / 2.0),
        Pos2::new(last + 7.0, track_y + track_h / 2.0),
    );
    p.rect_filled(track, track_h / 2.0, s.track);
    p.rect_stroke(
        track,
        track_h / 2.0,
        Stroke::new(1.0_f32, s.track_edge),
        StrokeKind::Outside,
    );
    for &x in v.positions {
        p.circle_filled(Pos2::new(x, track_y), 1.2, s.track_edge);
    }

    let x = v.positions[v.selected.min(v.positions.len() - 1)];
    let thumb = Rect::from_center_size(Pos2::new(x, track_y), Vec2::new(12.0, 10.0));
    if let Some(glow) = s.glow {
        let glow: Color32 = glow.into();
        p.rect_filled(
            thumb.expand(3.0),
            4.0,
            glow.gamma_multiply(if v.active { 1.0 } else { 0.6 }),
        );
    }
    let fill = if v.active { s.thumb_active } else { s.thumb };
    p.rect_filled(thumb, 2.0, fill);
    p.rect_stroke(
        thumb,
        2.0,
        Stroke::new(1.0_f32, s.thumb_edge),
        StrokeKind::Inside,
    );
    p.vline(
        x,
        thumb.shrink(3.0).y_range(),
        Stroke::new(1.0_f32, s.thumb_edge),
    );
}
