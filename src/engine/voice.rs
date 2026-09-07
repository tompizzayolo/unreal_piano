use crate::params::ParamSnapshot;

pub const STRING_MODES: usize = 8;

const HAMMER_MASS: f32 = 0.008;
const HAMMER_GAP: f32 = 0.002;
const STRING_FORCE_SCALE: f32 = 12.0;
const BRIDGE_SCALE: f32 = 40.0;
const DAMPER_MULTIPLIER: f32 = 30.0;

pub struct VoiceOutput {
    pub bridge_force: f32,
    pub direct_signal: f32,
}

/// A single piano voice: one hammer striking one string (modal reduction).
///
/// Paper references:
/// - Hammer felt force: equation 6.4 (unilateral nonlinear contact)
/// - String modal ODEs: equation 7.7
/// - Bridge force transmission: equation 6.18
/// - Velocity continuity coupling: equation 6.19
#[derive(Debug, Clone)]
pub struct Voice {
    /// Host-provided voice ID for polyphonic events.
    pub voice_id: i32,
    /// MIDI channel.
    pub channel: u8,
    /// MIDI note number.
    pub note: u8,
    /// Internal voice ID for age-based stealing.
    pub internal_voice_id: u64,
    /// Stereo pan position, derived from note number.
    pub pan: f32,
    /// Whether note-off has been received.
    pub releasing: bool,

    // -- Hammer state --
    hammer_active: bool,
    hammer_made_contact: bool,
    hammer_position: f32,
    hammer_velocity: f32,

    // -- String modal state --
    modal_displacements: [f32; STRING_MODES],
    modal_velocities: [f32; STRING_MODES],
    mode_eigenvalues: [f32; STRING_MODES],
    contact_mode_shapes: [f32; STRING_MODES],
    bridge_mode_shapes: [f32; STRING_MODES],
}

impl Voice {
    pub fn new(
        voice_id: i32,
        channel: u8,
        note: u8,
        internal_voice_id: u64,
        frequency: f32,
        velocity: f32,
        inharmonicity: f32,
    ) -> Self {
        let mut voice = Self {
            voice_id,
            channel,
            note,
            internal_voice_id,
            pan: ((note % 12) as f32 / 11.0 - 0.5) * 0.6,
            releasing: false,

            hammer_active: true,
            hammer_made_contact: false,
            hammer_position: -HAMMER_GAP,
            hammer_velocity: 0.3 + 5.0 * velocity,

            modal_displacements: [0.0; STRING_MODES],
            modal_velocities: [0.0; STRING_MODES],
            mode_eigenvalues: [1.0; STRING_MODES],
            contact_mode_shapes: [0.0; STRING_MODES],
            bridge_mode_shapes: [0.0; STRING_MODES],
        };

        let omega_fundamental = 2.0 * std::f32::consts::PI * frequency;
        for i in 0..STRING_MODES {
            let mode_number = (i + 1) as f32;
            // Stiff-string inharmonicity: omega_n = n * omega_1 * sqrt(1 + B * n^2)
            let omega_mode = omega_fundamental
                * mode_number
                * (1.0 + inharmonicity * mode_number * mode_number).sqrt();
            voice.mode_eigenvalues[i] = omega_mode * omega_mode;
            // Hammer strikes near L/7; bridge pickup near 0.94 * L
            voice.contact_mode_shapes[i] = (mode_number * std::f32::consts::PI / 7.0).sin();
            voice.bridge_mode_shapes[i] = (mode_number * std::f32::consts::PI * 0.94).sin();
        }

        voice
    }

    pub fn note_off(&mut self) {
        self.releasing = true;
    }

    /// Returns true when the voice has decayed enough to be freed.
    pub fn is_dead(&self) -> bool {
        if self.hammer_active {
            return false;
        }
        if !self.releasing {
            return false;
        }
        let energy: f32 = self
            .modal_displacements
            .iter()
            .zip(self.modal_velocities.iter())
            .map(|(displacement, velocity)| displacement.abs() + velocity.abs())
            .sum();
        energy < 1e-7
    }

    // -- Private helpers to sum modal contributions at specific string positions --

    fn string_displacement_at_contact(&self) -> f32 {
        self.contact_mode_shapes
            .iter()
            .zip(self.modal_displacements.iter())
            .map(|(shape, displacement)| shape * displacement)
            .sum()
    }

    fn string_velocity_at_contact(&self) -> f32 {
        self.contact_mode_shapes
            .iter()
            .zip(self.modal_velocities.iter())
            .map(|(shape, velocity)| shape * velocity)
            .sum()
    }

    fn string_velocity_at_bridge(&self) -> f32 {
        self.bridge_mode_shapes
            .iter()
            .zip(self.modal_velocities.iter())
            .map(|(shape, velocity)| shape * velocity)
            .sum()
    }

    /// Advance this voice by one sample. Returns the bridge force fed to the soundboard
    /// and a direct radiation signal.
    pub fn process(&mut self, dt: f32, params: &ParamSnapshot) -> VoiceOutput {
        let damper_active = self.releasing && !params.sustain_held;

        // ----------------------------------------------------------------
        // 1. Hammer felt contact force (paper equation 6.4, unilateral)
        // ----------------------------------------------------------------
        let mut felt_force = 0.0_f32;
        if self.hammer_active {
            let string_position = self.string_displacement_at_contact();
            let string_velocity = self.string_velocity_at_contact();
            let compression = self.hammer_position - string_position;

            if compression > 0.0 {
                let compression_power = compression.powf(params.hammer_exponent);
                let compression_rate = params.hammer_exponent
                    * compression.powf(params.hammer_exponent - 1.0)
                    * (self.hammer_velocity - string_velocity);
                felt_force = params.hammer_stiffness * compression_power
                    + params.hammer_hysteresis * params.hammer_stiffness * compression_rate;
                felt_force = felt_force.max(0.0);
                self.hammer_made_contact = true;
            } else if self.hammer_made_contact {
                // Hammer has separated from the string
                self.hammer_active = false;
                self.hammer_made_contact = false;
            }

            if self.hammer_active {
                self.hammer_velocity -= (felt_force / HAMMER_MASS) * dt;
                self.hammer_position += self.hammer_velocity * dt;
            }
        }

        // ----------------------------------------------------------------
        // 2. String modal update (paper equation 7.7, semi-implicit)
        // ----------------------------------------------------------------
        let damping_base = 1.0 / (params.string_decay.max(0.01) * self.mode_eigenvalues[0]);
        let damping_effective = if damper_active {
            damping_base * DAMPER_MULTIPLIER
        } else {
            damping_base
        };

        let bridge_velocity = self.string_velocity_at_bridge();
        // Velocity-continuity feedback (paper equation 6.19)
        let bridge_feedback = -params.soundboard_coupling * bridge_velocity * 2.0;

        for i in 0..STRING_MODES {
            let damping_for_mode = damping_effective * (1.0 + 0.02 * i as f32);
            let modal_force = self.contact_mode_shapes[i] * felt_force * STRING_FORCE_SCALE
                + self.bridge_mode_shapes[i] * bridge_feedback;
            let acceleration = modal_force
                - 2.0 * damping_for_mode * self.mode_eigenvalues[i] * self.modal_velocities[i]
                - self.mode_eigenvalues[i] * self.modal_displacements[i];
            self.modal_velocities[i] += acceleration * dt;
            self.modal_displacements[i] += self.modal_velocities[i] * dt;
        }

        // ----------------------------------------------------------------
        // 3. Bridge force to soundboard (paper equation 6.18, reduced)
        // ----------------------------------------------------------------
        let bridge_force = params.soundboard_coupling * bridge_velocity * BRIDGE_SCALE;
        let direct_signal = self.string_velocity_at_contact();

        VoiceOutput {
            bridge_force,
            direct_signal,
        }
    }
}
