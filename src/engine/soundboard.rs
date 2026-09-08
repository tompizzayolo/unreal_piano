//! Soundboard model (paper §3), modal reduction.
//!
//! The paper discretizes the full 3-D multi-layer orthotropic board with FEM,
//! solves the generalized eigenproblem K Φ = M Φ Ω² and truncates.  Here the
//! eigenpairs are *synthesized* with a seeded RNG using statistics typical
//! of a grand-piano board (roughly constant modal density, ζ ≈ 0.02–0.08,
//! modal masses of a few tenths of a kg).  Replacing this builder with FEM
//! data reproduces the paper's pipeline exactly.
//!
//! Coupling coefficients per mode:
//!  * bridge coupling (b_i^x, b_i^y, b_i^z): displacement of the bridge point
//!    per unit modal coordinate (§6.2 surface force transmission);
//!  * volume-displacement coupling ν_i: effective piston area of the mode
//!    (§6.3 soundboard–air coupling, reduced to an effective piston).

use super::modal::ModalOscillator;
use super::rng::DeterministicRandom;
use super::vector::Vector3;
use std::f64::consts::PI;

pub struct SoundboardDesign {
    /// Number of retained modes.
    pub mode_count: usize,
    /// Lowest synthesized mode frequency [Hz].
    pub lowest_mode_frequency: f64,
    /// Highest synthesized mode frequency [Hz].
    pub highest_mode_frequency: f64,
    /// Seed for the reproducible modal-data generator.
    pub random_seed: u64,
}

/// State of the bridge point (where the strings terminate).
pub struct BridgeState {
    /// Bridge point displacement in the string frame.
    pub displacement: Vector3,
    /// Bridge point acceleration in the string frame.
    pub acceleration: Vector3,
}

pub struct Soundboard {
    /// Modal oscillators of the board.
    pub modes: Vec<ModalOscillator>,
    /// b_i^z: vertical bridge coupling per mode.
    pub vertical_bridge_coupling: Vec<f64>,
    /// b_i^y: horizontal bridge coupling per mode.
    pub horizontal_bridge_coupling: Vec<f64>,
    /// b_i^x: longitudinal bridge coupling per mode (small).
    pub longitudinal_bridge_coupling: Vec<f64>,
    /// ν_i: volume-displacement coupling per mode [m²].
    pub volume_displacement_coupling: Vec<f64>,
    /// Modal force applied at the last step (bookkeeping for the coupling
    /// terms that need accelerations).
    last_applied_modal_forces: Vec<f64>,
}

impl Soundboard {
    pub fn new(design: &SoundboardDesign) -> Self {
        let mut random = DeterministicRandom::new(design.random_seed);
        let mode_count = design.mode_count.max(1);
        let mut modes = Vec::with_capacity(mode_count);
        let mut vertical_bridge_coupling = Vec::with_capacity(mode_count);
        let mut horizontal_bridge_coupling = Vec::with_capacity(mode_count);
        let mut longitudinal_bridge_coupling = Vec::with_capacity(mode_count);
        let mut volume_displacement_coupling = Vec::with_capacity(mode_count);
        let mut last_applied_modal_forces = vec![0.0; mode_count];

        for mode_index in 0..mode_count {
            let frequency = design.lowest_mode_frequency
                + (mode_index as f64 + random.unit_interval())
                    * (design.highest_mode_frequency - design.lowest_mode_frequency)
                    / mode_count as f64;
            let angular_frequency = 2.0 * PI * frequency;
            let damping_ratio = random.uniform(0.02, 0.06) * (1.0 + frequency / 2500.0);
            let modal_mass = random.uniform(0.3, 1.2);
            modes.push(ModalOscillator::new(
                angular_frequency,
                damping_ratio,
                modal_mass,
            ));

            let low_frequency_weight = (-frequency / 700.0).exp();
            vertical_bridge_coupling.push(
                random.random_sign()
                    * (0.10 + 0.80 * low_frequency_weight * random.unit_interval()),
            );
            horizontal_bridge_coupling.push(
                random.random_sign()
                    * (0.04 + 0.30 * low_frequency_weight * random.unit_interval()),
            );
            longitudinal_bridge_coupling.push(
                random.random_sign()
                    * (0.01 + 0.05 * low_frequency_weight * random.unit_interval()),
            );
            volume_displacement_coupling.push(
                random.random_sign()
                    * (0.02 + 0.30 * (-frequency / 500.0).exp() * random.unit_interval()),
            );
        }

        Soundboard {
            modes,
            vertical_bridge_coupling,
            horizontal_bridge_coupling,
            longitudinal_bridge_coupling,
            volume_displacement_coupling,
            last_applied_modal_forces,
        }
    }

    /// One modal step of the board (paper §7.2.1) under the total bridge
    /// force from the strings and the acoustic pressure load of the air.
    pub fn apply_forces_and_advance(
        &mut self,
        total_bridge_force_from_strings: Vector3,
        air_pressure_at_board: f64,
        step_size: f64,
    ) {
        for mode_index in 0..self.modes.len() {
            let modal_force = self.vertical_bridge_coupling[mode_index]
                * total_bridge_force_from_strings.z
                + self.horizontal_bridge_coupling[mode_index] * total_bridge_force_from_strings.y
                + self.longitudinal_bridge_coupling[mode_index] * total_bridge_force_from_strings.x
                - self.volume_displacement_coupling[mode_index] * air_pressure_at_board;
            self.last_applied_modal_forces[mode_index] = modal_force;
            self.modes[mode_index].advance(modal_force, step_size);
        }
    }

    /// Displacement and acceleration of the bridge point (§6.2).
    pub fn bridge_state(&self) -> BridgeState {
        let mut displacement = Vector3::ZERO;
        let mut acceleration = Vector3::ZERO;
        let coupling_sets = [
            &self.longitudinal_bridge_coupling,
            &self.horizontal_bridge_coupling,
            &self.vertical_bridge_coupling,
        ];
        for mode_index in 0..self.modes.len() {
            let modal_acceleration = self.modes[mode_index]
                .acceleration_for_force(self.last_applied_modal_forces[mode_index]);
            for axis in 0..3 {
                let coupling = coupling_sets[axis][mode_index];
                let displacement_contribution = coupling * self.modes[mode_index].displacement;
                let acceleration_contribution = coupling * modal_acceleration;
                match axis {
                    0 => {
                        displacement.x += displacement_contribution;
                        acceleration.x += acceleration_contribution;
                    }
                    1 => {
                        displacement.y += displacement_contribution;
                        acceleration.y += acceleration_contribution;
                    }
                    _ => {
                        displacement.z += displacement_contribution;
                        acceleration.z += acceleration_contribution;
                    }
                }
            }
        }
        BridgeState {
            displacement,
            acceleration,
        }
    }

    /// Total volume velocity injected into the room by the board [m³/s].
    pub fn volume_velocity(&self) -> f64 {
        let mut total_volume_velocity = 0.0;
        for mode_index in 0..self.modes.len() {
            total_volume_velocity +=
                self.volume_displacement_coupling[mode_index] * self.modes[mode_index].velocity;
        }
        total_volume_velocity
    }

    /// Total volume acceleration of the board [m³/s²] (drives the direct
    /// sound field).
    pub fn volume_acceleration(&self) -> f64 {
        let mut total_volume_acceleration = 0.0;
        for mode_index in 0..self.modes.len() {
            let modal_acceleration = self.modes[mode_index]
                .acceleration_for_force(self.last_applied_modal_forces[mode_index]);
            total_volume_acceleration +=
                self.volume_displacement_coupling[mode_index] * modal_acceleration;
        }
        total_volume_acceleration
    }

    pub fn reset(&mut self) {
        for mode in &mut self.modes {
            mode.displacement = 0.0;
            mode.velocity = 0.0;
        }
        self.last_applied_modal_forces.fill(0.0);
    }
}
