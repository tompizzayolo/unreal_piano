use nice_plug::prelude::*;

#[derive(Params)]
pub struct PianoParams {
    #[id = "out_gain"]
    pub out_gain: FloatParam,
    #[id = "hammer_stiffness"]
    pub hammer_stiffness: FloatParam,
    #[id = "hammer_exponent"]
    pub hammer_exponent: FloatParam,
    #[id = "hammer_hysteresis"]
    pub hammer_hysteresis: FloatParam,
    #[id = "string_decay"]
    pub string_decay: FloatParam,
    #[id = "inharmonicity"]
    pub inharmonicity: FloatParam,
    #[id = "sb_coupling"]
    pub sb_coupling: FloatParam,
    #[id = "sb_decay"]
    pub sb_decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
}

impl Default for PianoParams {
    fn default() -> Self {
        Self {
            out_gain: FloatParam::new(
                "Output",
                util::db_to_gain(-12.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(6.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            hammer_stiffness: FloatParam::new(
                "Hammer Stiffness",
                60_000.0,
                FloatRange::Skewed {
                    min: 5_000.0,
                    max: 500_000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_unit(" N/m^p"),

            hammer_exponent: FloatParam::new(
                "Hammer Exponent",
                2.5,
                FloatRange::Linear { min: 1.5, max: 4.0 },
            )
            .with_step_size(0.01),

            hammer_hysteresis: FloatParam::new(
                "Hammer Hysteresis",
                0.15,
                FloatRange::Linear { min: 0.0, max: 0.9 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_step_size(0.01),

            string_decay: FloatParam::new(
                "String Decay",
                6.0,
                FloatRange::Skewed {
                    min: 0.5,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-0.5),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" s")
            .with_step_size(0.1),

            inharmonicity: FloatParam::new(
                "Inharmonicity",
                0.0004,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.005,
                },
            )
            .with_step_size(0.0001),

            sb_coupling: FloatParam::new(
                "Soundboard Coupling",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_step_size(0.01),

            sb_decay: FloatParam::new(
                "Soundboard Decay",
                1.2,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-0.5),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" s")
            .with_step_size(0.1),

            // 0.0 = off, >= 0.5 = on. Using a float so we don't depend on BoolParam.
            sustain: FloatParam::new("Sustain", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(1.0),
        }
    }
}

/// Snapshot of smoothed parameter values, evaluated once per sample.
pub struct ParamSnapshot {
    pub output_gain: f32,
    pub hammer_stiffness: f32,
    pub hammer_exponent: f32,
    pub hammer_hysteresis: f32,
    pub string_decay: f32,
    pub soundboard_coupling: f32,
    pub soundboard_decay: f32,
    pub sustain_held: bool,
}

impl PianoParams {
    pub fn next_snapshot(&self) -> ParamSnapshot {
        ParamSnapshot {
            output_gain: self.out_gain.smoothed.next(),
            hammer_stiffness: self.hammer_stiffness.smoothed.next(),
            hammer_exponent: self.hammer_exponent.value(),
            hammer_hysteresis: self.hammer_hysteresis.smoothed.next(),
            string_decay: self.string_decay.smoothed.next(),
            soundboard_coupling: self.sb_coupling.smoothed.next(),
            soundboard_decay: self.sb_decay.smoothed.next(),
            sustain_held: self.sustain.value() >= 0.5,
        }
    }
}
