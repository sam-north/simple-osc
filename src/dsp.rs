//! Real-time DSP building blocks. Nothing in here allocates, locks, or does I/O, so it is all safe
//! to call from the audio thread.

use nih_plug::prelude::Enum;

/// How long switching filter types crossfades between the old and new one, to avoid clicks.
const XFADE_SECONDS: f32 = 0.005;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    #[id = "lowpass"]
    #[name = "Low Pass"]
    Lowpass,
    #[id = "highpass"]
    #[name = "High Pass"]
    Highpass,
    #[id = "bandpass"]
    #[name = "Band Pass"]
    Bandpass,
    #[id = "notch"]
    #[name = "Notch"]
    Notch,
}

/// Resonance 0..1 maps exponentially onto this Q range. 0.707 is the flattest (Butterworth)
/// response; 20 rings strongly without self-oscillating.
const MIN_Q: f32 = 0.707;
const MAX_Q: f32 = 20.0;

/// A 2-pole (12 dB/octave) state-variable filter using the topology-preserving transform (Andrew
/// Simper's SVF). It stays stable and click-free when cutoff and resonance move quickly, and
/// computes every filter type at once, so switching types can crossfade cheaply.
pub struct Filter {
    ic1eq: f32,
    ic2eq: f32,
    /// Inputs the coefficients below were computed for: (cutoff, resonance, sample rate).
    coeff_inputs: (f32, f32, f32),
    /// (k, a1, a2, a3). Recomputing them costs a `tan` and a `powf`, so it only happens when
    /// cutoff or resonance actually move.
    coeffs: (f32, f32, f32, f32),
    kind: FilterType,
    prev_kind: FilterType,
    xfade_remaining: u32,
    xfade_len: u32,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            ic1eq: 0.0,
            ic2eq: 0.0,
            coeff_inputs: (f32::NAN, f32::NAN, f32::NAN),
            coeffs: (0.0, 0.0, 0.0, 0.0),
            kind: FilterType::Lowpass,
            prev_kind: FilterType::Lowpass,
            xfade_remaining: 0,
            xfade_len: 1,
        }
    }

    pub fn reset(&mut self, sample_rate: f32) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
        self.xfade_remaining = 0;
        self.xfade_len = ((XFADE_SECONDS * sample_rate) as u32).max(1);
    }

    pub fn set_type(&mut self, kind: FilterType) {
        if kind != self.kind {
            self.prev_kind = self.kind;
            self.kind = kind;
            self.xfade_remaining = self.xfade_len;
        }
    }

    /// Filters one sample. `resonance` is 0..1.
    #[inline]
    pub fn process(&mut self, input: f32, cutoff: f32, resonance: f32, sample_rate: f32) -> f32 {
        if (cutoff, resonance, sample_rate) != self.coeff_inputs {
            self.coeff_inputs = (cutoff, resonance, sample_rate);
            let clamped = cutoff.clamp(10.0, sample_rate * 0.49);
            let g = (std::f32::consts::PI * clamped / sample_rate).tan();
            let k = 1.0 / (MIN_Q * (MAX_Q / MIN_Q).powf(resonance));
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            self.coeffs = (k, a1, a2, g * a2);
        }
        let (k, a1, a2, a3) = self.coeffs;
        let v3 = input - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        let select = |kind| match kind {
            FilterType::Lowpass => v2,
            FilterType::Highpass => input - k * v1 - v2,
            // Scaled by `k` so the peak stays at unity gain instead of growing with resonance
            FilterType::Bandpass => k * v1,
            FilterType::Notch => input - k * v1,
        };

        let mut out = select(self.kind);
        if self.xfade_remaining > 0 {
            let mix = self.xfade_remaining as f32 / self.xfade_len as f32;
            out = out * (1.0 - mix) + select(self.prev_kind) * mix;
            self.xfade_remaining -= 1;
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Envelopes below this level count as silent (-80 dB).
const SILENCE: f32 = 1e-4;
/// Decay and release times are measured as the time to cover 99.9% (-60 dB) of the distance.
const LN_1000: f32 = 6.907_755;

/// ADSR envelope with a linear attack and exponential decay and release, which sounds natural
/// to the ear. Retriggering starts the attack from the current level, so there is no click.
pub struct Adsr {
    stage: Stage,
    level: f32,
    attack_step: f32,
    decay_coef: f32,
    release_coef: f32,
    sustain: f32,
}

impl Adsr {
    pub fn new() -> Self {
        Self {
            stage: Stage::Idle,
            level: 0.0,
            attack_step: 1.0,
            decay_coef: 0.0,
            release_coef: 0.0,
            sustain: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }

    /// Times are in seconds. Cheap enough to call once per audio block.
    pub fn set_params(&mut self, attack: f32, decay: f32, sustain: f32, release: f32, sr: f32) {
        self.attack_step = 1.0 / (attack * sr).max(1.0);
        self.decay_coef = (-LN_1000 / (decay * sr).max(1.0)).exp();
        self.release_coef = (-LN_1000 / (release * sr).max(1.0)).exp();
        self.sustain = sustain;
    }

    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += self.attack_step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = self.sustain + (self.level - self.sustain) * self.decay_coef;
                if (self.level - self.sustain).abs() < SILENCE {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {
                // Follows the knob if sustain is changed while a note is held
                self.level = self.sustain + (self.level - self.sustain) * self.decay_coef;
            }
            Stage::Release => {
                self.level *= self.release_coef;
                if self.level < SILENCE {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

/// Held notes for a monophonic synth with last-note priority. When the newest note is released,
/// the synth falls back to the most recent note that is still held, like a classic mono synth.
pub struct NoteStack {
    notes: [u8; 128],
    len: usize,
}

impl NoteStack {
    pub fn new() -> Self {
        Self {
            notes: [0; 128],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn top(&self) -> Option<u8> {
        self.len.checked_sub(1).map(|i| self.notes[i])
    }

    pub fn push(&mut self, note: u8) {
        self.remove(note);
        if self.len < self.notes.len() {
            self.notes[self.len] = note;
            self.len += 1;
        }
    }

    pub fn remove(&mut self, note: u8) {
        if let Some(i) = self.notes[..self.len].iter().position(|&n| n == note) {
            self.notes.copy_within(i + 1..self.len, i);
            self.len -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// Peak output level after the filter settles, for a sine at `freq`.
    fn filtered_peak(kind: FilterType, freq: f32, cutoff: f32, resonance: f32) -> f32 {
        let sr = 48000.0;
        let mut filter = Filter::new();
        filter.reset(sr);
        filter.set_type(kind);
        let mut peak = 0.0f32;
        for i in 0..48000 {
            let x = (TAU * freq * i as f32 / sr).sin();
            let y = filter.process(x, cutoff, resonance, sr);
            if i > 24000 {
                peak = peak.max(y.abs());
            }
        }
        peak
    }

    #[test]
    fn filter_types_pass_and_reject_the_right_frequencies() {
        assert!(filtered_peak(FilterType::Lowpass, 100.0, 1000.0, 0.0) > 0.95);
        assert!(filtered_peak(FilterType::Lowpass, 10000.0, 1000.0, 0.0) < 0.02);
        assert!(filtered_peak(FilterType::Highpass, 10000.0, 1000.0, 0.0) > 0.95);
        assert!(filtered_peak(FilterType::Highpass, 100.0, 1000.0, 0.0) < 0.02);
        assert!(filtered_peak(FilterType::Bandpass, 1000.0, 1000.0, 0.5) > 0.95);
        assert!(filtered_peak(FilterType::Bandpass, 100.0, 1000.0, 0.5) < 0.1);
        assert!(filtered_peak(FilterType::Notch, 1000.0, 1000.0, 0.0) < 0.05);
        assert!(filtered_peak(FilterType::Notch, 100.0, 1000.0, 0.0) > 0.95);
    }

    #[test]
    fn filter_stays_stable_at_max_resonance_and_extreme_cutoff() {
        for cutoff in [10.0, 1000.0, 23000.0] {
            let peak = filtered_peak(FilterType::Lowpass, cutoff, cutoff, 1.0);
            assert!(peak.is_finite() && peak < 25.0, "cutoff {cutoff}: {peak}");
        }
    }

    #[test]
    fn envelope_runs_full_cycle() {
        let sr = 1000.0;
        let mut env = Adsr::new();
        env.set_params(0.01, 0.1, 0.5, 0.1, sr);
        env.gate_on();
        for _ in 0..500 {
            env.next();
        }
        assert!((env.next() - 0.5).abs() < 1e-3);
        env.gate_off();
        for _ in 0..500 {
            env.next();
        }
        assert!(env.is_idle());
    }

    #[test]
    fn note_stack_falls_back_to_previous_note() {
        let mut stack = NoteStack::new();
        stack.push(60);
        stack.push(64);
        stack.push(67);
        stack.remove(67);
        assert_eq!(stack.top(), Some(64));
        stack.remove(60);
        assert_eq!(stack.top(), Some(64));
        stack.remove(64);
        assert_eq!(stack.top(), None);
    }
}
