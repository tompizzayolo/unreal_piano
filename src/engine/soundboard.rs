pub const SOUNDBOARD_MODES: usize = 48;

const INPUT_GAIN_SCALE: f32 = 0.02;
const OUTPUT_GAIN_SCALE: f32 = 0.15;

/// Global soundboard resonator driven by the summed bridge forces of all active voices.
/// Modeled as a bank of second-order modal oscillators whose frequencies and gains are
/// pseudo-randomly distributed to approximate a real spruce-and-rib soundboard spectrum.
pub struct Soundboard {
    mode_eigenvalues: [f32; SOUNDBOARD_MODES],
    modal_displacements: [f32; SOUNDBOARD_MODES],
    modal_velocities: [f32; SOUNDBOARD_MODES],
    input_gains: [f32; SOUNDBOARD_MODES],
    output_gains: [f32; SOUNDBOARD_MODES],
}

impl Soundboard {
    pub fn new() -> Self {
        let mut soundboard = Self {
            mode_eigenvalues: [0.0; SOUNDBOARD_MODES],
            modal_displacements: [0.0; SOUNDBOARD_MODES],
            modal_velocities: [0.0; SOUNDBOARD_MODES],
            input_gains: [0.0; SOUNDBOARD_MODES],
            output_gains: [0.0; SOUNDBOARD_MODES],
        };
        soundboard.reset();
        soundboard
    }

    pub fn reset(&mut self) {
        let mut seed: u32 = 0x9E37_79B9;
        let mut random_unit = || -> f32 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / 16_777_216.0
        };

        for i in 0..SOUNDBOARD_MODES {
            let normalized_position = i as f32 / (SOUNDBOARD_MODES - 1) as f32;
            let frequency = 55.0
                * (5000.0_f32 / 55.0).powf(normalized_position)
                * (0.97 + 0.06 * random_unit());
            self.mode_eigenvalues[i] = (2.0 * std::f32::consts::PI * frequency).powi(2);
            self.modal_displacements[i] = 0.0;
            self.modal_velocities[i] = 0.0;
            self.input_gains[i] = (2.0 * random_unit() - 1.0) * INPUT_GAIN_SCALE;
            self.output_gains[i] = (2.0 * random_unit() - 1.0) * OUTPUT_GAIN_SCALE;
        }
    }

    /// Advance all soundboard modes by one sample, driven by the total bridge force.
    /// Returns the radiated velocity signal.
    pub fn process(&mut self, total_bridge_force: f32, dt: f32, decay_seconds: f32) -> f32 {
        let reference_eigenvalue = (2.0 * std::f32::consts::PI * 200.0).powi(2);
        let global_damping = 1.0 / (decay_seconds.max(0.01) * reference_eigenvalue);
        let mut output = 0.0_f32;

        for i in 0..SOUNDBOARD_MODES {
            let eigenvalue = self.mode_eigenvalues[i];
            let acceleration = self.input_gains[i] * total_bridge_force
                - 2.0 * global_damping * eigenvalue * self.modal_velocities[i]
                - eigenvalue * self.modal_displacements[i];
            self.modal_velocities[i] += acceleration * dt;
            self.modal_displacements[i] += self.modal_velocities[i] * dt;
            output += self.output_gains[i] * self.modal_velocities[i];
        }

        output
    }
}
