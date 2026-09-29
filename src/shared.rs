//! Lock-free state shared between the audio thread and the GUI thread. The audio thread only
//! ever does atomic stores here, so it never waits on the GUI.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Number of samples kept for the oscilloscope. At 48 kHz this is ~0.34 s, enough to show two
/// full cycles of notes down to about 13 Hz (below the lowest MIDI note at the -2 octave setting,
/// the scope shows the most recent samples instead).
pub const SCOPE_LEN: usize = 1 << 14;
const SCOPE_MASK: usize = SCOPE_LEN - 1;

pub struct Shared {
    /// Output samples, stored as `f32` bits.
    scope_samples: Box<[AtomicU32]>,
    /// Oscillator phase for each sample, so the GUI can lock the display to the start of a cycle
    /// no matter which waveform is playing.
    scope_phases: Box<[AtomicU32]>,
    /// Total number of samples written. Only the audio thread writes this.
    scope_write_pos: AtomicUsize,

    /// Whether the synth is making sound (envelope not idle). The GUI only animates while it is.
    voice_active: AtomicBool,
    /// The MIDI note currently gated on, or -1.
    pub sounding_note: AtomicI32,
    /// Bitmask of the notes held on the GUI (computer keyboard or mouse). Two words cover all 128
    /// MIDI notes.
    pub gui_notes: [AtomicU64; 2],
}

impl Shared {
    pub fn new() -> Self {
        Self {
            scope_samples: (0..SCOPE_LEN).map(|_| AtomicU32::new(0)).collect(),
            scope_phases: (0..SCOPE_LEN).map(|_| AtomicU32::new(0)).collect(),
            scope_write_pos: AtomicUsize::new(0),
            voice_active: AtomicBool::new(false),
            sounding_note: AtomicI32::new(-1),
            gui_notes: [AtomicU64::new(0), AtomicU64::new(0)],
        }
    }

    pub fn voice_active(&self) -> bool {
        self.voice_active.load(Ordering::Relaxed)
    }

    pub fn set_voice_active(&self, active: bool) {
        self.voice_active.store(active, Ordering::Relaxed);
    }

    pub fn gui_notes(&self) -> [u64; 2] {
        [
            self.gui_notes[0].load(Ordering::Relaxed),
            self.gui_notes[1].load(Ordering::Relaxed),
        ]
    }

    pub fn set_gui_notes(&self, notes: [u64; 2]) {
        self.gui_notes[0].store(notes[0], Ordering::Relaxed);
        self.gui_notes[1].store(notes[1], Ordering::Relaxed);
    }

    /// Audio thread: returns a writer that tracks its own position and publishes it on `finish`.
    pub fn scope_writer(&self) -> ScopeWriter<'_> {
        ScopeWriter {
            shared: self,
            pos: self.scope_write_pos.load(Ordering::Relaxed),
        }
    }

    /// GUI thread: total samples written so far. Samples in `[pos - SCOPE_LEN, pos)` are valid.
    pub fn scope_pos(&self) -> usize {
        self.scope_write_pos.load(Ordering::Acquire)
    }

    pub fn scope_sample(&self, index: usize) -> f32 {
        f32::from_bits(self.scope_samples[index & SCOPE_MASK].load(Ordering::Relaxed))
    }

    pub fn scope_phase(&self, index: usize) -> f32 {
        f32::from_bits(self.scope_phases[index & SCOPE_MASK].load(Ordering::Relaxed))
    }
}

pub struct ScopeWriter<'a> {
    shared: &'a Shared,
    pos: usize,
}

impl ScopeWriter<'_> {
    #[inline]
    pub fn push(&mut self, sample: f32, phase: f32) {
        let i = self.pos & SCOPE_MASK;
        self.shared.scope_samples[i].store(sample.to_bits(), Ordering::Relaxed);
        self.shared.scope_phases[i].store(phase.to_bits(), Ordering::Relaxed);
        self.pos = self.pos.wrapping_add(1);
    }

    pub fn finish(self) {
        self.shared
            .scope_write_pos
            .store(self.pos, Ordering::Release);
    }
}
