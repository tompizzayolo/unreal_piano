use super::air::{AirRoom, AirRoomDesign};
use super::complex::Complex64;
use super::hammer::{FeltProperties, Hammer, HammerGeometry};
use super::soundboard::{Soundboard, SoundboardDesign};
use super::strings::{DampingCoefficients, PianoString, PianoStringDesign};
use super::vector::Vector3;
use std::f64::consts::PI;

const UNISON_RELATIVE_DETUNES: [f64; 3] = [-0.0012, 0.0004, 0.0010];
const UNISON_LATERAL_OFFSETS: [f64; 3] = [-0.002, 0.0, 0.002];
const UNISON_LATERAL_MISALIGNMENTS: [f64; 3] = [0.28e-3, -0.22e-3, 0.34e-3];

pub const SUB_STEPS_DURING_STRIKE: usize = 16;
const STRIKE_PHASE_ANGLE_MARGIN: f64 = 0.05;
const HAMMER_REST_ANGLE_DROP: f64 = 0.25;

#[derive(Clone, Copy, PartialEq)]
pub enum NoteName {
    A3,
    C4,
    E5,
}

impl NoteName {
    pub fn label(&self) -> &'static str {
        match self {
            NoteName::A3 => "A3 (220 Hz)",
            NoteName::C4 => "C4 (261.6 Hz)",
            NoteName::E5 => "E5 (659.3 Hz)",
        }
    }
}

pub struct PianoConfiguration {
    pub note: NoteName,
    pub hammer_strike_velocity: f64,
    pub time_step: f64,
    pub coupling_sweeps: usize,
    pub simulation_duration: f64,
    pub audio_sample_rate: usize,
    pub random_seed: u64,
}

fn note_designs(note: NoteName) -> (PianoStringDesign, HammerGeometry, FeltProperties) {
    let hammer_geometry = HammerGeometry {
        horizontal_arm_length: 0.135,
        vertical_arm_length: 0.020,
        felt_thickness: 0.012,
        string_distance_from_pivot: 0.152,
        string_angle_from_pivot: 23.0_f64.to_radians(),
        string_inclination_angle: 0.0,
        shank_line_density: 0.185,
        pivot_damping: 2.0e-4,
    };

    match note {
        NoteName::A3 => (
            PianoStringDesign {
                speaking_length: 0.720,
                core_radius: 0.55e-3,
                material_density: 7850.0,
                youngs_modulus: 2.0e11,
                static_tension: 750.0,
                hammer_strike_ratio: 0.118,
                highest_modeled_frequency: 8000.0,
                maximum_transverse_mode_count: 48,
                vertical_damping: DampingCoefficients {
                    constant_part: 4.5e-4,
                    stiffness_proportional_part: 7.0e-8,
                },
                horizontal_damping: DampingCoefficients {
                    constant_part: 1.5e-4,
                    stiffness_proportional_part: 3.0e-8,
                },
                longitudinal_damping: DampingCoefficients {
                    constant_part: 1.0e-2,
                    stiffness_proportional_part: 0.0,
                },
            },
            hammer_geometry,
            FeltProperties {
                stiffness: [2.0e10, 2.0e10, 1.2e11],
                nonlinearity_exponent: [2.8, 2.8, 2.8],
                relaxation_time: [3.0e-4, 3.0e-4, 3.0e-4],
            },
        ),
        NoteName::C4 => (
            PianoStringDesign {
                speaking_length: 0.620,
                core_radius: 0.475e-3,
                material_density: 7850.0,
                youngs_modulus: 2.0e11,
                static_tension: 585.0,
                hammer_strike_ratio: 0.122,
                highest_modeled_frequency: 8000.0,
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
                    constant_part: 8.0e-3,
                    stiffness_proportional_part: 0.0,
                },
            },
            hammer_geometry,
            FeltProperties {
                stiffness: [3.0e10, 3.0e10, 2.0e11],
                nonlinearity_exponent: [3.0, 3.0, 3.0],
                relaxation_time: [2.5e-4, 2.5e-4, 2.5e-4],
            },
        ),
        NoteName::E5 => (
            PianoStringDesign {
                speaking_length: 0.420,
                core_radius: 0.33e-3,
                material_density: 7850.0,
                youngs_modulus: 2.0e11,
                static_tension: 820.0,
                hammer_strike_ratio: 0.130,
                highest_modeled_frequency: 8000.0,
                maximum_transverse_mode_count: 48,
                vertical_damping: DampingCoefficients {
                    constant_part: 3.0e-4,
                    stiffness_proportional_part: 5.0e-8,
                },
                horizontal_damping: DampingCoefficients {
                    constant_part: 1.2e-4,
                    stiffness_proportional_part: 2.0e-8,
                },
                longitudinal_damping: DampingCoefficients {
                    constant_part: 6.0e-3,
                    stiffness_proportional_part: 0.0,
                },
            },
            hammer_geometry,
            FeltProperties {
                stiffness: [6.0e10, 6.0e10, 4.0e11],
                nonlinearity_exponent: [3.3, 3.3, 3.3],
                relaxation_time: [1.5e-4, 1.5e-4, 1.5e-4],
            },
        ),
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
    #[allow(clippy::too_many_arguments)]
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

    #[inline]
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
    pub strings: Vec<PianoString>,
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
    snapshot_buffer: Vec<f64>,
    felt_compression_at_step_start: Vec<Vector3>,
    felt_compression_source: Vec<Vector3>,
    compression_reference_for_rate_term: Vec<Vector3>,
    hammer_forces_on_strings: Vec<Vector3>,
}

impl Piano {
    pub fn new(configuration: &PianoConfiguration) -> Self {
        let (string_design, hammer_geometry, felt_properties) = note_designs(configuration.note);

        let number_of_unison_strings = UNISON_RELATIVE_DETUNES.len();
        let mut strings = Vec::with_capacity(number_of_unison_strings);

        for string_index in 0..number_of_unison_strings {
            strings.push(PianoString::new(
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

        let mut snapshot_length = 2;

        for piano_string in &strings {
            snapshot_length += 2
                * (piano_string.vertical_modes.len()
                    + piano_string.horizontal_modes.len()
                    + piano_string.longitudinal_modes.len());
        }

        snapshot_length += 2 * soundboard.modes.len();
        snapshot_length += 2 * air.modal_states.len();

        let string_count = strings.len();

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
            snapshot_buffer: vec![0.0; snapshot_length],
            felt_compression_at_step_start: vec![Vector3::ZERO; string_count],
            felt_compression_source: vec![Vector3::ZERO; string_count],
            compression_reference_for_rate_term: vec![Vector3::ZERO; string_count],
            hammer_forces_on_strings: vec![Vector3::ZERO; string_count],
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

    fn write_state_snapshot(&mut self) {
        let snapshot = &mut self.snapshot_buffer;
        let mut cursor = 0usize;

        for piano_string in &self.strings {
            let all_modes = piano_string
                .vertical_modes
                .iter()
                .chain(piano_string.horizontal_modes.iter())
                .chain(piano_string.longitudinal_modes.iter());

            for mode in all_modes {
                snapshot[cursor] = mode.displacement;
                snapshot[cursor + 1] = mode.velocity;
                cursor += 2;
            }
        }

        for mode in &self.soundboard.modes {
            snapshot[cursor] = mode.displacement;
            snapshot[cursor + 1] = mode.velocity;
            cursor += 2;
        }

        for mode in &self.air.modal_states {
            snapshot[cursor] = mode.state.real_part;
            snapshot[cursor + 1] = mode.state.imaginary_part;
            cursor += 2;
        }

        snapshot[cursor] = self.hammer.rotation_angle;
        snapshot[cursor + 1] = self.hammer.angular_velocity;
    }

    fn restore_state_snapshot(&mut self) {
        let snapshot = &self.snapshot_buffer;
        let mut cursor = 0usize;

        for piano_string in &mut self.strings {
            let all_modes = piano_string
                .vertical_modes
                .iter_mut()
                .chain(piano_string.horizontal_modes.iter_mut())
                .chain(piano_string.longitudinal_modes.iter_mut());

            for mode in all_modes {
                mode.displacement = snapshot[cursor];
                mode.velocity = snapshot[cursor + 1];
                cursor += 2;
            }
        }

        for mode in &mut self.soundboard.modes {
            mode.displacement = snapshot[cursor];
            mode.velocity = snapshot[cursor + 1];
            cursor += 2;
        }

        for mode in &mut self.air.modal_states {
            mode.state = Complex64::new(snapshot[cursor], snapshot[cursor + 1]);
            cursor += 2;
        }

        self.hammer.rotation_angle = snapshot[cursor];
        self.hammer.angular_velocity = snapshot[cursor + 1];
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

        let bridge_state_at_step_start = self.soundboard.bridge_state();
        let hammer_angle_at_step_start = self.hammer.rotation_angle;

        for string_index in 0..string_count {
            let string_displacement_at_strike = self.strings[string_index]
                .displacement_at_strike_point(bridge_state_at_step_start.displacement);

            let (compression, _) = self.hammer.felt_compression(
                hammer_angle_at_step_start,
                string_displacement_at_strike,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );

            self.felt_compression_at_step_start[string_index] = compression;
        }

        self.write_state_snapshot();

        for string_index in 0..string_count {
            self.felt_compression_source[string_index] =
                self.felt_compression_at_step_start[string_index];

            self.compression_reference_for_rate_term[string_index] =
                self.previous_felt_compressions[string_index];
        }

        let mut hammer_angle_source = hammer_angle_at_step_start;
        let mut bridge_state_source = bridge_state_at_step_start;

        for sweep_index in 0..self.coupling_sweeps {
            if sweep_index > 0 {
                self.restore_state_snapshot();

                for string_index in 0..string_count {
                    self.compression_reference_for_rate_term[string_index] =
                        self.felt_compression_at_step_start[string_index];
                }
            }

            let mut total_felt_force_on_strings = Vector3::ZERO;

            for string_index in 0..string_count {
                let felt_frame_force = self.hammer.felt_contact_force(
                    self.felt_compression_source[string_index],
                    self.compression_reference_for_rate_term[string_index],
                    step_size,
                );

                total_felt_force_on_strings = total_felt_force_on_strings + felt_frame_force;

                let force_in_string_frame = self
                    .hammer
                    .force_on_string_in_string_frame(felt_frame_force, hammer_angle_source);

                self.hammer_forces_on_strings[string_index] = force_in_string_frame;
            }

            for string_index in 0..string_count {
                self.strings[string_index].apply_forces_and_advance(
                    self.hammer_forces_on_strings[string_index],
                    bridge_state_source.acceleration,
                    step_size,
                );
            }

            let total_shank_torque = self
                .hammer
                .shank_torque(hammer_angle_source, total_felt_force_on_strings)
                + self.hammer.rest_rail_torque();

            self.hammer
                .advance_shank_rotation(total_shank_torque, step_size);

            let mut total_bridge_force = Vector3::ZERO;

            for piano_string in &self.strings {
                total_bridge_force = total_bridge_force
                    + piano_string.force_exerted_on_bridge(bridge_state_source.displacement);
            }

            let air_pressure_at_soundboard = self.air.pressure_at_soundboard();

            self.soundboard.apply_forces_and_advance(
                total_bridge_force,
                air_pressure_at_soundboard,
                step_size,
            );

            let soundboard_volume_velocity = self.soundboard.volume_velocity();
            self.air.advance(soundboard_volume_velocity, step_size);

            hammer_angle_source = self.hammer.rotation_angle;
            bridge_state_source = self.soundboard.bridge_state();

            for string_index in 0..string_count {
                let string_displacement_at_strike = self.strings[string_index]
                    .displacement_at_strike_point(bridge_state_source.displacement);

                let (compression, _) = self.hammer.felt_compression(
                    hammer_angle_source,
                    string_displacement_at_strike,
                    self.strings[string_index].lateral_offset_from_hammer_center,
                    self.felt_contact_references[string_index],
                );

                self.felt_compression_source[string_index] = compression;
            }
        }

        for string_index in 0..string_count {
            self.previous_felt_compressions[string_index] =
                self.felt_compression_at_step_start[string_index];
        }
    }
}
