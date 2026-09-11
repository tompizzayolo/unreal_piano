use crate::engine::hammer::{Hammer, HammerGeometry};
use crate::engine::modal::ModalOscillator;
use crate::engine::soundboard::BridgeState;
use crate::engine::strings::KeysString;
use crate::engine::vector::Vector3;
use crate::instrument::NoteDesign;
use nice_plug::midi::{Channel, Key, VoiceID};
use std::f64::consts::PI;

pub const SUB_STEPS_DURING_STRIKE: usize = 32;
const HAMMER_REST_ANGLE_DROP: f64 = 0.25;
const STRIKE_PHASE_ANGLE_MARGIN: f64 = 0.05;
const UNISON_LATERAL_MISALIGNMENTS: [f64; 3] = [0.28e-3, -0.22e-3, 0.34e-3];
const MAXIMUM_UNISON_STRINGS: usize = 3;
const HAMMER_NOISE_WINDOW_SECONDS: f64 = 12.0e-3;
const HAMMER_NOISE_DECAY_SECONDS: f64 = 2.5e-3;
const HAMMER_THUMP_BRIDGE_SCALE: f64 = 0.12;
const KEY_RELEASE_NOISE_WINDOW_SECONDS: f64 = 0.030;
const KEY_RELEASE_NOISE_DECAY_SECONDS: f64 = 0.009;
const KEY_RELEASE_NOISE_AMPLITUDE: f64 = 0.05;
const KEY_RELEASE_NOISE_CUTOFF_HZ: f64 = 850.0;
const DUPLEX_FREQUENCY_RATIOS: [f64; 6] = [8.53, 10.47, 12.61, 15.13, 18.09, 21.47];
const DUPLEX_MODAL_MASS: f64 = 1.0e-3;
const DUPLEX_MODAL_DAMPING: f64 = 5.0e-4;
const DUPLEX_DRIVE_SCALE: f64 = 1.0e-3;
const DUPLEX_OUTPUT_SCALE: f64 = 0.05;
const DAMPER_AMPLITUDE_DECAY_LN: f64 = -6.907755278982137;

pub struct PianoVoice {
    pub strings: Vec<KeysString>,
    pub hammer: Hammer,
    strike_velocity: f64,
    release_noise_seconds_remaining: f64,
    release_noise_amplitude: f64,
    release_noise_filter_state: f64,
    release_noise_random_state: u64,
    previous_felt_compressions: [Vector3; MAXIMUM_UNISON_STRINGS],
    felt_contact_references: [Vector3; MAXIMUM_UNISON_STRINGS],
    grazing_contact_angle: f64,
    strike_activation_angle: f64,
    reference_lateral_offset: f64,
    hammer_force_scales: [f64; MAXIMUM_UNISON_STRINGS],
    damper_decay_per_step: f64,
    hammer_noise_seconds_remaining: f64,
    hammer_noise_amplitude: f64,
    hammer_noise_cutoff_hz: f64,
    hammer_noise_filter_state: f64,
    hammer_noise_random_state: u64,
    note_age_seconds: f64,
    blooming_envelope: f64,
    bridge_coupling_factor: f64,
    duplex_output_scale: f64,
    duplex_resonators: Vec<ModalOscillator>,
    duplex_reaction_coefficients: Vec<f64>,
    snapshot_buffer: Vec<f64>,
}

impl PianoVoice {
    pub fn new(design: &NoteDesign, shared_hammer_geometry: HammerGeometry) -> Self {
        let string_count = design
            .number_of_unison_strings
            .min(MAXIMUM_UNISON_STRINGS)
            .max(1);

        let mut strings = Vec::with_capacity(string_count);
        for string_index in 0..string_count {
            strings.push(KeysString::new(
                design.string,
                design.unison_lateral_offsets[string_index],
                design.unison_relative_detunes[string_index],
            ));
        }

        let mut hammer = Hammer::new(shared_hammer_geometry, design.felt);
        let reference_lateral_offset = design.unison_lateral_offsets[(string_count - 1) / 2];
        let contact_angle = hammer.resting_contact_angle(reference_lateral_offset);
        hammer.rest_angle = contact_angle - HAMMER_REST_ANGLE_DROP;
        hammer.rotation_angle = contact_angle;
        hammer.angular_velocity = 0.0;

        let mut felt_contact_references = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
        let mut previous_felt_compressions = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
        for string_index in 0..string_count {
            let reference = hammer.contact_reference_in_felt_frame(
                contact_angle,
                design.unison_lateral_offsets[string_index],
                UNISON_LATERAL_MISALIGNMENTS[string_index]
                    + design.voicing.lateral_misalignment_shift,
            );
            felt_contact_references[string_index] = reference;

            let (initial_compression, _contact_detected) = hammer.felt_compression(
                contact_angle,
                Vector3::ZERO,
                design.unison_lateral_offsets[string_index],
                reference,
            );
            previous_felt_compressions[string_index] = initial_compression;
        }

        let mut duplex_resonators = Vec::with_capacity(DUPLEX_FREQUENCY_RATIOS.len());
        let mut duplex_reaction_coefficients = Vec::with_capacity(DUPLEX_FREQUENCY_RATIOS.len());
        for &frequency_ratio in &DUPLEX_FREQUENCY_RATIOS {
            let resonant_frequency = (design.fundamental_frequency * frequency_ratio).min(20_000.0);
            let angular_frequency = 2.0 * PI * resonant_frequency;
            duplex_resonators.push(ModalOscillator::new(
                angular_frequency,
                DUPLEX_MODAL_DAMPING,
                DUPLEX_MODAL_MASS,
            ));
            duplex_reaction_coefficients.push(
                DUPLEX_MODAL_MASS * angular_frequency * angular_frequency * DUPLEX_OUTPUT_SCALE,
            );
        }

        let snapshot_capacity: usize = strings
            .iter()
            .map(|piano_string| {
                (piano_string.vertical_modes.len()
                    + piano_string.horizontal_modes.len()
                    + piano_string.longitudinal_modes.len())
                    * 2
            })
            .sum::<usize>()
            + 2;

        let mut hammer_force_scales = [1.0; MAXIMUM_UNISON_STRINGS];
        for string_index in 0..string_count {
            hammer_force_scales[string_index] = design.voicing.hammer_force_scales[string_index];
        }

        PianoVoice {
            strike_velocity: 0.0,
            release_noise_seconds_remaining: 0.0,
            release_noise_amplitude: 0.0,
            release_noise_filter_state: 0.0,
            release_noise_random_state: 0x2545_F491_4F6C_DD15,
            strings,
            hammer,
            previous_felt_compressions,
            felt_contact_references,
            grazing_contact_angle: contact_angle,
            strike_activation_angle: contact_angle - STRIKE_PHASE_ANGLE_MARGIN,
            reference_lateral_offset,
            hammer_force_scales,
            damper_decay_per_step: 1.0,
            hammer_noise_seconds_remaining: 0.0,
            hammer_noise_amplitude: design.voicing.hammer_noise_amplitude,
            hammer_noise_cutoff_hz: design.voicing.hammer_noise_cutoff_hz,
            hammer_noise_filter_state: 0.0,
            hammer_noise_random_state: 1,
            note_age_seconds: 0.0,
            blooming_envelope: 1.0,
            bridge_coupling_factor: 1.0,
            duplex_output_scale: 1.0,
            duplex_resonators,
            duplex_reaction_coefficients,
            snapshot_buffer: Vec::with_capacity(snapshot_capacity),
        }
    }

    pub fn strike(&mut self, hammer_velocity: f64, noise_seed: u64) {
        self.hammer.rotation_angle = self.grazing_contact_angle;
        let compression_rate_per_radian = self
            .hammer
            .compression_rate_per_radian(self.grazing_contact_angle, self.reference_lateral_offset);
        self.hammer.angular_velocity = hammer_velocity / compression_rate_per_radian.max(1.0e-9);
        self.strike_velocity = hammer_velocity;
        self.damper_decay_per_step = 1.0;
        self.hammer_noise_seconds_remaining = HAMMER_NOISE_WINDOW_SECONDS;
        self.hammer_noise_filter_state = 0.0;
        self.hammer_noise_random_state = noise_seed | 1;
        self.note_age_seconds = 0.0;
        self.blooming_envelope = 1.0;
        self.release_noise_seconds_remaining = 0.0;
        self.release_noise_amplitude = 0.0;
        self.release_noise_filter_state = 0.0;
        self.release_noise_random_state = noise_seed ^ 0x5A5A_5A5A_5A5A_5A5A | 1;
    }

    pub fn start_release(&mut self, release_seconds: f64, _engine_step_size: f64) {
        let decay_rate = DAMPER_AMPLITUDE_DECAY_LN / release_seconds.max(1.0e-3);
        self.damper_decay_per_step = decay_rate.exp();
    }

    pub fn trigger_key_release_noise(&mut self) {
        let velocity_scale = (self.strike_velocity / 4.0).clamp(0.2, 1.5);
        self.release_noise_seconds_remaining = KEY_RELEASE_NOISE_WINDOW_SECONDS;
        self.release_noise_amplitude = KEY_RELEASE_NOISE_AMPLITUDE * velocity_scale;
        self.release_noise_filter_state = 0.0;
        self.release_noise_random_state = self
            .release_noise_random_state
            .wrapping_add(0x9E37_79B9_7F4A_7C15)
            | 1;
    }

    fn generate_release_noise_force(&mut self, step_size: f64) -> Vector3 {
        if self.release_noise_seconds_remaining <= 0.0 || self.release_noise_amplitude <= 0.0 {
            return Vector3::ZERO;
        }
        self.release_noise_seconds_remaining -= step_size;
        let elapsed_seconds =
            KEY_RELEASE_NOISE_WINDOW_SECONDS - self.release_noise_seconds_remaining.max(0.0);
        let decay_envelope = (-(elapsed_seconds / KEY_RELEASE_NOISE_DECAY_SECONDS)).exp();

        let mut random_state = self.release_noise_random_state;
        random_state ^= random_state >> 12;
        random_state ^= random_state << 25;
        random_state ^= random_state >> 27;
        self.release_noise_random_state = random_state;

        let random_sample = 2.0
            * ((random_state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64
                / (1u64 << 53) as f64)
            - 1.0;

        let filter_coefficient = 1.0 - (-2.0 * PI * KEY_RELEASE_NOISE_CUTOFF_HZ * step_size).exp();
        self.release_noise_filter_state +=
            filter_coefficient * (random_sample - self.release_noise_filter_state);

        Vector3::new(
            0.0,
            0.0,
            self.release_noise_amplitude * self.release_noise_filter_state * decay_envelope,
        )
    }

    pub fn apply_damper_step(&mut self) {
        let decay_factor = self.damper_decay_per_step;
        if decay_factor >= 1.0 - 1.0e-15 {
            return;
        }
        for piano_string in &mut self.strings {
            let all_modes = piano_string
                .vertical_modes
                .iter_mut()
                .chain(piano_string.horizontal_modes.iter_mut())
                .chain(piano_string.longitudinal_modes.iter_mut());
            for mode in all_modes {
                mode.displacement *= decay_factor;
                mode.velocity *= decay_factor;
            }
        }
    }

    pub fn update_slow_modulations(
        &mut self,
        step_size: f64,
        blooming_energy: f64,
        blooming_inertia: f64,
        sympathetic_resonance: f64,
        duplex_scale: f64,
    ) {
        self.note_age_seconds += step_size;
        self.bridge_coupling_factor = sympathetic_resonance;
        self.duplex_output_scale = duplex_scale;

        if blooming_energy > 1.0e-4 && self.note_age_seconds < 30.0 * blooming_inertia {
            let inverse_inertia = 1.0 / blooming_inertia.max(1.0e-6);
            let age = self.note_age_seconds;
            let rise = 1.0 - (-age * inverse_inertia).exp();
            let fall = (-age * inverse_inertia / 6.0).exp();
            self.blooming_envelope = 1.0 + blooming_energy * rise * fall;
        } else {
            self.blooming_envelope = 1.0;
        }
    }

    fn generate_hammer_noise_force(&mut self, step_size: f64) -> Vector3 {
        if self.hammer_noise_amplitude <= 0.0 || self.hammer_noise_seconds_remaining <= 0.0 {
            return Vector3::ZERO;
        }
        self.hammer_noise_seconds_remaining -= step_size;
        let elapsed_seconds = HAMMER_NOISE_WINDOW_SECONDS - self.hammer_noise_seconds_remaining;
        let decay_envelope = (-(elapsed_seconds / HAMMER_NOISE_DECAY_SECONDS)).exp();

        let mut random_state = self.hammer_noise_random_state;
        random_state ^= random_state >> 12;
        random_state ^= random_state << 25;
        random_state ^= random_state >> 27;
        self.hammer_noise_random_state = random_state;

        let random_sample = 2.0
            * ((random_state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64
                / (1u64 << 53) as f64)
            - 1.0;

        let filter_coefficient = 1.0 - (-2.0 * PI * self.hammer_noise_cutoff_hz * step_size).exp();
        self.hammer_noise_filter_state +=
            filter_coefficient * (random_sample - self.hammer_noise_filter_state);

        Vector3::new(
            0.0,
            0.0,
            self.hammer_noise_amplitude * self.hammer_noise_filter_state * decay_envelope,
        )
    }

    pub fn advance(
        &mut self,
        step_size: f64,
        shared_bridge_state: &BridgeState,
        coupling_sweeps: usize,
    ) -> Vector3 {
        let string_count = self.strings.len();
        let sweep_count = coupling_sweeps.max(1);
        let hammer_angle_at_step_start = self.hammer.rotation_angle;

        let mut felt_compression_at_step_start = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
        for string_index in 0..string_count {
            let string_displacement_at_strike = self.strings[string_index]
                .displacement_at_strike_point(shared_bridge_state.displacement);
            let (compression, _contact_detected) = self.hammer.felt_compression(
                hammer_angle_at_step_start,
                string_displacement_at_strike,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );
            felt_compression_at_step_start[string_index] = compression;
        }

        let hammer_noise_force = self.generate_hammer_noise_force(step_size);
        let release_noise_force = self.generate_release_noise_force(step_size);

        let mut state_snapshot = Vec::new();
        if sweep_count > 1 {
            state_snapshot = std::mem::take(&mut self.snapshot_buffer);
            state_snapshot.clear();
            self.write_state_snapshot(&mut state_snapshot);
        }

        let scaled_bridge_acceleration = shared_bridge_state
            .acceleration
            .scaled(self.bridge_coupling_factor);

        let mut hammer_angle_source = hammer_angle_at_step_start;
        let mut felt_compression_source = felt_compression_at_step_start;
        let mut compression_reference_for_rate_term = self.previous_felt_compressions;
        let mut bridge_force = Vector3::ZERO;
        let last_sweep_index = sweep_count - 1;

        for sweep_index in 0..sweep_count {
            if sweep_index > 0 {
                self.restore_state_snapshot(&state_snapshot);
                compression_reference_for_rate_term = felt_compression_at_step_start;
            }

            let mut total_felt_force_on_strings = Vector3::ZERO;
            let mut forces_on_strings = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
            for string_index in 0..string_count {
                let mut felt_frame_force = self.hammer.felt_contact_force(
                    felt_compression_source[string_index],
                    compression_reference_for_rate_term[string_index],
                    step_size,
                );
                felt_frame_force = felt_frame_force * self.hammer_force_scales[string_index];
                total_felt_force_on_strings = total_felt_force_on_strings + felt_frame_force;
                forces_on_strings[string_index] = self
                    .hammer
                    .force_on_string_in_string_frame(felt_frame_force, hammer_angle_source)
                    + hammer_noise_force;
            }

            for string_index in 0..string_count {
                self.strings[string_index].apply_forces_and_advance(
                    forces_on_strings[string_index],
                    scaled_bridge_acceleration,
                    step_size,
                );
            }

            let total_shank_torque = self
                .hammer
                .shank_torque(hammer_angle_source, total_felt_force_on_strings)
                + self.hammer.rest_rail_torque();
            self.hammer
                .advance_shank_rotation(total_shank_torque, step_size);

            let mut voice_bridge_force = Vector3::ZERO;
            for piano_string in &self.strings {
                voice_bridge_force = voice_bridge_force
                    + piano_string.force_exerted_on_bridge(shared_bridge_state.displacement);
            }

            if sweep_index == last_sweep_index && self.duplex_output_scale > 1.0e-6 {
                let duplex_drive =
                    voice_bridge_force.z * self.duplex_output_scale * DUPLEX_DRIVE_SCALE;
                for resonator in &mut self.duplex_resonators {
                    resonator.advance(duplex_drive, step_size);
                }
                let mut duplex_reaction_force = 0.0;
                for (resonator, &reaction_coefficient) in self
                    .duplex_resonators
                    .iter()
                    .zip(self.duplex_reaction_coefficients.iter())
                {
                    duplex_reaction_force += reaction_coefficient * resonator.displacement;
                }
                voice_bridge_force.z += duplex_reaction_force;
            }

            bridge_force = voice_bridge_force * self.blooming_envelope;

            if sweep_index == last_sweep_index {
                bridge_force = bridge_force
                    + Vector3::new(
                        0.0,
                        hammer_noise_force.z * 0.25 * HAMMER_THUMP_BRIDGE_SCALE,
                        hammer_noise_force.z * HAMMER_THUMP_BRIDGE_SCALE,
                    )
                    + release_noise_force;
            }

            hammer_angle_source = self.hammer.rotation_angle;

            let mut predicted_felt_compressions = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
            for string_index in 0..string_count {
                let string_displacement_at_strike = self.strings[string_index]
                    .displacement_at_strike_point(shared_bridge_state.displacement);
                let (compression, _) = self.hammer.felt_compression(
                    hammer_angle_source,
                    string_displacement_at_strike,
                    self.strings[string_index].lateral_offset_from_hammer_center,
                    self.felt_contact_references[string_index],
                );
                predicted_felt_compressions[string_index] = compression;
            }
            felt_compression_source = predicted_felt_compressions;
        }

        if sweep_count > 1 {
            self.snapshot_buffer = state_snapshot;
        }

        self.previous_felt_compressions = felt_compression_at_step_start;
        bridge_force
    }

    pub fn is_in_strike_phase(&self, shared_bridge_displacement: Vector3) -> bool {
        if self.hammer.angular_velocity > 0.0
            && self.hammer.rotation_angle >= self.strike_activation_angle
        {
            return true;
        }
        self.any_string_in_contact(shared_bridge_displacement)
    }

    fn any_string_in_contact(&self, shared_bridge_displacement: Vector3) -> bool {
        if self.hammer.rotation_angle < self.strike_activation_angle - STRIKE_PHASE_ANGLE_MARGIN {
            return false;
        }
        for string_index in 0..self.strings.len() {
            let string_displacement =
                self.strings[string_index].displacement_at_strike_point(shared_bridge_displacement);
            let (_, contact_detected) = self.hammer.felt_compression(
                self.hammer.rotation_angle,
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

    pub fn string_energy(&self) -> f64 {
        let mut total_energy = 0.0;
        for piano_string in &self.strings {
            let all_modes = piano_string
                .vertical_modes
                .iter()
                .chain(piano_string.horizontal_modes.iter())
                .chain(piano_string.longitudinal_modes.iter());
            for mode in all_modes {
                total_energy += 0.5
                    * mode.modal_mass
                    * (mode.velocity * mode.velocity
                        + mode.angular_frequency
                            * mode.angular_frequency
                            * mode.displacement
                            * mode.displacement);
            }
        }
        total_energy
    }

    pub fn is_silent(&self, energy_threshold: f64) -> bool {
        self.string_energy() < energy_threshold
            && self.release_noise_seconds_remaining <= 0.0
            && self.hammer_noise_seconds_remaining <= 0.0
    }

    fn write_state_snapshot(&self, snapshot: &mut Vec<f64>) {
        for piano_string in &self.strings {
            let all_modes = piano_string
                .vertical_modes
                .iter()
                .chain(piano_string.horizontal_modes.iter())
                .chain(piano_string.longitudinal_modes.iter());
            for mode in all_modes {
                snapshot.push(mode.displacement);
                snapshot.push(mode.velocity);
            }
        }
        snapshot.push(self.hammer.rotation_angle);
        snapshot.push(self.hammer.angular_velocity);
    }

    fn restore_state_snapshot(&mut self, snapshot: &[f64]) {
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
        self.hammer.rotation_angle = snapshot[cursor];
        self.hammer.angular_velocity = snapshot[cursor + 1];
    }

    pub fn adopt_string_state_from(&mut self, other: &PianoVoice) {
        for (destination_string, source_string) in self.strings.iter_mut().zip(other.strings.iter())
        {
            for (destination, source) in destination_string
                .vertical_modes
                .iter_mut()
                .zip(source_string.vertical_modes.iter())
            {
                destination.displacement = source.displacement;
                destination.velocity = source.velocity;
            }
            for (destination, source) in destination_string
                .horizontal_modes
                .iter_mut()
                .zip(source_string.horizontal_modes.iter())
            {
                destination.displacement = source.displacement;
                destination.velocity = source.velocity;
            }
            for (destination, source) in destination_string
                .longitudinal_modes
                .iter_mut()
                .zip(source_string.longitudinal_modes.iter())
            {
                destination.displacement = source.displacement;
                destination.velocity = source.velocity;
            }
        }

        for (destination, source) in self
            .duplex_resonators
            .iter_mut()
            .zip(other.duplex_resonators.iter())
        {
            destination.displacement = source.displacement;
            destination.velocity = source.velocity;
        }

        for string_index in 0..self.strings.len() {
            let strike_point_displacement =
                self.strings[string_index].displacement_at_strike_point(Vector3::ZERO);
            let (compression, _) = self.hammer.felt_compression(
                self.grazing_contact_angle,
                strike_point_displacement,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );
            self.previous_felt_compressions[string_index] = compression;
        }
    }
}

pub struct SynthVoice {
    pub voice_id: VoiceID,
    pub internal_voice_id: u64,
    pub channel: Channel,
    pub key: Key,
    pub note: u8,
    pub releasing: bool,
    pub engine: PianoVoice,
}
