use crate::engine::air::{AirRoom, AirRoomDesign};
use crate::engine::hammer::{FeltProperties, HammerGeometry};
use crate::engine::soundboard::{Soundboard, SoundboardDesign};
use crate::engine::strings::{DampingCoefficients, PianoStringDesign};
use crate::engine::vector::Vector3;
use crate::voice::{SUB_STEPS_DURING_STRIKE, SynthVoice};
use std::f64::consts::PI;

pub const SIMULATION_RATE_HZ: f64 = 48_000.0;
pub const SIMULATION_STEP_SIZE: f64 = 1.0 / SIMULATION_RATE_HZ;
pub const COUPLING_SWEEPS: usize = 1;

const RINGOUT_SECONDS: f64 = 0.75;

#[inline]
fn linear_interpolation(low: f64, high: f64, fraction: f64) -> f64 {
    low + (high - low) * fraction
}

#[derive(Clone, Copy)]
pub struct StrikeVoicingControls {
    pub hammer_hardness: f64,
    pub hammer_hardness_piano: f64,
    pub hammer_hardness_mezzo: f64,
    pub hammer_hardness_forte: f64,
    pub hammer_noise: f64,
    pub hammer_tone: f64,
    pub soft_pedal: f64,
    pub unison_width: f64,
    pub string_length_scale: f64,
    pub strike_point_ratio: f64,
}

#[derive(Clone, Copy)]
pub struct LiveVoicingControls {
    pub sympathetic_resonance: f64,
    pub duplex_scale: f64,
    pub blooming_energy: f64,
    pub blooming_inertia: f64,
}

impl Default for LiveVoicingControls {
    fn default() -> Self {
        LiveVoicingControls {
            sympathetic_resonance: 1.0,
            duplex_scale: 1.0,
            blooming_energy: 0.5,
            blooming_inertia: 1.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct VoicingBaked {
    pub hammer_force_scales: [f64; 3],
    pub lateral_misalignment_shift: f64,
    pub hammer_noise_amplitude: f64,
    pub hammer_noise_cutoff_hz: f64,
}

#[derive(Clone, Copy)]
pub struct NoteDesign {
    pub string: PianoStringDesign,
    pub felt: FeltProperties,
    pub number_of_unison_strings: usize,
    pub unison_relative_detunes: [f64; 3],
    pub unison_lateral_offsets: [f64; 3],
    pub fundamental_frequency: f64,
    pub voicing: VoicingBaked,
}

fn velocity_hardness_curve(
    velocity_blend: f64,
    hardness_piano: f64,
    hardness_mezzo: f64,
    hardness_forte: f64,
) -> f64 {
    if velocity_blend < 0.5 {
        linear_interpolation(hardness_piano, hardness_mezzo, velocity_blend * 2.0)
    } else {
        linear_interpolation(hardness_mezzo, hardness_forte, (velocity_blend - 0.5) * 2.0)
    }
}

fn railsback_stretch_cents(note: usize) -> f64 {
    let normalized = ((note as f64 - 60.0) / 48.0).clamp(-1.0, 1.0);
    30.0 * normalized + 8.0 * normalized * normalized * normalized
}

pub fn note_design(
    midi_note: u8,
    controls: &StrikeVoicingControls,
    velocity_blend: f64,
) -> NoteDesign {
    let note = midi_note.clamp(21, 108) as usize;
    let note_fraction = (note - 21) as f64 / 87.0;

    let nominal_fundamental_frequency = 440.0 * ((note as f64 - 69.0) / 12.0).exp2();

    let stretch_factor = 2.0_f64.powf(railsback_stretch_cents(note) / 1200.0);

    let fundamental_frequency = nominal_fundamental_frequency * stretch_factor;

    let length_scale = controls.string_length_scale.clamp(0.8, 10.0);

    let base_speaking_length = if note >= 60 {
        0.62 * (0.052_f64 / 0.62).powf((note - 60) as f64 / 48.0)
    } else {
        (0.62 * (261.6256 / nominal_fundamental_frequency).powf(0.90)).min(1.42)
    };

    let speaking_length = base_speaking_length * length_scale;
    let strike_ratio = controls.strike_point_ratio.clamp(1.0 / 64.0, 0.5);

    let core_radius = if note <= 45 {
        linear_interpolation(0.95e-3, 0.72e-3, (note - 21) as f64 / 24.0)
    } else if note <= 60 {
        linear_interpolation(0.72e-3, 0.475e-3, (note - 45) as f64 / 15.0)
    } else {
        linear_interpolation(0.475e-3, 0.33e-3, (note - 60) as f64 / 48.0)
    };

    let tension = linear_interpolation(750.0, 550.0, note_fraction) * length_scale * length_scale;

    let linear_mass_density = tension / (2.0 * speaking_length * fundamental_frequency).powi(2);

    let effective_density = (linear_mass_density / (PI * core_radius * core_radius)).max(7850.0);

    let highest_modeled_frequency = (24.0 * fundamental_frequency)
        .min(8000.0)
        .max(4.5 * fundamental_frequency);

    let string = PianoStringDesign {
        speaking_length,
        core_radius,
        material_density: effective_density,
        youngs_modulus: 2.0e11,
        static_tension: tension,
        hammer_strike_ratio: strike_ratio,
        highest_modeled_frequency,
        maximum_transverse_mode_count: 48,
        vertical_damping: DampingCoefficients {
            constant_part: 4.0e-4,
            stiffness_proportional_part: 6.0e-8,
        },
        horizontal_damping: DampingCoefficients {
            constant_part: 1.5e-4,
            stiffness_proportional_part: 2.5e-8,
        },
        longitudinal_damping: DampingCoefficients {
            constant_part: 3.0e-3,
            stiffness_proportional_part: 0.0,
        },
    };

    let velocity_hardness = velocity_hardness_curve(
        velocity_blend,
        controls.hammer_hardness_piano,
        controls.hammer_hardness_mezzo,
        controls.hammer_hardness_forte,
    );

    let effective_hardness = (controls.hammer_hardness + velocity_hardness - 1.0).clamp(-3.0, 3.0);

    let stiffness_multiplier = 2.0_f64.powf(effective_hardness * 1.5);

    let base_felt_stiffness =
        linear_interpolation(9.0e10, 4.5e11, note_fraction) * stiffness_multiplier;

    let felt_exponent = (linear_interpolation(2.7, 3.4, note_fraction)
        + effective_hardness * 0.15
        + controls.hammer_tone * 0.25)
        .clamp(1.8, 4.2);

    let felt_relaxation_time = (linear_interpolation(3.2e-4, 1.2e-4, note_fraction)
        * 2.0_f64.powf(-effective_hardness * 0.5))
    .max(3.0e-5);

    let felt = FeltProperties {
        stiffness: [
            base_felt_stiffness / 6.0,
            base_felt_stiffness / 6.0,
            base_felt_stiffness,
        ],
        nonlinearity_exponent: [felt_exponent; 3],
        relaxation_time: [felt_relaxation_time; 3],
    };

    let soft_pedal = controls.soft_pedal.clamp(0.0, 1.0);
    let hammer_force_scales = [1.0 - soft_pedal, 1.0, 1.0];
    let lateral_misalignment_shift = soft_pedal * 1.5e-3;

    let hammer_noise_amplitude =
        controls.hammer_noise.clamp(0.0, 3.0) * (0.8 + 3.0 * velocity_blend.clamp(0.0, 1.0));

    let hammer_noise_cutoff_hz =
        1500.0 * 30.0_f64.powf((controls.hammer_tone.clamp(-1.0, 1.0) + 1.0) * 0.5);

    let unison_width = controls.unison_width.clamp(0.0, 20.0);
    let unison_relative_detunes = [-1.2, 0.4, 1.0].map(|detune| detune * unison_width * 1.0e-4);

    let number_of_unison_strings = if note < 33 {
        1
    } else if note < 45 {
        2
    } else {
        3
    };

    NoteDesign {
        string,
        felt,
        number_of_unison_strings,
        unison_relative_detunes,
        unison_lateral_offsets: [-0.002, 0.0, 0.002],
        fundamental_frequency,
        voicing: VoicingBaked {
            hammer_force_scales,
            lateral_misalignment_shift,
            hammer_noise_amplitude,
            hammer_noise_cutoff_hz,
        },
    }
}

pub fn shared_hammer_geometry() -> HammerGeometry {
    HammerGeometry {
        horizontal_arm_length: 0.135,
        vertical_arm_length: 0.020,
        felt_thickness: 0.012,
        string_distance_from_pivot: 0.152,
        string_angle_from_pivot: 23.0_f64.to_radians(),
        string_inclination_angle: 0.0,
        shank_line_density: 0.185,
        pivot_damping: 2.0e-4,
    }
}

struct SoundPath {
    delay_seconds: f64,
    gain: f64,
}

struct DirectAndEarlyField {
    sound_paths: Vec<SoundPath>,
}

impl DirectAndEarlyField {
    fn new(
        room_dimensions: Vector3,
        soundboard_position: Vector3,
        listener_position: Vector3,
        air_density: f64,
        speed_of_sound: f64,
        wall_reflection_factor: f64,
    ) -> Self {
        let distance_between = |from: Vector3, to: Vector3| -> f64 {
            let dx = from.x - to.x;
            let dy = from.y - to.y;
            let dz = from.z - to.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        };

        let mut sound_paths = Vec::with_capacity(7);

        let direct_distance = distance_between(soundboard_position, listener_position);

        sound_paths.push(SoundPath {
            delay_seconds: direct_distance / speed_of_sound,
            gain: air_density / (4.0 * PI * direct_distance),
        });

        for axis in 0..3 {
            for wall_side in 0..2 {
                let mirrored_source = if wall_side == 0 {
                    soundboard_position.with_component(axis, -soundboard_position.component(axis))
                } else {
                    soundboard_position.with_component(
                        axis,
                        2.0 * room_dimensions.component(axis) - soundboard_position.component(axis),
                    )
                };

                let image_distance = distance_between(mirrored_source, listener_position);

                sound_paths.push(SoundPath {
                    delay_seconds: image_distance / speed_of_sound,
                    gain: wall_reflection_factor * air_density / (4.0 * PI * image_distance),
                });
            }
        }

        DirectAndEarlyField { sound_paths }
    }

    fn pressure(&self, volume_acceleration_history: &SampleHistory) -> f64 {
        self.sound_paths
            .iter()
            .map(|path| path.gain * volume_acceleration_history.value_at_age(path.delay_seconds))
            .sum()
    }
}

struct SampleHistory {
    samples: Vec<f64>,
    write_index: usize,
    total_samples_pushed: usize,
    sample_rate: f64,
}

impl SampleHistory {
    fn new(capacity: usize, sample_rate: f64) -> Self {
        SampleHistory {
            samples: vec![0.0; capacity.max(4)],
            write_index: 0,
            total_samples_pushed: 0,
            sample_rate,
        }
    }

    fn clear(&mut self) {
        self.samples.fill(0.0);
        self.write_index = 0;
        self.total_samples_pushed = 0;
    }

    fn push(&mut self, value: f64) {
        self.samples[self.write_index] = value;
        self.write_index = (self.write_index + 1) % self.samples.len();
        self.total_samples_pushed += 1;
    }

    fn value_at_age(&self, age_seconds: f64) -> f64 {
        if age_seconds < 0.0 || self.total_samples_pushed == 0 {
            return 0.0;
        }

        let fractional_position = age_seconds * self.sample_rate;
        let whole_steps_back = fractional_position.floor() as usize;
        let fraction = fractional_position - whole_steps_back as f64;

        let capacity = self.samples.len();
        let available_history = self.total_samples_pushed.min(capacity);

        if whole_steps_back + 1 >= available_history {
            return 0.0;
        }

        let newest_index = (self.write_index + capacity - 1) % capacity;
        let index_newer = (newest_index + capacity - whole_steps_back) % capacity;
        let index_older = (newest_index + capacity - whole_steps_back - 1) % capacity;

        self.samples[index_newer] * (1.0 - fraction) + self.samples[index_older] * fraction
    }
}

pub struct Instrument {
    pub soundboard: Soundboard,
    pub air: AirRoom,
    pub live_voicing: LiveVoicingControls,
    volume_acceleration_history: SampleHistory,
    direct_and_early_field: DirectAndEarlyField,
    ringout_steps_remaining: u64,
    pub simulated_time: f64,
}

impl Instrument {
    pub fn new(random_seed: u64) -> Self {
        let room_dimensions = Vector3::new(4.7, 3.6, 2.8);
        let soundboard_center_position = Vector3::new(1.9, 1.35, 1.05);
        let listener_position = Vector3::new(3.3, 2.4, 1.2);
        let speed_of_sound = 343.0;
        let air_density = 1.2;

        let soundboard = Soundboard::new(&SoundboardDesign {
            mode_count: 56,
            lowest_mode_frequency: 70.0,
            highest_mode_frequency: 4000.0,
            random_seed,
        });

        let air = AirRoom::new(&AirRoomDesign {
            room_dimensions,
            speed_of_sound,
            air_density,
            highest_modeled_frequency: 300.0,
            soundboard_position: soundboard_center_position,
            listener_position,
        });

        let history_capacity = (12.0 / speed_of_sound * SIMULATION_RATE_HZ).ceil() as usize + 8;

        Instrument {
            soundboard,
            air,
            live_voicing: LiveVoicingControls::default(),
            volume_acceleration_history: SampleHistory::new(history_capacity, SIMULATION_RATE_HZ),
            direct_and_early_field: DirectAndEarlyField::new(
                room_dimensions,
                soundboard_center_position,
                listener_position,
                air_density,
                speed_of_sound,
                0.7,
            ),
            ringout_steps_remaining: 0,
            simulated_time: 0.0,
        }
    }

    pub fn step(&mut self, voices: &mut [Option<SynthVoice>]) -> f64 {
        let any_voice_active = voices.iter().any(|voice| voice.is_some());

        if !any_voice_active && self.ringout_steps_remaining == 0 {
            self.volume_acceleration_history.push(0.0);
            self.simulated_time += SIMULATION_STEP_SIZE;
            return 0.0;
        }

        self.ringout_steps_remaining = if any_voice_active {
            (RINGOUT_SECONDS * SIMULATION_RATE_HZ) as u64
        } else {
            self.ringout_steps_remaining.saturating_sub(1)
        };

        let current_volume_acceleration = self.soundboard.volume_acceleration();
        self.volume_acceleration_history
            .push(current_volume_acceleration);

        let strike_phase_is_active = if any_voice_active {
            let bridge_state_for_phase_check = self.soundboard.bridge_state();

            voices.iter().flatten().any(|voice| {
                voice
                    .engine
                    .is_in_strike_phase(bridge_state_for_phase_check.displacement)
            })
        } else {
            false
        };

        let sub_step_count = if strike_phase_is_active {
            SUB_STEPS_DURING_STRIKE
        } else {
            1
        };

        let sub_step_size = SIMULATION_STEP_SIZE / sub_step_count as f64;
        let live_voicing = self.live_voicing;

        for voice in voices.iter_mut().flatten() {
            if voice.releasing {
                voice.engine.apply_damper_step();
            }

            voice.engine.update_slow_modulations(
                SIMULATION_STEP_SIZE,
                live_voicing.blooming_energy,
                live_voicing.blooming_inertia,
                live_voicing.sympathetic_resonance,
                live_voicing.duplex_scale,
            );
        }

        for _ in 0..sub_step_count {
            let shared_bridge_state = self.soundboard.bridge_state();
            let mut total_bridge_force = Vector3::ZERO;

            for voice in voices.iter_mut().flatten() {
                total_bridge_force = total_bridge_force
                    + voice
                        .engine
                        .advance(sub_step_size, &shared_bridge_state, COUPLING_SWEEPS);
            }

            let air_pressure_at_soundboard = self.air.pressure_at_soundboard();

            self.soundboard.apply_forces_and_advance(
                total_bridge_force,
                air_pressure_at_soundboard,
                sub_step_size,
            );

            let soundboard_volume_velocity = self.soundboard.volume_velocity();
            self.air.advance(soundboard_volume_velocity, sub_step_size);
        }

        self.simulated_time += SIMULATION_STEP_SIZE;
        self.listener_pressure()
    }

    pub fn reset(&mut self) {
        self.soundboard.reset();
        self.air.reset();
        self.volume_acceleration_history.clear();
        self.ringout_steps_remaining = 0;
        self.simulated_time = 0.0;
        self.live_voicing = LiveVoicingControls::default();
    }

    pub fn listener_pressure(&self) -> f64 {
        self.air.pressure_at_listener()
            + self
                .direct_and_early_field
                .pressure(&self.volume_acceleration_history)
    }
}
