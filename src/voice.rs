//! One playable note: unison strings + hammer + per-voice contact
//! bookkeeping (paper §4, §6.1).
//!
//! This is the per-voice half of the former standalone engine.  The
//! soundboard, air and room are physically shared by every note of the
//! instrument, so they live in `instrument::Instrument` and are advanced
//! once for all voices.

use crate::engine::hammer::{FeltProperties, Hammer, HammerGeometry};
use crate::engine::soundboard::BridgeState;
use crate::engine::strings::PianoString;
use crate::engine::vector::Vector3;
use crate::instrument::NoteDesign;

/// Sub-steps per engine step while a hammer is engaging its strings.  The
/// felt's hysteretic term (eq. 6.4) acts as a very stiff damper on the
/// ~0.1 g of string mass that responds within a step; the explicit scheme
/// needs c·Δt/m < 1 there.
pub const SUB_STEPS_DURING_STRIKE: usize = 16;

/// How far below the grazing-contact angle the hammer rests on the action's
/// rest rail [rad] — the backcheck stand-in (see Hammer::rest_rail_torque).
const HAMMER_REST_ANGLE_DROP: f64 = 0.25;

/// Cheap pre-filter margin for the strike-phase test [rad].
const STRIKE_PHASE_ANGLE_MARGIN: f64 = 0.05;

/// Lateral misalignments of the unison contact points inside the felt [m];
/// they seed the horizontal polarization (§6.1.1).
const UNISON_LATERAL_MISALIGNMENTS: [f64; 3] = [0.28e-3, -0.22e-3, 0.34e-3];

/// Maximum number of strings in a unison group.
const MAXIMUM_UNISON_STRINGS: usize = 3;

/// The note-playing part of the model: everything a single key owns.
pub struct PianoVoice {
    pub strings: Vec<PianoString>,
    pub hammer: Hammer,
    /// ϑ^{n−1}: felt compression of each string at the previous committed
    /// (sub-)step (backward-difference reference of eq. (6.4)).
    previous_felt_compressions: [Vector3; MAXIMUM_UNISON_STRINGS],
    /// Per-string grazing-contact references in the felt frame.
    felt_contact_references: [Vector3; MAXIMUM_UNISON_STRINGS],
    /// Angle at which the felt first grazes the resting string.
    grazing_contact_angle: f64,
    /// Shank angle above which (and while rising, or while in contact) the
    /// strike phase is active.
    strike_activation_angle: f64,
    /// Lateral offset used for the contact-angle calculations.
    reference_lateral_offset: f64,
    /// Damper: multiplies every string modal state once per step while the
    /// key is released (the felt damper draining string energy).
    damper_decay_per_step: f64,
    /// Reusable snapshot buffer for the coupling sweeps.
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
            strings.push(PianoString::new(
                design.string,
                design.unison_lateral_offsets[string_index],
                design.unison_relative_detunes[string_index],
            ));
        }

        let mut hammer = Hammer::new(shared_hammer_geometry, design.felt);
        // The middle string defines the grazing-contact configuration.
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
                UNISON_LATERAL_MISALIGNMENTS[string_index],
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

        // Pre-sized so that the coupling sweeps never allocate on the audio
        // thread.
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

        PianoVoice {
            strings,
            hammer,
            previous_felt_compressions,
            felt_contact_references,
            grazing_contact_angle: contact_angle,
            strike_activation_angle: contact_angle - STRIKE_PHASE_ANGLE_MARGIN,
            reference_lateral_offset,
            damper_decay_per_step: 1.0,
            snapshot_buffer: Vec::with_capacity(snapshot_capacity),
        }
    }

    /// Begin a strike: place the hammer at the grazing-contact position with
    /// the given contact velocity [m/s] (paper §6.1.1: t = 0 is the instant
    /// of first touch).
    pub fn strike(&mut self, hammer_velocity: f64) {
        self.hammer.rotation_angle = self.grazing_contact_angle;
        self.hammer.angular_velocity = 0.0;
        let compression_rate_per_radian = self
            .hammer
            .compression_rate_per_radian(self.grazing_contact_angle, self.reference_lateral_offset);
        self.hammer.angular_velocity = hammer_velocity / compression_rate_per_radian.max(1.0e-9);
        self.damper_decay_per_step = 1.0;
    }

    /// Begin damping: the string modal states decay to 1e-3 of their
    /// amplitude after `release_seconds`.
    pub fn start_release(&mut self, release_seconds: f64, engine_step_size: f64) {
        let decay_rate = -3.0 / release_seconds.max(1.0e-3); // ln(1000)
        self.damper_decay_per_step = (decay_rate * engine_step_size).exp();
    }

    /// Apply one step of damper decay to every string modal state.
    pub fn apply_damper_step(&mut self) {
        let decay_factor = self.damper_decay_per_step;
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

    /// Advance one (sub-)step of this voice.  `shared_bridge_state` comes
    /// from the shared soundboard; the returned vector is this voice's
    /// bridge-force contribution (§6.2).  Uses only stack scratch memory.
    pub fn advance(
        &mut self,
        step_size: f64,
        shared_bridge_state: &BridgeState,
        coupling_sweeps: usize,
    ) -> Vector3 {
        let string_count = self.strings.len();

        // ---- source data at the beginning of the (sub-)step (t^n) --------
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

        // Snapshot of t^n so each sweep can restart from it (§7.2).
        let mut state_snapshot = std::mem::take(&mut self.snapshot_buffer);
        state_snapshot.clear();
        self.write_state_snapshot(&mut state_snapshot);

        // Sweep 0 uses the committed t^n sources; later sweeps use the
        // previous sweep's predictions.  The hysteretic rate term is lagged
        // one sweep for stability.
        let mut hammer_angle_source = hammer_angle_at_step_start;
        let mut felt_compression_source = felt_compression_at_step_start;
        let mut compression_reference_for_rate_term = self.previous_felt_compressions;
        let mut bridge_force = Vector3::ZERO;

        for sweep_index in 0..coupling_sweeps.max(1) {
            if sweep_index > 0 {
                self.restore_state_snapshot(&state_snapshot);
                compression_reference_for_rate_term = felt_compression_at_step_start;
            }

            // ---- 1. hammer-string contact (§6.1, eqs. (6.1)-(6.4)) ------
            let mut total_felt_force_on_strings = Vector3::ZERO;
            let mut forces_on_strings = [Vector3::ZERO; MAXIMUM_UNISON_STRINGS];
            for string_index in 0..string_count {
                let felt_frame_force = self.hammer.felt_contact_force(
                    felt_compression_source[string_index],
                    compression_reference_for_rate_term[string_index],
                    step_size,
                );
                total_felt_force_on_strings = total_felt_force_on_strings + felt_frame_force;
                forces_on_strings[string_index] = self
                    .hammer
                    .force_on_string_in_string_frame(felt_frame_force, hammer_angle_source);
            }

            // ---- 2. strings (§4, §7.2.1) ---------------------------------
            for string_index in 0..string_count {
                self.strings[string_index].apply_forces_and_advance(
                    forces_on_strings[string_index],
                    shared_bridge_state.acceleration,
                    step_size,
                );
            }

            // ---- 3. hammer shank rotation (eq. (6.6), §7.2.3) ------------
            let total_shank_torque = self
                .hammer
                .shank_torque(hammer_angle_source, total_felt_force_on_strings)
                + self.hammer.rest_rail_torque();
            self.hammer
                .advance_shank_rotation(total_shank_torque, step_size);

            // ---- 4. this voice's bridge-force contribution (§6.2) --------
            let mut voice_bridge_force = Vector3::ZERO;
            for piano_string in &self.strings {
                voice_bridge_force = voice_bridge_force
                    + piano_string.force_exerted_on_bridge(shared_bridge_state.displacement);
            }
            bridge_force = voice_bridge_force;

            // ---- 5. keep this sweep's predictions as next sweep's sources -
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

        self.snapshot_buffer = state_snapshot;
        self.previous_felt_compressions = felt_compression_at_step_start;
        bridge_force
    }

    /// True while the hammer is rising through the strike window or any
    /// string is in contact with the felt.
    pub fn is_in_strike_phase(&self, shared_bridge_displacement: Vector3) -> bool {
        if self.hammer.angular_velocity > 0.0
            && self.hammer.rotation_angle >= self.strike_activation_angle
        {
            return true;
        }
        self.any_string_in_contact(shared_bridge_displacement)
    }

    fn any_string_in_contact(&self, shared_bridge_displacement: Vector3) -> bool {
        // Once the hammer head has fallen out of the striking window it is
        // millimetres out of reach of any string excursion.
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

    /// Total string modal energy [J] — used to free silent voices.
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

    /// True when the voice has decayed into inaudibility.
    pub fn is_silent(&self, energy_threshold: f64) -> bool {
        self.string_energy() < energy_threshold
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
}

/// A plugin voice: note metadata (for event matching and voice stealing)
/// plus the physical-model voice.
pub struct SynthVoice {
    /// Host voice ID, or the fallback when the host provides none.
    pub voice_id: i32,
    pub channel: u8,
    pub note: u8,
    /// Monotonic ID used to steal the oldest voice.
    pub internal_voice_id: u64,
    /// True once the key (or the sustain pedal) has been released.
    pub releasing: bool,
    pub engine: PianoVoice,
}
