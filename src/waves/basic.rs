//! Waveforms computed from math, with no sample data. Hard edges and corners are smoothed with
//! PolyBLEP / PolyBLAMP so high notes don't alias into harsh, inharmonic noise.

use std::f32::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basic {
    Sine,
    Triangle,
    Saw,
    Square,
    /// Square with adjustable width.
    Pulse,
    /// Seven detuned saws (Roland JP-8000 style).
    Supersaw,
    Noise,
    /// Casio CZ-style phase distortion of a cosine.
    PhaseDistortion,
    /// A sine driven into a wavefolder (West Coast style).
    Fold,
}

/// Relative detune of the six side saws of a supersaw, from Adam Szabo's analysis of the JP-8000.
const SUPERSAW_OFFSETS: [f32; 6] = [
    -0.110_023, -0.062_884, -0.019_524, 0.019_912, 0.062_165, 0.107_452,
];
/// Maximum detune, as a fraction of those offsets. Past this it stops sounding like one note.
const SUPERSAW_MAX_DETUNE: f32 = 0.5;

/// Per-oscillator state some waveforms need beyond the main phase.
pub struct State {
    /// Phases of the supersaw's side voices. Always advanced, so switching to the supersaw
    /// doesn't restart them in sync.
    side_phases: [f32; 6],
    rng: u32,
    noise_lp: f32,
}

impl State {
    pub fn new() -> Self {
        Self {
            // Spread out so the voices don't start phase-aligned
            side_phases: [0.13, 0.71, 0.37, 0.89, 0.52, 0.24],
            rng: 0x9e37_79b9,
            noise_lp: 0.0,
        }
    }

    /// Advances the side voices. Called once per sample.
    #[inline]
    pub fn advance(&mut self, dt: f32, shape: f32) {
        for (phase, offset) in self.side_phases.iter_mut().zip(SUPERSAW_OFFSETS) {
            *phase += dt * (1.0 + offset * shape * SUPERSAW_MAX_DETUNE);
            if *phase >= 1.0 {
                *phase -= 1.0;
            }
        }
    }

    #[inline]
    fn white(&mut self) -> f32 {
        // xorshift32: fast, allocation-free, good enough for audio noise
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One sample of `wave` at phase `t` with phase increment `dt`. `shape` (0..1) is the waveform's
/// extra control: pulse width, supersaw detune, noise color, distortion amount or fold amount.
#[inline]
pub fn render(wave: Basic, t: f32, dt: f32, shape: f32, state: &mut State) -> f32 {
    match wave {
        Basic::Sine => (t * TAU).sin(),
        Basic::Saw => saw(t, dt),
        Basic::Square => pulse(t, dt, 0.5),
        Basic::Triangle => {
            // Peaks at t = 0 (slope turns from +4 to -4) and bottoms out at t = 0.5 (slope turns
            // from -4 to +4). A slope change of 8 per cycle is `8 * dt` per sample.
            let naive = 4.0 * (t - 0.5).abs() - 1.0;
            naive + 8.0 * dt * (poly_blamp((t + 0.5).fract(), dt) - poly_blamp(t, dt))
        }
        Basic::Pulse => pulse(t, dt, 0.5 - 0.45 * shape),
        Basic::Supersaw => {
            // The side voices fade in with the detune, so at zero it's a plain saw
            let side_gain = 0.35 * shape.sqrt();
            let center_gain = 1.0 - 0.3 * shape;
            let sides: f32 = state
                .side_phases
                .iter()
                .zip(SUPERSAW_OFFSETS)
                .map(|(&p, offset)| saw(p, dt * (1.0 + offset * shape * SUPERSAW_MAX_DETUNE)))
                .sum();
            (center_gain * saw(t, dt) + side_gain * sides) / (center_gain + side_gain * 2.45)
        }
        Basic::Noise => {
            // Shape darkens white noise with a one-pole low-pass, scaled to keep the level even
            let pole = shape * 0.98;
            let white = state.white() * 0.5;
            state.noise_lp += (1.0 - pole) * (white - state.noise_lp);
            state.noise_lp * ((1.0 + pole) / (1.0 - pole)).sqrt()
        }
        Basic::PhaseDistortion => {
            // Squeezes the first half of the cycle into a shorter time, turning a cosine into a
            // brighter, resonant shape as `shape` rises
            let knee = 0.5 - 0.45 * shape;
            let warped = if t < knee {
                0.5 * t / knee
            } else {
                0.5 + 0.5 * (t - knee) / (1.0 - knee)
            };
            -(warped * TAU).cos()
        }
        Basic::Fold => {
            let drive = 1.0 + 5.0 * shape;
            (drive * PI / 2.0 * (t * TAU).sin()).sin()
        }
    }
}

#[inline]
fn saw(t: f32, dt: f32) -> f32 {
    2.0 * t - 1.0 - poly_blep(t, dt)
}

/// A pulse that is high for `width` of the cycle. Its DC offset is removed (so the envelope doesn't
/// thump), which lifts narrow pulses; they're scaled back so the peak stays at 1.
#[inline]
fn pulse(t: f32, dt: f32, width: f32) -> f32 {
    let naive = if t < width { 1.0 } else { -1.0 };
    let edges = poly_blep(t, dt) - poly_blep((t - width).rem_euclid(1.0), dt);
    (naive + edges - (2.0 * width - 1.0)) / (2.0 - 2.0 * width).max(1.0)
}

/// PolyBLEP residual for a downward step of height 2 at phase 0.
#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// PolyBLAMP residual for a unit (per-sample) slope increase at phase 0.
#[inline]
fn poly_blamp(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = 1.0 - t / dt;
        x * x * x / 6.0
    } else if t > 1.0 - dt {
        let x = 1.0 + (t - 1.0) / dt;
        x * x * x / 6.0
    } else {
        0.0
    }
}
