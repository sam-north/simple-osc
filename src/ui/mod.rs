//! The plugin GUI, split into layers so the look and the language can change without touching
//! behavior:
//!
//! - [`controls`]: which controls exist and how they bind to parameters (no visuals, no text)
//! - [`readout`]: live values from the audio thread, as plain data
//! - [`input`] / [`widgets`]: turning keyboard and mouse input into notes and parameter changes
//! - [`layout`]: where things go, sized by the theme's metrics
//! - [`render`]: drawing, driven entirely by the active [`theme::Theme`]
//! - [`i18n`]: every visible string, looked up by key in the active language pack
//! - [`assets`]: loading built-in and user themes, language packs and keymaps
//!
//! This module just runs one frame: gather data, lay it out, handle input, draw.

mod assets;
mod controls;
mod fonts;
mod i18n;
mod input;
mod layout;
mod readout;
mod render;
mod theme;
mod widgets;

use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, Align2, Color32, Context, Pos2, Rect, TextureHandle, TextureOptions, Ui, Vec2,
};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use assets::{Library, Prefs};
use controls::{Role, Section};
use i18n::Strings;
use input::NoteInput;
use render::keyboard::{KeyState, PianoKey};
use render::knob::KnobView;
use render::Canvas;
use theme::{Background, Theme};
use widgets::Cycle;

use crate::shared::Shared;
use crate::SimpleOscParams;

/// The editor window size. Hosts size the plugin window from this, so it's fixed; themes lay
/// themselves out inside it.
const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 540;

/// How often an idle window refreshes, so host automation still shows up.
const IDLE_REFRESH: Duration = Duration::from_millis(100);

/// Text keys used outside the control table. Checked by the language-pack tests.
#[cfg(test)]
pub(crate) const STATIC_TEXT_KEYS: &[&str] = &[
    "app.title",
    "app.subtitle",
    "keyboard.title",
    "keyboard.hint",
    "scope.info",
    "scope.note",
    "wave.tooltip",
    "endless.tooltip",
    "value.inactive",
    "switcher.theme",
    "switcher.language",
    "unit.hz",
    "unit.khz",
    "unit.ms",
    "unit.s",
    "unit.semitones",
    "unit.cents",
    "unit.db",
    "unit.percent",
];

pub fn default_editor_state() -> Arc<EguiState> {
    EguiState::from_size(WINDOW_WIDTH, WINDOW_HEIGHT)
}

struct GuiState {
    library: Library,
    theme: usize,
    language: usize,
    input: NoteInput,
    /// Generated background textures, per theme. They live on the GPU and belong to one egui
    /// context, so they're dropped when the window closes and rebuilt when it reopens, rather
    /// than keeping a CPU copy around.
    textures: HashMap<String, TextureHandle>,
    /// The chipped-paint overlay for distressed titles, built on first use.
    flecks: Option<TextureHandle>,
    /// The Wave knob's browsing order and the language it was sorted for.
    wave_order: (usize, Vec<usize>),
}

pub fn create_editor(params: Arc<SimpleOscParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let library = Library::load();
    let prefs = Prefs::load();
    let state = GuiState {
        theme: library.theme_index(&prefs.theme),
        language: library.language_index(&prefs.language),
        input: NoteInput::new(&library.keymap),
        library,
        textures: HashMap::new(),
        flecks: None,
        wave_order: (usize::MAX, Vec::new()),
    };
    create_egui_editor(
        params.editor_state.clone(),
        state,
        |ctx, state| {
            ctx.set_visuals(egui::Visuals::dark());
            fonts::install(ctx);
            state.textures.clear();
            state.flecks = None;
        },
        move |ctx, setter, state| frame(ctx, setter, state, &params, &shared),
    )
}

fn frame(
    ctx: &Context,
    setter: &ParamSetter,
    st: &mut GuiState,
    params: &SimpleOscParams,
    shared: &Shared,
) {
    st.input.handle_events(ctx);
    let texture = background_texture(ctx, st);
    let flecks = flecks_texture(ctx, st);
    if st.wave_order.0 != st.language {
        let strings = Strings {
            language: &st.library.languages[st.language],
            fallback: st.library.default_language(),
        };
        st.wave_order = (st.language, controls::wave_order(&strings));
    }

    let mut switches = (Cycle::None, Cycle::None);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let view = View {
                library: &st.library,
                theme: &st.library.themes[st.theme],
                strings: Strings {
                    language: &st.library.languages[st.language],
                    fallback: st.library.default_language(),
                },
                wave_order: &st.wave_order.1,
            };
            switches = draw(
                ui,
                setter,
                &view,
                &mut st.input,
                texture.as_ref(),
                flecks.as_ref(),
                params,
                shared,
            );
        });
    st.input.publish(&st.library.keymap, shared);

    let (theme_switch, language_switch) = switches;
    let theme_changed = cycle(&mut st.theme, st.library.themes.len(), theme_switch);
    let language_changed = cycle(
        &mut st.language,
        st.library.languages.len(),
        language_switch,
    );
    if theme_changed || language_changed {
        Prefs {
            theme: st.library.themes[st.theme].id.clone(),
            language: st.library.languages[st.language].id.clone(),
        }
        .save();
    }

    // Redraw every frame only while the scope is live; otherwise refresh a few times a second so
    // changes from the host (automation, presets) still show up.
    if shared.voice_active() {
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after(IDLE_REFRESH);
    }
}

fn cycle(index: &mut usize, len: usize, direction: Cycle) -> bool {
    let len = len.max(1);
    *index = match direction {
        Cycle::None => return false,
        Cycle::Next => (*index + 1) % len,
        Cycle::Previous => (*index + len - 1) % len,
    };
    true
}

/// The theme's generated background texture, if it uses one.
fn background_texture(ctx: &Context, st: &mut GuiState) -> Option<TextureHandle> {
    let theme = &st.library.themes[st.theme];
    if !matches!(
        theme.background,
        Background::RustedMetal { .. } | Background::Tolex { .. }
    ) {
        return None;
    }
    if let Some(texture) = st.textures.get(&theme.id) {
        return Some(texture.clone());
    }
    let (w, h) = (WINDOW_WIDTH as usize, WINDOW_HEIGHT as usize);
    let image = match &theme.background {
        Background::RustedMetal { palette, .. } => render::texture::rusted_metal(w, h, palette),
        Background::Tolex {
            base,
            grain,
            highlight,
        } => render::texture::tolex(w, h, *base, *grain, *highlight),
        _ => return None,
    };
    let texture = ctx.load_texture(
        format!("background-{}", theme.id),
        image,
        TextureOptions::LINEAR,
    );
    st.textures.insert(theme.id.clone(), texture.clone());
    Some(texture)
}

/// The chipped-paint overlay, for themes with a distressed title. One small texture drawn as a
/// single quad, instead of hundreds of flecks painted every frame.
fn flecks_texture(ctx: &Context, st: &mut GuiState) -> Option<TextureHandle> {
    let theme = &st.library.themes[st.theme];
    if !matches!(theme.title.effect, theme::TitleEffect::Distressed { .. }) {
        return None;
    }
    Some(
        st.flecks
            .get_or_insert_with(|| {
                ctx.load_texture(
                    "flecks",
                    render::texture::flecks(384, 48),
                    TextureOptions::LINEAR,
                )
            })
            .clone(),
    )
}

/// Everything a frame reads but doesn't change.
struct View<'a> {
    library: &'a Library,
    theme: &'a Theme,
    strings: Strings<'a>,
    /// Catalog indices in the order the Wave knob steps through them.
    wave_order: &'a [usize],
}

impl View<'_> {
    fn cased(&self, key: &str, upper: bool) -> String {
        let text = self.strings.text(key);
        if upper {
            text.to_uppercase()
        } else {
            text
        }
    }
}

/// Draws the editor and returns the requested theme and language switches.
fn draw(
    ui: &mut Ui,
    setter: &ParamSetter,
    v: &View,
    input: &mut NoteInput,
    texture: Option<&TextureHandle>,
    flecks: Option<&TextureHandle>,
    params: &SimpleOscParams,
    shared: &Shared,
) -> (Cycle, Cycle) {
    let full = ui.max_rect();
    let painter = ui.painter().clone();
    let theme = v.theme;
    let c = Canvas {
        painter: &painter,
        theme,
        full,
        texture,
        flecks,
    };
    let sections = controls::sections(params);
    let slot_counts: Vec<usize> = sections.iter().map(|s| s.slots.len()).collect();
    let l = layout::compute(full, &theme.metrics, &slot_counts);
    let t = &theme.text;
    let shadow = t.shadow.map(Into::into);
    let on_background: Color32 = t.on_background.unwrap_or(t.muted).into();

    render::background::draw(&c);

    // Header: title on the left, switchers and subtitle on the right
    let title = v.cased("app.title", t.uppercase_title);
    let logo = render::logo::draw(&c, l.header.left_center() + Vec2::new(2.0, 0.0));
    render::text::title(
        &c,
        Pos2::new(logo.right() + theme.logo.gap, l.header.center().y),
        Align2::LEFT_CENTER,
        &title,
    );
    let switches = switchers(ui, &c, v, l.header);
    let subtitle = v.cased("app.subtitle", t.uppercase_labels);
    render::text::spaced(
        &painter,
        l.header.right_bottom() - Vec2::new(0.0, 4.0),
        Align2::RIGHT_BOTTOM,
        &subtitle,
        t.subtitle.id(),
        on_background,
        t.subtitle_spacing,
        shadow,
    );

    // Oscilloscope
    let sounding = readout::sounding(params, shared);
    let trace = readout::scope_trace(shared, l.scope.width() as usize);
    let display = |key: &str| {
        find_control(&sections, key)
            .map(|c| c.display_value(&v.strings))
            .unwrap_or_default()
    };
    let wave_category = crate::waves::catalog()
        .get(params.waveform.value() as usize)
        .map(|w| controls::category_name(w.category, &v.strings))
        .unwrap_or_default();
    let mut info = v.strings.format(
        "scope.info",
        &[
            ("category", &wave_category),
            ("wave", &display("wave")),
            ("filter", &display("type")),
            ("cutoff", &display("cutoff")),
        ],
    );
    if t.uppercase_info {
        info = info.to_uppercase();
    }
    let note = sounding.as_ref().map(|s| {
        v.strings.format(
            "scope.note",
            &[
                ("note", &v.strings.note_name(s.pitch)),
                ("frequency", &v.strings.number(s.frequency as f64, 1)),
            ],
        )
    });
    render::scope::draw(&c, l.scope, trace.as_deref(), &info, note.as_deref());

    // The Wave knob browses by category, then alphabetically in the current language
    let wave_tooltip = v.strings.format(
        "wave.tooltip",
        &[("category", &wave_category), ("wave", &display("wave"))],
    );

    // Control sections
    let knobs = &theme.knobs;
    for ((rect, slots), section) in l.sections.iter().zip(&sections) {
        render::panel::draw(&c, *rect);
        render::text::spaced(
            &painter,
            Pos2::new(
                rect.left() + theme.metrics.section_title_indent,
                rect.top() + theme.metrics.section_title_height / 2.0 + 2.0,
            ),
            Align2::LEFT_CENTER,
            &v.cased(&format!("section.{}", section.key), t.uppercase_sections),
            t.section.id(),
            t.muted.into(),
            t.section_spacing,
            shadow,
        );

        for (slot_def, rects) in section.slots.iter().zip(slots) {
            if let Some(above) = &slot_def.above {
                draw_switch(ui, setter, &c, v, above, rects.above);
            }

            let control = &slot_def.control;
            let slot = rects.knob;
            let center = Pos2::new(
                slot.center().x,
                slot.top() + knobs.top_margin + knobs.radius,
            );
            let hit_rect = Rect::from_center_size(center, Vec2::splat(knobs.radius * 2.0 + 14.0));
            let (knob_input, endless) = if control.role == Role::Endless {
                // Waveforms browse by category then A to Z; other endless knobs (like the filter
                // type) keep their natural order
                let (order, tooltip) = if matches!(control.format, controls::ValueFormat::Wave) {
                    (Cow::Borrowed(v.wave_order), wave_tooltip.clone())
                } else {
                    let steps = control.binding.step_count().unwrap_or(0);
                    let tooltip = v.strings.format(
                        "endless.tooltip",
                        &[("value", &control.display_value(&v.strings))],
                    );
                    (Cow::Owned((0..=steps).collect()), tooltip)
                };
                let input = widgets::endless(ui, setter, control, hit_rect, &order, tooltip);
                let pos = widgets::position_in(&order, control.binding.number().round() as usize);
                (input, Some(pos))
            } else {
                (widgets::knob(ui, setter, control, hit_rect), None)
            };
            let view = KnobView {
                style: if matches!(control.role, Role::Selector | Role::Endless) {
                    &knobs.selector
                } else {
                    &knobs.continuous
                },
                center,
                radius: knobs.radius,
                value: control.binding.normalized(),
                bipolar: control.role == Role::Bipolar,
                steps: control.binding.step_count(),
                endless,
                active: knob_input.active,
                seed: control
                    .key
                    .bytes()
                    .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)),
            };
            render::knob::draw(&c, &view);
            let label = v.cased(&control.label_key(), t.uppercase_labels);
            render::knob::draw_text(
                &c,
                &view,
                &label,
                &control.display_value(&v.strings),
                control.inactive,
            );
        }
    }

    // Keyboard
    let keymap = &v.library.keymap;
    let label_rect = render::text::spaced(
        &painter,
        l.keyboard_label.left_center() + Vec2::new(2.0, 0.0),
        Align2::LEFT_CENTER,
        &v.cased("keyboard.title", t.uppercase_sections),
        t.section.id(),
        on_background,
        t.section_spacing,
        shadow,
    );
    let hint = v.strings.text("keyboard.hint");
    render::text::shadowed(
        &painter,
        Pos2::new(label_rect.right() + 14.0, l.keyboard_label.center().y),
        Align2::LEFT_CENTER,
        &hint,
        t.hint.id(),
        on_background,
        shadow,
    );

    let first = input.base_note;
    let last = first
        .saturating_add(12 * theme.metrics.keyboard_octaves)
        .min(127);
    let (whites, blacks) = layout::piano_keys(l.keyboard, first, last);
    input.mouse_note = widgets::piano(ui, l.keyboard, &whites, &blacks);
    let sounding_note = sounding.map(|s| s.note);
    let to_keys = |keys: &[(u8, Rect)]| -> Vec<PianoKey> {
        keys.iter()
            .map(|&(note, rect)| PianoKey {
                note,
                rect,
                state: if sounding_note == Some(note) {
                    KeyState::Sounding
                } else if input.is_held(note, keymap) {
                    KeyState::Held
                } else {
                    KeyState::Idle
                },
                label: keymap.key_for(note - first).map(|k| k.name().to_owned()),
                octave_label: (note % 12 == 0).then(|| v.strings.note_name(note)),
            })
            .collect()
    };
    render::keyboard::draw(&c, l.keyboard, &to_keys(&whites), &to_keys(&blacks));

    switches
}

/// A left-to-right switch attached above a knob.
fn draw_switch(
    ui: &mut Ui,
    setter: &ParamSetter,
    c: &Canvas,
    v: &View,
    control: &controls::Control,
    rect: Rect,
) {
    let labels = control.position_labels(&v.strings);
    let (area, positions) = layout::switch_positions(rect, c.theme.switch.width, labels.len());
    let tooltip = v.strings.text(&control.label_key());
    let active = widgets::switch(ui, setter, control, area, &positions, tooltip);
    let steps = control.binding.step_count().unwrap_or(0);
    let selected = (control.binding.normalized() * steps as f32).round() as usize;
    render::switch::draw(
        c,
        &render::switch::SwitchView {
            area,
            positions: &positions,
            labels: &labels,
            selected,
            active,
        },
    );
}

fn find_control<'a, 'p>(
    sections: &'a [Section<'p>],
    key: &str,
) -> Option<&'a controls::Control<'p>> {
    sections
        .iter()
        .flat_map(|s| &s.slots)
        .flat_map(|s| s.controls())
        .find(|c| c.key == key)
}

/// Two small labels in the top-right corner: the theme name and the language. Click for the next
/// one, right-click for the previous one.
fn switchers(ui: &mut Ui, c: &Canvas, v: &View, header: Rect) -> (Cycle, Cycle) {
    let style = &c.theme.switcher;
    let language = v.strings.language;
    let items = [
        (
            "switch-theme",
            c.theme.name.to_uppercase(),
            v.strings
                .format("switcher.theme", &[("name", &c.theme.name)]),
        ),
        (
            "switch-language",
            language.short_name.clone(),
            v.strings
                .format("switcher.language", &[("name", &language.name)]),
        ),
    ];

    let gap = style.font.size;
    let mut right = header.right();
    let mut results = Vec::new();
    for (id, text, tooltip) in items.into_iter().rev() {
        let size = render::text::measure(c.painter, &text, style.font);
        let rect = Rect::from_min_size(Pos2::new(right - size.x, header.top()), size);
        right = rect.left() - gap;
        let (cycle, hovered) = widgets::cycler(ui, rect.expand(3.0), id, tooltip);
        let color = if hovered { style.hover } else { style.text };
        c.painter.text(
            rect.min,
            Align2::LEFT_TOP,
            text,
            style.font.id(),
            color.into(),
        );
        results.push(cycle);
    }
    let language_switch = results.remove(0);
    let theme_switch = results.remove(0);
    (theme_switch, language_switch)
}
