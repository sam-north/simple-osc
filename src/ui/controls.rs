//! What the GUI controls are, independent of how they're drawn or labelled. Each control binds a
//! host parameter to a role (how it behaves), a value format (how its value reads) and a stable
//! key used to look up its text in the language pack.

use nih_plug::prelude::*;

use super::i18n::Strings;
use crate::waves::{self, Category};
use crate::SimpleOscParams;

/// How a control behaves. Themes pick a drawing style per role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Continuous,
    /// Continuous, centered on zero (the value arc grows from the middle).
    Bipolar,
    /// Picks one of a few choices. A click advances to the next one.
    Selector,
    /// A few positions laid out left to right; click one to select it.
    Switch,
    /// Turns forever with no end stops, stepping through a long list of choices in both
    /// directions and wrapping around at the ends.
    Endless,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueFormat {
    /// Text comes from `choice.<group>.<variant id>`.
    Choice {
        group: &'static str,
    },
    /// A position in the waveform catalog, shown as the waveform's name.
    Wave,
    /// A whole number with an explicit sign, like "+1".
    SignedInteger,
    Semitones,
    Cents,
    Frequency,
    Seconds,
    Percent,
    /// The parameter holds linear gain, shown in decibels.
    GainDecibels,
}

/// Type-erased access to a parameter, so the GUI can treat every control the same way.
pub trait Binding {
    fn normalized(&self) -> f32;
    fn step_count(&self) -> Option<usize>;
    /// The plain value as a number (the choice index for choices).
    fn number(&self) -> f64;
    /// The value a normalized position would have, e.g. for labelling switch positions.
    fn number_at(&self, normalized: f32) -> f64;
    /// Stable id of the selected choice, for choice parameters.
    fn choice_id(&self) -> Option<&'static str>;
    fn begin(&self, setter: &ParamSetter);
    fn set_normalized(&self, setter: &ParamSetter, normalized: f32);
    fn reset(&self, setter: &ParamSetter);
    fn end(&self, setter: &ParamSetter);
}

struct NumberBinding<'a, P: Param>(&'a P);

impl<P: Param> Binding for NumberBinding<'_, P>
where
    P::Plain: Into<f64>,
{
    fn normalized(&self) -> f32 {
        self.0.modulated_normalized_value()
    }
    fn step_count(&self) -> Option<usize> {
        self.0.step_count()
    }
    fn number(&self) -> f64 {
        self.0.modulated_plain_value().into()
    }
    fn number_at(&self, normalized: f32) -> f64 {
        self.0.preview_plain(normalized).into()
    }
    fn choice_id(&self) -> Option<&'static str> {
        None
    }
    fn begin(&self, setter: &ParamSetter) {
        setter.begin_set_parameter(self.0);
    }
    fn set_normalized(&self, setter: &ParamSetter, normalized: f32) {
        setter.set_parameter(self.0, self.0.preview_plain(normalized.clamp(0.0, 1.0)));
    }
    fn reset(&self, setter: &ParamSetter) {
        setter.set_parameter(self.0, self.0.default_plain_value());
    }
    fn end(&self, setter: &ParamSetter) {
        setter.end_set_parameter(self.0);
    }
}

struct ChoiceBinding<'a, T: Enum + PartialEq + 'static>(&'a EnumParam<T>);

impl<T: Enum + PartialEq + 'static> Binding for ChoiceBinding<'_, T> {
    fn normalized(&self) -> f32 {
        self.0.modulated_normalized_value()
    }
    fn step_count(&self) -> Option<usize> {
        self.0.step_count()
    }
    fn number(&self) -> f64 {
        self.0.value().to_index() as f64
    }
    fn number_at(&self, normalized: f32) -> f64 {
        self.0.preview_plain(normalized).to_index() as f64
    }
    fn choice_id(&self) -> Option<&'static str> {
        T::ids().and_then(|ids| ids.get(self.0.value().to_index()).copied())
    }
    fn begin(&self, setter: &ParamSetter) {
        setter.begin_set_parameter(self.0);
    }
    fn set_normalized(&self, setter: &ParamSetter, normalized: f32) {
        setter.set_parameter(self.0, self.0.preview_plain(normalized.clamp(0.0, 1.0)));
    }
    fn reset(&self, setter: &ParamSetter) {
        setter.set_parameter(self.0, self.0.default_plain_value());
    }
    fn end(&self, setter: &ParamSetter) {
        setter.end_set_parameter(self.0);
    }
}

pub struct Control<'a> {
    /// Stable key: the label is `control.<key>` in the language pack, unless `label` overrides it.
    pub key: &'static str,
    /// A different text key for the label, for controls whose meaning changes.
    pub label: Option<String>,
    /// The control currently does nothing (e.g. Shape on a plain sine).
    pub inactive: bool,
    pub role: Role,
    pub format: ValueFormat,
    pub binding: Box<dyn Binding + 'a>,
}

impl Control<'_> {
    pub fn label_key(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| format!("control.{}", self.key))
    }

    /// Labels for each position of a stepped control, e.g. "-2" .. "+2".
    pub fn position_labels(&self, s: &Strings) -> Vec<String> {
        let steps = self.binding.step_count().unwrap_or(0);
        (0..=steps)
            .map(|i| {
                let normalized = i as f32 / steps.max(1) as f32;
                format_value(self.format, self.binding.number_at(normalized), None, s)
            })
            .collect()
    }

    pub fn display_value(&self, s: &Strings) -> String {
        if self.inactive {
            return s.text("value.inactive");
        }
        format_value(
            self.format,
            self.binding.number(),
            self.binding.choice_id(),
            s,
        )
    }
}

pub struct Section<'a> {
    /// Stable key: the title is `section.<key>` in the language pack.
    pub key: &'static str,
    pub slots: Vec<Slot<'a>>,
}

/// One column in a section: a knob, optionally with a smaller control attached above it.
pub struct Slot<'a> {
    pub control: Control<'a>,
    pub above: Option<Control<'a>>,
}

impl<'a> From<Control<'a>> for Slot<'a> {
    fn from(control: Control<'a>) -> Self {
        Self {
            control,
            above: None,
        }
    }
}

impl<'a> Slot<'a> {
    fn with_above(control: Control<'a>, above: Control<'a>) -> Self {
        Self {
            control,
            above: Some(above),
        }
    }

    /// Every control in this slot.
    pub fn controls(&self) -> impl Iterator<Item = &Control<'a>> {
        std::iter::once(&self.control).chain(&self.above)
    }
}

fn number<'a, P: Param>(key: &'static str, role: Role, format: ValueFormat, p: &'a P) -> Control<'a>
where
    P::Plain: Into<f64>,
{
    Control {
        key,
        role,
        format,
        label: None,
        inactive: false,
        binding: Box::new(NumberBinding(p)),
    }
}

fn choice<'a, T: Enum + PartialEq + 'static>(
    key: &'static str,
    group: &'static str,
    p: &'a EnumParam<T>,
) -> Control<'a> {
    Control {
        key,
        role: Role::Endless,
        format: ValueFormat::Choice { group },
        label: None,
        inactive: false,
        binding: Box::new(ChoiceBinding(p)),
    }
}

/// The Shape knob is labelled for what it does on the current waveform, e.g. "Width" on a pulse.
fn shape_control(p: &SimpleOscParams) -> Control<'_> {
    let role = waves::catalog()
        .get(p.waveform.value() as usize)
        .and_then(|w| w.shape);
    Control {
        label: role.map(|r| format!("shape.{}", r.id())),
        inactive: role.is_none(),
        ..number("shape", Role::Continuous, ValueFormat::Percent, &p.shape)
    }
}

/// The synth's controls, grouped the way they're shown.
pub fn sections(p: &SimpleOscParams) -> Vec<Section<'_>> {
    use Role::*;
    use ValueFormat::*;
    vec![
        Section {
            key: "oscillator",
            slots: vec![
                number("wave", Endless, Wave, &p.waveform).into(),
                shape_control(p).into(),
                Slot::with_above(
                    number("coarse", Bipolar, Semitones, &p.coarse),
                    number("octave", Switch, SignedInteger, &p.octave),
                ),
                number("fine", Bipolar, Cents, &p.fine).into(),
            ],
        },
        Section {
            key: "filter",
            slots: vec![
                choice("type", "filter", &p.filter_type).into(),
                number("cutoff", Continuous, Frequency, &p.cutoff).into(),
                number("reso", Continuous, Percent, &p.resonance).into(),
            ],
        },
        Section {
            key: "envelope",
            slots: vec![
                number("attack", Continuous, Seconds, &p.attack).into(),
                number("decay", Continuous, Seconds, &p.decay).into(),
                number("sustain", Continuous, Percent, &p.sustain).into(),
                number("release", Continuous, Seconds, &p.release).into(),
            ],
        },
        Section {
            key: "output",
            slots: vec![number("volume", Continuous, GainDecibels, &p.volume).into()],
        },
    ]
}

pub fn format_value(
    format: ValueFormat,
    value: f64,
    choice_id: Option<&str>,
    s: &Strings,
) -> String {
    let with_unit = |number: String, unit: &str| format!("{number}{}", s.text(unit));
    match format {
        ValueFormat::Choice { group } => match choice_id {
            Some(id) => s.text(&format!("choice.{group}.{id}")),
            None => s.number(value, 0),
        },
        ValueFormat::Wave => wave_name(value as usize, s),
        ValueFormat::SignedInteger if value > 0.5 => format!("+{}", s.number(value, 0)),
        ValueFormat::SignedInteger => s.number(value, 0),
        ValueFormat::Semitones => with_unit(s.number(value, 0), "unit.semitones"),
        ValueFormat::Cents => with_unit(s.number(value, 0), "unit.cents"),
        ValueFormat::Frequency if value < 1000.0 => with_unit(s.number(value, 0), "unit.hz"),
        ValueFormat::Frequency => with_unit(s.number(value / 1000.0, 1), "unit.khz"),
        ValueFormat::Seconds if value < 1.0 => with_unit(s.number(value * 1000.0, 0), "unit.ms"),
        ValueFormat::Seconds => with_unit(s.number(value, 2), "unit.s"),
        ValueFormat::Percent => with_unit(s.number(value * 100.0, 0), "unit.percent"),
        ValueFormat::GainDecibels => with_unit(
            s.number(util::gain_to_db(value as f32) as f64, 1),
            "unit.db",
        ),
    }
}

/// A waveform's translated name, falling back to its built-in English name.
pub fn wave_name(index: usize, s: &Strings) -> String {
    match waves::catalog().get(index) {
        Some(w) => s.text_or(&format!("wave.{}", w.id), w.name),
        None => String::new(),
    }
}

pub fn category_name(category: Category, s: &Strings) -> String {
    s.text(&format!("category.{}", category.id()))
}

/// The order the Wave knob steps through: by category, then alphabetically by translated name.
pub fn wave_order(s: &Strings) -> Vec<usize> {
    let mut order: Vec<(Category, String, usize)> = waves::catalog()
        .iter()
        .enumerate()
        .map(|(i, w)| (w.category, wave_name(i, s).to_lowercase(), i))
        .collect();
    order.sort();
    order.into_iter().map(|(_, _, i)| i).collect()
}
