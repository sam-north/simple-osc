use nih_plug::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::Arc;

mod dsp;
mod logo;
mod params;
mod shared;
mod ui;
mod waves;

use dsp::{Adsr, Filter, NoteStack};
pub use params::SimpleOscParams;
use shared::Shared;
use waves::Oscillator;

/// Velocity used for notes played from the GUI.
const GUI_VELOCITY: f32 = 0.8;
/// Smooths out level jumps when sliding between notes with different velocities.
const VELOCITY_SMOOTH_SECONDS: f32 = 0.005;

pub struct SimpleOsc {
    params: Arc<SimpleOscParams>,
    shared: Arc<Shared>,

    sample_rate: f32,
    osc: Oscillator,
    filter: Filter,
    env: Adsr,
    notes: NoteStack,
    /// The note the oscillator plays. Kept after release so the tail keeps its pitch.
    note: u8,
    velocity_target: f32,
    velocity: f32,
    velocity_coef: f32,
    /// GUI-held notes as of the previous block, to turn changes into note on/off events.
    prev_gui_notes: [u64; 2],
    /// (note, semitone offset, frequency) of the last pitch calculation.
    pitch_cache: (u8, f32, f32),
}

impl Default for SimpleOsc {
    fn default() -> Self {
        Self {
            params: Arc::new(SimpleOscParams::default()),
            shared: Arc::new(Shared::new()),
            sample_rate: 48000.0,
            osc: Oscillator::new(),
            filter: Filter::new(),
            env: Adsr::new(),
            notes: NoteStack::new(),
            note: 60,
            velocity_target: 0.0,
            velocity: 0.0,
            velocity_coef: 1.0,
            prev_gui_notes: [0; 2],
            pitch_cache: (0, f32::NAN, 0.0),
        }
    }
}

impl SimpleOsc {
    fn note_on(&mut self, note: u8, velocity: f32) {
        let was_held = self.notes.top().is_some();
        self.notes.push(note);
        self.note = note;
        self.velocity_target = velocity;
        // Retrigger the envelope unless we're sliding from a note that is still held (legato)
        if !was_held {
            self.env.gate_on();
        }
    }

    fn note_off(&mut self, note: u8) {
        self.notes.remove(note);
        match self.notes.top() {
            Some(prev) => self.note = prev,
            None => self.env.gate_off(),
        }
    }

    /// Turns changes in the GUI's held-note bitmask into note on/off events.
    fn apply_gui_notes(&mut self) {
        let current = self.shared.gui_notes();
        for word in 0..2 {
            let changed = current[word] ^ self.prev_gui_notes[word];
            let mut bits = changed;
            while bits != 0 {
                let bit = bits.trailing_zeros();
                bits &= bits - 1;
                let note = (word as u32 * 64 + bit) as u8;
                if current[word] & (1 << bit) != 0 {
                    self.note_on(note, GUI_VELOCITY);
                } else {
                    self.note_off(note);
                }
            }
        }
        self.prev_gui_notes = current;
    }
}

impl Plugin for SimpleOsc {
    const NAME: &'static str = "Simple Osc";
    const VENDOR: &'static str = "Sam North";
    const URL: &'static str = "https://github.com/sam-north/simple-osc";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create_editor(self.params.clone(), self.shared.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.velocity_coef = 1.0 - (-1.0 / (VELOCITY_SMOOTH_SECONDS * self.sample_rate)).exp();
        true
    }

    fn reset(&mut self) {
        self.osc.reset(self.sample_rate);
        self.filter.reset(self.sample_rate);
        self.env.reset();
        self.notes.clear();
        self.velocity = 0.0;
        self.prev_gui_notes = [0; 2];
        self.shared.sounding_note.store(-1, Ordering::Relaxed);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sr = self.sample_rate;
        let p = &self.params;
        self.env.set_params(
            p.attack.value(),
            p.decay.value(),
            p.sustain.value(),
            p.release.value(),
            sr,
        );
        self.osc.set_waveform(p.waveform.value() as usize);
        self.filter.set_type(p.filter_type.value());
        let transpose = (p.octave.value() * 12 + p.coarse.value()) as f32;
        let scope_enabled = p.editor_state.is_open();

        self.apply_gui_notes();

        let mut next_event = context.next_event();

        // Fast path: nothing is sounding and nothing will start this block, so write silence and
        // skip the DSP entirely. The smoothers still advance so their timing stays right.
        if self.env.is_idle() && self.notes.top().is_none() && next_event.is_none() {
            for channel in buffer.as_slice() {
                channel.fill(0.0);
            }
            let steps = buffer.samples() as u32;
            let p = &self.params;
            for param in [&p.fine, &p.shape, &p.cutoff, &p.resonance, &p.volume] {
                param.smoothed.next_step(steps);
            }
            self.velocity = self.velocity_target;
            self.shared.set_voice_active(false);
            self.shared.sounding_note.store(-1, Ordering::Relaxed);
            return ProcessStatus::KeepAlive;
        }

        let shared = self.shared.clone();
        let mut scope = shared.scope_writer();

        for (sample_idx, channel_samples) in buffer.iter_samples().enumerate() {
            // MIDI is applied at the exact sample it was timestamped with
            while let Some(event) = next_event {
                if event.timing() > sample_idx as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => self.note_on(note, velocity),
                    NoteEvent::NoteOff { note, .. } => self.note_off(note),
                    _ => (),
                }
                next_event = context.next_event();
            }

            // The pitch math (two exponentials) only reruns when the note or tuning changes
            let semitones = transpose + self.params.fine.smoothed.next() / 100.0;
            if self.note != self.pitch_cache.0 || semitones != self.pitch_cache.1 {
                let freq = util::midi_note_to_freq(self.note) * (semitones / 12.0).exp2();
                self.pitch_cache = (self.note, semitones, freq);
            }
            let freq = self.pitch_cache.2;
            let phase = self.osc.phase();

            self.velocity += (self.velocity_target - self.velocity) * self.velocity_coef;
            let raw = self.osc.next(freq, self.params.shape.smoothed.next(), sr);
            let filtered = self.filter.process(
                raw,
                self.params.cutoff.smoothed.next(),
                self.params.resonance.smoothed.next(),
                sr,
            );
            let voice = filtered * self.env.next() * self.velocity;
            let out = voice * self.params.volume.smoothed.next();

            for sample in channel_samples {
                *sample = out;
            }
            // The scope shows the voice before the volume knob, so turning down doesn't shrink it
            if scope_enabled {
                scope.push(voice, phase);
            }
        }
        scope.finish();

        let sounding = match self.notes.top() {
            Some(note) => note as i32,
            None => -1,
        };
        self.shared.sounding_note.store(sounding, Ordering::Relaxed);
        self.shared.set_voice_active(!self.env.is_idle());

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for SimpleOsc {
    const CLAP_ID: &'static str = "com.snorth.simple-osc";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A single-oscillator synth");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for SimpleOsc {
    const VST3_CLASS_ID: [u8; 16] = *b"SimpleOscSynth01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(SimpleOsc);
nih_export_vst3!(SimpleOsc);
