//! Where things go. All sizes come from the theme's metrics; nothing here draws.

use nih_plug_egui::egui::{Pos2, Rect, Vec2};

use super::theme::Metrics;

/// A control column: an optional strip for an attached control, then the knob area. Every
/// section reserves the strip, so knobs line up whether or not a slot uses it.
#[derive(Clone, Copy)]
pub struct SlotRects {
    pub above: Rect,
    pub knob: Rect,
}

pub struct Layout {
    pub header: Rect,
    pub scope: Rect,
    /// One rect per section, and the slots inside each.
    pub sections: Vec<(Rect, Vec<SlotRects>)>,
    pub keyboard_label: Rect,
    pub keyboard: Rect,
}

/// `slot_counts` is the number of controls in each section. Every control slot gets the same
/// width, so knobs line up across sections.
pub fn compute(full: Rect, m: &Metrics, slot_counts: &[usize]) -> Layout {
    let inner = full.shrink(m.margin);
    let header = Rect::from_min_size(inner.min, Vec2::new(inner.width(), m.header_height));
    let scope = Rect::from_min_size(
        Pos2::new(inner.left(), header.bottom()),
        Vec2::new(inner.width(), m.scope_height),
    );

    let top = scope.bottom() + m.section_gap;
    let total_slots: usize = slot_counts.iter().sum();
    let gaps = m.section_gap * slot_counts.len().saturating_sub(1) as f32;
    let padding = 2.0 * m.section_padding * slot_counts.len() as f32;
    let slot_w = (inner.width() - gaps - padding) / total_slots.max(1) as f32;

    let mut x = inner.left();
    let sections = slot_counts
        .iter()
        .map(|&count| {
            let rect = Rect::from_min_size(
                Pos2::new(x, top),
                Vec2::new(
                    count as f32 * slot_w + 2.0 * m.section_padding,
                    m.section_height,
                ),
            );
            x = rect.right() + m.section_gap;
            let slots = (0..count)
                .map(|i| {
                    let left = rect.left() + m.section_padding + i as f32 * slot_w;
                    let above_top = rect.top() + m.section_title_height;
                    let knob_top = above_top + m.attachment_height;
                    SlotRects {
                        above: Rect::from_min_size(
                            Pos2::new(left, above_top),
                            Vec2::new(slot_w, m.attachment_height),
                        ),
                        knob: Rect::from_min_max(
                            Pos2::new(left, knob_top),
                            Pos2::new(left + slot_w, rect.bottom()),
                        ),
                    }
                })
                .collect();
            (rect, slots)
        })
        .collect();

    let keyboard_label = Rect::from_min_size(
        Pos2::new(inner.left(), top + m.section_height),
        Vec2::new(inner.width(), m.keyboard_label_height),
    );
    let keyboard = Rect::from_min_max(Pos2::new(inner.left(), keyboard_label.bottom()), inner.max);

    Layout {
        header,
        scope,
        sections,
        keyboard_label,
        keyboard,
    }
}

/// On-screen piano keys for `first..=last`, as (note, rect). White keys fill the width; black
/// keys sit on the boundaries between white keys.
pub fn piano_keys(rect: Rect, first: u8, last: u8) -> (Vec<(u8, Rect)>, Vec<(u8, Rect)>) {
    let is_black = |n: u8| matches!(n % 12, 1 | 3 | 6 | 8 | 10);
    let white_count = (first..=last).filter(|&n| !is_black(n)).count();
    let white_w = rect.width() / white_count.max(1) as f32;
    let black_size = Vec2::new(white_w * 0.6, rect.height() * 0.6);

    let (mut whites, mut blacks) = (Vec::new(), Vec::new());
    let mut white_index = 0;
    for note in first..=last {
        let x = rect.left() + white_index as f32 * white_w;
        if is_black(note) {
            let min = Pos2::new(x - black_size.x / 2.0, rect.top());
            blacks.push((note, Rect::from_min_size(min, black_size)));
        } else {
            let min = Pos2::new(x, rect.top());
            whites.push((
                note,
                Rect::from_min_size(min, Vec2::new(white_w, rect.height())),
            ));
            white_index += 1;
        }
    }
    (whites, blacks)
}

/// Geometry of a left-to-right switch with `count` positions, centered in `rect`: its hit area
/// and the x coordinate of each position. It may be wider than its slot.
pub fn switch_positions(rect: Rect, width: f32, count: usize) -> (Rect, Vec<f32>) {
    let area = Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()));
    let step = area.width() / count.max(1) as f32;
    let xs = (0..count)
        .map(|i| area.left() + step * (i as f32 + 0.5))
        .collect();
    (area, xs)
}
