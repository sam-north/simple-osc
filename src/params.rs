//! Host-facing parameters. Names here are what DAWs show for automation, so they stay stable,
//! English identifiers; the plugin's own GUI shows translated text from language packs instead.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

use crate::dsp::FilterType;
use crate::waves;

#[derive(Params)]
pub struct SimpleOscParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// Position in [`waves::catalog`].
    #[id = "waveform"]
    pub waveform: IntParam,
    /// The waveform's extra control (pulse width, detune, ...). Unused by some waveforms.
    #[id = "shape"]
    pub shape: FloatParam,
    #[id = "octave"]
    pub octave: IntParam,
    #[id = "coarse"]
    pub coarse: IntParam,
    #[id = "fine"]
    pub fine: FloatParam,

    #[id = "ftype"]
    pub filter_type: EnumParam<FilterType>,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "reso"]
    pub resonance: FloatParam,

    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "release"]
    pub release: FloatParam,

    #[id = "volume"]
    pub volume: FloatParam,
}

fn time_param(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min: 0.001,
            max: 5.0,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_value_to_string(Arc::new(|secs| {
        if secs < 1.0 {
            format!("{:.0} ms", secs * 1000.0)
        } else {
            format!("{secs:.2} s")
        }
    }))
}

impl Default for SimpleOscParams {
    fn default() -> Self {
        Self {
            editor_state: crate::ui::default_editor_state(),

            waveform: IntParam::new(
                "Waveform",
                waves::DEFAULT_WAVE as i32,
                IntRange::Linear {
                    min: 0,
                    max: waves::catalog().len() as i32 - 1,
                },
            )
            .with_value_to_string(Arc::new(|index| {
                waves::catalog()
                    .get(index as usize)
                    .map(|w| w.name.to_owned())
                    .unwrap_or_default()
            })),
            shape: FloatParam::new("Shape", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            octave: IntParam::new("Octave", 0, IntRange::Linear { min: -2, max: 2 }),
            coarse: IntParam::new("Coarse", 0, IntRange::Linear { min: -24, max: 24 })
                .with_unit(" st"),
            fine: FloatParam::new(
                "Fine",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit(" ct")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            filter_type: EnumParam::new("Filter Type", FilterType::Lowpass),
            cutoff: FloatParam::new(
                "Cutoff",
                5000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(20.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            resonance: FloatParam::new(
                "Resonance",
                0.15,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            attack: time_param("Attack", 0.005),
            decay: time_param("Decay", 0.3),
            sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            release: time_param("Release", 0.25),

            volume: FloatParam::new(
                "Volume",
                util::db_to_gain(-12.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-60.0),
                    max: util::db_to_gain(0.0),
                    factor: FloatRange::gain_skew_factor(-60.0, 0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}
