//! The coupled piano system and its explicit time-stepping scheme
//! (paper §6 and §7.2–§7.3).
//!
//! Per time step the subsystems are advanced with the exact modal
//! propagators of §7.2 while all coupling forces are evaluated explicitly at
//! the sources of the current sweep.  The step is repeated
//! `coupling_sweeps` times (Gauss–Seidel style, restarted from the t^n
//! snapshot) so that later sweeps see the *predicted* end-of-step source
//! values — the paper's "more than one iteration of numeric integration at
//! each time step may be run to improve convergence".  The hysteretic rate
//! term of eq. (6.4) is lagged one sweep, which keeps the stiff contact
//! coupling convergent.
//!
//! While the hammer is engaging the strings, the step is additionally
//! sub-divided (`SUB_STEPS_DURING_STRIKE`): the felt's relaxation term acts
//! as a very stiff damper (c = r·k·p·|ϑ|^{p−1} ≈ 25–150 N·s/m) acting on the
//! ~0.1 g of string mass that responds within one step, and the explicit
//! scheme needs c·Δt/m < 1 there — a single 96 kHz step is marginal at
//! fortissimo, sixteen sub-steps are not.
//!
//! Sweep order: contact forces -> strings -> shank -> soundboard -> air.

use super::air::{AirRoom, AirRoomDesign};
use super::complex::Complex64;
use super::hammer::{FeltProperties, Hammer, HammerGeometry};
use super::soundboard::{Soundboard, SoundboardDesign};
use super::strings::{DampingCoefficients, PianoString, PianoStringDesign};
use super::vector::Vector3;
use std::f64::consts::PI;

/// Relative tension detuning of the unison strings (§6.1.2); produces the
/// characteristic slow beating between the three strings of one note.
const UNISON_RELATIVE_DETUNES: [f64; 3] = [-0.0012, 0.0004, 0.0010];

/// Lateral spacing of the unison strings under the felt [m].
const UNISON_LATERAL_OFFSETS: [f64; 3] = [-0.002, 0.0, 0.002];

/// Small lateral offsets of each unison string's contact point inside the
/// felt — the hammer never touches all three strings perfectly centered.
/// They seed the horizontal polarization (the "horizontal interaction
/// force" of §6.1.1) and, together with the unison detuning, produce the
/// slow beating / double-decay behaviour.
const UNISON_LATERAL_MISALIGNMENTS: [f64; 3] = [0.28e-3, -0.22e-3, 0.34e-3];

/// The strike phase is advanced in this many sub-steps.  The hysteretic term
/// of eq. (6.4) behaves like a damper with c ≈ 25–150 N·s/m acting on the
/// ~0.1 g of string mass that responds within a step; explicit integration
/// needs c·Δt/m < 1, i.e. Δt ≲ 1–5 µs at peak compression.  One 96 kHz step
/// (10.4 µs) is right at the edge; sixteen sub-steps (0.65 µs) sit
/// comfortably inside the stability region.  Raise this if you simulate
/// extreme (fff and beyond) strike velocities.
pub const SUB_STEPS_DURING_STRIKE: usize = 16;

/// How far below the grazing-contact angle (in radians) the hammer is still
/// considered capable of touching a vibrating string — used as a cheap
/// pre-filter before the per-string contact test.  0.05 rad ≈ 6.8 mm of
/// hammer-head travel, comfortably more than any string excursion.
const STRIKE_PHASE_ANGLE_MARGIN: f64 = 0.05;
const HAMMER_REST_ANGLE_DROP: f64 = 0.25;
/// Preset notes.  Values are physically plausible for a grand piano; every
/// one of them is exposed for tuning.
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
    /// Hammer speed at the moment of first contact [m/s] (§6.1.1).
    pub hammer_strike_velocity: f64,
    /// Simulation time step Δt [s] (§7.3).
    pub time_step: f64,
    /// Coupling iterations per (sub-)step (§7.2).
    pub coupling_sweeps: usize,
    /// How long to simulate [s].
    pub simulation_duration: f64,
    /// Audio output sample rate [Hz].
    pub audio_sample_rate: usize,
    /// Seed for the reproducible soundboard modal data.
    pub random_seed: u64,
}

fn note_designs(note: NoteName) -> (PianoStringDesign, HammerGeometry, FeltProperties) {
    // Shared hammer geometry (the hammer frame of fig. 5).
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

// ---------------------------------------------------------------------------
// Direct + early-reflection field (supplements the truncated modal air).
// ---------------------------------------------------------------------------

/// One propagation path: a delay and a gain.
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
            ((from.x - to.x).powi(2) + (from.y - to.y).powi(2) + (from.z - to.z).powi(2)).sqrt()
        };
        let mut sound_paths = Vec::with_capacity(7);

        // Direct path.
        let direct_distance = distance_between(soundboard_position, listener_position);
        sound_paths.push(SoundPath {
            delay_seconds: direct_distance / speed_of_sound,
            gain: air_density / (4.0 * PI * direct_distance),
        });

        // Six first-order wall images of the source.
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

    /// p_direct(t) = Σ gain_k · V̇(t − delay_k)  (Rayleigh-type far field).
    fn pressure(&self, volume_acceleration_history: &SampleHistory) -> f64 {
        self.sound_paths
            .iter()
            .map(|path| path.gain * volume_acceleration_history.value_at_age(path.delay_seconds))
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Ring buffer of the board's volume acceleration.
// ---------------------------------------------------------------------------

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

    fn push(&mut self, value: f64) {
        self.samples[self.write_index] = value;
        self.write_index = (self.write_index + 1) % self.samples.len();
        self.total_samples_pushed += 1;
    }

    /// Linearly interpolated value `age_seconds` before the newest sample.
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
            return 0.0; // before the beginning of the recorded history
        }
        let newest_index = (self.write_index + capacity - 1) % capacity;
        let index_newer = (newest_index + capacity - whole_steps_back) % capacity;
        let index_older = (newest_index + capacity - whole_steps_back - 1) % capacity;
        self.samples[index_newer] * (1.0 - fraction) + self.samples[index_older] * fraction
    }
}

// ---------------------------------------------------------------------------
// The coupled piano.
// ---------------------------------------------------------------------------

pub struct Piano {
    pub strings: Vec<PianoString>,
    pub hammer: Hammer,
    pub soundboard: Soundboard,
    pub air: AirRoom,
    pub time_step: f64,
    pub coupling_sweeps: usize,
    pub simulated_time: f64,
    /// ϑ^{n−1}: felt compression of each string at the previous committed
    /// (sub-)step (backward-difference reference of eq. (6.4)).
    previous_felt_compressions: Vec<Vector3>,
    /// Per-string grazing-contact reference in the felt frame.  Eq. (6.3)
    /// must be evaluated *relative to the point where the felt first touches
    /// the string* (a point fixed in the shank-attached felt frame); the raw
    /// x1'' coordinates otherwise smuggle ~1.4 cm of static contact geometry
    /// into the tangential components, and the power law (6.4) produces
    /// kilonewton-to-meganewton forces that destroy the simulation.
    felt_contact_references: Vec<Vector3>,
    /// Shank angle above which (and while rising, or while touching) the
    /// time step is sub-divided for contact stability.
    strike_activation_angle: f64,
    volume_acceleration_history: SampleHistory,
    direct_and_early_field: DirectAndEarlyField,
    snapshot_buffer: Vec<f64>,
}

impl Piano {
    pub fn new(configuration: &PianoConfiguration) -> Self {
        let (string_design, hammer_geometry, felt_properties) = note_designs(configuration.note);

        // ---- strings (§4, §6.1.2: one hammer striking three strings) ----
        let number_of_unison_strings = UNISON_RELATIVE_DETUNES.len();
        let mut strings = Vec::with_capacity(number_of_unison_strings);
        for string_index in 0..number_of_unison_strings {
            strings.push(PianoString::new(
                string_design,
                UNISON_LATERAL_OFFSETS[string_index],
                UNISON_RELATIVE_DETUNES[string_index],
            ));
        }

        // ---- hammer (§6.1): t = 0 is the grazing-contact instant ---------
        let mut hammer = Hammer::new(hammer_geometry, felt_properties);
        let contact_angle = hammer.resting_contact_angle(UNISON_LATERAL_OFFSETS[1]);
        hammer.rotation_angle = contact_angle;
        hammer.rest_angle = contact_angle - HAMMER_REST_ANGLE_DROP;
        let compression_rate_per_radian =
            hammer.compression_rate_per_radian(contact_angle, UNISON_LATERAL_OFFSETS[1]);
        hammer.angular_velocity =
            configuration.hammer_strike_velocity / compression_rate_per_radian.max(1.0e-9);

        // Grazing-contact reference of each string (fix for eq. (6.3)) plus
        // a small per-string lateral misalignment that seeds the horizontal
        // polarization.  The true initial compressions are recorded so the
        // first step's backward difference does not see a phantom jump.
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

        // Sub-stepping activates slightly before first touch, so the whole
        // approach and compression phase runs at the finer time step.
        let strike_activation_angle = contact_angle - STRIKE_PHASE_ANGLE_MARGIN;

        // ---- soundboard (§3, modal reduction) ----------------------------
        let soundboard = Soundboard::new(&SoundboardDesign {
            mode_count: 56,
            lowest_mode_frequency: 70.0,
            highest_mode_frequency: 4000.0,
            random_seed: configuration.random_seed,
        });

        // ---- air + room (§5) ---------------------------------------------
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

        // ---- direct + early-reflection path (§5.1 discussion) ------------
        let simulation_rate = 1.0 / configuration.time_step;
        let maximum_image_distance = 12.0; // generous bound for first-order images
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
            snapshot_buffer: Vec::new(),
        }
    }

    pub fn current_time(&self) -> f64 {
        self.simulated_time
    }

    /// Pressure at the listener: modal room response + direct/early field.
    pub fn listener_pressure(&self) -> f64 {
        self.air.pressure_at_listener()
            + self
                .direct_and_early_field
                .pressure(&self.volume_acceleration_history)
    }

    /// Numerical-health check used by the demo driver.
    pub fn state_is_finite(&self) -> bool {
        self.hammer.rotation_angle.is_finite()
            && self.hammer.angular_velocity.is_finite()
            && self
                .soundboard
                .modes
                .iter()
                .all(|mode| mode.displacement.is_finite() && mode.velocity.is_finite())
            && self
                .air
                .modal_states
                .iter()
                .all(|mode| mode.state.real_part.is_finite())
            && self.strings.iter().all(|piano_string| {
                piano_string
                    .vertical_modes
                    .iter()
                    .all(|mode| mode.displacement.is_finite())
            })
    }

    // -----------------------------------------------------------------------
    // Snapshot / restore (restart each coupling sweep from t^n, §7.2).
    // -----------------------------------------------------------------------

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
        for mode in &self.soundboard.modes {
            snapshot.push(mode.displacement);
            snapshot.push(mode.velocity);
        }
        for mode in &self.air.modal_states {
            snapshot.push(mode.state.real_part);
            snapshot.push(mode.state.imaginary_part);
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

    // -----------------------------------------------------------------------
    // Time stepping (§7.3).
    // -----------------------------------------------------------------------

    /// One full time step of the coupled system.
    ///
    /// The step is sub-divided while the hammer engages the strings: the
    /// felt law (6.4) is the stiffest element in the model and needs a
    /// smaller Δt than the audio-rate step.  The direct-field history and
    /// the output grid stay on the outer step.
    pub fn step(&mut self) {
        let outer_step_size = self.time_step;

        // Record the board's volume acceleration for the direct field.
        let current_volume_acceleration = self.soundboard.volume_acceleration();
        self.volume_acceleration_history
            .push(current_volume_acceleration);

        // Strike phase: hammer rising through the activation angle, or any
        // string currently in contact (covers a possible bounce re-contact).
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

    /// True if any string currently penetrates the felt surface.
    ///
    /// Guarded by a cheap angle pre-filter: once the hammer head has fallen
    /// more than `STRIKE_PHASE_ANGLE_MARGIN` below the grazing angle, it is
    /// millimetres out of reach of any string excursion and the per-string
    /// geometry test can be skipped.
    fn any_string_in_contact(&self) -> bool {
        if self.hammer.rotation_angle < self.strike_activation_angle - STRIKE_PHASE_ANGLE_MARGIN {
            return false;
        }
        let bridge_displacement = self.soundboard.bridge_state().displacement;
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

    /// One (sub-)step of the coupled system: the sweep machinery of §7.2/§7.3.
    ///
    /// Per sweep: hammer-string contact forces (§6.1) -> strings (§4) ->
    /// shank rotation (eq. 6.6) -> bridge forces -> soundboard (§6.2) ->
    /// air (§5.1).  Each sweep restarts from the t^n snapshot and uses the
    /// previous sweep's predictions as coupling sources.
    fn advance_coupled_sub_step(&mut self, step_size: f64) {
        let string_count = self.strings.len();

        // ---- source data at the beginning of the (sub-)step (t^n) --------
        let bridge_state_at_step_start = self.soundboard.bridge_state();
        let hammer_angle_at_step_start = self.hammer.rotation_angle;

        let mut felt_compression_at_step_start: Vec<Vector3> = Vec::with_capacity(string_count);
        for string_index in 0..string_count {
            let string_displacement_at_strike = self.strings[string_index]
                .displacement_at_strike_point(bridge_state_at_step_start.displacement);
            let (compression, _contact_detected) = self.hammer.felt_compression(
                hammer_angle_at_step_start,
                string_displacement_at_strike,
                self.strings[string_index].lateral_offset_from_hammer_center,
                self.felt_contact_references[string_index],
            );
            felt_compression_at_step_start.push(compression);
        }

        // Snapshot of t^n so each sweep can restart from it.
        let mut state_snapshot = std::mem::take(&mut self.snapshot_buffer);
        state_snapshot.clear();
        self.write_state_snapshot(&mut state_snapshot);

        // Coupling sources for the current sweep: sweep 0 uses the committed
        // t^n values; later sweeps use the previous sweep's predictions
        // ("rhs source terms of the next time step", §7.2).
        let mut hammer_angle_source = hammer_angle_at_step_start;
        let mut felt_compression_source = felt_compression_at_step_start.clone();
        let mut bridge_state_source = bridge_state_at_step_start;
        // Lagged reference of the hysteretic rate term: sweep 0 → ϑ^{n−1}
        // (backward difference), later sweeps → ϑ^n (forward difference).
        let mut compression_reference_for_rate_term = self.previous_felt_compressions.clone();

        for sweep_index in 0..self.coupling_sweeps {
            if sweep_index > 0 {
                self.restore_state_snapshot(&state_snapshot);
                compression_reference_for_rate_term = felt_compression_at_step_start.clone();
            }

            // ---- 1. hammer-string contact (§6.1, eqs. (6.1)-(6.4)) ------
            let mut total_felt_force_on_strings = Vector3::ZERO;
            let mut hammer_forces_on_strings: Vec<Vector3> = Vec::with_capacity(string_count);
            for string_index in 0..string_count {
                let felt_frame_force = self.hammer.felt_contact_force(
                    felt_compression_source[string_index],
                    compression_reference_for_rate_term[string_index],
                    step_size,
                );
                total_felt_force_on_strings = total_felt_force_on_strings + felt_frame_force;
                let force_in_string_frame = self
                    .hammer
                    .force_on_string_in_string_frame(felt_frame_force, hammer_angle_source);
                hammer_forces_on_strings.push(force_in_string_frame);
            }

            // ---- 2. strings (§4, §7.2.1) ---------------------------------
            for string_index in 0..string_count {
                self.strings[string_index].apply_forces_and_advance(
                    hammer_forces_on_strings[string_index],
                    bridge_state_source.acceleration,
                    step_size,
                );
            }

            // ---- 3. hammer shank rotation (eq. (6.6), §7.2.3) -------------
            let total_shank_torque = self
                .hammer
                .shank_torque(hammer_angle_source, total_felt_force_on_strings)
                + self.hammer.rest_rail_torque();
            self.hammer
                .advance_shank_rotation(total_shank_torque, step_size);

            // ---- 4. bridge forces -> soundboard (§6.2) --------------------
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

            // ---- 5. air (§5.1, §7.2.2) ------------------------------------
            let soundboard_volume_velocity = self.soundboard.volume_velocity();
            self.air.advance(soundboard_volume_velocity, step_size);

            // ---- 6. keep this sweep's predictions as next sweep's sources -
            hammer_angle_source = self.hammer.rotation_angle;
            bridge_state_source = self.soundboard.bridge_state();
            let mut predicted_felt_compressions = Vec::with_capacity(string_count);
            for string_index in 0..string_count {
                let string_displacement_at_strike = self.strings[string_index]
                    .displacement_at_strike_point(bridge_state_source.displacement);
                let (compression, _) = self.hammer.felt_compression(
                    hammer_angle_source,
                    string_displacement_at_strike,
                    self.strings[string_index].lateral_offset_from_hammer_center,
                    self.felt_contact_references[string_index],
                );
                predicted_felt_compressions.push(compression);
            }
            felt_compression_source = predicted_felt_compressions;
        }

        self.snapshot_buffer = state_snapshot;

        // Commit: ϑ^{n−1} for the next step's backward difference.
        self.previous_felt_compressions = felt_compression_at_step_start;
    }
}
