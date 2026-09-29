//! Interaction only: turning mouse input into parameter changes and notes. Drawing lives in
//! `render`, so a new look never touches this behavior.

use nih_plug::prelude::*;
use nih_plug_egui::egui::{Id, Rect, Sense, Ui};

use super::controls::{Control, Role};

/// Drag distance, in normalized units per pixel.
const DRAG_SPEED: f32 = 0.005;
const FINE_DRAG_SPEED: f32 = 0.0006;
const SELECTOR_DRAG_SPEED: f32 = 0.01;
const SCROLL_SPEED: f32 = 0.0015;

pub struct KnobInput {
    pub active: bool,
}

/// Vertical drag (Shift for fine), scroll wheel, double-click to reset. Selectors step through
/// their choices on click instead of resetting.
pub fn knob(ui: &mut Ui, setter: &ParamSetter, control: &Control, hit_rect: Rect) -> KnobInput {
    let id = ui.id().with(control.key);
    let response = ui.interact(hit_rect, id, Sense::click_and_drag());
    let b = &control.binding;

    // The drag start value and accumulated distance live in egui's memory so selectors can move
    // one step at a time.
    if response.drag_started() {
        b.begin(setter);
        store_drag(ui, id, (b.normalized(), 0.0));
    }
    if response.dragged() {
        let speed = match (control.role, ui.input(|i| i.modifiers.shift)) {
            (Role::Selector, _) => SELECTOR_DRAG_SPEED,
            (_, true) => FINE_DRAG_SPEED,
            (_, false) => DRAG_SPEED,
        };
        let (start, mut accum) = ui
            .data(|d| d.get_temp::<(f32, f32)>(id))
            .unwrap_or_default();
        accum -= response.drag_delta().y * speed;
        store_drag(ui, id, (start, accum));
        b.set_normalized(setter, start + accum);
    }
    if response.drag_stopped() {
        b.end(setter);
    }

    if control.role == Role::Selector {
        if response.clicked() {
            let steps = b.step_count().unwrap_or(1) as f32;
            let next = b.normalized() + 1.0 / steps;
            b.begin(setter);
            b.set_normalized(setter, if next > 1.0 + 1e-4 { 0.0 } else { next });
            b.end(setter);
        }
    } else if response.double_clicked() {
        b.begin(setter);
        b.reset(setter);
        b.end(setter);
    }

    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let delta = match b.step_count() {
                Some(steps) => scroll.signum() / steps as f32,
                None => scroll * SCROLL_SPEED,
            };
            b.begin(setter);
            b.set_normalized(setter, b.normalized() + delta);
            b.end(setter);
        }
    }

    KnobInput {
        active: response.hovered() || response.dragged(),
    }
}

fn store_drag(ui: &Ui, id: Id, value: (f32, f32)) {
    ui.data_mut(|d| d.insert_temp(id, value));
}

/// The key under the mouse while the button is held on the keyboard, if any.
pub fn piano(ui: &mut Ui, rect: Rect, whites: &[(u8, Rect)], blacks: &[(u8, Rect)]) -> Option<u8> {
    let response = ui.interact(rect, ui.id().with("piano"), Sense::click_and_drag());
    if !response.is_pointer_button_down_on() {
        return None;
    }
    let pos = response.interact_pointer_pos()?;
    // Black keys sit on top, so they win
    blacks
        .iter()
        .chain(whites)
        .find(|(_, r)| r.contains(pos))
        .map(|(n, _)| *n)
}

pub enum Cycle {
    None,
    Next,
    Previous,
}

/// A small clickable label: left click for the next option, right click for the previous one.
pub fn cycler(ui: &mut Ui, rect: Rect, id: &str, tooltip: String) -> (Cycle, bool) {
    let response = ui
        .interact(rect, ui.id().with(id), Sense::click())
        .on_hover_text(tooltip);
    let cycle = if response.clicked() {
        Cycle::Next
    } else if response.secondary_clicked() {
        Cycle::Previous
    } else {
        Cycle::None
    };
    (cycle, response.hovered())
}

/// A left-to-right position switch: click or drag to a position, or scroll. Returns whether it's
/// hovered or being dragged.
pub fn switch(
    ui: &mut Ui,
    setter: &ParamSetter,
    control: &Control,
    area: Rect,
    positions: &[f32],
    tooltip: String,
) -> bool {
    let response = ui
        .interact(area, ui.id().with(control.key), Sense::click_and_drag())
        .on_hover_text(tooltip);
    let b = &control.binding;
    let steps = positions.len().saturating_sub(1).max(1) as f32;
    let nearest = |x: f32| {
        positions
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map_or(0, |(i, _)| i)
    };
    let select = |i: usize| b.set_normalized(setter, i as f32 / steps);

    if response.drag_started() {
        b.begin(setter);
    }
    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            if response.clicked() {
                b.begin(setter);
                select(nearest(pos.x));
                b.end(setter);
            } else {
                select(nearest(pos.x));
            }
        }
    }
    if response.drag_stopped() {
        b.end(setter);
    }

    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta);
        let delta = if scroll.x != 0.0 { scroll.x } else { scroll.y };
        if delta != 0.0 {
            b.begin(setter);
            b.set_normalized(setter, b.normalized() + delta.signum() / steps);
            b.end(setter);
        }
    }

    response.hovered() || response.dragged()
}

/// Drag distance for one step of an endless knob.
const ENDLESS_DRAG_STEP_PX: f32 = 14.0;

/// Where `current` sits in `order`.
pub fn position_in(order: &[usize], current: usize) -> usize {
    order.iter().position(|&i| i == current).unwrap_or(0)
}

/// An endless knob stepping through `order` (the parameter values in browsing order), wrapping
/// at both ends. Clockwise (drag up or right, scroll up, click) moves forward; counter-clockwise
/// (drag down or left, scroll down, right-click) moves back.
pub fn endless(
    ui: &mut Ui,
    setter: &ParamSetter,
    control: &Control,
    hit_rect: Rect,
    order: &[usize],
    tooltip: String,
) -> KnobInput {
    let id = ui.id().with(control.key);
    let response = ui
        .interact(hit_rect, id, Sense::click_and_drag())
        .on_hover_text(tooltip);
    let mut steps = 0i32;

    if response.drag_started() {
        ui.data_mut(|d| d.insert_temp(id, 0.0f32));
    }
    if response.dragged() {
        let delta = response.drag_delta();
        let mut accum = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or_default() + delta.x - delta.y;
        let whole = (accum / ENDLESS_DRAG_STEP_PX).trunc();
        steps += whole as i32;
        accum -= whole * ENDLESS_DRAG_STEP_PX;
        ui.data_mut(|d| d.insert_temp(id, accum));
    }
    if response.clicked() {
        steps += 1;
    }
    if response.secondary_clicked() {
        steps -= 1;
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            steps += scroll.signum() as i32;
        }
    }

    if steps != 0 && !order.is_empty() {
        let b = &control.binding;
        let pos = position_in(order, b.number().round() as usize) as i32;
        let next = order[(pos + steps).rem_euclid(order.len() as i32) as usize];
        let max = b.step_count().unwrap_or(1).max(1) as f32;
        b.begin(setter);
        b.set_normalized(setter, next as f32 / max);
        b.end(setter);
    }

    KnobInput {
        active: response.hovered() || response.dragged(),
    }
}
