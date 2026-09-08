//! The shared body of the instrument: soundboard, air, room, and the master
//! step loop (paper §3, §5, §6.2, §6.3, §7.3).  Every voice drives, and
//! listens to, this one shared resonator — as in a real piano there is one
//! board and one room.
//!
//! The engine runs at a fixed internal rate of 48 kHz regardless of the
//! host's sample rate; output is produced by linear interpolation (all
//! modeled content stays far below half of it, and the strike phase uses
//! `SUB_STEPS_DURING_STRIKE` sub-steps, so the contact resolution is
//! unchanged from the validated standalone engine).

use crate::engine::air::{AirRoom, AirRoomDesign};
use crate::engine::hammer::{FeltProperties, HammerGeometry};
use crate::engine::soundboard::{Soundboard, SoundboardDesign};
use crate::engine::strings::{DampingCoefficients, PianoStringDesign};
use crate::engine::vector::Vector3;
use crate::voice::{PianoVoice, SUB_STEPS_DURING_STRIKE, SynthVoice};
use std::f64::consts::PI;

/// Fixed internal simulation rate [Hz].
pub const SIMULATION_RATE_HZ: f64 = 48_000.0;
/// Internal step size [s].
pub const SIMULATION_STEP_SIZE: f64 = 1.0 / SIMULATION_RATE_HZ;
/// Coupling sweeps per (sub-)step (paper §7.2).  1 keeps voices cheap; raise
/// to 2 for the extra contact refinement at ~2x per-voice cost.
pub const COUPLING_SWEEPS: usize = 1;
/// Room/soundboard ring-out after the last voice ends [s].
const RINGOUT_SECONDS: f64 = 0.75;

fn linear_interpolation(low: f64, high: f64, fraction: f64) -> f64 {
    low + (high - low) * fraction
}

// ---------------------------------------------------------------------------
// Per-note design (all 88 keys).
// ---------------------------------------------------------------------------

/// Everything needed to build the voice of one MIDI note.
#[derive(Clone, Copy)]
pub struct NoteDesign {
    pub string: PianoStringDesign,
    pub felt: FeltProperties,
    pub number_of_unison_strings: usize,
    pub unison_relative_detunes: [f64; 3],
    pub unison_lateral_offsets: [f64; 3],
}

/// Per-note physical scaling, anchored to the validated C4 preset: the
/// speaking length follows a realistic grand-piano curve, the core radius
/// schedule reproduces the string gauges, and the *effective density*
/// absorbs the winding mass of the bass strings so the fundamental is
/// exactly in tune at a roughly constant tension.  The winding trick is why
/// `material_density` may exceed steel down low — only the (thin) core
/// contributes to the bending stiffness, exactly as with a real wound
/// string.
pub fn note_design(midi_note: u8) -> NoteDesign {
    let note = midi_note.clamp(21, 108) as usize;
    let note_fraction = (note - 21) as f64 / 87.0; // 0 in the bass, 1 in the treble
    let fundamental_frequency = 440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0);

    // Speaking length: exponential C4 -> C8 in the treble, power law capped
    // at 1.42 m below the wound/plain transition.
    let speaking_length = if note >= 60 {
        0.62 * (0.052_f64 / 0.62).powf((note - 60) as f64 / 48.0)
    } else {
        (0.62 * (261.6256 / fundamental_frequency).powf(0.90)).min(1.42)
    };

    // Core radius [m], piecewise linear across the compass.
    let core_radius = if note <= 45 {
        linear_interpolation(0.95e-3, 0.72e-3, (note - 21) as f64 / 24.0)
    } else if note <= 60 {
        linear_interpolation(0.72e-3, 0.475e-3, (note - 45) as f64 / 15.0)
    } else {
        linear_interpolation(0.475e-3, 0.33e-3, (note - 60) as f64 / 48.0)
    };

    // Roughly iso-tension across the compass.
    let tension = linear_interpolation(750.0, 550.0, note_fraction);

    // Exact tuning by construction.
    let linear_mass_density = tension / (2.0 * speaking_length * fundamental_frequency).powi(2);
    let effective_density = (linear_mass_density / (PI * core_radius * core_radius)).max(7850.0);

    // Enough partials to be bright, few enough to stay cheap.
    let highest_modeled_frequency = (24.0 * fundamental_frequency)
        .min(8000.0)
        .max(4.5 * fundamental_frequency);

    let string = PianoStringDesign {
        speaking_length,
        core_radius,
        material_density: effective_density,
        youngs_modulus: 2.0e11,
        static_tension: tension,
        hammer_strike_ratio: 0.11 + 0.06 * note_fraction,
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
            constant_part: 8.0e-3,
            stiffness_proportional_part: 0.0,
        },
    };

    // Hammer felt gets noticeably harder toward the treble.
    let normal_felt_stiffness = linear_interpolation(9.0e10, 4.5e11, note_fraction);
    let felt_exponent = linear_interpolation(2.7, 3.4, note_fraction);
    let felt_relaxation_time = linear_interpolation(3.2e-4, 1.2e-4, note_fraction);
    let felt = FeltProperties {
        stiffness: [
            normal_felt_stiffness / 6.0,
            normal_felt_stiffness / 6.0,
            normal_felt_stiffness,
        ],
        nonlinearity_exponent: [felt_exponent; 3],
        relaxation_time: [felt_relaxation_time; 3],
    };

    let number_of_unison_strings = if note < 33 {
        1 // wound monochord
    } else if note < 45 {
        2 // wound bichord
    } else {
        3 // plain trichord
    };

    NoteDesign {
        string,
        felt,
        number_of_unison_strings,
        unison_relative_detunes: [-0.0012, 0.0004, 0.0010],
        unison_lateral_offsets: [-0.002, 0.0, 0.002],
    }
}

/// Hammer geometry shared by all notes (one hammer rail; only the felt law
/// varies per note).
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

// ---------------------------------------------------------------------------
// Direct + early-reflection field (supplements the truncated modal air).
// ---------------------------------------------------------------------------

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
            ((from.x - to.x).powi(2) + (from.y - to.y).powi(2) + (from.z - to.z).powi(2)).sqrt()
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

    /// p_direct(t) = Σ gain_k · V̇(t − delay_k).
    fn pressure(&self, volume_acceleration_history: &SampleHistory) -> f64 {
        self.sound_paths
            .iter()
            .map(|path| path.gain * volume_acceleration_history.value_at_age(path.delay_seconds))
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Ring buffer of the board's volume acceleration (96 kHz-style grid).
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
            return 0.0;
        }
        let newest_index = (self.write_index + capacity - 1) % capacity;
        let index_newer = (newest_index + capacity - whole_steps_back) % capacity;
        let index_older = (newest_index + capacity - whole_steps_back - 1) % capacity;
        self.samples[index_newer] * (1.0 - fraction) + self.samples[index_older] * fraction
    }
}

// ---------------------------------------------------------------------------
// The instrument.
// ---------------------------------------------------------------------------

pub struct Instrument {
    pub soundboard: Soundboard,
    pub air: AirRoom,
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

    /// Advance the whole instrument by one engine step.  Returns the
    /// listener pressure at the end of the step.  When no voice is active
    /// and the room has finished ringing out this is a cheap no-op that
    /// returns silence (the history keeps moving so the direct field stays
    /// valid).
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

        // Direct-field history on the engine-rate grid.
        let current_volume_acceleration = self.soundboard.volume_acceleration();
        self.volume_acceleration_history
            .push(current_volume_acceleration);

        // Strike-phase sub-stepping is global: all voices and the shared
        // resonators advance on the same clock.
        let bridge_state_for_phase_check = self.soundboard.bridge_state();
        let strike_phase_is_active = voices.iter().flatten().any(|voice| {
            voice
                .engine
                .is_in_strike_phase(bridge_state_for_phase_check.displacement)
        });
        let sub_step_count = if strike_phase_is_active {
            SUB_STEPS_DURING_STRIKE
        } else {
            1
        };
        let sub_step_size = SIMULATION_STEP_SIZE / sub_step_count as f64;

        // The damper advances once per engine step.
        for voice in voices.iter_mut().flatten() {
            if voice.releasing {
                voice.engine.apply_damper_step();
            }
        }

        for _ in 0..sub_step_count {
            // The shared bridge state is held constant over a sub-step; the
            // board/air coupling is weak, and each voice iterates its own
            // stiff hammer-string coupling through the sweeps.
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

    /// Zero all shared state (called from the plugin's `reset()`).
    pub fn reset(&mut self) {
        self.soundboard.reset();
        self.air.reset();
        self.volume_acceleration_history.clear();
        self.ringout_steps_remaining = 0;
        self.simulated_time = 0.0;
    }

    /// Pressure at the listener — the audio output of the model.
    pub fn listener_pressure(&self) -> f64 {
        self.air.pressure_at_listener()
            + self
                .direct_and_early_field
                .pressure(&self.volume_acceleration_history)
    }
}
