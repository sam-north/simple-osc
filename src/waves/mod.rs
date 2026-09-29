//! Every waveform the oscillator can play, and the oscillator itself.
//!
//! The [`catalog`] is the one list of waveforms. Its order is what hosts save (the waveform
//! parameter is a position in it), so entries are only ever appended. How the GUI orders and
//! names them (by category, then alphabetically, translated) is decided in the UI.

pub mod akwf;
pub mod basic;
pub mod wavetable;

use std::sync::OnceLock;

use basic::Basic;
use wavetable::Wavetable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Basic,
    AdventureKid,
}

impl Category {
    /// Stable id, used for the category's text key.
    pub fn id(self) -> &'static str {
        match self {
            Category::Basic => "basic",
            Category::AdventureKid => "akwf",
        }
    }
}

/// What the Shape knob does for a waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeRole {
    Width,
    Detune,
    Color,
    Amount,
    Fold,
}

impl ShapeRole {
    pub fn id(self) -> &'static str {
        match self {
            ShapeRole::Width => "width",
            ShapeRole::Detune => "detune",
            ShapeRole::Color => "color",
            ShapeRole::Amount => "amount",
            ShapeRole::Fold => "fold",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Basic(Basic),
    /// Index into [`tables`].
    Table(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct Wave {
    /// Stable id: the GUI looks up `wave.<id>` for a translated name.
    pub id: &'static str,
    /// English name, used by hosts and when no translation exists.
    pub name: &'static str,
    pub category: Category,
    pub source: Source,
    pub shape: Option<ShapeRole>,
}

const BASIC_WAVES: &[Wave] = &[
    basic("sine", "Sine", Basic::Sine, None),
    basic("triangle", "Triangle", Basic::Triangle, None),
    basic("saw", "Saw", Basic::Saw, None),
    basic("square", "Square", Basic::Square, None),
    basic("pulse", "Pulse", Basic::Pulse, Some(ShapeRole::Width)),
    basic(
        "supersaw",
        "Supersaw",
        Basic::Supersaw,
        Some(ShapeRole::Detune),
    ),
    basic("noise", "Noise", Basic::Noise, Some(ShapeRole::Color)),
    basic(
        "phase_dist",
        "Phase Dist",
        Basic::PhaseDistortion,
        Some(ShapeRole::Amount),
    ),
    basic("fold", "Fold", Basic::Fold, Some(ShapeRole::Fold)),
];

const fn basic(
    id: &'static str,
    name: &'static str,
    wave: Basic,
    shape: Option<ShapeRole>,
) -> Wave {
    Wave {
        id,
        name,
        category: Category::Basic,
        source: Source::Basic(wave),
        shape,
    }
}

/// The waveform the synth starts with.
pub const DEFAULT_WAVE: usize = 2;

/// All waveforms, in saved (append-only) order.
pub fn catalog() -> &'static [Wave] {
    static CATALOG: OnceLock<Vec<Wave>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let tables = akwf::WAVES
            .iter()
            .enumerate()
            .map(|(i, (id, name, _))| Wave {
                id,
                name,
                category: Category::AdventureKid,
                source: Source::Table(i),
                shape: None,
            });
        BASIC_WAVES.iter().copied().chain(tables).collect()
    })
}

/// The band-limited tables for the sample-based waves. Built once, the first time an
/// oscillator is created (never on the audio thread).
fn tables() -> &'static [Wavetable] {
    static TABLES: OnceLock<Vec<Wavetable>> = OnceLock::new();
    TABLES.get_or_init(|| {
        akwf::WAVES
            .iter()
            .map(|(id, _, bytes)| {
                let cycle =
                    akwf::decode_wav(bytes).unwrap_or_else(|e| panic!("built-in {id}: {e}"));
                Wavetable::from_cycle(&cycle)
            })
            .collect()
    })
}

/// How long switching waveforms crossfades between the old and new one, to avoid clicks.
const XFADE_SECONDS: f32 = 0.005;

/// A single oscillator that can play any waveform in the catalog.
pub struct Oscillator {
    /// Normalized phase in `[0, 1)`. Kept in `f64` so long notes don't drift out of tune.
    phase: f64,
    state: basic::State,
    catalog: &'static [Wave],
    tables: &'static [Wavetable],
    wave: usize,
    prev_wave: usize,
    xfade_remaining: u32,
    xfade_len: u32,
    /// Whether the current or fading-out waveform is a supersaw, whose side voices then need
    /// advancing every sample.
    side_voices: bool,
}

impl Oscillator {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            state: basic::State::new(),
            catalog: catalog(),
            tables: tables(),
            wave: DEFAULT_WAVE,
            prev_wave: DEFAULT_WAVE,
            xfade_remaining: 0,
            xfade_len: 1,
            side_voices: false,
        }
    }

    pub fn reset(&mut self, sample_rate: f32) {
        self.phase = 0.0;
        self.xfade_remaining = 0;
        self.xfade_len = ((XFADE_SECONDS * sample_rate) as u32).max(1);
    }

    pub fn phase(&self) -> f32 {
        self.phase as f32
    }

    /// Selects a waveform by catalog index. Out-of-range indices are ignored.
    pub fn set_waveform(&mut self, wave: usize) {
        if wave != self.wave && wave < self.catalog.len() {
            self.prev_wave = self.wave;
            self.wave = wave;
            self.xfade_remaining = self.xfade_len;
            let supersaw =
                |i: usize| matches!(self.catalog[i].source, Source::Basic(Basic::Supersaw));
            self.side_voices = supersaw(self.wave) || supersaw(self.prev_wave);
        }
    }

    /// Renders one sample at `freq` Hz and advances the phase. `shape` is 0..1.
    #[inline]
    pub fn next(&mut self, freq: f32, shape: f32, sample_rate: f32) -> f32 {
        // Keep the phase increment below 0.5 (Nyquist). The BLEP/BLAMP corrections also assume at
        // most one discontinuity per sample.
        let dt = (freq / sample_rate).clamp(0.0, 0.45);
        let t = self.phase as f32;

        let mut out = self.render(self.wave, t, dt, shape);
        if self.xfade_remaining > 0 {
            let mix = self.xfade_remaining as f32 / self.xfade_len as f32;
            out = out * (1.0 - mix) + self.render(self.prev_wave, t, dt, shape) * mix;
            self.xfade_remaining -= 1;
        }

        if self.side_voices {
            self.state.advance(dt, shape);
        }
        self.phase += dt as f64;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        out
    }

    #[inline]
    fn render(&mut self, wave: usize, t: f32, dt: f32, shape: f32) -> f32 {
        match self.catalog[wave].source {
            Source::Basic(b) => basic::render(b, t, dt, shape, &mut self.state),
            Source::Table(i) => self.tables[i].sample(t, dt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let ids: std::collections::HashSet<_> = catalog().iter().map(|w| w.id).collect();
        assert_eq!(ids.len(), catalog().len());
    }

    #[test]
    fn every_wave_stays_in_range_at_every_shape() {
        for (index, wave) in catalog().iter().enumerate() {
            for shape in [0.0, 0.5, 1.0] {
                for freq in [55.0, 880.0, 7040.0] {
                    let mut osc = Oscillator::new();
                    osc.reset(48000.0);
                    osc.set_waveform(index);
                    let peak = (0..24000)
                        .map(|_| osc.next(freq, shape, 48000.0).abs())
                        .fold(0.0, f32::max);
                    assert!(
                        peak.is_finite() && peak <= 1.6,
                        "{} shape {shape} at {freq} Hz: {peak}",
                        wave.id
                    );
                    assert!(
                        peak > 0.05,
                        "{} is silent at shape {shape}, {freq} Hz",
                        wave.id
                    );
                }
            }
        }
    }
}
