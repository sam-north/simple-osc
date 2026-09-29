use nih_plug_egui::egui::{Align2, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::texture::rand01;
use super::{panel, Canvas};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Idle,
    /// Held on the computer keyboard or with the mouse.
    Held,
    /// The note the synth is playing.
    Sounding,
}

pub struct PianoKey {
    pub note: u8,
    pub rect: Rect,
    pub state: KeyState,
    /// The computer key that plays it, if any.
    pub label: Option<String>,
    /// Octave name, shown on C keys.
    pub octave_label: Option<String>,
}

pub fn draw(c: &Canvas, rect: Rect, whites: &[PianoKey], blacks: &[PianoKey]) {
    let s = &c.theme.keyboard;
    let p = c.painter;
    if s.panel {
        panel::draw(c, rect.expand(6.0));
    }
    let font = s.font.id();
    let fill = |state: KeyState, idle: Color32| match state {
        KeyState::Sounding => s.sounding.into(),
        KeyState::Held => s.held.into(),
        KeyState::Idle => idle,
    };

    for key in whites {
        let r = key.rect.shrink2(Vec2::new(1.0, 0.0));
        let label_color = if key.state == KeyState::Idle {
            s.label
        } else {
            s.active_label
        };
        p.rect_filled(r, s.corner_radius, fill(key.state, s.white.into()));
        if let Some(grime) = s.grime {
            // Aged, dirty keys: darker toward the bottom plus a few stains
            let bottom =
                Rect::from_min_max(Pos2::new(r.left(), r.bottom() - r.height() * 0.4), r.max);
            p.rect_filled(
                bottom,
                s.corner_radius,
                Color32::from(grime).gamma_multiply(0.8),
            );
            for i in 0..8 {
                let pos = Pos2::new(
                    r.left() + rand01(key.note as i32, i, 21) * r.width(),
                    r.top() + rand01(key.note as i32, i + 8, 21) * r.height(),
                );
                p.circle_filled(pos, 0.6 + rand01(key.note as i32, i + 16, 21) * 2.2, grime);
            }
        }
        if let Some(outline) = s.outline {
            p.rect_stroke(
                r,
                s.corner_radius,
                Stroke::new(1.0_f32, outline),
                StrokeKind::Inside,
            );
        }
        if let Some(label) = &key.label {
            let pos = Pos2::new(r.center().x, r.bottom() - font.size - 1.0);
            p.text(
                pos,
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                label_color.into(),
            );
        }
        if let Some(octave) = &key.octave_label {
            let pos = Pos2::new(r.center().x, r.bottom() - font.size * 2.6);
            p.text(
                pos,
                Align2::CENTER_CENTER,
                octave,
                font.clone(),
                s.octave_label.into(),
            );
        }
    }

    for key in blacks {
        let r = key.rect;
        let label_color = if key.state == KeyState::Idle {
            s.black_label
        } else {
            s.active_label
        };
        p.rect_filled(
            r.translate(Vec2::new(1.5, 2.0)),
            s.corner_radius,
            Color32::from_black_alpha(120),
        );
        p.rect_filled(r, s.corner_radius, fill(key.state, s.black.into()));
        if let Some(outline) = s.outline {
            p.rect_stroke(
                r,
                s.corner_radius,
                Stroke::new(1.0_f32, outline),
                StrokeKind::Inside,
            );
        } else {
            p.hline(
                r.shrink(2.0).x_range(),
                r.bottom() - 4.0,
                Stroke::new(1.0_f32, Color32::from_white_alpha(18)),
            );
        }
        if let Some(label) = &key.label {
            let pos = Pos2::new(r.center().x, r.bottom() - font.size - 1.0);
            p.text(
                pos,
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                label_color.into(),
            );
        }
    }
}
