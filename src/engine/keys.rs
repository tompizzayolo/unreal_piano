use super::air::{AirRoom, AirRoomDesign};
use super::hammer::{FeltProperties, Hammer, HammerGeometry};
use super::soundboard::{Soundboard, SoundboardDesign};
use super::strings::{KeysString, KeysStringDesign};
use super::vector::Vector3;
use std::f64::consts::PI;

const UNISON_RELATIVE_DETUNES: [f64; 3] = [-0.0012, 0.0004, 0.0010];
const UNISON_LATERAL_OFFSETS: [f64; 3] = [-0.002, 0.0, 0.002];
const UNISON_LATERAL_MISALIGNMENTS: [f64; 3] = [0.28e-3, -0.22e-3, 0.34e-3];

pub const SUB_STEPS_DURING_STRIKE: usize = 2;
const STRIKE_PHASE_ANGLE_MARGIN: f64 = 0.05;
const HAMMER_REST_ANGLE_DROP: f64 = 0.25;

pub struct KeysConfiguration {
    pub string_design: KeysStringDesign,
    pub hammer_geometry: HammerGeometry,
    pub felt_properties: FeltProperties,
    pub hammer_strike_velocity: f64,
    pub time_step: f64,
    pub coupling_sweeps: usize,
    pub simulation_duration: f64,
    pub audio_sample_rate: usize,
    pub random_seed: u64,
}

struct DirectAndEarlyField {
    direct_gain: f64,
    direct_delay: f64,
}

impl DirectAndEarlyField {
    #[allow(clippy::too_many_arguments)]
    fn new(
        _room_dimensions: Vector3,
        soundboard_position: Vector3,
        listener_position: Vector3,
        air_density: f64,
        speed_of_sound: f64,
        _wall_reflection_factor: f64,
    ) -> Self {
        let dx = soundboard_position.x - listener_position.x;
        let dy = soundboard_position.y - listener_position.y;
        let dz = soundboard_position.z - listener_position.z;
        let direct_distance = (dx * dx + dy * dy + dz * dz).sqrt();

        DirectAndEarlyField {
            direct_delay: direct_distance / speed_of_sound,
            direct_gain: air_density / (4.0 * PI * direct_distance),
        }
    }

    #[inline]
    fn pressure(&self, volume_acceleration_history: &SampleHistory) -> f64 {
        self.direct_gain * volume_acceleration_history.value_at_age(self.direct_delay)
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

    #[inline]
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

pub struct Piano {
    pub strings: Vec<KeysString>,
    pub hammer: Hammer,
    pub soundboard: Soundboard,
    pub air: AirRoom,
    pub time_step: f64,
    pub coupling_sweeps: usize,
    pub simulated_time: f64,
    previous_felt_compressions: Vec<Vector3>,
    felt_contact_references: Vec<Vector3>,
    strike_activation_angle: f64,
    volume_acceleration_history: SampleHistory,
    direct_and_early_field: DirectAndEarlyField,
}

impl Piano {
    pub fn new(configuration: &KeysConfiguration) -> Self {
        let string_design = configuration.string_design;
        let hammer_geometry = configuration.hammer_geometry;
        let felt_properties = configuration.felt_properties;

        let number_of_unison_strings = UNISON_RELATIVE_DETUNES.len();
        let mut strings = Vec::with_capacity(number_of_unison_strings);

        for string_index in 0..number_of_unison_strings {
            strings.push(KeysString::new(
                string_design,
                UNISON_LATERAL_OFFSETS[string_index],
                UNISON_RELATIVE_DETUNES[string_index],
            ));
        }

        let mut hammer = Hammer::new(hammer_geometry, felt_properties);
        let contact_angle = hammer.resting_contact_angle(UNISON_LATERAL_OFFSETS[1]);
        hammer.rotation_angle = contact_angle;
        hammer.rest_angle = contact_angle - HAMMER_REST_ANGLE_DROP;

        let compression_rate_per_radian =
            hammer.compression_rate_per_radian(contact_angle, UNISON_LATERAL_OFFSETS[1]);

        hammer.angular_velocity =
            configuration.hammer_strike_velocity / compression_rate_per_radian.max(1.0e-9);

        let mut felt_contact_references = Vec::with_capacity(number_of_unison_strings);
        let mut previous_felt_compressions = Vec::with_capacity(number_of_unison_strings);

        for string_index in 0..number_of_unison_strings {
            let reference = hammer.contact_reference_in_felt_frame(
                contact_angle,
                UNISON_LATERAL_OFFSETS[string_index],
                UNISON_LATERAL_MISALIGNMENTS[string_index],
            );

            felt_contact_references.push(reference);

            let (initial_compression, _contact_detected) = hammer.felt_compression(
                contact_angle,
                Vector3::ZERO,
                UNISON_LATERAL_OFFSETS[string_index],
                reference,
            );

            previous_felt_compressions.push(initial_compression);
        }

        let strike_activation_angle = contact_angle - STRIKE_PHASE_ANGLE_MARGIN;

        let soundboard = Soundboard::new(&SoundboardDesign {
            mode_count: 56,
            lowest_mode_frequency: 70.0,
            highest_mode_frequency: 4000.0,
            random_seed: configuration.random_seed,
        });

        let room_dimensions = Vector3::new(4.7, 3.6, 2.8);
        let soundboard_center_position = Vector3::new(1.9, 1.35, 1.05);
        let listener_position = Vector3::new(3.3, 2.4, 1.2);
        let speed_of_sound = 343.0;
        let air_density = 1.2;

        let air = AirRoom::new(&AirRoomDesign {
            room_dimensions,
            speed_of_sound,
            air_density,
            highest_modeled_frequency: 380.0,
            soundboard_position: soundboard_center_position,
            listener_position,
        });

        let simulation_rate = 1.0 / configuration.time_step;
        let maximum_image_distance = 12.0;
        let history_capacity =
            (maximum_image_distance / speed_of_sound * simulation_rate).ceil() as usize + 8;

        let volume_acceleration_history = SampleHistory::new(history_capacity, simulation_rate);

        let direct_and_early_field = DirectAndEarlyField::new(
            room_dimensions,
            soundboard_center_position,
            listener_position,
            air_density,
            speed_of_sound,
            0.7,
        );

        Piano {
            strings,
            hammer,
            soundboard,
            air,
            time_step: configuration.time_step,
            coupling_sweeps: configuration.coupling_sweeps.max(1),
            simulated_time: 0.0,
            previous_felt_compressions,
            felt_contact_references,
            strike_activation_angle,
            volume_acceleration_history,
            direct_and_early_field,
        }
    }

    #[inline]
    pub fn current_time(&self) -> f64 {
        self.simulated_time
    }

    #[inline]
    pub fn listener_pressure(&self) -> f64 {
        self.air.pressure_at_listener()
            + self
                .direct_and_early_field
                .pressure(&self.volume_acceleration_history)
    }

    pub fn state_is_finite(&self) -> bool {
        self.hammer.rotation_angle.is_finite()
            && self.hammer.angular_velocity.is_finite()
            && self
                .soundboard
                .modes
                .iter()
                .all(|mode| mode.displacement.is_finite() && mode.velocity.is_finite())
            && self.air.modal_states.iter().all(|mode| {
                mode.state.real_part.is_finite() && mode.state.imaginary_part.is_finite()
            })
            && self.strings.iter().all(|piano_string| {
                piano_string
                    .vertical_modes
                    .iter()
                    .all(|mode| mode.displacement.is_finite() && mode.velocity.is_finite())
                    && piano_string
                        .horizontal_modes
                        .iter()
                        .all(|mode| mode.displacement.is_finite() && mode.velocity.is_finite())
                    && piano_string
                        .longitudinal_modes
                        .iter()
                        .all(|mode| mode.displacement.is_finite() && mode.velocity.is_finite())
            })
    }

    pub fn step(&mut self) {
        let outer_step_size = self.time_step;

        let current_volume_acceleration = self.soundboard.volume_acceleration();
        self.volume_acceleration_history
            .push(current_volume_acceleration);

        let hammer_is_rising_toward_the_strings = self.hammer.angular_velocity > 0.0
            && self.hammer.rotation_angle >= self.strike_activation_angle;

        let strike_phase_is_active =
            hammer_is_rising_toward_the_strings || self.any_string_in_contact();

        let sub_step_count = if strike_phase_is_active {
            SUB_STEPS_DURING_STRIKE
        } else {
            1
        };

        let sub_step_size = outer_step_size / sub_step_count as f64;

        for _ in 0..sub_step_count {
            self.advance_coupled_sub_step(sub_step_size);
        }

        self.simulated_time += outer_step_size;
    }

    fn any_string_in_contact(&self) -> bool {
        if self.hammer.rotation_angle < self.strike_activation_angle - STRIKE_PHASE_ANGLE_MARGIN {
            return false;
        }

        let bridge_displacement = self.soundboard.bridge_displacement();
        let hammer_angle = self.hammer.rotation_angle;

        for string_index in 0..self.strings.len() {
            let string_displacement =
                self.strings[string_index].displacement_at_strike_point(bridge_displacement);

            let (_, contact_detected) = self.hammer.felt_compression(
                hammer_angle,
                string_displacement,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );

            if contact_detected {
                return true;
            }
        }

        false
    }

    fn advance_coupled_sub_step(&mut self, step_size: f64) {
        let string_count = self.strings.len();
        let bridge_state = self.soundboard.bridge_state();
        let hammer_angle = self.hammer.rotation_angle;

        let mut total_felt_force_on_strings = Vector3::ZERO;

        for string_index in 0..string_count {
            let string_displacement =
                self.strings[string_index].displacement_at_strike_point(bridge_state.displacement);

            let (compression, _) = self.hammer.felt_compression(
                hammer_angle,
                string_displacement,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );

            let felt_frame_force = self.hammer.felt_contact_force(
                compression,
                self.previous_felt_compressions[string_index],
                step_size,
            );

            total_felt_force_on_strings = total_felt_force_on_strings + felt_frame_force;

            let force_in_string_frame = self
                .hammer
                .force_on_string_in_string_frame(felt_frame_force, hammer_angle);

            self.strings[string_index].apply_forces_and_advance(
                force_in_string_frame,
                bridge_state.acceleration,
                step_size,
            );

            self.previous_felt_compressions[string_index] = compression;
        }

        let total_shank_torque = self.hammer.total_torque(total_felt_force_on_strings);
        self.hammer
            .advance_shank_rotation(total_shank_torque, step_size);

        let mut total_bridge_force = Vector3::ZERO;
        for piano_string in &self.strings {
            total_bridge_force = total_bridge_force
                + piano_string.force_exerted_on_bridge(bridge_state.displacement);
        }

        let air_pressure = self.air.pressure_at_soundboard();
        self.soundboard
            .apply_forces_and_advance(total_bridge_force, air_pressure, step_size);

        let volume_velocity = self.soundboard.volume_velocity();
        self.air.advance(volume_velocity, step_size);
    }
}
