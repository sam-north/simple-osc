use nih_plug_egui::egui::{Align2, Color32, FontId, Pos2, Shape, Stroke, Vec2};
use std::f32::consts::PI;

use super::text;
use super::texture::rand01;
use super::Canvas;
use crate::ui::theme::{KnobShape, KnobStyle};

pub struct KnobView<'a> {
    pub style: &'a KnobStyle,
    pub center: Pos2,
    pub radius: f32,
    /// Normalized value, 0..1.
    pub value: f32,
    /// Draw the value arc from the middle instead of the start.
    pub bipolar: bool,
    /// Number of choices minus one, for selectors.
    pub steps: Option<usize>,
    /// For endless knobs: the position in the browsing order. The knob turns one detent per
    /// step, without end stops.
    pub endless: Option<usize>,
    pub active: bool,
    /// Varies the per-knob blemishes so knobs don't all look identical.
    pub seed: u32,
}

impl KnobView<'_> {
    /// Angle of detent `i` on an endless knob, starting straight up.
    fn detent_angle(&self, detents: u32, i: f32) -> f32 {
        -PI / 2.0 + i * std::f32::consts::TAU / detents.max(1) as f32
    }

    fn angle(&self, sweep: f32, normalized: f32) -> f32 {
        // The sweep is centered on "straight up"
        -PI / 2.0 - sweep / 2.0 + normalized * sweep
    }
}

pub fn draw(c: &Canvas, k: &KnobView) {
    let sweep = c.theme.knobs.sweep_degrees.to_radians();
    // Round knobs share a round drop shadow; the chicken-head casts one shaped like itself
    if let (Some(shadow), false) = (k.style.shadow, k.style.shape == KnobShape::ChickenHead) {
        c.painter
            .circle_filled(k.center + Vec2::new(2.0, 3.0), k.radius + 2.0, shadow);
    }
    match k.style.shape {
        KnobShape::Iron => iron(c, k, sweep),
        KnobShape::HexBolt => hex_bolt(c, k, sweep),
        KnobShape::NeonRing => neon_ring(c, k, sweep),
        KnobShape::NeonSegments => neon_segments(c, k, sweep),
        KnobShape::Skirted => skirted(c, k, sweep),
        KnobShape::ChickenHead => chicken_head(c, k, sweep),
    }
}

/// Label under the knob, then its value. `dim` fades both, for a control that does nothing
/// right now.
pub fn draw_text(c: &Canvas, k: &KnobView, label: &str, value: &str, dim: bool) {
    let fade = |color: Color32| {
        if dim {
            color.gamma_multiply(0.45)
        } else {
            color
        }
    };
    let knobs = &c.theme.knobs;
    let t = &c.theme.text;
    let shadow = t.shadow.map(Into::into);
    let label_pos = Pos2::new(k.center.x, k.center.y + k.radius + knobs.label_offset);
    text::shadowed(
        c.painter,
        label_pos,
        Align2::CENTER_CENTER,
        label,
        t.label.id(),
        fade(k.style.label.into()),
        shadow,
    );
    let color = if k.active {
        k.style.value_active
    } else {
        k.style.value
    };
    let value_pos = label_pos + Vec2::new(0.0, knobs.value_offset);
    text::shadowed(
        c.painter,
        value_pos,
        Align2::CENTER_CENTER,
        value,
        t.value.id(),
        fade(color.into()),
        shadow,
    );
}

fn arc(center: Pos2, radius: f32, from: f32, to: f32, stroke: Stroke) -> Shape {
    let segments = ((to - from).abs() * 12.0).ceil().max(2.0) as usize;
    let points = (0..=segments)
        .map(|i| center + Vec2::angled(from + (to - from) * i as f32 / segments as f32) * radius)
        .collect();
    Shape::line(points, stroke)
}

/// The value arc around the knob, glowing while hovered or dragged.
fn value_arc(c: &Canvas, k: &KnobView, sweep: f32, radius: f32, width: f32, glow: f32) {
    let s = k.style;
    c.painter.add(arc(
        k.center,
        radius,
        k.angle(sweep, 0.0),
        k.angle(sweep, 1.0),
        Stroke::new(width, s.track),
    ));
    let from = if k.bipolar { 0.5 } else { 0.0 };
    if (k.value - from).abs() < 1e-4 {
        return;
    }
    let (a, b) = (
        k.angle(sweep, from.min(k.value)),
        k.angle(sweep, from.max(k.value)),
    );
    let color: Color32 = if k.active {
        s.arc_active.into()
    } else {
        s.arc.into()
    };
    if glow > 0.0 {
        c.painter.add(arc(
            k.center,
            radius,
            a,
            b,
            Stroke::new(width * 2.6, color.gamma_multiply(0.18 * glow)),
        ));
    }
    c.painter
        .add(arc(k.center, radius, a, b, Stroke::new(width, color)));
}

fn speckles(c: &Canvas, k: &KnobView, count: i32, spread: f32, salt: u32) {
    let s = k.style;
    for i in 0..count {
        let Some(color) = (if i % 3 == 0 {
            s.speckle_alt.or(s.speckle)
        } else {
            s.speckle
        }) else {
            return;
        };
        let a = rand01(i, 0, k.seed ^ salt) * 2.0 * PI;
        let d = rand01(i, 1, k.seed ^ salt).sqrt() * spread;
        let size = 0.5 + rand01(i, 2, k.seed ^ salt) * 1.6;
        c.painter
            .circle_filled(k.center + Vec2::angled(a) * d, size, color);
    }
}

fn iron(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    value_arc(c, k, sweep, r + 6.0, 3.5, 1.0);

    // Knurled edge that turns with the knob
    let turn = k.angle(sweep, k.value);
    p.circle_filled(k.center, r, s.rim);
    for i in 0..30 {
        let dir = Vec2::angled(turn + i as f32 * (2.0 * PI / 30.0));
        p.line_segment(
            [k.center + dir * (r - 3.5), k.center + dir * r],
            Stroke::new(1.2_f32, s.body),
        );
    }
    p.circle_stroke(
        k.center,
        r,
        Stroke::new(1.0_f32, Color32::from_black_alpha(200)),
    );

    // Cap with a soft highlight
    p.circle_filled(k.center, r - 4.0, s.face);
    p.circle_filled(k.center - Vec2::new(1.2, 1.8), r - 7.0, s.highlight);
    p.circle_filled(k.center, r - 8.0, s.face);
    speckles(c, k, 7, r - 7.0, 0);

    let dir = Vec2::angled(turn);
    p.line_segment(
        [k.center + dir * (r * 0.25), k.center + dir * (r - 5.0)],
        Stroke::new(2.5_f32, s.pointer),
    );
}

fn step_markers(c: &Canvas, k: &KnobView, sweep: f32, radius: f32) {
    let steps = k.steps.unwrap_or(1).max(1);
    let current = (k.value * steps as f32).round() as usize;
    let s = k.style;
    for i in 0..=steps {
        let p = k.center + Vec2::angled(k.angle(sweep, i as f32 / steps as f32)) * radius;
        if i == current {
            c.painter
                .circle_filled(p, 5.0, Color32::from(s.marker_active).gamma_multiply(0.2));
            c.painter.circle_filled(p, 2.6, s.marker_active);
        } else {
            c.painter.circle_filled(p, 1.8, s.marker);
        }
    }
}

fn hex_bolt(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    let detents = c.theme.knobs.endless_detents;
    let turn = match k.endless {
        Some(pos) => {
            endless_markers(c, k, detents, pos, r + 8.0);
            k.detent_angle(detents, pos as f32)
        }
        None => {
            step_markers(c, k, sweep, r + 8.0);
            k.angle(sweep, k.value)
        }
    };

    // The bolt head rotates with the selection
    let hex: Vec<Pos2> = (0..6)
        .map(|i| k.center + Vec2::angled(turn + i as f32 * PI / 3.0) * (r + 1.0))
        .collect();
    p.add(Shape::convex_polygon(
        hex,
        s.body,
        Stroke::new(1.2_f32, s.rim),
    ));
    p.circle_filled(k.center, r - 5.0, s.face);
    p.circle_filled(k.center - Vec2::new(1.2, 1.8), r - 8.0, s.highlight);
    p.circle_filled(k.center, r - 9.0, s.face);
    p.circle_stroke(k.center, r - 5.0, Stroke::new(1.0_f32, s.rim));
    speckles(c, k, 9, r - 4.0, 0x55);

    let dir = Vec2::angled(turn);
    p.line_segment(
        [k.center, k.center + dir * (r - 3.0)],
        Stroke::new(3.0_f32, s.pointer),
    );
    p.circle_filled(k.center, 2.5, s.pointer);
}

fn neon_ring(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    value_arc(c, k, sweep, r + 5.0, 2.5, 1.6);

    p.circle_filled(k.center, r - 2.0, s.body);
    let rim: Color32 = s.rim.into();
    p.circle_stroke(
        k.center,
        r - 2.0,
        Stroke::new(4.0_f32, rim.gamma_multiply(0.15)),
    );
    p.circle_stroke(k.center, r - 2.0, Stroke::new(1.2_f32, rim));
    p.circle_filled(k.center, r - 7.0, s.face);
    p.circle_stroke(k.center, r - 10.0, Stroke::new(1.0_f32, s.highlight));

    let dir = Vec2::angled(k.angle(sweep, k.value));
    let tip = k.center + dir * (r - 6.0);
    p.line_segment(
        [k.center + dir * (r * 0.3), tip],
        Stroke::new(2.0_f32, s.pointer),
    );
    p.circle_filled(tip, 3.5, Color32::from(s.pointer).gamma_multiply(0.25));
    p.circle_filled(tip, 1.8, s.pointer);
}

fn neon_segments(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    let detents = c.theme.knobs.endless_detents;
    // Selectors get one segment per choice across the sweep; endless knobs get a full ring of
    // detents with the current one lit, turning forever.
    let (count, current) = match k.endless {
        Some(pos) => (detents.max(1) as usize, pos % detents.max(1) as usize),
        None => {
            let steps = k.steps.unwrap_or(1).max(1);
            (steps + 1, (k.value * steps as f32).round() as usize)
        }
    };
    let slot_angle = |x: f32| match k.endless {
        Some(_) => k.detent_angle(detents, x - 0.5),
        None => k.angle(sweep, x / count as f32),
    };

    for i in 0..count {
        let (a, b) = (slot_angle(i as f32 + 0.12), slot_angle(i as f32 + 0.88));
        if i == current {
            let lit: Color32 = if k.active {
                s.arc_active.into()
            } else {
                s.arc.into()
            };
            p.add(arc(
                k.center,
                r + 5.0,
                a,
                b,
                Stroke::new(8.0_f32, lit.gamma_multiply(0.25)),
            ));
            p.add(arc(k.center, r + 5.0, a, b, Stroke::new(3.5_f32, lit)));
        } else {
            p.add(arc(k.center, r + 5.0, a, b, Stroke::new(3.0_f32, s.track)));
        }
    }

    // Diamond body that turns toward the selected segment
    let turn = match k.endless {
        Some(pos) => k.detent_angle(detents, pos as f32),
        None => slot_angle(current as f32 + 0.5),
    };
    let diamond: Vec<Pos2> = (0..4)
        .map(|i| k.center + Vec2::angled(turn + i as f32 * PI / 2.0) * (r - 1.0))
        .collect();
    p.add(Shape::convex_polygon(diamond.clone(), s.body, Stroke::NONE));
    let rim: Color32 = s.rim.into();
    p.add(Shape::closed_line(
        diamond.clone(),
        Stroke::new(4.0_f32, rim.gamma_multiply(0.15)),
    ));
    p.add(Shape::closed_line(diamond, Stroke::new(1.2_f32, rim)));
    p.circle_filled(k.center, r * 0.42, s.face);
    p.circle_stroke(k.center, r * 0.42, Stroke::new(1.0_f32, s.highlight));

    let dir = Vec2::angled(turn);
    p.line_segment(
        [k.center, k.center + dir * (r - 4.0)],
        Stroke::new(2.0_f32, s.pointer),
    );
    p.circle_filled(k.center, 2.2, s.pointer);
}

/// A full ring of detent dots for an endless knob, with the current detent lit.
fn endless_markers(c: &Canvas, k: &KnobView, detents: u32, pos: usize, radius: f32) {
    let s = k.style;
    let lit = pos % detents.max(1) as usize;
    for i in 0..detents.max(1) as usize {
        let p = k.center + Vec2::angled(k.detent_angle(detents, i as f32)) * radius;
        if i == lit {
            c.painter
                .circle_filled(p, 5.0, Color32::from(s.marker_active).gamma_multiply(0.2));
            c.painter.circle_filled(p, 2.4, s.marker_active);
        } else {
            c.painter.circle_filled(p, 1.4, s.marker);
        }
    }
}

/// Printed scale around a knob: 11 ticks, labelled at both ends and the middle.
fn printed_scale(c: &Canvas, k: &KnobView, sweep: f32, font: FontId) {
    let color: Color32 = k.style.marker.into();
    let r = k.radius;
    for i in 0..=10 {
        let dir = Vec2::angled(k.angle(sweep, i as f32 / 10.0));
        let (inner, width) = if i % 5 == 0 {
            (r + 3.0, 1.4_f32)
        } else {
            (r + 4.5, 1.0_f32)
        };
        c.painter.line_segment(
            [k.center + dir * inner, k.center + dir * (r + 7.0)],
            Stroke::new(width, color),
        );
    }
    let labels = if k.bipolar {
        ["-", "0", "+"]
    } else {
        ["0", "5", "10"]
    };
    for (label, at) in labels.iter().zip([0.0, 0.5, 1.0]) {
        let pos = k.center + Vec2::angled(k.angle(sweep, at)) * (r + 13.0);
        c.painter
            .text(pos, Align2::CENTER_CENTER, *label, font.clone(), color);
    }
}

fn skirted(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    if let Some(font) = s.scale {
        printed_scale(c, k, sweep, font.id());
    }
    let dir = Vec2::angled(k.angle(sweep, k.value));

    // Metal skirt with a notch line showing the value
    p.circle_filled(k.center, r, s.rim);
    p.circle_stroke(
        k.center,
        r - 0.5,
        Stroke::new(1.0_f32, Color32::from_black_alpha(90)),
    );
    p.circle_stroke(
        k.center,
        r * 0.86,
        Stroke::new(1.0_f32, Color32::from_white_alpha(50)),
    );
    p.line_segment(
        [k.center + dir * (r * 0.7), k.center + dir * (r - 1.0)],
        Stroke::new(2.0_f32, s.body),
    );

    // Knurled cap
    let cap = r * 0.68;
    p.circle_filled(
        k.center + Vec2::new(0.0, 1.5),
        cap,
        Color32::from_black_alpha(110),
    );
    p.circle_filled(k.center, cap, s.body);
    for i in 0..24 {
        let d = Vec2::angled(k.angle(sweep, k.value) + i as f32 * (2.0 * PI / 24.0));
        p.line_segment(
            [k.center + d * (cap - 2.5), k.center + d * cap],
            Stroke::new(1.0_f32, s.face),
        );
    }
    p.circle_filled(k.center - Vec2::new(1.0, 1.5), cap * 0.62, s.highlight);
    p.circle_filled(k.center, cap * 0.55, s.face);

    let color = if k.active { s.arc_active } else { s.pointer };
    p.line_segment(
        [k.center + dir * (cap * 0.2), k.center + dir * (cap - 1.5)],
        Stroke::new(2.0_f32, color),
    );
}

fn chicken_head(c: &Canvas, k: &KnobView, sweep: f32) {
    let (s, r, p) = (k.style, k.radius, c.painter);
    let detents = c.theme.knobs.endless_detents;
    let angle = match k.endless {
        Some(pos) => {
            endless_markers(c, k, detents, pos, r + 6.0);
            k.detent_angle(detents, pos as f32)
        }
        None => {
            step_markers(c, k, sweep, r + 6.0);
            k.angle(sweep, k.value)
        }
    };
    let d = Vec2::angled(angle);
    let n = Vec2::new(-d.y, d.x);
    let at = |along: f32, across: f32| k.center + d * along + n * across;

    // The head: a rounded back centered on the pivot, tapering to a pointed nose that reaches
    // the knob's edge. Keeping the round part centered is what makes it look centered.
    let back = r * 0.66;
    let mut head = vec![at(r * 0.98, 0.0)];
    head.extend((0..=12).map(|i| {
        let a = (70.0 + 220.0 * i as f32 / 12.0).to_radians();
        at(back * a.cos(), back * a.sin())
    }));

    // A shadow the shape of the head (the generic round knob shadow would peek out around it)
    if let Some(shadow) = s.shadow {
        let offset = Vec2::new(1.5, 2.0);
        let shadow_shape = head.iter().map(|&pt| pt + offset).collect();
        p.add(Shape::convex_polygon(shadow_shape, shadow, Stroke::NONE));
    }
    p.add(Shape::convex_polygon(
        head,
        s.body,
        Stroke::new(1.0_f32, s.rim),
    ));
    p.circle_filled(k.center - Vec2::new(1.0, 1.2), back * 0.55, s.highlight);
    p.circle_filled(k.center, back * 0.4, s.face);

    let color = if k.active { s.arc_active } else { s.pointer };
    p.line_segment(
        [at(back * 0.5, 0.0), at(r * 0.88, 0.0)],
        Stroke::new(2.0_f32, color),
    );
}
