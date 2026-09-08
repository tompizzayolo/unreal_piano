//! Air + room model (paper §5), reduced to the first-order modal system.
//!
//! The paper's first-order acoustic equations (5.5)–(5.6) for (p, u) are
//! projected onto the rigid-wall cavity modes ψ_j of a shoebox room.  The
//! eigenvalue analysis of the resulting 2×2 blocks gives complex conjugate
//! eigenvalues λ_j = -ζ_j ω_j ± i ω_d,j (vortical modes are dropped: they
//! carry no pressure).  Projecting the soundboard-air velocity-continuity
//! source with the left eigenvector w_j = [1, -λ_j ρ] of each block gives
//!
//!     q_j' = λ_j q_j + g_j,
//!     g_j = (ρ c² / Λ_j) ψ_j(x_source) V̇ / (2 + 2 ζ_j λ_j / ω_j)
//!
//! where Λ_j is the modal volume, V̇ the board's volume velocity and
//! P_j = 2 Re(q_j) the real pressure coefficient.  This is precisely the
//! paper's first-order modal transformation (§7.1.2) and its exact-step
//! update (§7.2.2).
//!
//! Modal damping ζ_j maps a frequency-dependent reverberation time RT60(f)
//! (the room barriers' absorption, §5.2) to ζ = 6.9 / (ω RT60).

use super::complex::Complex64;
use super::modal::ComplexModalMode;
use super::vector::Vector3;
use std::f64::consts::PI;

pub struct AirRoomDesign {
    /// Room interior dimensions (x, y, z) [m].
    pub room_dimensions: Vector3,
    /// Speed of sound c [m/s].
    pub speed_of_sound: f64,
    /// Air density ρ [kg/m³].
    pub air_density: f64,
    /// Highest modeled room-mode frequency [Hz].
    pub highest_modeled_frequency: f64,
    /// Effective position of the soundboard source.
    pub soundboard_position: Vector3,
    /// Position of the listener's ear.
    pub listener_position: Vector3,
}

pub struct AirRoom {
    /// Complex modal states (one per retained cavity mode).
    pub modal_states: Vec<ComplexModalMode>,
    /// ψ_j evaluated at the soundboard position.
    pub mode_shape_at_soundboard: Vec<f64>,
    /// ψ_j evaluated at the listener position.
    pub mode_shape_at_listener: Vec<f64>,
    /// g_j per unit of board volume velocity (left-eigenvector projection).
    source_gain_per_volume_velocity: Vec<Complex64>,
}

fn mode_shape_at_position(
    index_x: usize,
    index_y: usize,
    index_z: usize,
    position: Vector3,
    room_dimensions: Vector3,
) -> f64 {
    (index_x as f64 * PI * position.x / room_dimensions.x).cos()
        * (index_y as f64 * PI * position.y / room_dimensions.y).cos()
        * (index_z as f64 * PI * position.z / room_dimensions.z).cos()
}

impl AirRoom {
    pub fn new(design: &AirRoomDesign) -> Self {
        let room = design.room_dimensions;
        let room_volume = room.x * room.y * room.z;
        let maximum_wavenumber =
            2.0 * PI * design.highest_modeled_frequency / design.speed_of_sound;
        let maximum_index_x = (maximum_wavenumber * room.x / PI).ceil() as usize;
        let maximum_index_y = (maximum_wavenumber * room.y / PI).ceil() as usize;
        let maximum_index_z = (maximum_wavenumber * room.z / PI).ceil() as usize;

        let mut modal_states = Vec::new();
        let mut mode_shape_at_soundboard = Vec::new();
        let mut mode_shape_at_listener = Vec::new();
        let mut source_gain_per_volume_velocity = Vec::new();

        for index_x in 0..=maximum_index_x {
            for index_y in 0..=maximum_index_y {
                for index_z in 0..=maximum_index_z {
                    // Skip the mean-pressure (DC) mode: a closed room cannot
                    // sustain a static pressure from a dipole-like board.
                    if index_x + index_y + index_z == 0 {
                        continue;
                    }
                    let wavenumber_x = index_x as f64 * PI / room.x;
                    let wavenumber_y = index_y as f64 * PI / room.y;
                    let wavenumber_z = index_z as f64 * PI / room.z;
                    let wavenumber_squared = wavenumber_x * wavenumber_x
                        + wavenumber_y * wavenumber_y
                        + wavenumber_z * wavenumber_z;
                    let wavenumber = wavenumber_squared.sqrt();
                    if wavenumber > maximum_wavenumber {
                        continue;
                    }

                    let angular_frequency = design.speed_of_sound * wavenumber;
                    // Modal volume Λ_j = V · 2^(-number of nonzero indices).
                    let nonzero_index_count =
                        (index_x > 0) as usize + (index_y > 0) as usize + (index_z > 0) as usize;
                    let modal_volume = room_volume / (1usize << nonzero_index_count) as f64;

                    // Wall absorption (§5.2) -> modal damping via RT60(f).
                    let frequency = angular_frequency / (2.0 * PI);
                    let reverberation_time = 0.30 + 0.55 * (-frequency / 160.0).exp();
                    let damping_ratio =
                        (6.9 / (angular_frequency * reverberation_time)).clamp(1.0e-4, 0.5);

                    let shape_at_soundboard = mode_shape_at_position(
                        index_x,
                        index_y,
                        index_z,
                        design.soundboard_position,
                        room,
                    );
                    let shape_at_listener = mode_shape_at_position(
                        index_x,
                        index_y,
                        index_z,
                        design.listener_position,
                        room,
                    );

                    modal_states.push(ComplexModalMode::new(angular_frequency, damping_ratio));
                    mode_shape_at_soundboard.push(shape_at_soundboard);
                    mode_shape_at_listener.push(shape_at_listener);

                    // Left-eigenvector projection factor 1 / (2 + 2 ζ λ / ω).
                    let complex_eigenvalue = Complex64::new(
                        -damping_ratio * angular_frequency,
                        angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt(),
                    );
                    let projection_denominator = Complex64::new(2.0, 0.0)
                        + complex_eigenvalue * (2.0 * damping_ratio / angular_frequency);
                    let pressure_source_scale =
                        design.air_density * design.speed_of_sound * design.speed_of_sound
                            / modal_volume
                            * shape_at_soundboard;
                    source_gain_per_volume_velocity.push(
                        Complex64::from_real(pressure_source_scale)
                            * projection_denominator.inverse(),
                    );
                }
            }
        }

        AirRoom {
            modal_states,
            mode_shape_at_soundboard,
            mode_shape_at_listener,
            source_gain_per_volume_velocity,
        }
    }

    pub fn mode_count(&self) -> usize {
        self.modal_states.len()
    }

    /// One exact modal step of the air (paper §7.2.2) driven by the
    /// soundboard's volume velocity (§6.3 velocity continuity).
    pub fn advance(&mut self, soundboard_volume_velocity: f64, step_size: f64) {
        for mode_index in 0..self.modal_states.len() {
            let source_term =
                self.source_gain_per_volume_velocity[mode_index] * soundboard_volume_velocity;
            self.modal_states[mode_index].advance(source_term, step_size);
        }
    }

    /// Acoustic pressure at the soundboard (feeds back on the board).
    pub fn pressure_at_soundboard(&self) -> f64 {
        let mut pressure = 0.0;
        for mode_index in 0..self.modal_states.len() {
            pressure += 2.0
                * self.modal_states[mode_index].state.real_part
                * self.mode_shape_at_soundboard[mode_index];
        }
        pressure
    }

    /// Acoustic pressure at the listener — the paper's digital audio signal.
    pub fn pressure_at_listener(&self) -> f64 {
        let mut pressure = 0.0;
        for mode_index in 0..self.modal_states.len() {
            pressure += 2.0
                * self.modal_states[mode_index].state.real_part
                * self.mode_shape_at_listener[mode_index];
        }
        pressure
    }

    pub fn reset(&mut self) {
        for mode in &mut self.modal_states {
            mode.state = Complex64::ZERO;
        }
    }
}
