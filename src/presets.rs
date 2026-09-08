//! Factory voicing presets.
//!
//! Each preset is a named table of plain parameter values.  A value given as
//! a Min/Max pair defines a randomization range: the engine draws a fresh
//! value on every note-on, so repeated strikes of the same key never sound
//! identical.  Parameters not listed in a preset keep their current value —
//! presets voice the instrument, not the mix (gain, strike velocities and
//! the damper time are deliberately left alone).
//!
//! Adding a preset: append a `Preset { .. }` entry to `PRESETS` at the
//! bottom; a button appears in the editor automatically.  Values are plain
//! parameter values, so anything outside a parameter's range gets clamped
//! on apply.

use nice_plug::prelude::*;

use crate::UnrealPianoParams;

/// Identifies which parameter a GUI control edits or a preset assigns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdjustableParameter {
    OutputGain,
    MinimumStrike,
    MaximumStrike,
    Damper,
    HammerHardness,
    HammerHardnessPiano,
    HammerHardnessMezzo,
    HammerHardnessForte,
    HammerNoiseMinimum,
    HammerNoiseMaximum,
    HammerTone,
    SoftPedal,
    UnisonWidthMinimum,
    UnisonWidthMaximum,
    StringLength,
    StrikePoint,
    SympatheticResonance,
    DuplexScale,
    BloomingEnergy,
    BloomingInertia,
}

impl AdjustableParameter {
    /// The parameter this identifier refers to.
    pub fn param<'a>(&self, params: &'a UnrealPianoParams) -> &'a FloatParam {
        match self {
            AdjustableParameter::OutputGain => &params.output_gain,
            AdjustableParameter::MinimumStrike => &params.minimum_strike_velocity,
            AdjustableParameter::MaximumStrike => &params.maximum_strike_velocity,
            AdjustableParameter::Damper => &params.damper_release_ms,
            AdjustableParameter::HammerHardness => &params.hammer_hardness,
            AdjustableParameter::HammerHardnessPiano => &params.hammer_hardness_piano,
            AdjustableParameter::HammerHardnessMezzo => &params.hammer_hardness_mezzo,
            AdjustableParameter::HammerHardnessForte => &params.hammer_hardness_forte,
            AdjustableParameter::HammerNoiseMinimum => &params.hammer_noise_min,
            AdjustableParameter::HammerNoiseMaximum => &params.hammer_noise_max,
            AdjustableParameter::HammerTone => &params.hammer_tone,
            AdjustableParameter::SoftPedal => &params.soft_pedal,
            AdjustableParameter::UnisonWidthMinimum => &params.unison_width_min,
            AdjustableParameter::UnisonWidthMaximum => &params.unison_width_max,
            AdjustableParameter::StringLength => &params.string_length,
            AdjustableParameter::StrikePoint => &params.strike_point,
            AdjustableParameter::SympatheticResonance => &params.sympathetic_resonance,
            AdjustableParameter::DuplexScale => &params.duplex_scale_resonance,
            AdjustableParameter::BloomingEnergy => &params.blooming_energy,
            AdjustableParameter::BloomingInertia => &params.blooming_inertia,
        }
    }
}

/// One value assignment inside a preset.
pub struct PresetValue {
    /// The parameter to set.
    pub parameter: AdjustableParameter,
    /// The plain (non-normalized) value to set it to.
    pub value: f32,
}

/// One factory preset.
pub struct Preset {
    /// Button label and preset name.
    pub name: &'static str,
    /// The assignments applied when the preset is selected.
    pub values: &'static [PresetValue],
}

/// The factory preset bank.
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Felt 1",
        values: &[
            PresetValue { parameter: AdjustableParameter::HammerHardness, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessPiano, value: 0.30 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessMezzo, value: 0.81 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessForte, value: 1.46 },
            PresetValue { parameter: AdjustableParameter::HammerNoiseMinimum, value: 0.70 },
            PresetValue { parameter: AdjustableParameter::HammerNoiseMaximum, value: 1.58 },
            PresetValue { parameter: AdjustableParameter::HammerTone, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::SoftPedal, value: 0.30 },
            PresetValue { parameter: AdjustableParameter::UnisonWidthMinimum, value: 0.75 },
            PresetValue { parameter: AdjustableParameter::UnisonWidthMaximum, value: 3.82 },
            PresetValue { parameter: AdjustableParameter::StringLength, value: 2.78 },
            PresetValue { parameter: AdjustableParameter::StrikePoint, value: 1.0 / 9.0 },
            PresetValue { parameter: AdjustableParameter::SympatheticResonance, value: 1.0 },
            PresetValue { parameter: AdjustableParameter::DuplexScale, value: 1.0 },
            PresetValue { parameter: AdjustableParameter::BloomingEnergy, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::BloomingInertia, value: 1.0 },
        ],
    },
    Preset {
        name: "Felt 2",
        values: &[
            PresetValue { parameter: AdjustableParameter::HammerHardness, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessPiano, value: 0.28 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessMezzo, value: 0.65 },
            PresetValue { parameter: AdjustableParameter::HammerHardnessForte, value: 1.30 },
            PresetValue { parameter: AdjustableParameter::HammerNoiseMinimum, value: 0.62 },
            PresetValue { parameter: AdjustableParameter::HammerNoiseMaximum, value: 1.73 },
            PresetValue { parameter: AdjustableParameter::HammerTone, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::SoftPedal, value: 0.30 },
            PresetValue { parameter: AdjustableParameter::UnisonWidthMinimum, value: 0.60 },
            PresetValue { parameter: AdjustableParameter::UnisonWidthMaximum, value: 5.12 },
            PresetValue { parameter: AdjustableParameter::StringLength, value: 2.72 },
            PresetValue { parameter: AdjustableParameter::StrikePoint, value: 1.0 / 12.0 },
            PresetValue { parameter: AdjustableParameter::SympatheticResonance, value: 1.73 },
            PresetValue { parameter: AdjustableParameter::DuplexScale, value: 1.0 },
            PresetValue { parameter: AdjustableParameter::BloomingEnergy, value: 0.0 },
            PresetValue { parameter: AdjustableParameter::BloomingInertia, value: 1.0 },
        ],
    },
];
