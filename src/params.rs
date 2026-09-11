use nice_plug::prelude::*;

#[derive(Params)]
pub struct ShadeParams {
    #[id = "gain"]
    pub output_gain: FloatParam,
    #[id = "min_strike"]
    pub minimum_strike_velocity: FloatParam,
    #[id = "max_strike"]
    pub maximum_strike_velocity: FloatParam,
    #[id = "damper"]
    pub damper_release_ms: FloatParam,
    #[id = "hardness"]
    pub hammer_hardness: FloatParam,
    #[id = "hardness_p"]
    pub hammer_hardness_piano: FloatParam,
    #[id = "hardness_m"]
    pub hammer_hardness_mezzo: FloatParam,
    #[id = "hardness_f"]
    pub hammer_hardness_forte: FloatParam,
    #[id = "hammer_noise_min"]
    pub hammer_noise_min: FloatParam,
    #[id = "hammer_noise_max"]
    pub hammer_noise_max: FloatParam,
    #[id = "hammer_tone"]
    pub hammer_tone: FloatParam,
    #[id = "soft_pedal"]
    pub soft_pedal: FloatParam,
    #[id = "unison_width_min"]
    pub unison_width_min: FloatParam,
    #[id = "unison_width_max"]
    pub unison_width_max: FloatParam,
    #[id = "string_length"]
    pub string_length: FloatParam,
    #[id = "strike_point"]
    pub strike_point: FloatParam,
    #[id = "sympathetic"]
    pub sympathetic_resonance: FloatParam,
    #[id = "duplex"]
    pub duplex_scale_resonance: FloatParam,
    #[id = "bloom_energy"]
    pub blooming_energy: FloatParam,
    #[id = "bloom_inertia"]
    pub blooming_inertia: FloatParam,
}

impl Default for ShadeParams {
    fn default() -> Self {
        Self {
            output_gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            minimum_strike_velocity: FloatParam::new(
                "Min Strike",
                0.5,
                FloatRange::Linear { min: 0.1, max: 2.0 },
            )
            .with_unit(" m/s"),
            maximum_strike_velocity: FloatParam::new(
                "Max Strike",
                4.0,
                FloatRange::Linear { min: 1.0, max: 8.0 },
            )
            .with_unit(" m/s"),
            damper_release_ms: FloatParam::new(
                "Damper",
                120.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            hammer_hardness: FloatParam::new(
                "Hammer Hardness",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            hammer_hardness_piano: FloatParam::new(
                "Hardness Piano",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            hammer_hardness_mezzo: FloatParam::new(
                "Hardness Mezzo",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            hammer_hardness_forte: FloatParam::new(
                "Hardness Forte",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            hammer_noise_min: FloatParam::new(
                "Hammer Noise Min",
                0.9,
                FloatRange::Linear { min: 0.1, max: 3.0 },
            )
            .with_step_size(0.01),
            hammer_noise_max: FloatParam::new(
                "Hammer Noise Max",
                1.1,
                FloatRange::Linear { min: 0.1, max: 3.0 },
            )
            .with_step_size(0.01),
            hammer_tone: FloatParam::new(
                "Hammer Tone",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            soft_pedal: FloatParam::new(
                "Soft Pedal",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            unison_width_min: FloatParam::new(
                "Unison Width Min",
                9.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 20.0,
                },
            )
            .with_step_size(0.1),
            unison_width_max: FloatParam::new(
                "Unison Width Max",
                10.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 20.0,
                },
            )
            .with_step_size(0.1),
            string_length: FloatParam::new(
                "String Length",
                1.0,
                FloatRange::Linear {
                    min: 0.8,
                    max: 10.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(" m"),
            strike_point: FloatParam::new(
                "Strike Point",
                0.122,
                FloatRange::Linear {
                    min: 1.0 / 64.0,
                    max: 0.5,
                },
            )
            .with_step_size(0.001)
            .with_unit(" x L"),
            sympathetic_resonance: FloatParam::new(
                "Sympathetic Resonance",
                1.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_step_size(0.01),
            duplex_scale_resonance: FloatParam::new(
                "Duplex Scale Resonance",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 20.0,
                },
            )
            .with_step_size(0.1),
            blooming_energy: FloatParam::new(
                "Blooming Energy",
                0.5,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            blooming_inertia: FloatParam::new(
                "Blooming Inertia",
                1.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 3.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" s"),
        }
    }
}
