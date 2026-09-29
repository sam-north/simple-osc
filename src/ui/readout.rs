//! Live values for the GUI to display, read from the audio thread's shared state. Pure data: no
//! drawing and no text, so any renderer can use them.

use nih_plug::prelude::*;
use std::sync::atomic::Ordering;

use crate::shared::{Shared, SCOPE_LEN};
use crate::SimpleOscParams;

pub struct Sounding {
    /// The key being played.
    pub note: u8,
    /// The note actually heard, after the octave switch.
    pub pitch: u8,
    /// Frequency after the octave switch and both tuning knobs.
    pub frequency: f32,
}

/// The note currently gated on and what it sounds like after transposing.
pub fn sounding(params: &SimpleOscParams, shared: &Shared) -> Option<Sounding> {
    let note = shared.sounding_note.load(Ordering::Relaxed);
    let note = u8::try_from(note).ok()?;
    let octave_shift = params.octave.value() * 12;
    let semitones = (octave_shift + params.coarse.value()) as f32 + params.fine.value() / 100.0;
    Some(Sounding {
        note,
        pitch: (note as i32 + octave_shift).clamp(0, 127) as u8,
        frequency: util::midi_note_to_freq(note) * (semitones / 12.0).exp2(),
    })
}

/// `points + 1` evenly spaced samples covering exactly two cycles of the waveform, starting where
/// the oscillator's phase wrapped to zero. Locking to the phase (instead of looking for zero
/// crossings in the audio) keeps every waveform perfectly still. `None` if nothing has played.
pub fn scope_trace(shared: &Shared, points: usize) -> Option<Vec<f32>> {
    // Nothing is playing: show a flat line instead of the last note's leftover samples
    if !shared.voice_active() {
        return None;
    }
    let (start, span) = scope_window(shared)?;
    Some(
        (0..=points)
            .map(|k| {
                let x = start + span * k as f64 / points as f64;
                let i = x.floor() as usize;
                let frac = (x - x.floor()) as f32;
                shared.scope_sample(i) * (1.0 - frac) + shared.scope_sample(i + 1) * frac
            })
            .collect(),
    )
}

/// Returns the fractional start index and the length in samples of the stretch to show.
fn scope_window(shared: &Shared) -> Option<(f64, f64)> {
    let written = shared.scope_pos();
    if written < 4 {
        return None;
    }
    let last = written - 1;
    let increment =
        (shared.scope_phase(last) - shared.scope_phase(last - 1)).rem_euclid(1.0) as f64;
    if increment < 1e-7 {
        return None;
    }
    let period = 1.0 / increment;
    let span = (2.0 * period).min(SCOPE_LEN as f64 * 0.45);
    let oldest = written.saturating_sub(SCOPE_LEN) + 1;

    // The newest window that still fits. Search backwards from there for the latest phase wrap.
    let search_from = (last as f64 - span - 1.0).floor() as usize;
    let search_len = period.ceil() as usize + 2;
    if search_from < oldest + search_len {
        return None;
    }
    for i in (search_from - search_len..=search_from).rev() {
        let phase = shared.scope_phase(i);
        if phase < shared.scope_phase(i - 1) {
            // The wrap happened `phase / increment` samples before sample `i`
            return Some((i as f64 - phase as f64 / increment, span));
        }
    }
    // Very low notes may not wrap within the window. Just show the most recent samples.
    Some((search_from as f64, span))
}
